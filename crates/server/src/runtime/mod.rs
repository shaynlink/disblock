//! Hosts the bots: one gateway connection set per enabled bot, all in this
//! process, with programs swapped in place when their owner saves them.

mod commands;
mod dispatch;
pub mod log;
mod variables;

use std::collections::HashMap;
use std::sync::Arc;

use disblock_program::{CompileError, CompiledProgram, GatewayNeeds, Program};
use serde::Serialize;
use sqlx::PgPool;
use sqlx::types::Json;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::{Mutex, watch};
use tokio::task::{JoinHandle, JoinSet};
use tokio_util::sync::CancellationToken;

use twilight_gateway::queue::InMemoryQueue;
use twilight_gateway::{CloseFrame, ConfigBuilder, EventTypeFlags, Intents, Shard, StreamExt as _};
use twilight_http::Client;
use twilight_http::error::ErrorType;
use twilight_model::channel::message::{AllowedMentions, MentionType};
use twilight_model::gateway::event::Event;
use twilight_model::id::Id;
use twilight_model::id::marker::ApplicationMarker;
use twilight_model::oauth::ApplicationFlags;
use uuid::Uuid;

use crate::crypto::{CryptoError, TokenCipher};
use log::ExecutionLog;

/// Events the runtime reacts to. Everything else is skipped before parsing.
const EVENTS: EventTypeFlags = EventTypeFlags::READY
    .union(EventTypeFlags::INTERACTION_CREATE)
    .union(EventTypeFlags::MEMBER_ADD)
    .union(EventTypeFlags::MESSAGE_CREATE);

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("bot not found")]
    NotFound,
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error(transparent)]
    Crypto(#[from] CryptoError),
    #[error("the saved program is not valid anymore: {0}")]
    Program(#[from] CompileError),
    #[error("{0}")]
    Discord(String),
}

/// What a running bot shares with its event handlers.
pub struct BotContext {
    pub id: Uuid,
    pub application_id: Id<ApplicationMarker>,
    pub http: Client,
    db: PgPool,
    log: ExecutionLog,
}

struct RunningBot {
    program: watch::Sender<Arc<CompiledProgram>>,
    needs: GatewayNeeds,
    cancel: CancellationToken,
    task: JoinHandle<()>,
}

struct Inner {
    db: PgPool,
    cipher: TokenCipher,
    log: ExecutionLog,
    bots: Mutex<HashMap<Uuid, RunningBot>>,
}

#[derive(Clone)]
pub struct Runtime {
    inner: Arc<Inner>,
}

/// Privileged gateway intents enabled on a Discord application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct BotIntents {
    pub members: bool,
    pub message_content: bool,
}

#[must_use]
pub fn intents_from_flags(flags: Option<ApplicationFlags>) -> BotIntents {
    let flags = flags.unwrap_or_else(ApplicationFlags::empty);
    BotIntents {
        members: flags.intersects(
            ApplicationFlags::GATEWAY_GUILD_MEMBERS
                | ApplicationFlags::GATEWAY_GUILD_MEMBERS_LIMITED,
        ),
        message_content: flags.intersects(
            ApplicationFlags::GATEWAY_MESSAGE_CONTENT
                | ApplicationFlags::GATEWAY_MESSAGE_CONTENT_LIMITED,
        ),
    }
}

/// A bot's credentials and program, loaded from the database.
struct Loaded {
    application_id: Id<ApplicationMarker>,
    token: String,
    program: CompiledProgram,
}

impl Runtime {
    pub fn new(db: PgPool, cipher: TokenCipher, log: ExecutionLog) -> Self {
        Self {
            inner: Arc::new(Inner {
                db,
                cipher,
                log,
                bots: Mutex::new(HashMap::new()),
            }),
        }
    }

