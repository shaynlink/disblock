use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};

use thiserror::Error;

use crate::compile::{
    Calc, CompiledButton, CompiledComponentRow, CompiledEmbed, CompiledHandler, CompiledMessage,
    CompiledProgram, MAX_BUTTON_LABEL_LEN, MAX_EMBED_AUTHOR_NAME_LEN, MAX_EMBED_DESCRIPTION_LEN,
    MAX_EMBED_FIELD_NAME_LEN, MAX_EMBED_FIELD_VALUE_LEN, MAX_EMBED_FOOTER_TEXT_LEN,
    MAX_EMBED_TITLE_LEN, MAX_EMBED_TOTAL_LEN, MAX_SELECT_OPTION_DESCRIPTION_LEN,
    MAX_SELECT_OPTION_LABEL_LEN, MAX_SELECT_PLACEHOLDER_LEN, MAX_WAIT_SECONDS, Step,
};
use crate::model::{ButtonStyle, CompareOp, MathOp, ScheduleUnit, Scope};
use crate::template::{Context, MAX_MESSAGE_LEN, render_max};
use crate::value::Value;

/// Effects one run may perform, so a handler cannot flood a server.
const MAX_EFFECTS: usize = 50;
/// Total waiting in one run. Discord accepts answers to a slash command for
/// 15 minutes.
const MAX_TOTAL_WAIT_SECONDS: u32 = 2 * MAX_WAIT_SECONDS;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmojiRef {
    Unicode(String),
    Custom {
        name: String,
        id: u64,
        animated: bool,
    },
}

/// Emoji on a gateway reaction event, without allocating.
#[derive(Debug, Clone, Copy)]
pub enum EventEmoji<'a> {
    Unicode(&'a str),
    Custom { id: u64 },
}

/// A Discord event, reduced to what programs can react to.
#[derive(Debug, Clone, Copy)]
pub enum EventKind<'a> {
    SlashCommand {
        name: &'a str,
    },
    MemberJoin,
    MemberLeave,
    Message {
        content: &'a str,
    },
    ReactionAdd {
        emoji: EventEmoji<'a>,
        message_id: u64,
    },
    ReactionRemove {
        emoji: EventEmoji<'a>,
        message_id: u64,
    },
    ButtonClick {
        id: &'a str,
    },
    SelectMenu {
        id: &'a str,
    },
    Schedule {
        interval: u32,
        unit: ScheduleUnit,
    },
}

#[derive(Debug, Clone, Copy)]
pub struct Event<'a> {
    pub kind: EventKind<'a>,
    pub context: Context<'a>,
}

/// Member and server variables, loaded before a run.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SavedVariables {
    pub member: HashMap<String, Value>,
    pub server: HashMap<String, Value>,
}

/// Source of randomness, injected so runs can be replayed in tests.
pub trait Random {
    /// A number between `min` and `max`, both included, with `min <= max`.
    fn between(&mut self, min: i64, max: i64) -> i64;
}

/// A rich message ready to send to Discord, with templates rendered.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MessagePayload {
    pub content: Option<String>,
    pub embeds: Vec<RenderedEmbed>,
    pub components: Vec<RenderedComponentRow>,
}

