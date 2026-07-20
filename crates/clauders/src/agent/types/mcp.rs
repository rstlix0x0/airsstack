//! External MCP server configuration (opaque pass-through) and status.
//!
//! In-process MCP tools are unimplemented; external MCP servers are forwarded
//! to the binary opaquely — the SDK carries the raw JSON config untouched, so
//! a newer binary's config shape needs no SDK change.

use serde::{Deserialize, Serialize};

/// An external MCP server configuration, forwarded opaquely to the binary.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
// `serde_json::Value` does not implement `Eq` (it wraps f64), so `Eq` cannot
// be derived here.
#[expect(
    clippy::derive_partial_eq_without_eq,
    reason = "serde_json::Value does not implement Eq; cannot derive it for this struct"
)]
pub struct McpServerConfig {
    name: String,
    config: serde_json::Value,
}

impl McpServerConfig {
    /// Pair a server name with its opaque JSON configuration.
    #[must_use]
    pub fn new(name: impl Into<String>, config: serde_json::Value) -> Self {
        Self {
            name: name.into(),
            config,
        }
    }

    /// The server's logical name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The opaque configuration forwarded to the binary.
    #[must_use]
    pub const fn config(&self) -> &serde_json::Value {
        &self.config
    }
}

/// Connection status of a single MCP server, parsed from a status response.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerStatus {
    /// The server's logical name.
    pub name: String,
    /// The binary-reported status string (e.g. `connected`, `failed`).
    pub status: String,
}

/// Aggregate MCP status returned by the `mcp_status` control request.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpStatus {
    /// Per-server status entries.
    ///
    /// The binary spells this key `mcpServers` on the wire; the rename is
    /// required for the field to bind at all.
    #[serde(rename = "mcpServers", default)]
    pub servers: Vec<ServerStatus>,
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::expect_used,
        reason = "tests assert known-valid fixtures; a panic is the intended failure signal"
    )]

    use super::{McpServerConfig, McpStatus, ServerStatus};

    #[test]
    fn config_is_opaque_passthrough() {
        let raw = serde_json::json!({"command": "node", "args": ["server.js"]});
        let cfg = McpServerConfig::new("fs", raw.clone());
        assert_eq!(cfg.name(), "fs");
        assert_eq!(cfg.config(), &raw);
    }

    #[test]
    fn server_status_deserializes_from_frame() {
        let json = r#"{"name":"fs","status":"connected"}"#;
        let status: ServerStatus = serde_json::from_str(json).expect("deserialize");
        assert_eq!(status.name, "fs");
        assert_eq!(status.status, "connected");
    }

    #[test]
    fn status_binds_the_wire_key_the_binary_actually_sends() {
        // The payload shape is the `response` object of an `mcp_status`
        // control response: the binary keys the list `mcpServers`, not
        // `servers`. Binding the wrong key deserializes to an empty list
        // without erroring, so this asserts a non-empty parse.
        let json = r#"{"mcpServers":[{"name":"fs","status":"connected"}]}"#;
        let status: McpStatus = serde_json::from_str(json).expect("deserialize");
        assert_eq!(
            status.servers.len(),
            1,
            "wire key `mcpServers` must bind; an empty list means the field name drifted"
        );
        assert_eq!(status.servers[0].name, "fs");
    }

    #[test]
    fn status_tolerates_a_response_with_no_server_list() {
        let status: McpStatus = serde_json::from_str("{}").expect("deserialize");
        assert!(status.servers.is_empty());
    }
}
