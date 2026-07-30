//! Initialize-handshake frame construction.
//!
//! The handshake is a control request the SDK sends first. Hook definitions
//! are declared here, keyed by the callback ids the registry minted; agent
//! definitions remain out of scope. The capability manifest itself arrives
//! later, on the `system`/`init` message frame — not from this handshake.

use crate::agent::options::Options;

/// Build the `initialize` request body as a JSON value.
///
/// Shared by the initial handshake (wrapped as a `control_request`) and by
/// `reinitialize`, which sends this same body over the live control channel.
pub(super) fn initialize_body(options: &Options) -> serde_json::Value {
    let mut request = serde_json::json!({
        "subtype": "initialize",
        "system_prompt": options.system_prompt.native_text(),
    });
    if !options.hooks.is_empty() {
        let hooks = options.hooks.initialize_payload();
        if let Some(obj) = request.as_object_mut() {
            obj.insert("hooks".to_string(), hooks);
        }
    }
    if let Some(config) = &options.output_format {
        if let (Some(obj), Ok(value)) = (request.as_object_mut(), serde_json::to_value(config)) {
            obj.insert("output_format".to_string(), value);
        }
    }
    if let Some(title) = &options.title {
        if let Some(obj) = request.as_object_mut() {
            obj.insert(
                "title".to_string(),
                serde_json::Value::String(title.clone()),
            );
        }
    }
    request
}

/// Build the `initialize` control-request frame as a JSON value.
pub(super) fn initialize_request(options: &Options, request_id: &str) -> serde_json::Value {
    serde_json::json!({
        "type": "control_request",
        "request_id": request_id,
        "request": initialize_body(options),
    })
}

#[cfg(test)]
mod tests {
    use super::{initialize_body, initialize_request};
    use crate::agent::options::Options;

    #[test]
    fn initialize_body_carries_subtype_and_system_prompt() {
        let opts = Options::builder().system_prompt("hi").build();
        let body = initialize_body(&opts);
        assert_eq!(body["subtype"], "initialize");
        assert_eq!(body["system_prompt"], "hi");
    }

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
                _cancel: crate::agent::cancel::CancelSignal,
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

    #[test]
    fn initialize_request_carries_preset_append_as_text() {
        let opts = Options::builder()
            .system_prompt_preset(Some("appended".to_owned()), false)
            .build();
        let value = initialize_request(&opts, "req_0");
        assert_eq!(value["request"]["system_prompt"], "appended");
    }

    #[test]
    fn initialize_request_carries_output_format_when_set() {
        let opts = Options::builder()
            .output_schema(serde_json::json!({ "type": "object" }))
            .build();
        let value = initialize_request(&opts, "req_0");
        assert_eq!(
            value["request"]["output_format"]["format"]["type"],
            "json_schema"
        );
    }

    #[test]
    fn initialize_request_omits_output_format_when_unset() {
        let opts = Options::builder().build();
        let value = initialize_request(&opts, "req_0");
        assert!(value["request"].get("output_format").is_none());
    }

    #[test]
    fn initialize_request_carries_title_when_set() {
        let opts = Options::builder().title("Nightly triage").build();
        let value = initialize_request(&opts, "req_0");
        assert_eq!(value["request"]["title"], "Nightly triage");
    }

    #[test]
    fn initialize_request_omits_title_when_unset() {
        let opts = Options::builder().build();
        let value = initialize_request(&opts, "req_0");
        assert!(value["request"].get("title").is_none());
    }
}
