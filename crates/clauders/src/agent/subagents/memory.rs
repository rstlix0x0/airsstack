//! Memory source for a programmatic subagent.

use serde::Serialize;

/// The memory scope a subagent reads from, mirroring the official
/// `memory: "user" | "project" | "local"` field. `None` on a definition
/// inherits the parent agent's memory scope.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MemorySource {
    /// User-scoped memory.
    User,
    /// Project-scoped memory.
    Project,
    /// Local (working-directory) memory.
    Local,
}

#[cfg(test)]
mod tests {
    #![expect(clippy::expect_used, reason = "test asserts known-valid serialization")]

    use super::MemorySource;

    #[test]
    fn serializes_to_lowercase_wire_values() {
        assert_eq!(
            serde_json::to_value(MemorySource::User).expect("serialize"),
            serde_json::json!("user")
        );
        assert_eq!(
            serde_json::to_value(MemorySource::Project).expect("serialize"),
            serde_json::json!("project")
        );
        assert_eq!(
            serde_json::to_value(MemorySource::Local).expect("serialize"),
            serde_json::json!("local")
        );
    }
}
