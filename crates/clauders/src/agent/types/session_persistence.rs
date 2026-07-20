//! Whether the binary persists the session for later resumption.

/// Whether the binary persists this session for later resumption.
///
/// The binary persists by default; [`Self::Disabled`] lowers to
/// `--no-session-persistence`. Modelled as an enum rather than a `bool` so
/// the non-`false` default cannot be lost to a derived `Default`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SessionPersistence {
    /// The session is persisted and can be resumed later.
    #[default]
    Enabled,
    /// The session is ephemeral.
    Disabled,
}

#[cfg(test)]
mod tests {
    use super::SessionPersistence;

    #[test]
    fn defaults_to_enabled() {
        assert_eq!(SessionPersistence::default(), SessionPersistence::Enabled);
    }

    #[test]
    fn variants_are_distinct() {
        assert_ne!(SessionPersistence::Enabled, SessionPersistence::Disabled);
    }
}