    /// Start every enabled bot, after a deploy or a restart.
    pub async fn start_enabled(&self) -> Result<(), RuntimeError> {
        let ids = sqlx::query_scalar!("SELECT id FROM bots WHERE enabled ORDER BY created_at")
            .fetch_all(&self.inner.db)
            .await?;
        tracing::info!(count = ids.len(), "starting bots");
        for id in ids {
            if let Err(error) = self.start(id, false).await {
                tracing::warn!(bot = %id, %error, "bot failed to start");
                self.set_status(id, "error", Some(&error.to_string())).await;
            }
        }
        Ok(())
    }

    /// (Re)start a bot. With `register`, its slash commands are pushed to
    /// Discord first, which is needed when they may have changed.
    pub async fn start(&self, id: Uuid, register: bool) -> Result<(), RuntimeError> {
        let loaded = self.load(id).await?;
        let http = bot_client(&loaded.token);
        if register {
            register_commands(&http, loaded.application_id, &loaded.program).await?;
        }

        let mut bots = self.inner.bots.lock().await;
        if let Some(previous) = bots.remove(&id) {
            stop_bot(previous).await;
        }

        let needs = loaded.program.gateway_needs();
        let (program_tx, program_rx) = watch::channel(Arc::new(loaded.program));
        let cancel = CancellationToken::new();
        let context = Arc::new(BotContext {
            id,
            application_id: loaded.application_id,
            http,
            db: self.inner.db.clone(),
            log: self.inner.log.clone(),
        });
        let task = tokio::spawn(run_bot(
            self.clone(),
            context,
            loaded.token,
            needs,
            program_rx,
            cancel.clone(),
        ));
        bots.insert(
            id,
            RunningBot {
                program: program_tx,
                needs,
                cancel,
                task,
            },
        );
        Ok(())
    }

    pub async fn stop(&self, id: Uuid) {
        let running = self.inner.bots.lock().await.remove(&id);
        if let Some(running) = running {
            stop_bot(running).await;
        }
    }

    /// Apply a freshly saved program: register its commands, then swap it into
    /// the running bot, reconnecting only when it needs other gateway intents.
    pub async fn apply_program(&self, id: Uuid, enabled: bool) -> Result<(), RuntimeError> {
        if !enabled {
            let loaded = self.load(id).await?;
            return register_commands(
                &bot_client(&loaded.token),
                loaded.application_id,
                &loaded.program,
            )
            .await;
        }

        let loaded = self.load(id).await?;
        let bots = self.inner.bots.lock().await;
        let swappable = bots.get(&id).is_some_and(|running| {
            running.needs == loaded.program.gateway_needs() && !running.task.is_finished()
        });
        if swappable {
            register_commands(
                &bot_client(&loaded.token),
                loaded.application_id,
                &loaded.program,
            )
            .await?;
            if let Some(running) = bots.get(&id) {
                running.program.send_replace(Arc::new(loaded.program));
            }
            return Ok(());
        }
        drop(bots);
        self.start(id, true).await
    }

    /// Remove the bot's slash commands from Discord, once it leaves Disblock.
    pub async fn clear_commands(&self, id: Uuid) -> Result<(), RuntimeError> {
        let loaded = self.load(id).await?;
        bot_client(&loaded.token)
            .interaction(loaded.application_id)
            .set_global_commands(&[])
            .await
            .map(drop)
            .map_err(|error| RuntimeError::Discord(error.to_string()))
    }

    pub async fn shutdown(&self) {
        let running: Vec<RunningBot> = self
            .inner
            .bots
            .lock()
            .await
            .drain()
            .map(|(_, bot)| bot)
            .collect();
        tracing::info!(count = running.len(), "stopping bots");
        for bot in &running {
            bot.cancel.cancel();
        }
        for bot in running {
            if let Err(error) = bot.task.await {
                tracing::warn!(%error, "bot task panicked");
            }
        }
    }

