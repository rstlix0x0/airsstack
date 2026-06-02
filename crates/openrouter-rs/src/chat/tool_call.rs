//! The tool-call carrier shared by request (assistant-replay) and response decode.
//!
//! Exists as its own file because `ToolCall` appears in both the request's
//! assistant-replay messages and the response's `message.tool_calls` array;
//! a single definition avoids duplication and ensures wire shapes match on
//! both sides.
//!
//! Responsibilities:
//! - [`FunctionCall`] — the name and raw JSON-string arguments for a function
//!   invocation.
//! - [`ToolCall`] — a complete tool call with server-issued id, type, and
//!   [`FunctionCall`].
//!
//! Not responsible for parsing the `arguments` string — callers interpret the
//! raw JSON payload. Not responsible for tool definitions — see `tool.rs`.

use serde::{Deserialize, Serialize};

use crate::chat::tool::ToolType;
use crate::types::ToolCallId;

/// The name and raw JSON-string arguments for a function invocation.
///
/// The `arguments` field is the **unparsed JSON string** as returned by the
/// server (e.g. `"{\"q\":\"rust\"}"`) rather than a parsed object. Callers
/// are responsible for deserializing this into the expected parameter type.
/// Keeping it as a `String` matches the wire format and avoids lossy
/// round-trips for partial or streamed payloads.
///
/// # Examples
///
/// ```
/// use openrouter_rs::chat::FunctionCall;
///
/// let fc: FunctionCall = serde_json::from_value(serde_json::json!({
///     "name": "search_books",
///     "arguments": "{\"q\":\"rust programming\"}"
/// })).unwrap();
/// assert_eq!(fc.name, "search_books");
/// assert_eq!(fc.arguments, "{\"q\":\"rust programming\"}");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionCall {
    /// The function's name as echoed by the server.
    pub name: String,
    /// The function arguments as a raw JSON-encoded string.
    ///
    /// This is NOT a parsed JSON object. Callers must deserialize the string
    /// contents into the appropriate type for the function's parameter schema.
    pub arguments: String,
}

/// A complete tool call emitted by the model.
///
/// Appears in the response's `message.tool_calls[]` array and must be echoed
/// back in an assistant-replay [`crate::chat::Message`] before a tool-result
/// message can follow.
///
/// # Examples
///
/// ```
/// use openrouter_rs::chat::{FunctionCall, ToolCall, ToolType};
/// use openrouter_rs::types::ToolCallId;
///
/// let tc: ToolCall = serde_json::from_value(serde_json::json!({
///     "id": "call_abc123",
///     "type": "function",
///     "function": { "name": "search_books", "arguments": "{\"q\":\"rust\"}" }
/// })).unwrap();
/// assert_eq!(tc.id.as_str(), "call_abc123");
/// assert_eq!(tc.r#type, ToolType::Function);
/// assert_eq!(tc.function.name, "search_books");
/// assert_eq!(tc.function.arguments, "{\"q\":\"rust\"}");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolCall {
    /// The server-issued opaque identifier for this tool call.
    pub id: ToolCallId,
    /// The kind of tool (always `function` in the current API).
    pub r#type: ToolType,
    /// The function name and arguments.
    pub function: FunctionCall,
}

// ToolType Deserialize is needed for ToolCall deserialization; add it here.
impl<'de> Deserialize<'de> for ToolType {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        match s.as_str() {
            "function" => Ok(Self::Function),
            other => Err(serde::de::Error::unknown_variant(other, &["function"])),
        }
    }
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::unwrap_used,
        reason = "tests unwrap known-valid fixtures; a panic is the intended failure signal"
    )]

    use super::*;
    use serde_json::json;

    // --- FunctionCall ---

    #[test]
    fn function_call_round_trips_serialize_deserialize() {
        let fc = FunctionCall {
            name: "search_books".into(),
            arguments: r#"{"q":"rust programming"}"#.into(),
        };
        let v = serde_json::to_value(&fc).unwrap();
        assert_eq!(v["name"], json!("search_books"));
        assert_eq!(v["arguments"], json!(r#"{"q":"rust programming"}"#));

        let back: FunctionCall = serde_json::from_value(v).unwrap();
        assert_eq!(back, fc);
    }

    #[test]
    fn arguments_survives_as_exact_string() {
        // Confirm no accidental re-serialization of the JSON inside arguments.
        let raw = r#"{"q":"hello","n":3}"#;
        let fc = FunctionCall {
            name: "fn".into(),
            arguments: raw.into(),
        };
        let v = serde_json::to_value(&fc).unwrap();
        let back: FunctionCall = serde_json::from_value(v).unwrap();
        assert_eq!(back.arguments, raw);
    }

    // --- ToolCall ---

    #[test]
    fn tool_call_deserializes_from_wire_shape() {
        let v = json!({
            "id": "call_abc123",
            "type": "function",
            "function": { "name": "search_books", "arguments": "{\"q\":\"rust\"}" }
        });
        let tc: ToolCall = serde_json::from_value(v).unwrap();
        assert_eq!(tc.id.as_str(), "call_abc123");
        assert_eq!(tc.r#type, ToolType::Function);
        assert_eq!(tc.function.name, "search_books");
        assert_eq!(tc.function.arguments, r#"{"q":"rust"}"#);
    }

    #[test]
    fn tool_call_round_trips_serialize_deserialize() {
        let tc = ToolCall {
            id: ToolCallId::new("call_xyz").unwrap(),
            r#type: ToolType::Function,
            function: FunctionCall {
                name: "get_weather".into(),
                arguments: r#"{"city":"Paris"}"#.into(),
            },
        };
        let v = serde_json::to_value(&tc).unwrap();
        let back: ToolCall = serde_json::from_value(v).unwrap();
        assert_eq!(back, tc);
    }

    #[test]
    fn tool_call_arguments_exact_on_round_trip() {
        let raw = r#"{"key":"value with \"quotes\""}"#;
        let tc = ToolCall {
            id: ToolCallId::new("call_1").unwrap(),
            r#type: ToolType::Function,
            function: FunctionCall {
                name: "fn".into(),
                arguments: raw.into(),
            },
        };
        let v = serde_json::to_value(&tc).unwrap();
        let back: ToolCall = serde_json::from_value(v).unwrap();
        assert_eq!(back.function.arguments, raw);
    }
}
