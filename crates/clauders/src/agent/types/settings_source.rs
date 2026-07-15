//! Source of the binary's session settings (`--settings`).

use std::path::PathBuf;

/// Where the binary's session settings come from.
///
/// Lowers to the binary's `--settings <file-or-json>` flag: a filesystem path
/// to a settings JSON file, or an inline settings JSON value serialized onto
/// the argument. (The official TypeScript SDK accepts both forms; Python
/// accepts a path only — this enum is the superset.)
#[derive(Clone, Debug, PartialEq)]
// `serde_json::Value` does not implement `Eq` (it wraps f64), so `Eq` cannot
// be derived here.
#[expect(
    clippy::derive_partial_eq_without_eq,
    reason = "serde_json::Value does not implement Eq; cannot derive it for this enum"
)]
pub enum SettingsSource {
    /// Path to a settings JSON file.
    Path(PathBuf),
    /// Inline settings JSON, serialized compactly onto the flag argument.
    Inline(serde_json::Value),
}

#[cfg(test)]
mod tests {
    use super::SettingsSource;

    #[test]
    fn path_and_inline_are_distinct_and_comparable() {
        let p = SettingsSource::Path("/etc/claude/settings.json".into());
        let i = SettingsSource::Inline(serde_json::json!({ "theme": "dark" }));
        assert_eq!(p, SettingsSource::Path("/etc/claude/settings.json".into()));
        assert_ne!(p, i);
        assert_eq!(
            i,
            SettingsSource::Inline(serde_json::json!({ "theme": "dark" }))
        );
    }
}
