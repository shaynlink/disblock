//! Disblock bot programs.
//!
//! A [`Program`] is the JSON document produced by the visual block editor: a
//! list of handlers, each made of one trigger and a list of statements. It is
//! untrusted input, so it is turned into a [`CompiledProgram`] by
//! [`Program::compile`], which checks every Discord limit and parses every ID.
//!
//! Running a handler is pure: [`CompiledProgram::handle`] plans it against the
//! event, the member's roles and their [`SavedVariables`], and returns the
//! [`Effect`]s to perform and the [`VariableWrite`]s to save. The runtime does
//! the I/O.

mod compile;
mod model;
mod run;
mod template;
mod value;

pub use compile::{
    Branch, CompileError, CompileFailure, CompiledProgram, ComponentLocation, EmbedLocation,
    EmbedPart, ErrorLocation, GatewayNeeds, HandlerError, ScheduleSpec, SlashCommandSpec,
    StepError, StepRef,
};
pub use model::{
    ButtonSpec, ButtonStyle, CommandOption, CompareOp, ComponentRow, EmbedAuthorSpec,
    EmbedFieldSpec, EmbedFooterSpec, EmbedSpec, Expr, Handler, MathOp, OptionKind, Program,
    ScheduleUnit, Scope, SelectOptionSpec, Statement, Trigger,
};
pub use run::{
    Effect, EmojiRef, Event, EventEmoji, EventKind, Execution, MessagePayload, Random,
    RenderedButton, RenderedComponentRow, RenderedEmbed, RenderedEmbedAuthor, RenderedEmbedField,
    RenderedEmbedFooter, RenderedSelectOption, RunError, SavedVariables, VariableWrite,
};
pub use template::{Context, MAX_MESSAGE_LEN};
pub use value::Value;
