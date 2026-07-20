//! User prompt input for one or more agent turns.

use std::fmt;
use std::pin::Pin;

use futures_core::Stream;

/// Input to an agent run: either a single user turn or a stream of user turns.
///
/// `Single` carries one UTF-8 text turn. `Stream` carries user-message texts
/// fed into a live turn as they arrive (streaming input). `Prompt` implements
/// `From<&str>` and `From<String>` — both map to `Single` — so call sites can
/// pass either via `impl Into<Prompt>`; use [`Prompt::stream`] for the
/// streaming form.
pub enum Prompt {
    /// A single user turn of plain UTF-8 text.
    Single(String),
    /// A stream of user-message texts fed into a live turn as they arrive.
    Stream(Pin<Box<dyn Stream<Item = String> + Send + 'static>>),
}

impl Prompt {
    /// Wrap a single turn of prompt text.
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self::Single(text.into())
    }

    /// Wrap a stream of user-message texts as streaming input.
    #[must_use]
    pub fn stream(stream: impl Stream<Item = String> + Send + 'static) -> Self {
        Self::Stream(Box::pin(stream))
    }
}

impl fmt::Debug for Prompt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Single(text) => f.debug_tuple("Prompt::Single").field(text).finish(),
            Self::Stream(_) => f.write_str("Prompt::Stream(<stream>)"),
        }
    }
}

impl From<&str> for Prompt {
    fn from(s: &str) -> Self {
        Self::Single(s.to_string())
    }
}

impl From<String> for Prompt {
    fn from(s: String) -> Self {
        Self::Single(s)
    }
}

#[cfg(test)]
mod tests {
    use super::Prompt;
    use futures_util::stream;

    #[test]
    fn from_str_slice_maps_to_single() {
        let p: Prompt = "hello".into();
        assert!(matches!(p, Prompt::Single(text) if text == "hello"));
    }

    #[test]
    fn from_owned_string_maps_to_single() {
        let p: Prompt = String::from("hi there").into();
        assert!(matches!(p, Prompt::Single(text) if text == "hi there"));
    }

    #[test]
    fn new_maps_to_single() {
        let p = Prompt::new("payload");
        assert!(matches!(p, Prompt::Single(text) if text == "payload"));
    }

    #[test]
    fn stream_ctor_maps_to_stream() {
        let p = Prompt::stream(stream::iter(vec!["a".to_string(), "b".to_string()]));
        assert!(matches!(p, Prompt::Stream(_)));
    }

    #[test]
    fn debug_single_shows_text_stream_shows_placeholder() {
        let single = format!("{:?}", Prompt::new("hi"));
        assert!(single.contains("Single"));
        assert!(single.contains("hi"));
        let streamed = format!("{:?}", Prompt::stream(stream::iter(Vec::<String>::new())));
        assert_eq!(streamed, "Prompt::Stream(<stream>)");
    }
}
