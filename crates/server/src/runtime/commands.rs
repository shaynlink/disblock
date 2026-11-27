use disblock_program::{
    CommandOption, ContextCommandKind, ContextCommandSpec, OptionKind, SlashCommandSpec,
    SlashSubcommandSpec,
};
use twilight_model::application::command::{
    Command, CommandOption as TwilightCommandOption, CommandType,
};
use twilight_model::application::interaction::InteractionContextType;
use twilight_util::builder::command::{
    AttachmentBuilder, BooleanBuilder, ChannelBuilder, CommandBuilder, IntegerBuilder,
    MentionableBuilder, NumberBuilder, RoleBuilder, StringBuilder, SubCommandBuilder,
    SubCommandGroupBuilder, UserBuilder,
};

/// Discord command payloads for a program's slash and context menu commands.
/// Commands only work inside servers, since actions target a server member or channel.
pub fn build(specs: &[SlashCommandSpec], context_specs: &[ContextCommandSpec]) -> Vec<Command> {
    let mut commands: Vec<Command> = specs
        .iter()
        .map(|spec| {
            let mut builder =
                CommandBuilder::new(&spec.name, &spec.description, CommandType::ChatInput)
                    .contexts([InteractionContextType::Guild]);

            for option in &spec.options {
                builder = builder.option(build_option(option));
            }
            for sub in &spec.subcommands {
                builder = builder.option(build_subcommand(sub));
            }
            for group in &spec.groups {
                let mut group_builder =
                    SubCommandGroupBuilder::new(&group.name, &group.description);
                for sub in &group.subcommands {
                    group_builder = group_builder.subcommands([build_subcommand(sub)]);
                }
                builder = builder.option(group_builder);
            }

            builder.build()
        })
        .collect();

    for spec in context_specs {
        let kind = match spec.kind {
            ContextCommandKind::User => CommandType::User,
            ContextCommandKind::Message => CommandType::Message,
        };
        commands.push(
            CommandBuilder::new(&spec.name, "", kind)
                .contexts([InteractionContextType::Guild])
                .build(),
        );
    }

    commands
}

fn build_subcommand(spec: &SlashSubcommandSpec) -> SubCommandBuilder {
    spec.options.iter().fold(
        SubCommandBuilder::new(&spec.name, &spec.description),
        |sub, option| sub.option(build_option(option)),
    )
}

