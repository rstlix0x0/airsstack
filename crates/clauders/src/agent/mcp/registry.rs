//! The per-session registry of in-process MCP servers.

use std::collections::HashMap;
use std::sync::Arc;

use super::server::SdkMcpServer;

/// The in-process MCP servers registered for a session, keyed by name.
///
/// Mirrors the hook registry: cheap to clone (servers are `Arc`-shared), and
/// the single source of both dispatch lookups and the argv declarations.
#[derive(Clone, Default)]
pub struct SdkMcpRegistry {
    servers: HashMap<String, Arc<SdkMcpServer>>,
}

impl SdkMcpRegistry {
    /// Register `server`. A later server with the same name replaces an earlier one.
    pub fn register(&mut self, server: SdkMcpServer) -> &mut Self {
        self.servers
            .insert(server.name().to_string(), Arc::new(server));
        self
    }

    /// Whether any server is registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.servers.is_empty()
    }

    /// Look up a server by name.
    pub(crate) fn lookup(&self, server_name: &str) -> Option<Arc<SdkMcpServer>> {
        self.servers.get(server_name).map(Arc::clone)
    }

    /// The `--mcp-config` declaration objects
    /// (`{"mcpServers":{"<name>":{"type":"sdk"}}}`), one per registered server.
    ///
    /// The `mcpServers` wrapper is required: the binary validates each
    /// `--mcp-config` payload against a schema whose only key is `mcpServers`,
    /// and rejects an unwrapped server map at startup with
    /// `Invalid MCP configuration: mcpServers: Invalid input: expected record,
    /// received undefined` before any session begins.
    pub(crate) fn declarations(&self) -> impl Iterator<Item = serde_json::Value> + '_ {
        self.servers
            .keys()
            .map(|name| serde_json::json!({ "mcpServers": { name.as_str(): { "type": "sdk" } } }))
    }
}

#[cfg(test)]
mod tests {
    use super::SdkMcpRegistry;
    use crate::agent::mcp::server::SdkMcpServer;

    #[test]
    fn empty_by_default() {
        assert!(SdkMcpRegistry::default().is_empty());
    }

    #[test]
    fn register_and_lookup() {
        let mut reg = SdkMcpRegistry::default();
        reg.register(SdkMcpServer::builder("calc").build());
        assert!(!reg.is_empty());
        assert!(reg.lookup("calc").is_some());
        assert!(reg.lookup("nope").is_none());
    }

    #[test]
    fn declarations_mark_type_sdk() {
        let mut reg = SdkMcpRegistry::default();
        reg.register(SdkMcpServer::builder("calc").build());
        let decls: Vec<_> = reg.declarations().collect();
        assert_eq!(decls.len(), 1);
        assert_eq!(decls[0]["mcpServers"]["calc"]["type"], "sdk");
    }

    // The binary rejects a `--mcp-config` payload that is not wrapped in
    // `mcpServers`, exiting before the session starts. Assert the wrapper is
    // the only top-level key so the unwrapped shape cannot come back.
    #[test]
    fn declarations_wrap_servers_under_mcp_servers_key() {
        let mut reg = SdkMcpRegistry::default();
        reg.register(SdkMcpServer::builder("calc").build());
        let decls: Vec<_> = reg.declarations().collect();
        let keys: Vec<&str> = decls[0]
            .as_object()
            .map(|object| object.keys().map(String::as_str).collect())
            .unwrap_or_default();
        assert_eq!(
            keys,
            vec!["mcpServers"],
            "the server map must be wrapped, not emitted at the top level"
        );
        assert!(
            decls[0].get("calc").is_none(),
            "the server name must not appear at the top level"
        );
    }
}
