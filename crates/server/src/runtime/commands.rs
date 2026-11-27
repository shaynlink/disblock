use disblock_program::{OptionKind, SlashCommandSpec};
use twilight_model::application::command::{Command, CommandType};
use twilight_model::application::interaction::InteractionContextType;
use twilight_util::builder::command::{
    AttachmentBuilder, BooleanBuilder, ChannelBuilder, CommandBuilder, IntegerBuilder,
    MentionableBuilder, NumberBuilder, RoleBuilder, StringBuilder, UserBuilder,
};

/// Discord command payloads for a program's slash commands. Commands only
/// work inside servers, since every action targets a server member or channel.
pub fn build(specs: &[SlashCommandSpec]) -> Vec<Command> {
    specs
        .iter()
        .map(|spec| {
            spec.options
                .iter()
                .fold(
                    CommandBuilder::new(&spec.name, &spec.description, CommandType::ChatInput)
                        .contexts([InteractionContextType::Guild]),
                    |command, option| {
                        let (name, description) = (&option.name, &option.description);
                        match option.kind {
                            OptionKind::String => command.option(
                                StringBuilder::new(name, description).required(option.required),
                            ),
                            OptionKind::Integer => command.option(
                                IntegerBuilder::new(name, description).required(option.required),
                            ),
                            OptionKind::Number => command.option(
                                NumberBuilder::new(name, description).required(option.required),
                            ),
                            OptionKind::Boolean => command.option(
                                BooleanBuilder::new(name, description).required(option.required),
                            ),
                            OptionKind::User => command.option(
                                UserBuilder::new(name, description).required(option.required),
                            ),
                            OptionKind::Channel => command.option(
                                ChannelBuilder::new(name, description).required(option.required),
                            ),
                            OptionKind::Role => command.option(
                                RoleBuilder::new(name, description).required(option.required),
                            ),
                            OptionKind::Mentionable => command.option(
                                MentionableBuilder::new(name, description)
                                    .required(option.required),
                            ),
                            OptionKind::Attachment => command.option(
                                AttachmentBuilder::new(name, description).required(option.required),
                            ),
                        }
                    },
                )
                .build()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use disblock_program::CommandOption;
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
            })
            .collect(),
        }];

        let commands = build(&specs);
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
}
