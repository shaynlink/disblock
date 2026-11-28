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
    Branch, CompileError, CompileFailure, CompiledProgram, ComponentLocation, ContextCommandKind,
    ContextCommandSpec, EmbedLocation, EmbedPart, ErrorLocation, GatewayNeeds, HandlerError,
    ScheduleSpec, SlashCommandSpec, SlashSubcommandGroupSpec, SlashSubcommandSpec, StepError,
    StepRef,
};
pub use model::{
    AttachmentSpec, ButtonSpec, ButtonStyle, ChannelFilter, CommandOption, CommandOptionChoiceSpec,
    CompareOp, ComponentRow, EmbedAuthorSpec, EmbedFieldSpec, EmbedFooterSpec, EmbedSpec,
    EntitySelectKind, Expr, Handler, MathOp, ModalInputSpec, OptionKind, PollAnswerSpec, PollSpec,
    Program, ScheduleUnit, Scope, SelectOptionSpec, Statement, TargetField, TextInputStyle,
    Trigger,
};
pub use run::{
    Effect, EmojiRef, Event, EventEmoji, EventKind, Execution, MessagePayload, Random,
    RenderedAttachment, RenderedButton, RenderedComponentRow, RenderedEmbed, RenderedEmbedAuthor,
    RenderedEmbedField, RenderedEmbedFooter, RenderedModalInput, RenderedPoll, RenderedPollAnswer,
    RenderedSelectOption, RunError, SavedVariables, VariableWrite,
};
pub use template::{Context, MAX_MESSAGE_LEN};
pub use value::Value;
