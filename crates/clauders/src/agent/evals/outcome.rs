//! The collected result of one single-turn run — the read surface scorers judge.

use crate::agent::content::ContentBlock;
use crate::agent::message::{Message, ResultMessage, Usage};

/// The messages collected from one single-turn run, with a read surface for scorers.
pub struct Outcome {
    messages: Vec<Message>,
}

impl Outcome {
    /// Build an outcome from the drained message frames of one turn.
    pub(crate) const fn from_messages(messages: Vec<Message>) -> Self {
        Self { messages }
    }

    /// Concatenation of every assistant `Text` content block across the turn.
    #[must_use]
    pub fn assistant_text(&self) -> String {
        let mut out = String::new();
        for message in &self.messages {
            if let Message::Assistant(assistant) = message {
                for block in &assistant.content {
                    if let ContentBlock::Text { text } = block {
                        out.push_str(text);
                    }
                }
            }
        }
        out
    }

    /// The terminal `Result` frame, if the turn produced one.
    #[must_use]
    pub fn result(&self) -> Option<&ResultMessage> {
        self.messages.iter().find_map(|message| match message {
            Message::Result(result) => Some(result),
            _ => None,
        })
    }

    /// Token usage as reported on the `Result` frame.
    #[must_use]
    pub fn usage(&self) -> Option<&Usage> {
        self.result().and_then(|result| result.usage.as_ref())
    }

    /// Names of every `ToolUse` block requested during the turn.
    pub fn tool_uses(&self) -> impl Iterator<Item = &str> {
        self.messages
            .iter()
            .flat_map(|message| match message {
                Message::Assistant(assistant) => assistant.content.as_slice(),
                _ => &[],
            })
            .filter_map(|block| match block {
                ContentBlock::ToolUse { name, .. } => Some(name.as_str()),
                _ => None,
            })
    }

    /// Whether the terminal `Result` frame reported `is_error`.
    #[must_use]
    pub fn is_error(&self) -> bool {
        self.result().is_some_and(|result| result.is_error)
    }
}

#[cfg(test)]
mod tests {
    use super::Outcome;
    use crate::agent::content::ContentBlock;
    use crate::agent::message::{AssistantMessage, Message, ResultMessage, Usage};
    use crate::agent::types::SessionId;

    fn assistant_text(text: &str) -> Message {
        Message::Assistant(AssistantMessage {
            content: vec![ContentBlock::Text { text: text.into() }],
            parent_tool_use_id: None,
        })
    }

    fn assistant_tool(name: &str) -> Message {
        Message::Assistant(AssistantMessage {
            content: vec![ContentBlock::ToolUse {
                id: "1".into(),
                name: name.into(),
                input: serde_json::Value::Null,
            }],
            parent_tool_use_id: None,
        })
    }

    fn result_frame(is_error: bool, usage: Option<Usage>) -> Message {
        Message::Result(ResultMessage {
            result: "done".into(),
            is_error,
            total_cost_usd: None,
            stop_reason: None,
            usage,
            session_id: SessionId::new("s1"),
            num_turns: 1,
        })
    }

    #[test]
    fn assistant_text_concatenates_text_blocks() {
        let o = Outcome::from_messages(vec![assistant_text("hel"), assistant_text("lo")]);
        assert_eq!(o.assistant_text(), "hello");
    }

    #[test]
    fn tool_uses_lists_tool_names() {
        let o = Outcome::from_messages(vec![assistant_tool("search"), assistant_text("x")]);
        let names: Vec<&str> = o.tool_uses().collect();
        assert_eq!(names, vec!["search"]);
    }

    #[test]
    fn usage_and_is_error_read_the_result_frame() {
        let o = Outcome::from_messages(vec![result_frame(
            true,
            Some(Usage {
                input_tokens: 10,
                output_tokens: 5,
                ..Default::default()
            }),
        )]);
        assert!(o.is_error());
        assert_eq!(
            o.usage().map(|u| u.input_tokens + u.output_tokens),
            Some(15)
        );
    }

    #[test]
    fn missing_result_frame_yields_no_usage_and_no_error() {
        let o = Outcome::from_messages(vec![assistant_text("x")]);
        assert!(o.usage().is_none());
        assert!(!o.is_error());
    }
}
