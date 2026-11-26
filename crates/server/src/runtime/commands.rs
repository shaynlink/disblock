use disblock_program::{OptionKind, SlashCommandSpec};
use twilight_model::application::command::{Command, CommandType};
use twilight_model::application::interaction::InteractionContextType;
use twilight_util::builder::command::{
    BooleanBuilder, CommandBuilder, IntegerBuilder, StringBuilder, UserBuilder,
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
                            OptionKind::Boolean => command.option(
                                BooleanBuilder::new(name, description).required(option.required),
                            ),
                            OptionKind::User => command.option(
                                UserBuilder::new(name, description).required(option.required),
                            ),
                        }
                    },
                )
                .build()
        })
        .collect()
}
