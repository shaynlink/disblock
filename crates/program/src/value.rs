use std::fmt;

use serde::{Deserialize, Serialize};

/// A value held by a variable or computed by an expression.
///
/// Blocks are loosely typed, like spreadsheet cells: text that looks like a
/// number counts as that number, and every value can be shown in a message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    Bool(bool),
    Number(i64),
    Text(String),
}

impl Default for Value {
    fn default() -> Self {
        Self::Text(String::new())
    }
}

impl Value {
    /// The value as a whole number, if it is one or reads as one.
    #[must_use]
    pub fn as_number(&self) -> Option<i64> {
        match self {
            Self::Number(number) => Some(*number),
            Self::Bool(value) => Some(i64::from(*value)),
            Self::Text(text) => text.trim().parse().ok(),
        }
    }

    /// Whether the value counts as "yes": `true`, a non-zero number, or text
    /// other than empty, `0`, `false` and `no`.
    #[must_use]
    pub fn is_truthy(&self) -> bool {
        match self {
            Self::Bool(value) => *value,
            Self::Number(number) => *number != 0,
            Self::Text(text) => {
                let text = text.trim();
                !(text.is_empty()
                    || text == "0"
                    || text.eq_ignore_ascii_case("false")
                    || text.eq_ignore_ascii_case("no"))
            }
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bool(true) => formatter.write_str("yes"),
            Self::Bool(false) => formatter.write_str("no"),
            Self::Number(number) => write!(formatter, "{number}"),
            Self::Text(text) => formatter.write_str(text),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_between_kinds() {
        assert_eq!(Value::Text(" 42 ".to_owned()).as_number(), Some(42));
        assert_eq!(Value::Text("abc".to_owned()).as_number(), None);
        assert_eq!(Value::Bool(true).as_number(), Some(1));
        assert!(!Value::Text("No".to_owned()).is_truthy());
        assert!(Value::Text("ok".to_owned()).is_truthy());
        assert_eq!(Value::Bool(false).to_string(), "no");
    }

    #[test]
    fn stores_as_plain_json() {
        assert_eq!(serde_json::to_string(&Value::Number(3)).unwrap(), "3");
        assert_eq!(
            serde_json::from_str::<Value>("\"hi\"").unwrap(),
            Value::Text("hi".to_owned())
        );
        assert_eq!(
            serde_json::from_str::<Value>("true").unwrap(),
            Value::Bool(true)
        );
    }
}
