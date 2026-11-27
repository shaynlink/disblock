//! Execution history, written to Postgres in batches so a busy bot costs one
//! insert per batch instead of one per event.

use std::time::Duration;

use sqlx::PgPool;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use uuid::Uuid;

const CHANNEL_CAPACITY: usize = 10_000;
const BATCH_SIZE: usize = 500;
/// Pause after each flush, so bursts are grouped into a single insert.
const COALESCE_DELAY: Duration = Duration::from_millis(250);

#[derive(Debug)]
pub struct Record {
    pub bot_id: Uuid,
    pub trigger: String,
    pub error: Option<String>,
    pub duration: Duration,
}

#[derive(Clone)]
pub struct ExecutionLog {
    tx: mpsc::Sender<Record>,
}

impl ExecutionLog {
    /// Start the writer. It stops once every `ExecutionLog` clone is dropped
    /// and the remaining records are flushed.
    pub fn spawn(db: PgPool) -> (Self, JoinHandle<()>) {
        let (tx, rx) = mpsc::channel(CHANNEL_CAPACITY);
        (Self { tx }, tokio::spawn(write(db, rx)))
    }

    pub fn record(&self, record: Record) {
        report_metrics(&record);
        if let Err(error) = self.tx.try_send(record) {
            tracing::warn!(%error, "execution log is full, dropping a record");
        }
    }
}

/// Trigger kind, without the user-chosen names, so metrics stay low cardinality.
fn trigger_kind(trigger: &str) -> &'static str {
    match trigger {
        t if t.starts_with('/') => "slash_command",
        t if t.starts_with("message") => "message",
        t if t.starts_with("member join") => "member_join",
        t if t.starts_with("member leave") => "member_leave",
        t if t.starts_with("reaction") => "reaction",
        t if t.starts_with("button") => "button",
        t if t.starts_with("select") => "select_menu",
        t if t.starts_with("every") || t.starts_with("schedule") => "schedule",
        _ => "other",
    }
}

fn report_metrics(record: &Record) {
    let kind = trigger_kind(&record.trigger);
    let outcome = if record.error.is_none() {
        "success"
    } else {
        "error"
    };
    sentry::metrics::counter("bot.executions", 1)
        .attribute("trigger.kind", kind)
        .attribute("outcome", outcome)
        .capture();
    sentry::metrics::distribution(
        "bot.execution.duration",
        record.duration.as_secs_f64() * 1000.0,
    )
    .unit(sentry::protocol::Unit::Millisecond)
    .attribute("trigger.kind", kind)
    .attribute("outcome", outcome)
    .capture();
}

async fn write(db: PgPool, mut rx: mpsc::Receiver<Record>) {
    let mut batch = Vec::with_capacity(BATCH_SIZE);
    while rx.recv_many(&mut batch, BATCH_SIZE).await > 0 {
        if let Err(error) = flush(&db, &batch).await {
            tracing::error!(%error, count = batch.len(), "failed to write executions");
        }
        batch.clear();
        tokio::time::sleep(COALESCE_DELAY).await;
    }
}

async fn flush(db: &PgPool, batch: &[Record]) -> Result<(), sqlx::Error> {
    let mut bot_ids = Vec::with_capacity(batch.len());
    let mut triggers = Vec::with_capacity(batch.len());
    let mut successes = Vec::with_capacity(batch.len());
    let mut errors = Vec::with_capacity(batch.len());
    let mut durations = Vec::with_capacity(batch.len());
    for record in batch {
        bot_ids.push(record.bot_id);
        triggers.push(record.trigger.clone());
        successes.push(record.error.is_none());
        errors.push(record.error.clone());
        durations.push(i32::try_from(record.duration.as_millis()).unwrap_or(i32::MAX));
    }

    // Records of a bot deleted in the meantime are skipped by the join.
    sqlx::query!(
        r#"INSERT INTO executions (bot_id, trigger, success, error, duration_ms)
           SELECT r.bot_id, r.trigger, r.success, r.error, r.duration_ms
           FROM UNNEST($1::uuid[], $2::text[], $3::bool[], $4::text[], $5::int4[])
                AS r (bot_id, trigger, success, error, duration_ms)
           JOIN bots b ON b.id = r.bot_id"#,
        &bot_ids,
        &triggers,
        &successes,
        &errors as &[Option<String>],
        &durations,
    )
    .execute(db)
    .await?;
    Ok(())
}
