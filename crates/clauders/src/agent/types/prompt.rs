//! User prompt input for a single agent turn.

/// The text of one user turn sent to the agent.
///
/// Accepts plain UTF-8 text; richer structured prompts are a future additive
/// change. `Prompt` implements `From<&str>` and `From<String>` so
/// call sites can pass either via `impl Into<Prompt>`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Prompt(String);

impl Prompt {
    /// Wrap prompt text.
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self(text.into())
    }

    /// Borrow the prompt text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume the prompt, yielding the owned text.
    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl From<&str> for Prompt {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for Prompt {
    fn from(s: String) -> Self {
        Self(s)
    }
}

#[cfg(test)]
mod tests {
    use super::Prompt;

    #[test]
    fn from_str_slice() {
        let p: Prompt = "hello".into();
        assert_eq!(p.as_str(), "hello");
    }

    #[test]
    fn from_owned_string() {
        let p: Prompt = String::from("hi there").into();
        assert_eq!(p.as_str(), "hi there");
    }

    #[test]
    fn into_inner_yields_text() {
        let p = Prompt::new("payload");
        assert_eq!(p.into_inner(), "payload");
    }
}
