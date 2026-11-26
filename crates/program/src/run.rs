use crate::compile::{CompiledAction, CompiledHandler, CompiledProgram};
use crate::template::Context;

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

/// A side effect for the runtime to perform, with templates already rendered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    Reply { content: String, ephemeral: bool },
    SendMessage { channel_id: u64, content: String },
    AddRole { role_id: u64 },
    RemoveRole { role_id: u64 },
    React { emoji: EmojiRef },
}

/// The effects of one handler matched by an event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Execution {
    /// Human readable trigger, such as `/hello`.
    pub trigger: String,
    pub effects: Vec<Effect>,
}

impl CompiledProgram {
    /// Every handler matched by `event`, in program order.
    #[must_use]
    pub fn handle(&self, event: &Event<'_>) -> Vec<Execution> {
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
            .map(|index| run_handler(&self.handlers[index], &event.context))
            .collect()
    }
}

fn run_handler(handler: &CompiledHandler, context: &Context<'_>) -> Execution {
    let effects = handler
        .actions
        .iter()
        .map(|action| match action {
            CompiledAction::Reply { content, ephemeral } => Effect::Reply {
                content: context.render(content),
                ephemeral: *ephemeral,
            },
            CompiledAction::SendMessage {
                channel_id,
                content,
            } => Effect::SendMessage {
                channel_id: *channel_id,
                content: context.render(content),
            },
            CompiledAction::AddRole { role_id } => Effect::AddRole { role_id: *role_id },
            CompiledAction::RemoveRole { role_id } => Effect::RemoveRole { role_id: *role_id },
            CompiledAction::React { emoji } => Effect::React {
                emoji: emoji.clone(),
            },
        })
        .collect();

    Execution {
        trigger: handler.label.clone(),
        effects,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Program;

    fn program(json: &str) -> CompiledProgram {
        serde_json::from_str::<Program>(json)
            .expect("valid json")
            .compile()
            .expect("valid program")
    }

    fn context(options: &[(String, String)]) -> Context<'_> {
        Context {
            user_id: 42,
            user_name: "ada",
            guild_id: Some(7),
            channel_id: Some(9),
            options,
        }
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
        let executions = compiled.handle(&Event {
            kind: EventKind::SlashCommand { name: "greet" },
            context: context(&options),
        });
        assert_eq!(
            executions,
            vec![Execution {
                trigger: "/greet".to_owned(),
                effects: vec![Effect::Reply {
                    content: "Hello <@1>!".to_owned(),
                    ephemeral: true,
                }],
            }]
        );
    }

    #[test]
    fn matches_messages_with_and_without_case() {
        let compiled = program(
            r#"{"handlers":[
                {"trigger":{"type":"message_contains","pattern":"Ping"},
                 "actions":[{"type":"react","emoji":"🏓"}]},
                {"trigger":{"type":"message_contains","pattern":"Ping","case_sensitive":true},
                 "actions":[{"type":"reply","content":"Pong"}]}
            ]}"#,
        );
        let lower = compiled.handle(&Event {
            kind: EventKind::Message {
                content: "ping please",
            },
            context: context(&[]),
        });
        assert_eq!(lower.len(), 1);

        let exact = compiled.handle(&Event {
            kind: EventKind::Message {
                content: "Ping please",
            },
            context: context(&[]),
        });
        assert_eq!(exact.len(), 2);
    }

    #[test]
    fn rejects_unknown_fields() {
        let result = serde_json::from_str::<Program>(
            r#"{"handlers":[{"trigger":{"type":"member_join","extra":1},"actions":[]}]}"#,
        );
        assert!(result.is_err());
    }
}
