//! Pure impedance mapping between the wire Messages API and the agent frame
//! surface: tool-name namespacing, content-block and usage conversion, and
//! error mapping. No I/O, no transport — the unit-test seam of the runtime.
// `dead_code` fires on the lib target (nothing outside `#[cfg(test)]` calls
// these yet) but not on the test target (every item is exercised by `tests`
// below). Because the lint fires conditionally across targets, `#[expect]`
// would be reported "unfulfilled" by the test-target pass; per
// M-LINT-OVERRIDE-EXPECT, `#[allow]` is the correct suppression for a
// conditionally-firing lint. Drop it once a runtime caller makes these live.
#![allow(dead_code)]

/// The wire tool-name prefix separating the MCP namespace segments.
const NS_SEP: &str = "__";
/// The leading marker identifying an in-process MCP tool on the wire.
const NS_PREFIX: &str = "mcp__";

/// The Messages-API wire name for an in-process tool: `mcp__<server>__<tool>`.
///
/// Prefixing by server makes cross-server tool-name collisions impossible and
/// matches the naming convention the models are trained against.
pub(super) fn declare_name(server: &str, tool: &str) -> String {
    format!("{NS_PREFIX}{server}{NS_SEP}{tool}")
}

/// A wire tool name that does not decode to a `(server, tool)` pair.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Unroutable;

/// Split a `mcp__<server>__<tool>` wire name back into `(server, tool)`.
///
/// # Errors
/// Returns [`Unroutable`] when `name` lacks the `mcp__` prefix or a
/// `__` separator between the server and tool segments.
pub(super) fn route(name: &str) -> Result<(&str, &str), Unroutable> {
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
