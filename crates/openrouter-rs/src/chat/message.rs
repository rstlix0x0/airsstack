//! Request-side chat message building blocks: author role, message content, and
//! content parts.
//!
//! Exists as its own file because these are the pieces a caller assembles to
//! build a [`crate::chat::ChatRequest`]; the response-side message lives
//! separately in `response.rs` because it decodes a different shape.
//!
//! Responsibilities:
//! - [`Role`] — the author of a message (system / user / assistant / tool).
//! - [`MessageContent`] — a bare string or a list of [`ContentPart`]s.
//! - [`ContentPart`] — one piece of structured content (text only in this release).
//! - [`Message`] — a role paired with content, with role-named constructors.
//!
//! Not responsible for decoding responses — see `response.rs`.

use serde::{Deserialize, Serialize};

/// The author role of a chat message.
///
/// # Examples
/// ```
/// use openrouter_rs::chat::Role;
/// assert_eq!(serde_json::to_string(&Role::Assistant).unwrap(), "\"assistant\"");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// System / developer instruction.
    System,
    /// End-user turn.
    User,
    /// Model turn.
    Assistant,
    /// Tool-result turn.
    Tool,
}

/// The content of a chat message: either a bare string or a list of parts.
///
/// The bare-string form serializes to a JSON string; the parts form serializes
/// to a JSON array. Most messages are plain text — reach for the parts form when
/// a message needs structured content.
///
/// # Examples
/// ```
/// use openrouter_rs::chat::MessageContent;
/// let c: MessageContent = "hello".into();
/// assert_eq!(serde_json::to_value(&c).unwrap(), serde_json::json!("hello"));
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    /// A single text string.
    Text(String),
    /// An ordered list of content parts.
    Parts(Vec<ContentPart>),
}

impl From<String> for MessageContent {
    fn from(s: String) -> Self {
        Self::Text(s)
    }
}

impl From<&str> for MessageContent {
    fn from(s: &str) -> Self {
        Self::Text(s.to_owned())
    }
}

impl From<Vec<ContentPart>> for MessageContent {
    fn from(parts: Vec<ContentPart>) -> Self {
        Self::Parts(parts)
    }
}

/// One structured piece of a message's content.
///
/// Only the `text` variant exists in this release; it serializes to
/// `{ "type": "text", "text": "…" }`.
///
/// # Examples
/// ```
/// use openrouter_rs::chat::ContentPart;
/// let part = ContentPart::text("hi");
/// assert_eq!(
///     serde_json::to_value(&part).unwrap(),
///     serde_json::json!({ "type": "text", "text": "hi" }),
/// );
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ContentPart {
    /// A text fragment.
    Text {
        /// The text payload.
        text: String,
    },
}

impl ContentPart {
    /// Build a text content part.
    #[must_use]
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text { text: text.into() }
    }
}

/// A chat message: an author role paired with content.
///
/// Use the role-named constructors ([`Message::user`], [`Message::system`],
/// [`Message::assistant`], [`Message::tool`]). Each accepts anything convertible
/// into [`MessageContent`] — a `&str`/`String` for plain text, or a
/// `Vec<ContentPart>` for structured content.
///
/// # Examples
/// ```
/// use openrouter_rs::chat::Message;
/// let m = Message::user("what is 2 + 2?");
/// assert_eq!(
///     serde_json::to_value(&m).unwrap(),
///     serde_json::json!({ "role": "user", "content": "what is 2 + 2?" }),
/// );
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    pub(crate) role: Role,
    pub(crate) content: MessageContent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) name: Option<String>,
}

impl Message {
    /// Build a message with an explicit role.
    #[must_use]
    pub fn new(role: Role, content: impl Into<MessageContent>) -> Self {
        Self {
            role,
            content: content.into(),
            name: None,
        }
    }

    /// Build a `system` message.
    #[must_use]
    pub fn system(content: impl Into<MessageContent>) -> Self {
        Self::new(Role::System, content)
    }

    /// Build a `user` message.
    #[must_use]
    pub fn user(content: impl Into<MessageContent>) -> Self {
        Self::new(Role::User, content)
    }

    /// Build an `assistant` message.
    #[must_use]
    pub fn assistant(content: impl Into<MessageContent>) -> Self {
        Self::new(Role::Assistant, content)
    }

    /// Build a `tool` message.
    #[must_use]
    pub fn tool(content: impl Into<MessageContent>) -> Self {
        Self::new(Role::Tool, content)
    }

    /// Attach an optional participant `name` to this message.
    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// The author role.
    #[must_use]
    pub const fn role(&self) -> Role {
        self.role
    }

    /// The message content.
    #[must_use]
    pub const fn content(&self) -> &MessageContent {
        &self.content
    }
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::unwrap_used,
        reason = "tests unwrap known-valid fixtures; a panic is the intended failure signal"
    )]

    use super::*;
    use serde_json::json;

    #[test]
    fn role_serializes_lowercase() {
        assert_eq!(serde_json::to_value(Role::System).unwrap(), json!("system"));
        assert_eq!(serde_json::to_value(Role::Tool).unwrap(), json!("tool"));
    }

    #[test]
    fn text_content_serializes_as_bare_string() {
        let c = MessageContent::from("hi");
        assert_eq!(serde_json::to_value(&c).unwrap(), json!("hi"));
    }

    #[test]
    fn parts_content_serializes_as_array() {
        let c = MessageContent::from(vec![ContentPart::text("a"), ContentPart::text("b")]);
        assert_eq!(
            serde_json::to_value(&c).unwrap(),
            json!([{ "type": "text", "text": "a" }, { "type": "text", "text": "b" }]),
        );
    }

    #[test]
    fn content_round_trips_both_shapes() {
        for c in [
            MessageContent::Text("x".into()),
            MessageContent::Parts(vec![ContentPart::text("y")]),
        ] {
            let v = serde_json::to_value(&c).unwrap();
            let back: MessageContent = serde_json::from_value(v).unwrap();
            assert_eq!(back, c);
        }
    }

    #[test]
    fn role_constructors_set_role_and_content() {
        assert_eq!(Message::system("s").role(), Role::System);
        assert_eq!(Message::user("u").role(), Role::User);
        assert_eq!(Message::assistant("a").role(), Role::Assistant);
        assert_eq!(Message::tool("t").role(), Role::Tool);
        assert_eq!(
            Message::user("u").content(),
            &MessageContent::Text("u".into())
        );
    }

    #[test]
    fn name_is_omitted_when_absent_and_present_when_set() {
        assert_eq!(
            serde_json::to_value(Message::user("hi")).unwrap(),
            json!({ "role": "user", "content": "hi" }),
        );
        assert_eq!(
            serde_json::to_value(Message::user("hi").with_name("alice")).unwrap(),
            json!({ "role": "user", "content": "hi", "name": "alice" }),
        );
    }
}