    async fn load(&self, id: Uuid) -> Result<Loaded, RuntimeError> {
        let row = sqlx::query!(
            r#"SELECT application_id, token_nonce, token_ciphertext, program AS "program: Json<Program>"
               FROM bots WHERE id = $1"#,
            id,
        )
        .fetch_optional(&self.inner.db)
        .await?
        .ok_or(RuntimeError::NotFound)?;

        let application_id = row
            .application_id
            .parse()
            .ok()
            .and_then(Id::new_checked)
            .ok_or_else(|| RuntimeError::Discord("stored application id is invalid".to_owned()))?;
        Ok(Loaded {
            application_id,
            token: self
                .inner
                .cipher
                .open(id, &row.token_nonce, &row.token_ciphertext)?,
            program: row.program.0.compile()?,
        })
    }

    async fn load_token(&self, id: Uuid) -> Result<String, RuntimeError> {
        let row = sqlx::query!(
            "SELECT token_nonce, token_ciphertext FROM bots WHERE id = $1",
            id,
        )
        .fetch_optional(&self.inner.db)
        .await?
        .ok_or(RuntimeError::NotFound)?;

        Ok(self
            .inner
            .cipher
            .open(id, &row.token_nonce, &row.token_ciphertext)?)
    }

    /// Ask Discord which privileged gateway intents are currently enabled on
    /// the bot's application.
    pub async fn fetch_intents(&self, id: Uuid) -> Result<BotIntents, RuntimeError> {
        let token = self.load_token(id).await?;
        let http = bot_client(&token);
        let response = http.current_user_application().await.map_err(|error| {
            RuntimeError::Discord(if is_unauthorized(&error) {
                "Discord rejected the bot token: it was probably reset".to_owned()
            } else {
                format!("Discord did not answer when checking intents: {error}")
            })
        })?;
        let application = response
            .model()
            .await
            .map_err(|error| RuntimeError::Discord(error.to_string()))?;
        Ok(intents_from_flags(application.flags))
    }

    async fn set_status(&self, id: Uuid, status: &str, detail: Option<&str>) {
        let result = sqlx::query!(
            "UPDATE bots SET status = $2, status_detail = $3 WHERE id = $1",
            id,
            status,
            detail,
        )
        .execute(&self.inner.db)
        .await;
        if let Err(error) = result {
            tracing::error!(bot = %id, %error, "failed to save bot status");
        }
    }
}

/// Whether Discord refused the bot token, either now (401) or earlier in the
/// client's life.
pub fn is_unauthorized(error: &twilight_http::Error) -> bool {
    match error.kind() {
        ErrorType::Unauthorized => true,
        ErrorType::Response { status, .. } => status.get() == 401,
        _ => false,
    }
}

/// HTTP client for a bot. User content can only ping users, never
/// `@everyone` or roles.
pub fn bot_client(token: &str) -> Client {
    Client::builder()
        .token(token.to_owned())
        .default_allowed_mentions(AllowedMentions {
            parse: vec![MentionType::Users],
            ..AllowedMentions::default()
        })
        .build()
}

async fn register_commands(
    http: &Client,
    application_id: Id<ApplicationMarker>,
    program: &CompiledProgram,
) -> Result<(), RuntimeError> {
    let commands = commands::build(program.commands());
    http.interaction(application_id)
        .set_global_commands(&commands)
        .await
        .map(drop)
        .map_err(|error| {
            RuntimeError::Discord(if is_unauthorized(&error) {
                "Discord rejected the bot token: it was probably reset".to_owned()
            } else {
                format!("Discord refused the slash commands: {error}")
            })
        })
}

async fn stop_bot(running: RunningBot) {
    running.cancel.cancel();
    if let Err(error) = running.task.await {
        tracing::warn!(%error, "bot task panicked");
    }
}

fn intents(needs: GatewayNeeds) -> Intents {
    let mut intents = Intents::GUILDS;
    if needs.members {
        intents |= Intents::GUILD_MEMBERS;
    }
    if needs.messages {
        intents |= Intents::GUILD_MESSAGES | Intents::MESSAGE_CONTENT;
    }
    intents
}

