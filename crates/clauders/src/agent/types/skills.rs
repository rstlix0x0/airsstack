//! Skill gating for a session (`skills` startup option).

/// Skill gating.
///
/// Lowers into `--allowed-tools`, not a flag of its own: [`Skills::All`]
/// appends the bare `Skill` tool; [`Skills::List`] appends `Skill(<name>)` for
/// each named skill — matching the official SDK.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Skills {
    /// Enable every skill (`Skill`).
    All,
    /// Enable only the named skills (`Skill(<name>)` each).
    List(Vec<String>),
}

#[cfg(test)]
mod tests {
    use super::Skills;

    #[test]
    fn variants_compare_by_value() {
        assert_eq!(Skills::All, Skills::All);
        assert_eq!(
            Skills::List(vec!["pdf".into(), "xlsx".into()]),
            Skills::List(vec!["pdf".into(), "xlsx".into()])
        );
        assert_ne!(Skills::All, Skills::List(vec!["pdf".into()]));
    }
}
