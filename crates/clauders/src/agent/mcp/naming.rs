//! MCP tool-name namespacing shared by the native runtimes.
//!
//! In-process MCP tools are exposed to a model under a namespaced wire name,
//! `mcp__<server>__<tool>`, so cross-server tool-name collisions are impossible
//! and the name matches the convention models are trained against. Both native
//! runtimes (`api`, `openrouter`) declare and route tool calls through these
//! helpers, keeping one source of truth for the convention.
#![expect(
    clippy::redundant_pub_crate,
    reason = "explicit pub(crate) documents the crate-wide visibility intent at each item"
)]

/// The wire tool-name prefix separating the MCP namespace segments.
const NS_SEP: &str = "__";
/// The leading marker identifying an in-process MCP tool on the wire.
const NS_PREFIX: &str = "mcp__";

/// The wire name for an in-process tool: `mcp__<server>__<tool>`.
pub(crate) fn declare_name(server: &str, tool: &str) -> String {
    format!("{NS_PREFIX}{server}{NS_SEP}{tool}")
}

/// A wire tool name that does not decode to a `(server, tool)` pair.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Unroutable;

/// Split a `mcp__<server>__<tool>` wire name back into `(server, tool)`.
///
/// # Errors
/// Returns [`Unroutable`] when `name` lacks the `mcp__` prefix or a `__`
/// separator between the server and tool segments.
pub(crate) fn route(name: &str) -> Result<(&str, &str), Unroutable> {
    let rest = name.strip_prefix(NS_PREFIX).ok_or(Unroutable)?;
    rest.split_once(NS_SEP).ok_or(Unroutable)
}

#[cfg(test)]
mod tests {
    #![expect(clippy::expect_used, reason = "test assertions use expect for context")]

    use super::{declare_name, route};

    #[test]
    fn declare_name_prefixes_server_and_tool() {
        assert_eq!(declare_name("calc", "add"), "mcp__calc__add");
    }

    #[test]
    fn route_splits_a_declared_name() {
        let (server, tool) = route("mcp__calc__add").expect("routable");
        assert_eq!(server, "calc");
        assert_eq!(tool, "add");
    }

    #[test]
    fn route_round_trips_declare_name() {
        let name = declare_name("files", "read");
        let (server, tool) = route(&name).expect("routable");
        assert_eq!((server, tool), ("files", "read"));
    }

    #[test]
    fn route_rejects_an_unprefixed_name() {
        assert!(route("bash").is_err());
        assert!(route("calc__add").is_err());
    }
}
