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

    /// Iterate the registered servers (dispatch + declaration source).
    pub(crate) fn servers(&self) -> impl Iterator<Item = &Arc<SdkMcpServer>> {
        self.servers.values()
    }

    /// The `--mcp-config` declaration objects (`{"<name>":{"type":"sdk"}}`),
    /// one per registered server.
    pub(crate) fn declarations(&self) -> impl Iterator<Item = serde_json::Value> + '_ {
        self.servers
            .keys()
            .map(|name| serde_json::json!({ name.as_str(): { "type": "sdk" } }))
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
        assert_eq!(decls[0]["calc"]["type"], "sdk");
    }
}