fn build_option(option: &CommandOption) -> TwilightCommandOption {
    let (name, description) = (&option.name, &option.description);
    let has_autocomplete = !option.autocomplete.is_empty();
    match option.kind {
        OptionKind::String => {
            let mut builder = StringBuilder::new(name, description).required(option.required);
            if has_autocomplete {
                builder = builder.autocomplete(true);
            } else if !option.choices.is_empty() {
                builder = builder.choices(
                    option
                        .choices
                        .iter()
                        .map(|choice| (choice.name.as_str(), choice.value.as_str())),
                );
            }
            builder.build()
        }
        OptionKind::Integer => {
            let mut builder = IntegerBuilder::new(name, description).required(option.required);
            if has_autocomplete {
                builder = builder.autocomplete(true);
            } else if !option.choices.is_empty() {
                let choices: Vec<(&str, i64)> = option
                    .choices
                    .iter()
                    .filter_map(|choice| {
                        choice
                            .value
                            .trim()
                            .parse::<i64>()
                            .ok()
                            .map(|val| (choice.name.as_str(), val))
                    })
                    .collect();
                builder = builder.choices(choices);
            }
            builder.build()
        }
        OptionKind::Number => {
            let mut builder = NumberBuilder::new(name, description).required(option.required);
            if has_autocomplete {
                builder = builder.autocomplete(true);
            } else if !option.choices.is_empty() {
                let choices: Vec<(&str, f64)> = option
                    .choices
                    .iter()
                    .filter_map(|choice| {
                        choice
                            .value
                            .trim()
                            .parse::<f64>()
                            .ok()
                            .filter(|val| val.is_finite())
                            .map(|val| (choice.name.as_str(), val))
                    })
                    .collect();
                builder = builder.choices(choices);
            }
            builder.build()
        }
        OptionKind::Boolean => BooleanBuilder::new(name, description)
            .required(option.required)
            .build(),
        OptionKind::User => UserBuilder::new(name, description)
            .required(option.required)
            .build(),
        OptionKind::Channel => ChannelBuilder::new(name, description)
            .required(option.required)
            .build(),
        OptionKind::Role => RoleBuilder::new(name, description)
            .required(option.required)
            .build(),
        OptionKind::Mentionable => MentionableBuilder::new(name, description)
            .required(option.required)
            .build(),
        OptionKind::Attachment => AttachmentBuilder::new(name, description)
            .required(option.required)
            .build(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use disblock_program::{CommandOption, CommandOptionChoiceSpec, SlashSubcommandGroupSpec};
    use twilight_model::application::command::CommandOptionType;

    #[test]
    fn registers_every_supported_discord_option_kind() {
        let specs = [SlashCommandSpec {
            name: "configure".to_owned(),
            description: "Configure the bot".to_owned(),
            options: [
                ("text", OptionKind::String),
                ("whole", OptionKind::Integer),
                ("decimal", OptionKind::Number),
                ("enabled", OptionKind::Boolean),
                ("member", OptionKind::User),
                ("channel", OptionKind::Channel),
                ("role", OptionKind::Role),
                ("mention", OptionKind::Mentionable),
                ("file", OptionKind::Attachment),
            ]
            .into_iter()
            .map(|(name, kind)| CommandOption {
                name: name.to_owned(),
                description: format!("Select a {name}"),
                kind,
                required: false,
                choices: Vec::new(),
                autocomplete: Vec::new(),
            })
            .collect(),
            subcommands: Vec::new(),
            groups: Vec::new(),
        }];

        let commands = build(&specs, &[]);
        assert_eq!(
            commands[0]
                .options
                .iter()
                .map(|option| option.kind)
                .collect::<Vec<_>>(),
            vec![
                CommandOptionType::String,
                CommandOptionType::Integer,
                CommandOptionType::Number,
                CommandOptionType::Boolean,
                CommandOptionType::User,
                CommandOptionType::Channel,
                CommandOptionType::Role,
                CommandOptionType::Mentionable,
                CommandOptionType::Attachment,
            ],
        );
    }

    #[test]
    fn registers_subcommands_choices_autocomplete_and_context_commands() {
        let specs = [SlashCommandSpec {
            name: "admin".to_owned(),
            description: "admin commands".to_owned(),
            options: Vec::new(),
            subcommands: vec![SlashSubcommandSpec {
                name: "ping".to_owned(),
                description: "Ping".to_owned(),
                options: vec![CommandOption {
                    name: "mode".to_owned(),
                    description: "Mode".to_owned(),
                    kind: OptionKind::String,
                    required: false,
                    choices: vec![CommandOptionChoiceSpec {
                        name: "Fast".to_owned(),
                        value: "fast".to_owned(),
                    }],
                    autocomplete: Vec::new(),
                }],
            }],
            groups: vec![SlashSubcommandGroupSpec {
                name: "user".to_owned(),
                description: "user commands".to_owned(),
                subcommands: vec![SlashSubcommandSpec {
                    name: "lookup".to_owned(),
                    description: "Lookup user".to_owned(),
                    options: vec![CommandOption {
                        name: "tag".to_owned(),
                        description: "Tag".to_owned(),
                        kind: OptionKind::String,
                        required: false,
                        choices: Vec::new(),
                        autocomplete: vec!["vip".to_owned(), "mod".to_owned()],
                    }],
                }],
            }],
        }];
        let context_specs = [
            ContextCommandSpec {
                name: "Inspect User".to_owned(),
                kind: ContextCommandKind::User,
            },
            ContextCommandSpec {
                name: "Bookmark".to_owned(),
                kind: ContextCommandKind::Message,
            },
        ];

        let commands = build(&specs, &context_specs);
        assert_eq!(commands.len(), 3);
        assert_eq!(commands[0].kind, CommandType::ChatInput);
        assert_eq!(commands[0].options.len(), 2);
        assert_eq!(commands[0].options[0].kind, CommandOptionType::SubCommand);
        assert_eq!(
            commands[0].options[1].kind,
            CommandOptionType::SubCommandGroup
        );
        assert_eq!(commands[1].kind, CommandType::User);
        assert_eq!(commands[1].description, "");
        assert_eq!(commands[2].kind, CommandType::Message);
        assert_eq!(commands[2].description, "");
    }
}
