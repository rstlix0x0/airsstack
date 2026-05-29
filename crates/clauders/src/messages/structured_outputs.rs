//! Structured Outputs support for the Messages API.
//!
//! Constrains the model's response to a JSON Schema, ensuring the output
//! can be parsed without extra validation on the caller's side.
//! See <https://platform.claude.com/docs/en/build-with-claude/structured-outputs>.
//!
//! Responsibilities:
//! - Define [`OutputConfig`], the top-level request field that activates
//!   structured output for a given request.
//! - Define [`OutputFormat`], the format discriminant (currently only
//!   `json_schema`).
//! - Provide [`OutputConfig::json_schema`], a convenience constructor that
//!   builds the common case.
//!
//! Not responsible for:
//! - Sending the request — that is `resource.rs`.
//! - Strict schema enforcement on tool inputs — that is the `Tool.strict`
//!   field in `tools.rs`.

/// Top-level output constraint applied to a Messages API request.
///
/// Attaching this to a request instructs the model to produce output that
/// conforms to the enclosed format. The model serializes its response as
/// valid JSON that matches the provided schema.
///
/// # Examples
///
/// ```
/// use clauders::messages::structured_outputs::{OutputConfig, OutputFormat};
///
/// let cfg = OutputConfig {
///     format: OutputFormat::JsonSchema {
///         schema: serde_json::json!({
///             "type": "object",
///             "properties": { "name": { "type": "string" } },
///             "required": ["name"]
///         }),
///     },
/// };
/// let j = serde_json::to_value(&cfg).unwrap();
/// assert_eq!(j["format"]["type"], "json_schema");
/// ```
#[derive(Clone, Debug, serde::Serialize)]
pub struct OutputConfig {
    /// The output format constraint to apply.
    pub format: OutputFormat,
}

impl OutputConfig {
    /// Construct an `OutputConfig` that constrains the response to the given
    /// JSON Schema.
    ///
    /// The `schema` value must be a valid JSON Schema object. The API enforces
    /// conformance at inference time; the SDK does not pre-validate the schema.
    ///
    /// # Examples
    ///
    /// ```
    /// use clauders::messages::structured_outputs::OutputConfig;
    ///
    /// let cfg = OutputConfig::json_schema(serde_json::json!({
    ///     "type": "object",
    ///     "properties": {
    ///         "name": { "type": "string" },
    ///         "age":  { "type": "integer" }
    ///     },
    ///     "required": ["name", "age"]
    /// }));
    ///
    /// let j = serde_json::to_value(&cfg).unwrap();
    /// assert_eq!(j["format"]["type"], "json_schema");
    /// assert!(j["format"]["schema"]["properties"]["name"].is_object());
    /// ```
    #[must_use]
    #[expect(
        clippy::missing_const_for_fn,
        reason = "serde_json::Value is not const-constructible; the function body cannot be const"
    )]
    pub fn json_schema(schema: serde_json::Value) -> Self {
        Self {
            format: OutputFormat::JsonSchema { schema },
        }
    }
}

/// Output format variant for a structured-output request.
///
/// Currently the API supports only `json_schema`.  The enum is
/// `#[serde(tag = "type")]` so the wire format includes a `"type"` field.
#[derive(Clone, Debug, serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OutputFormat {
    /// Constrain the response to a JSON Schema.
    ///
    /// The `schema` value is forwarded verbatim to the API as the
    /// `format.schema` field.
    JsonSchema {
        /// JSON Schema the model's response must conform to.
        schema: serde_json::Value,
    },
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::unwrap_used,
        reason = "tests unwrap known-valid fixtures; a panic is the intended failure signal"
    )]

    use super::*;

    #[test]
    fn json_schema_ctor_produces_correct_format_type() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": { "name": { "type": "string" } },
            "required": ["name"]
        });
        let cfg = OutputConfig::json_schema(schema.clone());

        let j = serde_json::to_value(&cfg).unwrap();
        assert_eq!(
            j["format"]["type"], "json_schema",
            "format.type must be 'json_schema'"
        );
        assert_eq!(
            j["format"]["schema"], schema,
            "format.schema must carry the provided schema verbatim"
        );
    }

    #[test]
    fn output_format_json_schema_wire_shape() {
        let format = OutputFormat::JsonSchema {
            schema: serde_json::json!({"type": "object"}),
        };
        let j = serde_json::to_value(&format).unwrap();
        assert_eq!(j["type"], "json_schema");
        assert_eq!(j["schema"]["type"], "object");
        // Confirm no extra nesting (e.g. no "json_schema" wrapper key).
        assert!(j.get("json_schema").is_none(), "must not double-nest");
    }
}
