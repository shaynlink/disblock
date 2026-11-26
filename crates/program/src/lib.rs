//! Disblock bot programs.
//!
//! A [`Program`] is the JSON document produced by the visual block editor: a
//! list of handlers, each made of one trigger and a list of actions. It is
//! untrusted input, so it is turned into a [`CompiledProgram`] by
//! [`Program::compile`], which checks every Discord limit and parses every ID.
//! The runtime only ever works with compiled programs and asks them, for each
//! incoming Discord event, which [`Effect`]s to perform.

mod compile;
mod model;
mod run;
mod template;

pub use compile::{CompileError, CompiledProgram, GatewayNeeds, SlashCommandSpec};
pub use model::{Action, CommandOption, Handler, OptionKind, Program, Trigger};
pub use run::{Effect, EmojiRef, Event, EventKind, Execution};
pub use template::{Context, MAX_MESSAGE_LEN};
