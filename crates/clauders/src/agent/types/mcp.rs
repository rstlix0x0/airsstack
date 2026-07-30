//! External MCP server configuration (opaque pass-through) and status.
//!
//! In-process MCP tools are unimplemented; external MCP servers are forwarded
//! to the binary opaquely — the SDK carries the raw JSON config untouched, so
//! a newer binary's config shape needs no SDK change.

use std::collections::HashMap;

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

/// Connection state of a single MCP server.
///
/// Closed upstream set, kept `#[non_exhaustive]` with an `Unknown` arm so a
/// state this release does not model degrades to an inspectable value rather
/// than failing the decode.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(rename_all = "kebab-case")]
pub enum ServerConnection {
    /// The server is connected and usable.
    Connected,
    /// The server failed to connect.
    Failed,
    /// The server requires authentication before use.
    NeedsAuth,
    /// The server is still connecting.
    Pending,
    /// The server is configured but disabled.
    Disabled,
    /// A status string this release does not model.
    #[serde(untagged)]
    Unknown(String),
}

/// Connection status of a single MCP server, parsed from a status response.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
// `serde_json::Value` does not implement `Eq` (it wraps f64), so `Eq` cannot
// be derived here.
#[expect(
    clippy::derive_partial_eq_without_eq,
    reason = "serde_json::Value does not implement Eq; cannot derive it for this struct"
)]
pub struct ServerStatus {
    /// The server's logical name.
    pub name: String,
    /// The server's connection state.
    pub status: ServerConnection,
    /// Server-reported identity/info block, when present. Opaque.
    #[serde(
        rename = "serverInfo",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub server_info: Option<serde_json::Value>,
    /// Failure detail, when the server reported one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// The server's effective configuration, when present. Opaque.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config: Option<serde_json::Value>,
}

/// Aggregate MCP status returned by the `mcp_status` control request.
///
/// `ServerStatus` cannot derive `Eq` (it carries `Option<serde_json::Value>`
/// fields), so `McpStatus` cannot derive it transitively either; clippy does
/// not flag the missing `Eq` here since it is not derivable.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct McpStatus {
    /// Per-server status entries.
    ///
    /// The binary spells this key `mcpServers` on the wire; the rename is
    /// required for the field to bind at all.
    #[serde(rename = "mcpServers", default)]
    pub servers: Vec<ServerStatus>,
}

/// Result of `set_mcp_servers`: the servers added, removed, and any errors.
///
/// [probe v2.1.216]: the binary builds this response as
/// `{added: string[], removed: string[], errors: Record<string, string>}` —
/// `errors` is an object keyed by server name to an error message string, not
/// an array of strings.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetMcpServersResult {
    /// Server names newly added.
    #[serde(default)]
    pub added: Vec<String>,
    /// Server names removed.
    #[serde(default)]
    pub removed: Vec<String>,
    /// Per-server error messages, keyed by server name, when any server
    /// failed to apply.
    #[serde(default)]
    pub errors: HashMap<String, String>,
}

/// Result of `set_mcp_permission_mode_override`: an optional warning.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetMcpPermissionModeResult {
    /// A warning the binary attached to the mode change, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::expect_used,
        reason = "tests assert known-valid fixtures; a panic is the intended failure signal"
    )]

    use super::{
        McpServerConfig, McpStatus, ServerConnection, ServerStatus, SetMcpPermissionModeResult,
        SetMcpServersResult,
    };

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
        assert_eq!(status.status, ServerConnection::Connected);
    }

    #[test]
    fn status_enum_binds_known_variants_including_hyphenated() {
        // [binary v2.1.216]: `needs-auth` is a status literal; the closed set is
        // connected/failed/needs-auth/pending/disabled.
        let json = r#"{"mcpServers":[
          {"name":"a","status":"connected"},
          {"name":"b","status":"needs-auth"},
          {"name":"c","status":"disabled"}]}"#;
        let s: McpStatus = serde_json::from_str(json).expect("deserialize");
        assert_eq!(s.servers[0].status, ServerConnection::Connected);
        assert_eq!(s.servers[1].status, ServerConnection::NeedsAuth);
        assert_eq!(s.servers[2].status, ServerConnection::Disabled);
    }

    #[test]
    fn unknown_status_string_degrades_to_unknown_not_error() {
        let json = r#"{"mcpServers":[{"name":"a","status":"some_future_state"}]}"#;
        let s: McpStatus = serde_json::from_str(json).expect("must not fail on an unknown status");
        assert_eq!(
            s.servers[0].status,
            ServerConnection::Unknown("some_future_state".into())
        );
    }

    #[test]
    fn server_status_carries_optional_detail_fields() {
        // [binary v2.1.216]: serverInfo is present on the wire alongside status.
        let json = r#"{"name":"a","status":"failed","serverInfo":{"x":1},"error":"boom","config":{"y":2}}"#;
        let st: ServerStatus = serde_json::from_str(json).expect("deserialize");
        assert_eq!(st.status, ServerConnection::Failed);
        assert!(st.server_info.is_some());
        assert_eq!(st.error.as_deref(), Some("boom"));
        assert!(st.config.is_some());
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

    #[test]
    fn set_mcp_permission_mode_result_tolerates_empty_object() {
        // [binary v2.1.216]: setMcpPermissionModeOverride returns `response ?? {}`,
        // so an empty object is the common success payload; `warning` is optional.
        let r: SetMcpPermissionModeResult = serde_json::from_str("{}").expect("deserialize");
        assert!(r.warning.is_none());
        let r2: SetMcpPermissionModeResult =
            serde_json::from_str(r#"{"warning":"deprecated"}"#).expect("deserialize");
        assert_eq!(r2.warning.as_deref(), Some("deprecated"));
    }

    #[test]
    fn set_mcp_servers_result_binds_its_lists() {
        // [probe v2.1.216]: `mcp_set_servers`'s response is built as
        // `{added: string[], removed: string[], errors: Record<string, string>}` —
        // the default seed is `{added:[],removed:[],errors:{}}` and errors are
        // merged with object-spread (`{...u,...m,...L.response.errors}`) keyed by
        // server name to an error message string, not an array of strings.
        let json = r#"{"added":["a"],"removed":["b"],"errors":{"c":"blocked by policy"}}"#;
        let r: SetMcpServersResult = serde_json::from_str(json).expect("deserialize");
        assert_eq!(r.added, vec!["a"]);
        assert_eq!(r.removed, vec!["b"]);
        assert_eq!(
            r.errors.get("c").map(String::as_str),
            Some("blocked by policy")
        );
    }
}
