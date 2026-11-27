/// Discord's message content limit, in characters.
pub const MAX_MESSAGE_LEN: usize = 2000;

/// What a handler knows about the event that triggered it.
#[derive(Debug, Clone, Copy, Default)]
pub struct Context<'a> {
    pub user_id: Option<u64>,
    pub user_name: Option<&'a str>,
    /// Roles of the member, when the event happened in a server.
    pub roles: &'a [u64],
    pub guild_id: Option<u64>,
    pub channel_id: Option<u64>,
    /// Message ID when the event happened on a message (message, component, reaction).
    pub message_id: Option<u64>,
    /// Slash command options as `(name, rendered value)`.
    pub options: &'a [(String, String)],
    /// Selected value(s) when triggered by a select menu.
    pub selected: Option<&'a str>,
    /// Emoji rendered as text (`👍` or `<:name:id>`) when triggered by a reaction.
    pub emoji: Option<&'a str>,
    /// Modal text input values as `(custom_id, value)`.
    pub modal_fields: &'a [(String, String)],
    /// Right-clicked target ID (user ID for user command, message ID for message command).
    pub target_id: Option<u64>,
    /// Target mention string (`<@id>` for user command).
    pub target_mention: Option<&'a str>,
    /// Target display name (for user command) or author name (for message command).
    pub target_name: Option<&'a str>,
    /// Target message content (for message command).
    pub target_content: Option<&'a str>,
    /// Target message author ID (for message command).
    pub target_author_id: Option<u64>,
    /// Text content on a message event (e.g. edited message).
    pub message_content: Option<&'a str>,
    /// Name of a newly created thread.
    pub thread_name: Option<&'a str>,
    /// Code of a newly created invite.
    pub invite_code: Option<&'a str>,
    /// ID of the AutoMod rule that executed.
    pub automod_rule_id: Option<u64>,
    /// Keyword matched by AutoMod.
    pub automod_keyword: Option<&'a str>,
    /// Content flagged by AutoMod.
    pub automod_content: Option<&'a str>,
    /// Poll answer ID voted for.
    pub poll_answer_id: Option<u8>,
}

impl Context<'_> {
    /// Value of a built-in placeholder such as `user`, `option.NAME`, `modal.ID` or `selected`.
    pub(crate) fn builtin(&self, key: &str) -> Option<String> {
        match key {
            "user" | "user.mention" => self.user_id.map(|id| format!("<@{id}>")),
            "user.name" => self.user_name.map(str::to_owned),
            "user.id" => self.user_id.map(|id| id.to_string()),
            "server.id" => self.guild_id.map(|id| id.to_string()),
            "channel" => self.channel_id.map(|id| format!("<#{id}>")),
            "channel.id" => self.channel_id.map(|id| id.to_string()),
            "message.id" => self.message_id.map(|id| id.to_string()),
            "message.content" => self.message_content.map(str::to_owned),
            "selected" => self.selected.map(str::to_owned),
            "emoji" => self.emoji.map(str::to_owned),
            "target" | "target.mention" => self.target_mention.map(str::to_owned),
            "target.id" => self.target_id.map(|id| id.to_string()),
            "target.name" => self.target_name.map(str::to_owned),
            "target.content" => self.target_content.map(str::to_owned),
            "target.author" => self.target_author_id.map(|id| format!("<@{id}>")),
            "target.author.id" => self.target_author_id.map(|id| id.to_string()),
            "thread.name" => self.thread_name.map(str::to_owned),
            "invite.code" => self.invite_code.map(str::to_owned),
            "automod.rule_id" => self.automod_rule_id.map(|id| id.to_string()),
            "automod.keyword" => self.automod_keyword.map(str::to_owned),
            "automod.content" => self.automod_content.map(str::to_owned),
            "poll.answer_id" => self.poll_answer_id.map(|id| id.to_string()),
            _ => {
                if let Some(name) = key.strip_prefix("option.") {
                    return Some(self.option(name).unwrap_or_default().to_owned());
                }
                if let Some(id) = key.strip_prefix("modal.") {
                    return Some(self.modal_field(id).unwrap_or_default().to_owned());
                }
                None
            }
        }
    }

    pub(crate) fn option(&self, name: &str) -> Option<&str> {
        self.options
            .iter()
            .find(|(option, _)| option == name)
            .map(|(_, value)| value.as_str())
    }

    pub(crate) fn modal_field(&self, id: &str) -> Option<&str> {
        self.modal_fields
            .iter()
            .find(|(field, _)| field == id)
            .map(|(_, value)| value.as_str())
    }
}

/// Replace `{placeholders}` that `lookup` knows, leave the others untouched,
/// and cut the result to Discord's message limit.
#[cfg(test)]
pub(crate) fn render(template: &str, lookup: impl Fn(&str) -> Option<String>) -> String {
    render_max(template, MAX_MESSAGE_LEN, lookup)
}

/// Replace `{placeholders}` that `lookup` knows, leave the others untouched,
/// and cut the result to `max_chars` characters.
pub(crate) fn render_max(
    template: &str,
    max_chars: usize,
    lookup: impl Fn(&str) -> Option<String>,
) -> String {
    let mut output = String::with_capacity(template.len());
    let mut rest = template;

    while let Some(start) = rest.find('{') {
        output.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let replacement = after
            .find(['}', '{'])
            .filter(|&end| after.as_bytes()[end] == b'}')
            .and_then(|end| lookup(&after[..end]).map(|value| (value, end)));

        if let Some((value, end)) = replacement {
            output.push_str(&value);
            rest = &after[end + 1..];
        } else {
            output.push('{');
            rest = after;
        }
    }
    output.push_str(rest);

    match output.char_indices().nth(max_chars) {
        Some((cut, _)) => output[..cut].to_owned(),
        None => output,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context(options: &[(String, String)]) -> Context<'_> {
        Context {
            user_id: Some(42),
            user_name: Some("ada"),
            roles: &[],
            guild_id: Some(7),
            channel_id: Some(9),
            message_id: Some(11),
            options,
            ..Context::default()
        }
    }

    #[test]
    fn renders_known_placeholders() {
        let options = [("who".to_owned(), "world".to_owned())];
        let modal_fields = [("reason".to_owned(), "because".to_owned())];
        let mut ctx = context(&options);
        ctx.selected = Some("blue");
        ctx.emoji = Some("👍");
        ctx.modal_fields = &modal_fields;
        ctx.target_id = Some(99);
        ctx.target_mention = Some("<@99>");
        assert_eq!(
            render(
                "Hi {user} ({user.name}) in {channel} on {message.id}, hello {option.who}! Picked {selected} {emoji} [{modal.reason}] -> {target}",
                |key| ctx.builtin(key)
            ),
            "Hi <@42> (ada) in <#9> on 11, hello world! Picked blue 👍 [because] -> <@99>"
        );
    }

    #[test]
    fn keeps_unknown_and_unbalanced_braces() {
        let ctx = context(&[]);
        let lookup = |key: &str| ctx.builtin(key);
        assert_eq!(render("{nope} {user", lookup), "{nope} {user");
        assert_eq!(render("{{user}}", lookup), "{<@42>}");
        assert_eq!(render("{option.missing}|{modal.missing}|", lookup), "||");
    }

    #[test]
    fn truncates_to_the_message_limit() {
        let long = "é".repeat(MAX_MESSAGE_LEN + 10);
        assert_eq!(render(&long, |_| None).chars().count(), MAX_MESSAGE_LEN);
    }
}
