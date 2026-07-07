//! Pure impedance mapping between the wire Messages API and the agent frame
//! surface: tool-name namespacing, content-block and usage conversion, and
//! error mapping. No I/O, no transport — the unit-test seam of the runtime.

use crate::agent::content::ContentBlock as AgentBlock;
use crate::agent::error::AgentError;
use crate::agent::message::Usage as AgentUsage;
use crate::error::Error as WireError;
use crate::messages::content::ContentBlock as WireBlock;
use crate::messages::response::{StopReason, Usage as WireUsage};

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

/// Map one wire content block to its agent-frame counterpart.
pub(super) fn content_block(block: &WireBlock) -> AgentBlock {
    match block {
        WireBlock::Text(t) => AgentBlock::Text {
            text: t.text.clone(),
        },
        WireBlock::Thinking(t) => AgentBlock::Thinking {
            thinking: t.thinking.clone(),
        },
        WireBlock::ToolUse(u) => AgentBlock::ToolUse {
            id: u.id.as_str().to_string(),
            name: u.name.as_str().to_string(),
            input: u.input.clone(),
        },
        WireBlock::ToolResult(r) => AgentBlock::ToolResult {
            tool_use_id: r.tool_use_id.as_str().to_string(),
            content: serde_json::to_value(&r.content).unwrap_or(serde_json::Value::Null),
            is_error: r.is_error.unwrap_or(false),
        },
    }
}

/// The agent-frame usage subset (input/output tokens) from a wire `Usage`.
pub(super) fn usage(u: &WireUsage) -> AgentUsage {
    AgentUsage {
        input_tokens: u64::from(u.input_tokens),
        output_tokens: u64::from(u.output_tokens),
    }
}

/// Concatenate the text of every `Text` block, in order — the result summary.
pub(super) fn last_text(blocks: &[WireBlock]) -> String {
    blocks
        .iter()
        .filter_map(|b| match b {
            WireBlock::Text(t) => Some(t.text.as_str()),
            _ => None,
        })
        .collect()
}

/// The wire string name for a stop reason, as the agent result frame carries it.
pub(super) const fn stop_reason_wire(reason: StopReason) -> &'static str {
    match reason {
        StopReason::EndTurn => "end_turn",
        StopReason::MaxTokens => "max_tokens",
        StopReason::StopSequence => "stop_sequence",
        StopReason::ToolUse => "tool_use",
        StopReason::Refusal => "refusal",
    }
}

/// Fold a wire-client error into the (CLI-centric) agent error surface.
pub(super) fn map_wire_error(error: WireError) -> AgentError {
    match error {
        WireError::Transport(_) => AgentError::TransportClosed,
        WireError::Serde { source, .. } => AgentError::Decode(source.to_string()),
        other => AgentError::Protocol {
            detail: other.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    #![expect(clippy::expect_used, reason = "test assertions use expect for context")]
    #![expect(clippy::panic, reason = "test failure signal via panic in match arms")]

    use super::{
        content_block, declare_name, last_text, map_wire_error, route, stop_reason_wire, usage,
    };
    use crate::agent::content::ContentBlock as AgentBlock;
    use crate::messages::content::{ContentBlock as WireBlock, TextBlock};
    use crate::messages::response::{StopReason, Usage as WireUsage};

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

    #[test]
    fn maps_a_text_block() {
        let wire = WireBlock::Text(TextBlock::new("hi"));
        assert!(matches!(content_block(&wire), AgentBlock::Text { text } if text == "hi"));
    }

    #[test]
    fn maps_a_tool_use_block() {
        use crate::messages::tools::ToolUseBlock;
        use crate::types::{ToolName, ToolUseId};
        let wire = WireBlock::ToolUse(ToolUseBlock {
            id: ToolUseId::new("toolu_1").expect("id"),
            name: ToolName::new("mcp__calc__add").expect("name"),
            input: serde_json::json!({"a": 1}),
            cache_control: None,
        });
        match content_block(&wire) {
            AgentBlock::ToolUse { id, name, input } => {
                assert_eq!(id, "toolu_1");
                assert_eq!(name, "mcp__calc__add");
                assert_eq!(input["a"], 1);
            }
            other => panic!("expected ToolUse, got {other:?}"),
        }
    }

    #[test]
    fn maps_usage_subset() {
        let wire: WireUsage =
            serde_json::from_str(r#"{"input_tokens":25,"output_tokens":5}"#).expect("usage");
        let agent = usage(&wire);
        assert_eq!(agent.input_tokens, 25);
        assert_eq!(agent.output_tokens, 5);
    }

    #[test]
    fn last_text_joins_trailing_text_blocks() {
        let blocks = vec![
            WireBlock::Text(TextBlock::new("a")),
            WireBlock::Text(TextBlock::new("b")),
        ];
        assert_eq!(last_text(&blocks), "ab");
    }

    #[test]
    fn stop_reason_wire_names() {
        assert_eq!(stop_reason_wire(StopReason::EndTurn), "end_turn");
        assert_eq!(stop_reason_wire(StopReason::ToolUse), "tool_use");
    }

    #[test]
    fn maps_wire_errors_to_agent_errors() {
        use crate::agent::error::AgentError;
        let api = map_wire_error(crate::error::Error::InvalidRequest("bad url".into()));
        assert!(matches!(api, AgentError::Protocol { .. }));
    }
}
