/// Discord's message content limit, in characters.
pub const MAX_MESSAGE_LEN: usize = 2000;

/// Values available to `{placeholders}` in message templates.
#[derive(Debug, Clone, Copy)]
pub struct Context<'a> {
    pub user_id: u64,
    pub user_name: &'a str,
    pub guild_id: Option<u64>,
    pub channel_id: Option<u64>,
    /// Slash command options as `(name, rendered value)`.
    pub options: &'a [(String, String)],
}

impl Context<'_> {
    fn lookup(&self, key: &str) -> Option<String> {
        match key {
            "user" | "user.mention" => Some(format!("<@{}>", self.user_id)),
            "user.name" => Some(self.user_name.to_owned()),
            "user.id" => Some(self.user_id.to_string()),
            "server.id" => self.guild_id.map(|id| id.to_string()),
            "channel" => self.channel_id.map(|id| format!("<#{id}>")),
            "channel.id" => self.channel_id.map(|id| id.to_string()),
            _ => {
                let name = key.strip_prefix("option.")?;
                Some(
                    self.options
                        .iter()
                        .find(|(option, _)| option == name)
                        .map(|(_, value)| value.clone())
                        .unwrap_or_default(),
                )
            }
        }
    }

    /// Replace known `{placeholders}`, leave unknown ones untouched, and cut
    /// the result to Discord's message limit.
    #[must_use]
    pub fn render(&self, template: &str) -> String {
        let mut output = String::with_capacity(template.len());
        let mut rest = template;

        while let Some(start) = rest.find('{') {
            output.push_str(&rest[..start]);
            let after = &rest[start + 1..];
            let replacement = after
                .find(['}', '{'])
                .filter(|&end| after.as_bytes()[end] == b'}')
                .and_then(|end| self.lookup(&after[..end]).map(|value| (value, end)));

            if let Some((value, end)) = replacement {
                output.push_str(&value);
                rest = &after[end + 1..];
            } else {
                output.push('{');
                rest = after;
            }
        }
        output.push_str(rest);

        match output.char_indices().nth(MAX_MESSAGE_LEN) {
            Some((cut, _)) => output[..cut].to_owned(),
            None => output,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn renders_known_placeholders() {
        let options = [("who".to_owned(), "world".to_owned())];
        let ctx = context(&options);
        assert_eq!(
            ctx.render("Hi {user} ({user.name}) in {channel}, hello {option.who}!"),
            "Hi <@42> (ada) in <#9>, hello world!"
        );
    }

    #[test]
    fn keeps_unknown_and_unbalanced_braces() {
        let ctx = context(&[]);
        assert_eq!(ctx.render("{nope} {user"), "{nope} {user");
        assert_eq!(ctx.render("{{user}}"), "{<@42>}");
        assert_eq!(ctx.render("{option.missing}|"), "|");
    }

    #[test]
    fn truncates_to_the_message_limit() {
        let ctx = context(&[]);
        let long = "é".repeat(MAX_MESSAGE_LEN + 10);
        assert_eq!(ctx.render(&long).chars().count(), MAX_MESSAGE_LEN);
    }
}
