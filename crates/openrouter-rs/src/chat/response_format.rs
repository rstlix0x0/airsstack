//! Response-format types for structured-output requests.
//!
//! Exists as its own file to keep request-side response-format types separate
//! from tool definitions and message types, which evolve independently.
//!
//! Responsibilities:
//! - [`SchemaStrictness`] — a two-variant semantic flag controlling whether
//!   the model must strictly follow the schema.
//! - [`JsonSchemaConfig`] — the structured schema config carrying a name,
//!   optional strictness, and a JSON Schema value.
//! - [`ResponseFormat`] — the top-level variant selecting `json_object` or
//!   `json_schema` mode.
//!
//! Not responsible for decoding structured content in responses — the caller
//! parses the JSON string inside `message.content` themselves.

use serde::Serialize;
use serde::ser::SerializeMap;

use crate::types::SchemaName;

/// Controls whether the model must strictly follow the JSON Schema.
///
/// Serializes as a bare bool inside [`JsonSchemaConfig`]: `Strict` → `true`,
/// `Lenient` → `false`. The field is `Option<SchemaStrictness>` so omitting
/// it (using `None`) tells the provider to apply its default behavior.
///
/// # Examples
///
/// ```
/// use openrouter_rs::chat::SchemaStrictness;
///
/// assert!(SchemaStrictness::Strict.as_bool());
/// assert!(!SchemaStrictness::Lenient.as_bool());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SchemaStrictness {
    /// The model must strictly follow the schema.
    Strict,
    /// The model applies best-effort schema adherence.
    Lenient,
}

impl SchemaStrictness {
    /// Convert to the wire boolean representation.
    #[must_use]
    pub const fn as_bool(self) -> bool {
        matches!(self, Self::Strict)
    }
}

/// The schema configuration for a `json_schema` response format.
///
/// # Examples
///
/// ```
/// use openrouter_rs::chat::JsonSchemaConfig;
/// use openrouter_rs::types::SchemaName;
///
/// let schema = serde_json::json!({ "type": "object", "properties": {} });
/// let cfg = JsonSchemaConfig::new(SchemaName::new("weather").unwrap(), schema);
/// assert_eq!(
///     serde_json::to_value(&cfg).unwrap(),
///     serde_json::json!({
///         "name": "weather",
///         "schema": { "type": "object", "properties": {} }
///     }),
/// );
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JsonSchemaConfig {
    /// The schema's validated name.
    pub name: SchemaName,
    /// Optional strictness flag; `None` uses the provider's default.
    pub strict: Option<SchemaStrictness>,
    /// A JSON Schema object describing the expected response structure.
    pub schema: serde_json::Value,
}

impl JsonSchemaConfig {
    /// Build a minimal config with just a name and schema (no strictness).
    #[must_use]
    pub const fn new(name: SchemaName, schema: serde_json::Value) -> Self {
        Self {
            name,
            strict: None,
            schema,
        }
    }
}

impl Serialize for JsonSchemaConfig {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let field_count = if self.strict.is_some() { 3 } else { 2 };
        let mut map = s.serialize_map(Some(field_count))?;
        map.serialize_entry("name", self.name.as_str())?;
        if let Some(strictness) = self.strict {
            map.serialize_entry("strict", &strictness.as_bool())?;
        }
        map.serialize_entry("schema", &self.schema)?;
        map.end()
    }
}

/// The format in which the model should return its response.
///
/// - `JsonObject` requests a valid JSON object without a schema constraint.
/// - `JsonSchema` requests output conforming to a specific JSON Schema.
///
/// Serializes as a tagged object:
/// - `JsonObject` → `{ "type": "json_object" }`
/// - `JsonSchema(cfg)` → `{ "type": "json_schema", "json_schema": <cfg> }`
///
/// # Examples
///
/// ```
/// use openrouter_rs::chat::{JsonSchemaConfig, ResponseFormat};
/// use openrouter_rs::types::SchemaName;
///
/// // Basic JSON object mode.
/// assert_eq!(
///     serde_json::to_value(&ResponseFormat::JsonObject).unwrap(),
///     serde_json::json!({ "type": "json_object" }),
/// );
///
/// // JSON Schema mode with a full config.
/// let schema = serde_json::json!({
///     "type": "object",
///     "properties": { "city": { "type": "string" } },
///     "required": ["city"]
/// });
/// let cfg = JsonSchemaConfig::new(SchemaName::new("weather").unwrap(), schema);
/// assert_eq!(
///     serde_json::to_value(&ResponseFormat::JsonSchema(cfg)).unwrap(),
///     serde_json::json!({
///         "type": "json_schema",
///         "json_schema": {
///             "name": "weather",
///             "schema": {
///                 "type": "object",
///                 "properties": { "city": { "type": "string" } },
///                 "required": ["city"]
///             }
///         }
///     }),
/// );
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResponseFormat {
    /// Request a valid JSON object without a schema constraint.
    JsonObject,
    /// Request output conforming to the given JSON Schema.
    JsonSchema(JsonSchemaConfig),
}

