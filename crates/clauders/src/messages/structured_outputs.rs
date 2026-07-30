//! The `output_config` request field for the Messages API.
//!
//! Carries two independent constraints on what the model produces: a JSON
//! Schema the response must satisfy, so the output parses without extra
//! validation on the caller's side, and how much reasoning effort the model
//! spends reaching it.
//! See <https://platform.claude.com/docs/en/build-with-claude/structured-outputs>.
//!
//! Responsibilities:
//! - Define [`OutputConfig`], the top-level request field holding both
//!   constraints, each optional and omitted from the wire when unset.
//! - Define [`OutputFormat`], the format discriminant (currently only
//!   `json_schema`).
//! - Provide the constructors and chainers that build it:
//!   [`OutputConfig::json_schema`], [`OutputConfig::effort`],
//!   [`OutputConfig::with_format`], [`OutputConfig::with_effort`].
//!
//! Not responsible for:
//! - Sending the request — that is `resource.rs`.
//! - Strict schema enforcement on tool inputs — that is the `Tool.strict`
//!   field in `tools.rs`.

/// Top-level output constraint applied to a Messages API request.
///
/// Both fields are optional and independent: a request may constrain the
/// output format, set the reasoning effort, do both, or neither.
///
/// # Examples
///
/// ```
/// use clauders::messages::structured_outputs::OutputConfig;
/// use clauders::types::EffortLevel;
///
/// let cfg = OutputConfig::json_schema(serde_json::json!({
///     "type": "object",
///     "properties": { "name": { "type": "string" } },
///     "required": ["name"]
/// }))
/// .with_effort(EffortLevel::High);
///
/// let j = serde_json::to_value(&cfg).unwrap();
/// assert_eq!(j["format"]["type"], "json_schema");
/// assert_eq!(j["effort"], "high");
/// ```
#[derive(Clone, Debug, serde::Serialize)]
pub struct OutputConfig {
    /// The output format constraint to apply, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<OutputFormat>,
    /// How much reasoning effort the model should spend, if specified.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<crate::types::EffortLevel>,
}

impl OutputConfig {
    /// Construct an `OutputConfig` that constrains the response to the given
    /// JSON Schema, leaving `effort` unset.
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
            format: Some(OutputFormat::JsonSchema { schema }),
            effort: None,
        }
    }

    /// Construct an `OutputConfig` that sets only the reasoning effort.
    ///
    /// # Examples
    ///
    /// ```
    /// use clauders::messages::structured_outputs::OutputConfig;
    /// use clauders::types::EffortLevel;
    ///
    /// let cfg = OutputConfig::effort(EffortLevel::Max);
    /// assert_eq!(serde_json::to_value(&cfg).unwrap()["effort"], "max");
    /// ```
    #[must_use]
    pub const fn effort(effort: crate::types::EffortLevel) -> Self {
        Self {
            format: None,
            effort: Some(effort),
        }
    }

    /// Set the reasoning effort, keeping any format already set.
    #[must_use]
    pub const fn with_effort(mut self, effort: crate::types::EffortLevel) -> Self {
        self.effort = Some(effort);
        self
    }

    /// Set the output format, keeping any effort already set.
    #[must_use]
    pub fn with_format(mut self, format: OutputFormat) -> Self {
        self.format = Some(format);
        self
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
    use crate::types::EffortLevel;

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

    #[test]
    fn effort_ctor_emits_effort_only() {
        let cfg = OutputConfig::effort(EffortLevel::High);
        let j = serde_json::to_value(&cfg).unwrap();
        assert_eq!(j, serde_json::json!({"effort": "high"}));
    }

    #[test]
    fn json_schema_ctor_leaves_effort_absent() {
        let cfg = OutputConfig::json_schema(serde_json::json!({"type": "object"}));
        let j = serde_json::to_value(&cfg).unwrap();
        assert!(
            j.get("effort").is_none(),
            "effort must be absent when not set"
        );
    }

    #[test]
    fn with_effort_adds_to_an_existing_format() {
        let cfg = OutputConfig::json_schema(serde_json::json!({"type": "object"}))
            .with_effort(EffortLevel::Xhigh);
        let j = serde_json::to_value(&cfg).unwrap();
        assert_eq!(j["format"]["type"], "json_schema");
        assert_eq!(j["effort"], "xhigh");
    }

    #[test]
    fn with_format_adds_to_an_existing_effort() {
        let cfg = OutputConfig::effort(EffortLevel::Low).with_format(OutputFormat::JsonSchema {
            schema: serde_json::json!({"type": "object"}),
        });
        let j = serde_json::to_value(&cfg).unwrap();
        assert_eq!(j["effort"], "low");
        assert_eq!(j["format"]["type"], "json_schema");
    }

    #[test]
    fn an_empty_output_config_serializes_to_an_empty_object() {
        let cfg = OutputConfig {
            format: None,
            effort: None,
        };
        let j = serde_json::to_value(&cfg).unwrap();
        assert_eq!(
            j,
            serde_json::json!({}),
            "an all-None OutputConfig is an empty object; the request builder \
             must therefore omit it entirely rather than send this"
        );
    }
}