impl MessagePayload {
    #[must_use]
    pub fn text(content: impl Into<String>) -> Self {
        Self {
            content: Some(content.into()),
            embeds: Vec::new(),
            components: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RenderedEmbed {
    pub title: Option<String>,
    pub url: Option<String>,
    pub description: Option<String>,
    pub color: Option<u32>,
    pub author: Option<RenderedEmbedAuthor>,
    pub footer: Option<RenderedEmbedFooter>,
    pub thumbnail_url: Option<String>,
    pub image_url: Option<String>,
    pub fields: Vec<RenderedEmbedField>,
    pub timestamp: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedEmbedAuthor {
    pub name: String,
    pub url: Option<String>,
    pub icon_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedEmbedFooter {
    pub text: String,
    pub icon_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedEmbedField {
    pub name: String,
    pub value: String,
    pub inline: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderedComponentRow {
    Buttons(Vec<RenderedButton>),
    SelectMenu {
        id: String,
        placeholder: Option<String>,
        min_values: u8,
        max_values: u8,
        disabled: bool,
        options: Vec<RenderedSelectOption>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderedButton {
    Action {
        id: String,
        label: Option<String>,
        emoji: Option<EmojiRef>,
        style: ButtonStyle,
        disabled: bool,
    },
    Link {
        url: String,
        label: Option<String>,
        emoji: Option<EmojiRef>,
        disabled: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedSelectOption {
    pub label: String,
    pub value: String,
    pub description: Option<String>,
    pub emoji: Option<EmojiRef>,
    pub default: bool,
}

/// A side effect for the runtime to perform, with templates already rendered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    Reply {
        message: MessagePayload,
        ephemeral: bool,
    },
    SendMessage {
        channel_id: u64,
        message: MessagePayload,
    },
    UpdateMessage {
        message: MessagePayload,
    },
    AddRole {
        role_id: u64,
    },
    RemoveRole {
        role_id: u64,
    },
    Timeout {
        member_id: u64,
        seconds: u32,
        reason: Option<String>,
    },
    Kick {
        member_id: u64,
        reason: Option<String>,
    },
    Ban {
        member_id: u64,
        reason: Option<String>,
    },
    DeleteMessage,
    DirectMessage {
        member_id: u64,
        content: String,
    },
    React {
        emoji: EmojiRef,
    },
    Wait {
        seconds: u32,
    },
}

/// A member or server variable to save once the run is planned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableWrite {
    pub scope: Scope,
    pub name: String,
    pub value: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RunError {
    #[error("`{0}` is not a number")]
    NotANumber(String),
    #[error("division by zero")]
    DivisionByZero,
    #[error("a number got too big")]
    Overflow,
    #[error("a run can perform at most {MAX_EFFECTS} actions")]
    TooManyEffects,
    #[error("a run can wait at most {MAX_TOTAL_WAIT_SECONDS} seconds in total")]
    TooMuchWaiting,
    #[error("a message ended up empty after replacing placeholders")]
    EmptyMessage,
    #[error("an embed ended up empty after replacing placeholders")]
    EmptyEmbed,
    #[error("embeds exceeded {MAX_EMBED_TOTAL_LEN} characters after replacing placeholders")]
    EmbedTooLarge,
    #[error("a button ended up with neither a label nor an emoji")]
    EmptyButton,
    #[error("{0} ended up empty after replacing placeholders")]
    EmptyField(&'static str),
    #[error("the member ID is invalid after replacing placeholders")]
    InvalidMemberId,
}

/// One handler matched by an event: what to do, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Execution {
    /// Human readable trigger, such as `/hello`.
    pub trigger: String,
    /// Effects planned before the run ended or failed.
    pub effects: Vec<Effect>,
    pub writes: Vec<VariableWrite>,
    /// Why the run stopped early, after `effects`.
    pub error: Option<RunError>,
}

impl CompiledProgram {
    /// Plan every handler matched by `event`, in program order.
    #[must_use]
    pub fn handle(
        &self,
        event: &Event<'_>,
        saved: &SavedVariables,
        random: &mut impl Random,
    ) -> Vec<Execution> {
        let matched: Vec<usize> = match event.kind {
            EventKind::SlashCommand { name } => self.slash.get(name).copied().into_iter().collect(),
            EventKind::MemberJoin => self.member_join.clone(),
            EventKind::MemberLeave => self.member_leave.clone(),
            EventKind::Message { content } => {
                let lowered = content.to_lowercase();
                self.messages
                    .iter()
                    .filter(|matcher| {
                        let haystack = if matcher.case_sensitive {
                            content
                        } else {
                            &lowered
                        };
                        haystack.contains(&matcher.pattern)
                    })
                    .map(|matcher| matcher.handler)
                    .collect()
            }
            EventKind::ReactionAdd { emoji, message_id } => {
                matching_reactions(&self.reaction_add, emoji, message_id)
            }
            EventKind::ReactionRemove { emoji, message_id } => {
                matching_reactions(&self.reaction_remove, emoji, message_id)
            }
            EventKind::ButtonClick { id } => self.buttons.get(id).copied().into_iter().collect(),
            EventKind::SelectMenu { id } => self.selects.get(id).copied().into_iter().collect(),
            EventKind::Schedule { interval, unit } => self
                .schedules
                .iter()
                .filter(|entry| entry.spec.interval == interval && entry.spec.unit == unit)
                .map(|entry| entry.handler)
                .collect(),
        };

        matched
            .into_iter()
            .map(|index| plan(&self.handlers[index], &event.context, saved, random))
            .collect()
    }
}

fn matching_reactions(
    matchers: &[crate::compile::ReactionMatcher],
    emoji: EventEmoji<'_>,
    message_id: u64,
) -> Vec<usize> {
    matchers
        .iter()
        .filter(|matcher| {
            matcher.message_id.is_none_or(|want| want == message_id)
                && emoji_matches(matcher.emoji.as_ref(), emoji)
        })
        .map(|matcher| matcher.handler)
        .collect()
}

fn emoji_matches(expected: Option<&EmojiRef>, actual: EventEmoji<'_>) -> bool {
    match (expected, actual) {
        (None, _) => true,
        (Some(EmojiRef::Unicode(want)), EventEmoji::Unicode(got)) => want == got,
        (Some(EmojiRef::Custom { id: want, .. }), EventEmoji::Custom { id: got }) => *want == got,
        _ => false,
    }
}

fn plan(
    handler: &CompiledHandler,
    context: &Context<'_>,
    saved: &SavedVariables,
    random: &mut impl Random,
) -> Execution {
    let mut runner = Runner {
        context,
        saved,
        random,
        locals: HashMap::new(),
        changed: BTreeMap::new(),
        roles: context.roles.to_vec(),
        effects: Vec::new(),
        waited: 0,
    };
    let error = runner.block(&handler.steps).err();
    let writes = runner
        .changed
        .into_iter()
        .map(|((scope, name), value)| VariableWrite { scope, name, value })
        .collect();

    Execution {
        trigger: handler.label.clone(),
        effects: runner.effects,
        writes,
        error,
    }
}

enum Flow {
    Continue,
    Stop,
}

struct Runner<'a, R> {
    context: &'a Context<'a>,
    saved: &'a SavedVariables,
    random: &'a mut R,
    locals: HashMap<String, Value>,
    /// Member and server variables changed by this run.
    changed: BTreeMap<(Scope, String), Value>,
    /// The member's roles, including the ones this run adds or removes.
    roles: Vec<u64>,
    effects: Vec<Effect>,
    waited: u32,
}

impl<R: Random> Runner<'_, R> {
    fn block(&mut self, steps: &[Step]) -> Result<Flow, RunError> {
        for step in steps {
            if let Flow::Stop = self.step(step)? {
                return Ok(Flow::Stop);
            }
        }
        Ok(Flow::Continue)
    }

    fn step(&mut self, step: &Step) -> Result<Flow, RunError> {
        match step {
            Step::Reply { message, ephemeral } => {
                let message = self.message(message, true)?;
                self.emit(Effect::Reply {
                    message,
                    ephemeral: *ephemeral,
                })?;
            }
            Step::SendMessage {
                channel_id,
                message,
            } => {
                let message = self.message(message, true)?;
                self.emit(Effect::SendMessage {
                    channel_id: *channel_id,
                    message,
                })?;
            }
            Step::UpdateMessage { message } => {
                let message = self.message(message, false)?;
                self.emit(Effect::UpdateMessage { message })?;
            }
            Step::AddRole(role_id) => {
                self.emit(Effect::AddRole { role_id: *role_id })?;
                if !self.roles.contains(role_id) {
                    self.roles.push(*role_id);
                }
            }
            Step::RemoveRole(role_id) => {
                self.emit(Effect::RemoveRole { role_id: *role_id })?;
                self.roles.retain(|role| role != role_id);
            }
            Step::Timeout {
                member_id,
                seconds,
                reason,
            } => self.emit(Effect::Timeout {
                member_id: self.member_id(member_id)?,
                seconds: *seconds,
                reason: self.render_optional(reason.as_deref(), 512),
            })?,
            Step::Kick { member_id, reason } => self.emit(Effect::Kick {
                member_id: self.member_id(member_id)?,
                reason: self.render_optional(reason.as_deref(), 512),
            })?,
            Step::Ban { member_id, reason } => self.emit(Effect::Ban {
                member_id: self.member_id(member_id)?,
                reason: self.render_optional(reason.as_deref(), 512),
            })?,
            Step::DeleteMessage => self.emit(Effect::DeleteMessage)?,
            Step::DirectMessage { member_id, content } => self.emit(Effect::DirectMessage {
                member_id: self.member_id(member_id)?,
                content: self.render_required(content, MAX_MESSAGE_LEN, "direct message")?,
            })?,
            Step::React(emoji) => self.emit(Effect::React {
                emoji: emoji.clone(),
            })?,
            Step::If {
                condition,
                then_steps,
                else_steps,
            } => {
                let branch = if self.eval(condition)?.is_truthy() {
                    then_steps
                } else {
                    else_steps
                };
                return self.block(branch);
            }
            Step::Set { scope, name, value } => {
                let value = self.eval(value)?;
                self.set(*scope, name, value);
            }
            Step::Change { scope, name, by } => {
                let current = number(&self.get(*scope, name))?;
                let by = number(&self.eval(by)?)?;
                let value = current.checked_add(by).ok_or(RunError::Overflow)?;
                self.set(*scope, name, Value::Number(value));
            }
            Step::Wait(seconds) => {
                self.waited += seconds;
                if self.waited > MAX_TOTAL_WAIT_SECONDS {
                    return Err(RunError::TooMuchWaiting);
                }
                self.emit(Effect::Wait { seconds: *seconds })?;
            }
            Step::Stop => return Ok(Flow::Stop),
        }
        Ok(Flow::Continue)
    }

    fn eval(&mut self, calc: &Calc) -> Result<Value, RunError> {
        Ok(match calc {
            Calc::Text(template) => Value::Text(self.render(template)),
            Calc::Number(value) => Value::Number(*value),
            Calc::Bool(value) => Value::Bool(*value),
            Calc::Variable(scope, name) => self.get(*scope, name),
            Calc::Option(name) => {
                Value::Text(self.context.option(name).unwrap_or_default().to_owned())
            }
            Calc::Selected => Value::Text(self.context.selected.unwrap_or_default().to_owned()),
            Calc::Random(min, max) => {
                let (a, b) = (number(&self.eval(min)?)?, number(&self.eval(max)?)?);
                Value::Number(self.random.between(a.min(b), a.max(b)))
            }
            Calc::Math(op, left, right) => {
                let (left, right) = (number(&self.eval(left)?)?, number(&self.eval(right)?)?);
                Value::Number(math(*op, left, right)?)
            }
            Calc::Compare(op, left, right) => {
                let (left, right) = (self.eval(left)?, self.eval(right)?);
                Value::Bool(compare(*op, &left, &right)?)
            }
            Calc::And(left, right) => {
                Value::Bool(self.eval(left)?.is_truthy() && self.eval(right)?.is_truthy())
            }
            Calc::Or(left, right) => {
                Value::Bool(self.eval(left)?.is_truthy() || self.eval(right)?.is_truthy())
            }
            Calc::Not(value) => Value::Bool(!self.eval(value)?.is_truthy()),
            Calc::HasRole(role_id) => Value::Bool(self.roles.contains(role_id)),
        })
    }

    fn emit(&mut self, effect: Effect) -> Result<(), RunError> {
        if self.effects.len() >= MAX_EFFECTS {
            return Err(RunError::TooManyEffects);
        }
        self.effects.push(effect);
        Ok(())
    }

    fn get(&self, scope: Scope, name: &str) -> Value {
        let value = match scope {
            Scope::Local => self.locals.get(name),
            Scope::Member | Scope::Server => {
                self.changed.get(&(scope, name.to_owned())).or_else(|| {
                    let saved = if scope == Scope::Member {
                        &self.saved.member
                    } else {
                        &self.saved.server
                    };
                    saved.get(name)
                })
            }
        };
        value.cloned().unwrap_or_default()
    }

    fn set(&mut self, scope: Scope, name: &str, value: Value) {
        match scope {
            Scope::Local => {
                self.locals.insert(name.to_owned(), value);
            }
            Scope::Member | Scope::Server => {
                self.changed.insert((scope, name.to_owned()), value);
            }
        }
    }

    fn render(&self, template: &str) -> String {
        self.render_max(template, MAX_MESSAGE_LEN)
    }

    fn render_max(&self, template: &str, max_chars: usize) -> String {
        render_max(template, max_chars, |key| {
            let variable = |scope| {
                key.split_once(':')
                    .map(|(_, name)| self.get(scope, name).to_string())
            };
            match key.split_once(':').map(|(prefix, _)| prefix) {
                Some("var") => variable(Scope::Local),
                Some("member") => variable(Scope::Member),
                Some("server") => variable(Scope::Server),
                _ => self.context.builtin(key),
            }
        })
    }

    fn render_optional(&self, template: Option<&str>, max_chars: usize) -> Option<String> {
        let rendered = self.render_max(template?, max_chars);
        (!rendered.trim().is_empty()).then_some(rendered)
    }

    fn render_required(
        &self,
        template: &str,
        max_chars: usize,
        field: &'static str,
    ) -> Result<String, RunError> {
        let rendered = self.render_max(template, max_chars);
        if rendered.trim().is_empty() {
            Err(RunError::EmptyField(field))
        } else {
            Ok(rendered)
        }
    }

    fn member_id(&self, template: &str) -> Result<u64, RunError> {
        let rendered = self.render_max(template, 100);
        parse_member_id(&rendered).ok_or(RunError::InvalidMemberId)
    }

    fn message(
        &self,
        compiled: &CompiledMessage,
        require_body: bool,
    ) -> Result<MessagePayload, RunError> {
        let content = self.render_optional(compiled.content.as_deref(), MAX_MESSAGE_LEN);
        let mut embeds = Vec::with_capacity(compiled.embeds.len());
        let mut total_chars = 0usize;
        for embed in &compiled.embeds {
            let (rendered, chars) = self.embed(embed)?;
            total_chars += chars;
            if total_chars > MAX_EMBED_TOTAL_LEN {
                return Err(RunError::EmbedTooLarge);
            }
            embeds.push(rendered);
        }

        let mut components = Vec::with_capacity(compiled.components.len());
        for row in &compiled.components {
            components.push(self.component_row(row)?);
        }

        if require_body && content.is_none() && embeds.is_empty() {
            return Err(RunError::EmptyMessage);
        }
        if !require_body && content.is_none() && embeds.is_empty() && components.is_empty() {
            return Err(RunError::EmptyMessage);
        }

        Ok(MessagePayload {
            content,
            embeds,
            components,
        })
    }

    fn embed(&self, compiled: &CompiledEmbed) -> Result<(RenderedEmbed, usize), RunError> {
        let title = self.render_optional(compiled.title.as_deref(), MAX_EMBED_TITLE_LEN);
        let description =
            self.render_optional(compiled.description.as_deref(), MAX_EMBED_DESCRIPTION_LEN);
        let author = compiled
            .author
            .as_ref()
            .map(|author| {
                Ok(RenderedEmbedAuthor {
                    name: self.render_required(
                        &author.name,
                        MAX_EMBED_AUTHOR_NAME_LEN,
                        "embed author",
                    )?,
                    url: author.url.clone(),
                    icon_url: author.icon_url.clone(),
                })
            })
            .transpose()?;
        let footer = compiled
            .footer
            .as_ref()
            .map(|footer| {
                Ok(RenderedEmbedFooter {
                    text: self.render_required(
                        &footer.text,
                        MAX_EMBED_FOOTER_TEXT_LEN,
                        "embed footer",
                    )?,
                    icon_url: footer.icon_url.clone(),
                })
            })
            .transpose()?;

        let mut fields = Vec::with_capacity(compiled.fields.len());
        for field in &compiled.fields {
            fields.push(RenderedEmbedField {
                name: self.render_required(
                    &field.name,
                    MAX_EMBED_FIELD_NAME_LEN,
                    "embed field name",
                )?,
                value: self.render_required(
                    &field.value,
                    MAX_EMBED_FIELD_VALUE_LEN,
                    "embed field value",
                )?,
                inline: field.inline,
            });
        }

        let has_content = title.is_some()
            || description.is_some()
            || author.is_some()
            || footer.is_some()
            || compiled.thumbnail_url.is_some()
            || compiled.image_url.is_some()
            || !fields.is_empty();
        if !has_content {
            return Err(RunError::EmptyEmbed);
        }

        let chars = title.as_deref().map_or(0, |s| s.chars().count())
            + description.as_deref().map_or(0, |s| s.chars().count())
            + author.as_ref().map_or(0, |a| a.name.chars().count())
            + footer.as_ref().map_or(0, |f| f.text.chars().count())
            + fields
                .iter()
                .map(|f| f.name.chars().count() + f.value.chars().count())
                .sum::<usize>();
        if chars > MAX_EMBED_TOTAL_LEN {
            return Err(RunError::EmbedTooLarge);
        }

        Ok((
            RenderedEmbed {
                title,
                url: compiled.url.clone(),
                description,
                color: compiled.color,
                author,
                footer,
                thumbnail_url: compiled.thumbnail_url.clone(),
                image_url: compiled.image_url.clone(),
                fields,
                timestamp: compiled.timestamp,
            },
            chars,
        ))
    }

    fn component_row(&self, row: &CompiledComponentRow) -> Result<RenderedComponentRow, RunError> {
        match row {
            CompiledComponentRow::Buttons(buttons) => {
                let mut rendered = Vec::with_capacity(buttons.len());
                for button in buttons {
                    rendered.push(match button {
                        CompiledButton::Action {
                            id,
                            label,
                            emoji,
                            style,
                            disabled,
                        } => {
                            let label =
                                self.render_optional(label.as_deref(), MAX_BUTTON_LABEL_LEN);
                            if label.is_none() && emoji.is_none() {
                                return Err(RunError::EmptyButton);
                            }
                            RenderedButton::Action {
                                id: id.clone(),
                                label,
                                emoji: emoji.clone(),
                                style: *style,
                                disabled: *disabled,
                            }
                        }
                        CompiledButton::Link {
                            url,
                            label,
                            emoji,
                            disabled,
                        } => {
                            let label =
                                self.render_optional(label.as_deref(), MAX_BUTTON_LABEL_LEN);
                            if label.is_none() && emoji.is_none() {
                                return Err(RunError::EmptyButton);
                            }
                            RenderedButton::Link {
                                url: url.clone(),
                                label,
                                emoji: emoji.clone(),
                                disabled: *disabled,
                            }
                        }
                    });
                }
                Ok(RenderedComponentRow::Buttons(rendered))
            }
            CompiledComponentRow::SelectMenu {
                id,
                placeholder,
                min_values,
                max_values,
                disabled,
                options,
            } => {
                let placeholder =
                    self.render_optional(placeholder.as_deref(), MAX_SELECT_PLACEHOLDER_LEN);
                let mut rendered_options = Vec::with_capacity(options.len());
                for option in options {
                    rendered_options.push(RenderedSelectOption {
                        label: self.render_required(
                            &option.label,
                            MAX_SELECT_OPTION_LABEL_LEN,
                            "select option label",
                        )?,
                        value: option.value.clone(),
                        description: self.render_optional(
                            option.description.as_deref(),
                            MAX_SELECT_OPTION_DESCRIPTION_LEN,
                        ),
                        emoji: option.emoji.clone(),
                        default: option.default,
                    });
                }
                Ok(RenderedComponentRow::SelectMenu {
                    id: id.clone(),
                    placeholder,
                    min_values: *min_values,
                    max_values: *max_values,
                    disabled: *disabled,
                    options: rendered_options,
                })
            }
        }
    }
}

/// Unset variables and empty texts count as 0, so counters just work.
fn number(value: &Value) -> Result<i64, RunError> {
    match value {
        Value::Text(text) if text.trim().is_empty() => Ok(0),
        _ => value
            .as_number()
            .ok_or_else(|| RunError::NotANumber(value.to_string())),
    }
}

fn math(op: MathOp, left: i64, right: i64) -> Result<i64, RunError> {
    let result = match op {
        MathOp::Add => left.checked_add(right),
        MathOp::Subtract => left.checked_sub(right),
        MathOp::Multiply => left.checked_mul(right),
        MathOp::Divide | MathOp::Modulo if right == 0 => return Err(RunError::DivisionByZero),
        MathOp::Divide => left.checked_div(right),
        MathOp::Modulo => left.checked_rem(right),
    };
    result.ok_or(RunError::Overflow)
}

/// Equality compares numbers when both sides read as numbers, text otherwise.
/// Ordering needs numbers. `contains` ignores case.
fn compare(op: CompareOp, left: &Value, right: &Value) -> Result<bool, RunError> {
    let numbers = left.as_number().zip(right.as_number());
    Ok(match op {
        CompareOp::Equal | CompareOp::NotEqual => {
            let equal = match numbers {
                Some((left, right)) => left == right,
                None => left.to_string() == right.to_string(),
            };
            equal == (op == CompareOp::Equal)
        }
        CompareOp::Contains => left
            .to_string()
            .to_lowercase()
            .contains(&right.to_string().to_lowercase()),
        CompareOp::Less
        | CompareOp::LessOrEqual
        | CompareOp::Greater
        | CompareOp::GreaterOrEqual => {
            let ordering = number(left)?.cmp(&number(right)?);
            match op {
                CompareOp::Less => ordering == Ordering::Less,
                CompareOp::LessOrEqual => ordering != Ordering::Greater,
                CompareOp::Greater => ordering == Ordering::Greater,
                _ => ordering != Ordering::Less,
            }
        }
    })
}

/// Parse an ID produced by a member target template. User command options are
/// rendered as `<@id>` so they can be used both in messages and moderation.
fn parse_member_id(value: &str) -> Option<u64> {
    let value = value.trim();
    let value = value
        .strip_prefix("<@")
        .and_then(|value| value.strip_suffix('>'))
        .unwrap_or(value)
        .strip_prefix('!')
        .unwrap_or(value);
    ((17..=20).contains(&value.len()) && value.bytes().all(|byte| byte.is_ascii_digit()))
        .then(|| value.parse::<u64>().ok())
        .flatten()
        .filter(|id| *id != 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Program;

    /// Always picks the highest number.
    struct Highest;

    impl Random for Highest {
        fn between(&mut self, _min: i64, max: i64) -> i64 {
            max
        }
    }

    fn program(json: &str) -> CompiledProgram {
        serde_json::from_str::<Program>(json)
            .expect("valid json")
            .compile()
            .expect("valid program")
    }

    fn context<'a>(options: &'a [(String, String)], roles: &'a [u64]) -> Context<'a> {
        Context {
            user_id: Some(42),
            user_name: Some("ada"),
            roles,
            guild_id: Some(7),
            channel_id: Some(9),
            message_id: Some(99),
            options,
            selected: None,
            emoji: None,
        }
    }

    fn run_slash(
        compiled: &CompiledProgram,
        name: &str,
        ctx: Context<'_>,
        saved: &SavedVariables,
    ) -> Execution {
        compiled
            .handle(
                &Event {
                    kind: EventKind::SlashCommand { name },
                    context: ctx,
                },
                saved,
                &mut Highest,
            )
            .pop()
            .expect("command matched")
    }

    #[test]
    fn runs_the_matching_slash_command() {
        let compiled = program(
            r#"{"handlers":[
                {"trigger":{"type":"slash_command","name":"greet","description":"Greet someone",
                  "options":[{"name":"who","description":"Who","kind":"user","required":true}]},
                 "actions":[{"type":"reply","content":"Hello {option.who}!","ephemeral":true}]},
                {"trigger":{"type":"slash_command","name":"other","description":"Other"},
                 "actions":[{"type":"reply","content":"no"}]}
            ]}"#,
        );
        let options = [("who".to_owned(), "<@1>".to_owned())];
        let execution = run_slash(
            &compiled,
            "greet",
            context(&options, &[]),
            &SavedVariables::default(),
        );
        assert_eq!(
            execution.effects,
            vec![Effect::Reply {
                message: MessagePayload::text("Hello <@1>!"),
                ephemeral: true,
            }]
        );
        assert_eq!(execution.trigger, "/greet");
        assert_eq!(execution.error, None);
    }

    #[test]
    fn plans_moderation_effects_for_a_user_command_option() {
        let compiled = program(
            r#"{"handlers":[{"trigger":{"type":"slash_command","name":"moderate","description":"Moderate",
              "options":[{"name":"member","description":"Member","kind":"user","required":true}]},
             "actions":[
                {"type":"timeout","member_id":"{option.member}","seconds":600,"reason":"Rule violation"},
                {"type":"kick","member_id":"{option.member}","reason":"Escalated"},
                {"type":"ban","member_id":"{option.member}","reason":"Repeated violations"},
                {"type":"direct_message","member_id":"{option.member}","content":"You were moderated."}
             ]}]}"#,
        );
        let options = [("member".to_owned(), "<@!123456789012345678>".to_owned())];

        let execution = run_slash(
            &compiled,
            "moderate",
            context(&options, &[]),
            &SavedVariables::default(),
        );

        assert_eq!(
            execution.effects,
            vec![
                Effect::Timeout {
                    member_id: 123_456_789_012_345_678,
                    seconds: 600,
                    reason: Some("Rule violation".to_owned()),
                },
                Effect::Kick {
                    member_id: 123_456_789_012_345_678,
                    reason: Some("Escalated".to_owned()),
                },
                Effect::Ban {
                    member_id: 123_456_789_012_345_678,
                    reason: Some("Repeated violations".to_owned()),
                },
                Effect::DirectMessage {
                    member_id: 123_456_789_012_345_678,
                    content: "You were moderated.".to_owned(),
                },
            ]
        );
        assert_eq!(execution.error, None);
    }

    #[test]
    fn plans_deleting_the_matched_message() {
        let compiled = program(
            r#"{"handlers":[{"trigger":{"type":"message_contains","pattern":"spam"},"actions":[
                {"type":"delete_message"},{"type":"direct_message","member_id":"123456789012345678","content":"Please do not spam."}
            ]}]}"#,
        );
        let execution = compiled
            .handle(
                &Event {
                    kind: EventKind::Message {
                        content: "spam links",
                    },
                    context: context(&[], &[]),
                },
                &SavedVariables::default(),
                &mut Highest,
            )
            .pop()
            .expect("message matched");

        assert_eq!(
            execution.effects,
            vec![
                Effect::DeleteMessage,
                Effect::DirectMessage {
                    member_id: 123_456_789_012_345_678,
                    content: "Please do not spam.".to_owned(),
                },
            ]
        );
    }

    #[test]
    fn branches_on_roles_and_sees_roles_it_adds() {
        let compiled = program(
            r#"{"handlers":[{"trigger":{"type":"slash_command","name":"verify","description":"Verify"},"actions":[
                {"type":"if","condition":{"type":"has_role","role_id":"111111111111111111"},
                 "then":[{"type":"reply","content":"Already verified"},{"type":"stop"}]},
                {"type":"add_role","role_id":"111111111111111111"},
                {"type":"if","condition":{"type":"has_role","role_id":"111111111111111111"},
                 "then":[{"type":"reply","content":"Verified!"}],
                 "else":[{"type":"reply","content":"unreachable"}]},
                {"type":"reply","content":"after"}
            ]}]}"#,
        );
        let saved = SavedVariables::default();

        let verified = run_slash(
            &compiled,
            "verify",
            context(&[], &[111_111_111_111_111_111]),
            &saved,
        );
        assert_eq!(
            verified.effects,
            vec![Effect::Reply {
                message: MessagePayload::text("Already verified"),
                ephemeral: false
            }]
        );

        let fresh = run_slash(&compiled, "verify", context(&[], &[]), &saved);
        let replies: Vec<_> = fresh
            .effects
            .iter()
            .filter_map(|effect| match effect {
                Effect::Reply { message, .. } => message.content.as_deref(),
                _ => None,
            })
            .collect();
        assert_eq!(replies, ["Verified!", "after"]);
    }

    #[test]
    fn counts_with_saved_variables() {
        let compiled = program(
            r#"{"handlers":[{"trigger":{"type":"message_contains","pattern":"gg"},"actions":[
                {"type":"change_variable","scope":"member","name":"xp","by":{"type":"random",
                  "min":{"type":"number","value":5},"max":{"type":"number","value":10}}},
                {"type":"change_variable","scope":"server","name":"messages","by":{"type":"number","value":1}},
                {"type":"if","condition":{"type":"compare","op":"greater_or_equal",
                   "left":{"type":"variable","scope":"member","name":"xp"},"right":{"type":"number","value":100}},
                 "then":[{"type":"set_variable","scope":"local","name":"level","value":{"type":"math","op":"divide",
                   "left":{"type":"variable","scope":"member","name":"xp"},"right":{"type":"number","value":100}}},
                   {"type":"reply","content":"{user} reached level {var:level} with {member:xp} XP"}]}
            ]}]}"#,
        );
        let mut saved = SavedVariables::default();
        saved.member.insert("xp".to_owned(), Value::Number(95));

        let execution = compiled
            .handle(
                &Event {
                    kind: EventKind::Message {
                        content: "GG everyone",
                    },
                    context: context(&[], &[]),
                },
                &saved,
                &mut Highest,
            )
            .pop()
            .expect("message matched");

        assert_eq!(
            execution.effects,
            vec![Effect::Reply {
                message: MessagePayload::text("<@42> reached level 1 with 105 XP"),
                ephemeral: false
            }]
        );
        assert_eq!(
            execution.writes,
            vec![
                VariableWrite {
                    scope: Scope::Member,
                    name: "xp".to_owned(),
                    value: Value::Number(105)
                },
                VariableWrite {
                    scope: Scope::Server,
                    name: "messages".to_owned(),
                    value: Value::Number(1)
                },
            ]
        );
    }

    #[test]
    fn compares_text_and_numbers() {
        let text = |value: &str| Value::Text(value.to_owned());
        assert_eq!(
            compare(CompareOp::Equal, &text("10"), &Value::Number(10)),
            Ok(true)
        );
        assert_eq!(
            compare(CompareOp::Equal, &text("Yes"), &text("yes")),
            Ok(false)
        );
        assert_eq!(
            compare(CompareOp::Contains, &text("Hello World"), &text("world")),
            Ok(true)
        );
        assert_eq!(
            compare(CompareOp::Less, &text(""), &Value::Number(1)),
            Ok(true)
        );
        assert_eq!(
            compare(CompareOp::Greater, &text("abc"), &Value::Number(1)),
            Err(RunError::NotANumber("abc".to_owned()))
        );
    }

    #[test]
    fn keeps_effects_planned_before_an_error() {
        let compiled = program(
            r#"{"handlers":[{"trigger":{"type":"slash_command","name":"split","description":"Split"},"actions":[
                {"type":"reply","content":"Computing…"},
                {"type":"wait","seconds":2},
                {"type":"set_variable","scope":"local","name":"x","value":{"type":"math","op":"divide",
                  "left":{"type":"number","value":1},"right":{"type":"number","value":0}}},
                {"type":"reply","content":"never"}
            ]}]}"#,
        );
        let execution = run_slash(
            &compiled,
            "split",
            context(&[], &[]),
            &SavedVariables::default(),
        );
        assert_eq!(
            execution.effects,
            vec![
                Effect::Reply {
                    message: MessagePayload::text("Computing…"),
                    ephemeral: false
                },
                Effect::Wait { seconds: 2 },
            ]
        );
        assert_eq!(execution.error, Some(RunError::DivisionByZero));
    }

    #[test]
    fn runs_button_and_select_menu_triggers_with_rich_messages() {
        let compiled = program(
            r##"{"handlers":[
                {"trigger":{"type":"button_click","id":"claim"},
                 "actions":[
                    {"type":"update_message","content":"","embeds":[{"title":"Claimed by {user.name}","color":"#22C55E","fields":[{"name":"User","value":"{user}","inline":true}]}],
                     "components":[{"type":"buttons","buttons":[{"type":"action","id":"claim","label":"Claimed","style":"secondary","disabled":true}]}]},
                    {"type":"reply","content":"All yours, {user}!","ephemeral":true}
                 ]},
                {"trigger":{"type":"select_menu","id":"role_menu"},
                 "actions":[
                    {"type":"if","condition":{"type":"compare","op":"equal","left":{"type":"selected"},"right":{"type":"text","value":"vip"}},
                     "then":[{"type":"add_role","role_id":"111111111111111111"}]},
                    {"type":"reply","content":"Picked {selected}","ephemeral":true}
                 ]}
            ]}"##,
        );
        let saved = SavedVariables::default();

        let button_exec = compiled
            .handle(
                &Event {
                    kind: EventKind::ButtonClick { id: "claim" },
                    context: context(&[], &[]),
                },
                &saved,
                &mut Highest,
            )
            .pop()
            .expect("button matched");
        assert_eq!(button_exec.trigger, "button \"claim\"");
        assert_eq!(button_exec.error, None);
        assert_eq!(button_exec.effects.len(), 2);
        match &button_exec.effects[0] {
            Effect::UpdateMessage { message } => {
                assert_eq!(message.content, None);
                assert_eq!(message.embeds[0].title.as_deref(), Some("Claimed by ada"));
                assert_eq!(message.embeds[0].color, Some(0x0022_C55E));
                assert_eq!(message.embeds[0].fields[0].value, "<@42>");
            }
            other => panic!("unexpected first effect: {other:?}"),
        }

        let mut select_ctx = context(&[], &[]);
        select_ctx.selected = Some("vip");
        let select_exec = compiled
            .handle(
                &Event {
                    kind: EventKind::SelectMenu { id: "role_menu" },
                    context: select_ctx,
                },
                &saved,
                &mut Highest,
            )
            .pop()
            .expect("select menu matched");
        assert_eq!(select_exec.trigger, "select menu \"role_menu\"");
        assert_eq!(
            select_exec.effects,
            vec![
                Effect::AddRole {
                    role_id: 111_111_111_111_111_111
                },
                Effect::Reply {
                    message: MessagePayload::text("Picked vip"),
                    ephemeral: true
                }
            ]
        );
    }

    #[test]
    fn rejects_unknown_fields() {
        let result = serde_json::from_str::<Program>(
            r#"{"handlers":[{"trigger":{"type":"member_join","extra":1},"actions":[]}]}"#,
        );
        assert!(result.is_err());
    }

    #[test]
    fn runs_member_leave_and_schedule_handlers() {
        let compiled = program(
            r#"{"handlers":[
                {"trigger":{"type":"member_leave"},
                 "actions":[{"type":"send_message","channel_id":"123456789012345678","content":"Goodbye {user.name}"}]},
                {"trigger":{"type":"schedule","interval":15,"unit":"minutes"},
                 "actions":[{"type":"set_variable","scope":"local","name":"n","value":{"type":"number","value":3}},
                            {"type":"send_message","channel_id":"123456789012345678","content":"Tick {var:n}"}]}
            ]}"#,
        );
        let saved = SavedVariables::default();

        let leave = compiled
            .handle(
                &Event {
                    kind: EventKind::MemberLeave,
                    context: context(&[], &[]),
                },
                &saved,
                &mut Highest,
            )
            .pop()
            .expect("member leave matched");
        assert_eq!(leave.trigger, "member leave");
        assert_eq!(
            leave.effects,
            vec![Effect::SendMessage {
                channel_id: 123_456_789_012_345_678,
                message: MessagePayload::text("Goodbye ada"),
            }]
        );

        let schedule_ctx = Context {
            user_id: None,
            user_name: None,
            roles: &[],
            guild_id: None,
            channel_id: None,
            message_id: None,
            options: &[],
            selected: None,
            emoji: None,
        };
        let schedule = compiled
            .handle(
                &Event {
                    kind: EventKind::Schedule {
                        interval: 15,
                        unit: ScheduleUnit::Minutes,
                    },
                    context: schedule_ctx,
                },
                &saved,
                &mut Highest,
            )
            .pop()
            .expect("schedule matched");
        assert_eq!(schedule.trigger, "every 15 minutes");
        assert_eq!(
            schedule.effects,
            vec![Effect::SendMessage {
                channel_id: 123_456_789_012_345_678,
                message: MessagePayload::text("Tick 3"),
            }]
        );
    }

    #[test]
    fn runs_reaction_add_and_remove_handlers() {
        let compiled = program(
            r#"{"handlers":[
                {"trigger":{"type":"reaction_add","emoji":"👍","message_id":"123456789012345679"},
                 "actions":[{"type":"add_role","role_id":"111111111111111111"},{"type":"reply","content":"Reacted {emoji} on {message.id}"}]},
                {"trigger":{"type":"reaction_remove"},
                 "actions":[{"type":"remove_role","role_id":"111111111111111111"}]}
            ]}"#,
        );
        let saved = SavedVariables::default();
        let mut reaction_ctx = context(&[], &[]);
        reaction_ctx.message_id = Some(123_456_789_012_345_679);
        reaction_ctx.emoji = Some("👍");

        let wrong_msg = compiled.handle(
            &Event {
                kind: EventKind::ReactionAdd {
                    emoji: EventEmoji::Unicode("👍"),
                    message_id: 999,
                },
                context: reaction_ctx,
            },
            &saved,
            &mut Highest,
        );
        assert!(wrong_msg.is_empty());

        let reaction_add = compiled
            .handle(
                &Event {
                    kind: EventKind::ReactionAdd {
                        emoji: EventEmoji::Unicode("👍"),
                        message_id: 123_456_789_012_345_679,
                    },
                    context: reaction_ctx,
                },
                &saved,
                &mut Highest,
            )
            .pop()
            .expect("reaction add matched");
        assert_eq!(
            reaction_add.effects,
            vec![
                Effect::AddRole {
                    role_id: 111_111_111_111_111_111,
                },
                Effect::Reply {
                    message: MessagePayload::text("Reacted 👍 on 123456789012345679"),
                    ephemeral: false,
                },
            ]
        );

        let reaction_remove = compiled
            .handle(
                &Event {
                    kind: EventKind::ReactionRemove {
                        emoji: EventEmoji::Custom { id: 555 },
                        message_id: 999,
                    },
                    context: reaction_ctx,
                },
                &saved,
                &mut Highest,
            )
            .pop()
            .expect("reaction remove matched wildcard");
        assert_eq!(
            reaction_remove.effects,
            vec![Effect::RemoveRole {
                role_id: 111_111_111_111_111_111,
            }]
        );
    }
}