impl Serialize for ResponseFormat {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::JsonObject => {
                let mut map = s.serialize_map(Some(1))?;
                map.serialize_entry("type", "json_object")?;
                map.end()
            }
            Self::JsonSchema(cfg) => {
                let mut map = s.serialize_map(Some(2))?;
                map.serialize_entry("type", "json_schema")?;
                map.serialize_entry("json_schema", cfg)?;
                map.end()
            }
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

    fn sname(s: &str) -> SchemaName {
        SchemaName::new(s).unwrap()
    }

    // --- SchemaStrictness ---

    #[test]
    fn strict_maps_to_true() {
        assert!(SchemaStrictness::Strict.as_bool());
    }

    #[test]
    fn lenient_maps_to_false() {
        assert!(!SchemaStrictness::Lenient.as_bool());
    }

    // --- JsonSchemaConfig ---

    #[test]
    fn minimal_config_omits_strict() {
        let cfg = JsonSchemaConfig::new(sname("weather"), json!({ "type": "object" }));
        assert_eq!(
            serde_json::to_value(&cfg).unwrap(),
            json!({ "name": "weather", "schema": { "type": "object" } }),
        );
    }

    #[test]
    fn config_with_strict_emits_true() {
        let mut cfg = JsonSchemaConfig::new(sname("weather"), json!({}));
        cfg.strict = Some(SchemaStrictness::Strict);
        assert_eq!(
            serde_json::to_value(&cfg).unwrap(),
            json!({ "name": "weather", "strict": true, "schema": {} }),
        );
    }

    #[test]
    fn config_with_lenient_emits_false() {
        let mut cfg = JsonSchemaConfig::new(sname("weather"), json!({}));
        cfg.strict = Some(SchemaStrictness::Lenient);
        assert_eq!(
            serde_json::to_value(&cfg).unwrap(),
            json!({ "name": "weather", "strict": false, "schema": {} }),
        );
    }

    #[test]
    fn config_full_shape_matches_wire_doc() {
        let schema = json!({
            "type": "object",
            "properties": { "city": { "type": "string" } },
            "required": ["city"]
        });
        let mut cfg = JsonSchemaConfig::new(sname("weather"), schema);
        cfg.strict = Some(SchemaStrictness::Strict);
        assert_eq!(
            serde_json::to_value(&cfg).unwrap(),
            json!({
                "name": "weather",
                "strict": true,
                "schema": {
                    "type": "object",
                    "properties": { "city": { "type": "string" } },
                    "required": ["city"]
                }
            }),
        );
    }

    // --- ResponseFormat ---

    #[test]
    fn json_object_serializes_to_type_tag_only() {
        assert_eq!(
            serde_json::to_value(&ResponseFormat::JsonObject).unwrap(),
            json!({ "type": "json_object" }),
        );
    }

    #[test]
    fn json_schema_serializes_with_nested_config() {
        let cfg = JsonSchemaConfig::new(sname("weather"), json!({ "type": "object" }));
        assert_eq!(
            serde_json::to_value(ResponseFormat::JsonSchema(cfg)).unwrap(),
            json!({
                "type": "json_schema",
                "json_schema": {
                    "name": "weather",
                    "schema": { "type": "object" }
                }
            }),
        );
    }

    #[test]
    fn json_schema_with_strict_true_full_wire_shape() {
        let schema = json!({
            "type": "object",
            "properties": {},
            "required": []
        });
        let mut cfg = JsonSchemaConfig::new(sname("weather"), schema);
        cfg.strict = Some(SchemaStrictness::Strict);
        let v = serde_json::to_value(ResponseFormat::JsonSchema(cfg)).unwrap();
        assert_eq!(v["type"], json!("json_schema"));
        assert_eq!(v["json_schema"]["name"], json!("weather"));
        assert_eq!(v["json_schema"]["strict"], json!(true));
    }
}
