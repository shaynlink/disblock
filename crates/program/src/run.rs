use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};

use thiserror::Error;

use crate::compile::{Calc, CompiledHandler, CompiledProgram, MAX_WAIT_SECONDS, Step};
use crate::model::{CompareOp, MathOp, Scope};
use crate::template::{Context, render};
use crate::value::Value;

/// Effects one run may perform, so a handler cannot flood a server.
const MAX_EFFECTS: usize = 50;
/// Total waiting in one run. Discord accepts answers to a slash command for
/// 15 minutes.
const MAX_TOTAL_WAIT_SECONDS: u32 = 2 * MAX_WAIT_SECONDS;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmojiRef {
    Unicode(String),
    Custom { name: String, id: u64 },
}

/// A Discord event, reduced to what programs can react to.
#[derive(Debug, Clone, Copy)]
pub enum EventKind<'a> {
    SlashCommand { name: &'a str },
    MemberJoin,
    Message { content: &'a str },
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

/// A side effect for the runtime to perform, with templates already rendered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    Reply { content: String, ephemeral: bool },
    SendMessage { channel_id: u64, content: String },
    AddRole { role_id: u64 },
    RemoveRole { role_id: u64 },
    React { emoji: EmojiRef },
    Wait { seconds: u32 },
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
        };

        matched
            .into_iter()
            .map(|index| plan(&self.handlers[index], &event.context, saved, random))
            .collect()
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
            Step::Reply { content, ephemeral } => {
                let content = self.render(content);
                self.emit(Effect::Reply {
                    content,
                    ephemeral: *ephemeral,
                })?;
            }
            Step::SendMessage {
                channel_id,
                content,
            } => {
                let content = self.render(content);
                self.emit(Effect::SendMessage {
                    channel_id: *channel_id,
                    content,
                })?;
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
        render(template, |key| {
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
            user_id: 42,
            user_name: "ada",
            roles,
            guild_id: Some(7),
            channel_id: Some(9),
            options,
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
                content: "Hello <@1>!".to_owned(),
                ephemeral: true,
            }]
        );
        assert_eq!(execution.trigger, "/greet");
        assert_eq!(execution.error, None);
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
                content: "Already verified".to_owned(),
                ephemeral: false
            }]
        );

        let fresh = run_slash(&compiled, "verify", context(&[], &[]), &saved);
        let replies: Vec<_> = fresh
            .effects
            .iter()
            .filter_map(|effect| match effect {
                Effect::Reply { content, .. } => Some(content.as_str()),
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
                content: "<@42> reached level 1 with 105 XP".to_owned(),
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
                    content: "Computing…".to_owned(),
                    ephemeral: false
                },
                Effect::Wait { seconds: 2 },
            ]
        );
        assert_eq!(execution.error, Some(RunError::DivisionByZero));
    }

    #[test]
    fn rejects_unknown_fields() {
        let result = serde_json::from_str::<Program>(
            r#"{"handlers":[{"trigger":{"type":"member_join","extra":1},"actions":[]}]}"#,
        );
        assert!(result.is_err());
    }
}
