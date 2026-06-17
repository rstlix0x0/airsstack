//! Initialize-handshake frame construction and capability parsing.
//!
//! The handshake is a control request the SDK sends first; the backend's
//! control response carries the capability manifest. Hook definitions are
//! declared here, keyed by the callback ids the registry minted; agent
//! definitions remain out of scope.

use crate::agent::capabilities::Capabilities;
use crate::agent::options::Options;

/// Build the `initialize` control-request frame as a JSON value.
pub(super) fn initialize_request(options: &Options, request_id: &str) -> serde_json::Value {
    let mut request = serde_json::json!({
        "subtype": "initialize",
        "system_prompt": options.system_prompt,
    });
    if !options.hooks.is_empty() {
        // Caps unknown pre-handshake: declare all registered hooks; the binary
        // simply never fires events it does not support.
        let hooks = options.hooks.initialize_payload(&Capabilities::default());
        if let Some(obj) = request.as_object_mut() {
            obj.insert("hooks".to_string(), hooks);
        }
    }
    serde_json::json!({
        "type": "control_request",
        "request_id": request_id,
        "request": request,
    })
}

/// Log a warning for each registered hook event the binary does not support.
///
/// Called after the handshake, once capabilities are known. Unsupported events
/// are a no-op at runtime (the binary never fires them); this surfaces the
/// mismatch to the developer.
pub(super) fn warn_unsupported_hooks(options: &Options, caps: &Capabilities) {
    // Reuse the gating in initialize_payload: anything it drops is unsupported,
    // and it already logs a warning per dropped event.
    let _ = options.hooks.initialize_payload(caps);
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

    #[test]
    fn initialize_request_includes_registered_hooks() {
        use crate::agent::capabilities::HookEvent;
        use crate::agent::hooks::{Hook, HookInput, HookOutput};
        use std::sync::Arc;

        struct H;
        #[async_trait::async_trait]
        impl Hook for H {
            async fn call(
                &self,
                _i: HookInput,
            ) -> Result<HookOutput, crate::agent::error::AgentError> {
                Ok(HookOutput::default())
            }
        }

        let opts = Options::builder()
            .system_prompt("hi")
            .hook(HookEvent::PreToolUse, Some("Bash".to_string()), Arc::new(H))
            .build();
        let value = initialize_request(&opts, "req_0");
        assert_eq!(
            value["request"]["hooks"]["PreToolUse"][0]["hookCallbackIds"][0],
            "hook_0"
        );
    }

    #[test]
    fn initialize_request_omits_hooks_when_none_registered() {
        let opts = Options::builder().system_prompt("hi").build();
        let value = initialize_request(&opts, "req_0");
        assert!(value["request"].get("hooks").is_none());
    }
}
