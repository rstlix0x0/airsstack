//! Initialize-handshake frame construction and capability parsing.
//!
//! The handshake is a control request the SDK sends first; the backend's
//! control response carries the capability manifest. Hook and agent
//! definitions are not sent here yet — they arrive with the in-loop handler
//! work — so the request carries only the system prompt.

use crate::agent::capabilities::Capabilities;
use crate::agent::options::Options;

/// Build the `initialize` control-request frame as a JSON value.
pub(super) fn initialize_request(options: &Options, request_id: &str) -> serde_json::Value {
    serde_json::json!({
        "type": "control_request",
        "request_id": request_id,
        "request": {
            "subtype": "initialize",
            "system_prompt": options.system_prompt,
        }
    })
}

/// Parse the capability manifest from an initialize control response.
///
/// Tolerant by design: an unrecognized or malformed payload yields the
/// default (empty) manifest so an absent feature reads as unsupported rather
/// than failing the handshake.
pub(super) fn parse_capabilities(response: &serde_json::Value) -> Capabilities {
    serde_json::from_value(response.clone()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{initialize_request, parse_capabilities};
    use crate::agent::capabilities::HookEvent;
    use crate::agent::options::Options;

    #[test]
    fn initialize_request_carries_id_and_system_prompt() {
        let opts = Options::builder().system_prompt("hello").build();
        let value = initialize_request(&opts, "req_0");
        assert_eq!(value["type"], "control_request");
        assert_eq!(value["request_id"], "req_0");
        assert_eq!(value["request"]["subtype"], "initialize");
        assert_eq!(value["request"]["system_prompt"], "hello");
    }

    #[test]
    fn parse_capabilities_reads_manifest() {
        let response = serde_json::json!({
            "protocol_version": "1.0",
            "supported_hook_events": ["PreToolUse"],
            "supported_control_methods": ["interrupt"]
        });
        let caps = parse_capabilities(&response);
        assert_eq!(caps.protocol_version, "1.0");
        assert!(caps.supports_hook(HookEvent::PreToolUse));
        assert!(caps.supports_control("interrupt"));
    }

    #[test]
    fn parse_capabilities_defaults_on_garbage() {
        let caps = parse_capabilities(&serde_json::json!(42));
        assert!(caps.protocol_version.is_empty());
    }
}