async fn run_bot(
    runtime: Runtime,
    bot: Arc<BotContext>,
    token: String,
    needs: GatewayNeeds,
    program: watch::Receiver<Arc<CompiledProgram>>,
    cancel: CancellationToken,
) {
    runtime.set_status(bot.id, "starting", None).await;

    let info = match bot.http.gateway().authed().await {
        Ok(response) => response.model().await.map_err(|error| error.to_string()),
        Err(error) => Err(error.to_string()),
    };
    let info = match info {
        Ok(info) => info,
        Err(error) => {
            tracing::warn!(bot = %bot.id, %error, "could not fetch gateway information");
            let detail = "Discord rejected the bot token, or is unreachable. Check the token and restart the bot.";
            runtime.set_status(bot.id, "error", Some(detail)).await;
            return;
        }
    };

    // Shards share one queue so identifies respect the bot's session limits.
    let limit = info.session_start_limit;
    let queue = InMemoryQueue::new(
        limit.max_concurrency,
        limit.remaining.min(limit.total),
        Duration::from_millis(limit.reset_after),
        limit.total,
    );
    let config = ConfigBuilder::new(token, intents(needs))
        .queue(queue)
        .build();

    let mut shard_tasks = JoinSet::new();
    for shard_id in twilight_gateway::bucket(0, 1, info.shards) {
        let shard = Shard::with_config(shard_id, config.clone());
        shard_tasks.spawn(run_shard(
            runtime.clone(),
            bot.clone(),
            shard,
            program.clone(),
            cancel.clone(),
        ));
    }

    let mut failure = None;
    while let Some(result) = shard_tasks.join_next().await {
        let reason = match result {
            Ok(reason) => reason,
            Err(error) => Some(format!("internal error: {error}")),
        };
        if let Some(reason) = reason {
            failure.get_or_insert(reason);
            cancel.cancel();
        }
    }

    match failure {
        Some(reason) => runtime.set_status(bot.id, "error", Some(&reason)).await,
        None => runtime.set_status(bot.id, "stopped", None).await,
    }
}

/// Runs one shard until it is cancelled (`None`) or closed by Discord for
/// good (`Some(reason)`).
async fn run_shard(
    runtime: Runtime,
    bot: Arc<BotContext>,
    mut shard: Shard,
    program: watch::Receiver<Arc<CompiledProgram>>,
    cancel: CancellationToken,
) -> Option<String> {
    let mut last_close = None;
    loop {
        let item = tokio::select! {
            () = cancel.cancelled() => {
                shard.close(CloseFrame::NORMAL);
                return None;
            }
            item = shard.next_event(EVENTS) => item,
        };

        match item {
            None => return Some(close_reason(last_close)),
            Some(Ok(Event::Ready(_))) => {
                tracing::info!(bot = %bot.id, shard = %shard.id(), "shard ready");
                runtime.set_status(bot.id, "online", None).await;
            }
            Some(Ok(Event::GatewayClose(frame))) => last_close = frame.map(|frame| frame.code),
            Some(Ok(event)) => {
                let program = program.borrow().clone();
                tokio::spawn(dispatch::handle(bot.clone(), program, event));
            }
            Some(Err(error)) => tracing::warn!(bot = %bot.id, %error, "gateway error"),
        }
    }
}

fn close_reason(code: Option<u16>) -> String {
    match code {
        Some(4004) => "Discord rejected the bot token: it was probably reset. Add the bot again with its new token.".to_owned(),
        Some(4014) => "A privileged intent is disabled. In the Discord developer portal, open Bot and enable \
                       \"Server Members Intent\" (member join) and \"Message Content Intent\" (message triggers)."
            .to_owned(),
        Some(code) => format!("Discord closed the connection (code {code})."),
        None => "Discord closed the connection.".to_owned(),
    }
}
