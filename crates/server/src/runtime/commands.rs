use disblock_program::{
    CommandOption, ContextCommandKind, ContextCommandSpec, OptionKind, SlashCommandSpec,
    SlashSubcommandSpec,
};
use twilight_model::application::command::{
    Command, CommandOption as TwilightCommandOption, CommandType,
};
use twilight_model::application::interaction::InteractionContextType;
use twilight_model::channel::ChannelType;
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
            if let Some(min) = option
                .min_length
                .as_deref()
                .and_then(|value| value.trim().parse::<u16>().ok())
            {
                builder = builder.min_length(min);
            }
            if let Some(max) = option
                .max_length
                .as_deref()
                .and_then(|value| value.trim().parse::<u16>().ok())
            {
                builder = builder.max_length(max);
            }
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
            if let Some(min) = option
                .min_value
                .as_deref()
                .and_then(|value| value.trim().parse::<i64>().ok())
            {
                builder = builder.min_value(min);
            }
            if let Some(max) = option
                .max_value
                .as_deref()
                .and_then(|value| value.trim().parse::<i64>().ok())
            {
                builder = builder.max_value(max);
            }
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
            if let Some(min) = option
                .min_value
                .as_deref()
                .and_then(|value| value.trim().parse::<f64>().ok())
                .filter(|value| value.is_finite())
            {
                builder = builder.min_value(min);
            }
            if let Some(max) = option
                .max_value
                .as_deref()
                .and_then(|value| value.trim().parse::<f64>().ok())
                .filter(|value| value.is_finite())
            {
                builder = builder.max_value(max);
            }
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
        OptionKind::Channel => {
            let mut builder = ChannelBuilder::new(name, description).required(option.required);
            if !option.channel_types.is_empty() {
                builder =
                    builder.channel_types(option.channel_types.iter().map(|filter| match filter {
                        disblock_program::ChannelFilter::Text => ChannelType::GuildText,
                        disblock_program::ChannelFilter::Voice => ChannelType::GuildVoice,
                        disblock_program::ChannelFilter::Announcement => {
                            ChannelType::GuildAnnouncement
                        }
                        disblock_program::ChannelFilter::Forum => ChannelType::GuildForum,
                        disblock_program::ChannelFilter::Stage => ChannelType::GuildStageVoice,
                        disblock_program::ChannelFilter::Category => ChannelType::GuildCategory,
                        disblock_program::ChannelFilter::Media => ChannelType::GuildMedia,
                    }));
            }
            builder.build()
        }
        OptionKind::Role => RoleBuilder::new(name, description)
            .required(option.required)
            .build(),
        OptionKind::Mentionable => MentionableBuilder::new(name, description)
            .required(option.required)
            .build(),
        OptionKind::Attachment => {
            let mut builder = AttachmentBuilder::new(name, description).required(option.required);
            if !option.file_types.is_empty() {
                builder = builder.file_types(option.file_types.iter().map(String::as_str));
            }
            builder.build()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use disblock_program::{CommandOption, CommandOptionChoiceSpec, SlashSubcommandGroupSpec};
    use twilight_model::application::command::{CommandOptionType, CommandOptionValue};

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
                min_value: match kind {
                    OptionKind::Integer => Some("-5".to_owned()),
                    OptionKind::Number => Some("-1.5".to_owned()),
                    _ => None,
                },
                max_value: match kind {
                    OptionKind::Integer => Some("10".to_owned()),
                    OptionKind::Number => Some("3.5".to_owned()),
                    _ => None,
                },
                min_length: (kind == OptionKind::String).then(|| "2".to_owned()),
                max_length: (kind == OptionKind::String).then(|| "32".to_owned()),
                channel_types: if kind == OptionKind::Channel {
                    vec![
                        disblock_program::ChannelFilter::Text,
                        disblock_program::ChannelFilter::Forum,
                    ]
                } else {
                    Vec::new()
                },
                file_types: if kind == OptionKind::Attachment {
                    vec!["image".to_owned(), ".pdf".to_owned()]
                } else {
                    Vec::new()
                },
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
        assert_eq!(commands[0].options[0].min_length, Some(2));
        assert_eq!(commands[0].options[0].max_length, Some(32));
        assert_eq!(
            commands[0].options[1].min_value,
            Some(CommandOptionValue::Integer(-5))
        );
        assert_eq!(
            commands[0].options[1].max_value,
            Some(CommandOptionValue::Integer(10))
        );
        assert_eq!(
            commands[0].options[2].min_value,
            Some(CommandOptionValue::Number(-1.5))
        );
        assert_eq!(
            commands[0].options[2].max_value,
            Some(CommandOptionValue::Number(3.5))
        );
        assert_eq!(
            commands[0].options[5].channel_types,
            Some(vec![ChannelType::GuildText, ChannelType::GuildForum])
        );
        assert_eq!(
            commands[0].options[8].file_types,
            Some(vec!["image".to_owned(), ".pdf".to_owned()])
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
                    min_value: None,
                    max_value: None,
                    min_length: None,
                    max_length: None,
                    channel_types: Vec::new(),
                    file_types: Vec::new(),
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
                        min_value: None,
                        max_value: None,
                        min_length: None,
                        max_length: None,
                        channel_types: Vec::new(),
                        file_types: Vec::new(),
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
