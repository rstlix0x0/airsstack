//! Tool-definition types for the OpenRouter function-calling API.
//!
//! Exists as its own file to keep request-side tool types separate from the
//! shared tool-call carrier in `tool_call.rs` and the response decode in
//! `response.rs`.
//!
//! Responsibilities:
//! - [`ToolType`] — the discriminant for tool kinds (currently only `function`).
//! - [`FunctionDef`] — the function metadata (name, description, parameters,
//!   strict flag).
//! - [`Tool`] — a complete tool definition carrying a [`ToolType`] and a
//!   [`FunctionDef`].
//! - [`ToolChoice`] — how the model selects which tool to call.
//!
//! Not responsible for decoding server-side tool calls — see `tool_call.rs`.

use serde::Serialize;

use crate::types::FunctionName;

/// The kind of tool being defined.
///
/// `Function` is the only variant recognized by the API in this release;
/// additional server-side tool types (e.g. built-in web search) are reserved
/// for future extension.
///
/// # Examples
///
/// ```
/// use openrouter_rs::chat::ToolType;
/// assert_eq!(
///     serde_json::to_string(&ToolType::Function).unwrap(),
///     "\"function\""
/// );
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolType {
    /// A caller-defined function the model may invoke.
    Function,
}

/// The metadata describing a callable function.
///
/// # Examples
///
/// ```
/// use openrouter_rs::chat::FunctionDef;
/// use openrouter_rs::types::FunctionName;
///
/// let def = FunctionDef::new(FunctionName::new("get_weather").unwrap());
/// assert_eq!(
///     serde_json::to_value(&def).unwrap(),
///     serde_json::json!({ "name": "get_weather" }),
/// );
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FunctionDef {
    /// The function's validated name.
    pub name: FunctionName,
    /// An optional human-readable description the model uses to decide when to
    /// call this function.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// An optional JSON Schema object describing the function's parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<serde_json::Value>,
    /// Whether the model must strictly follow the parameter schema.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

impl FunctionDef {
    /// Build a minimal function definition with just a name.
    #[must_use]
    pub const fn new(name: FunctionName) -> Self {
        Self {
            name,
            description: None,
            parameters: None,
            strict: None,
        }
    }
}

/// A complete tool definition sent in the request's `tools` array.
///
/// Build with [`Tool::function`].
///
/// # Examples
///
/// ```
/// use openrouter_rs::chat::{FunctionDef, Tool};
/// use openrouter_rs::types::FunctionName;
///
/// let tool = Tool::function(FunctionDef::new(FunctionName::new("search").unwrap()));
/// assert_eq!(
///     serde_json::to_value(&tool).unwrap(),
///     serde_json::json!({ "type": "function", "function": { "name": "search" } }),
/// );
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Tool {
    /// The kind of tool.
    pub r#type: ToolType,
    /// The function definition.
    pub function: FunctionDef,
}

impl Tool {
    /// Build a function-type tool from a [`FunctionDef`].
    #[must_use]
    pub const fn function(function: FunctionDef) -> Self {
        Self {
            r#type: ToolType::Function,
            function,
        }
    }
}

/// Controls which tool, if any, the model calls.
///
/// Serializes to a string or object depending on the variant:
/// - `None` → `"none"`
/// - `Auto` → `"auto"`
/// - `Required` → `"required"`
/// - `Function { name }` → `{"type":"function","function":{"name":"…"}}`
///
/// Omitting the `tool_choice` field entirely (using `Option::<ToolChoice>::None`
/// in the request) lets the model decide.
///
/// # Examples
///
/// ```
/// use openrouter_rs::chat::ToolChoice;
/// use openrouter_rs::types::FunctionName;
///
/// assert_eq!(serde_json::to_value(&ToolChoice::Auto).unwrap(), serde_json::json!("auto"));
/// let force = ToolChoice::Function { name: FunctionName::new("search").unwrap() };
/// assert_eq!(
///     serde_json::to_value(&force).unwrap(),
///     serde_json::json!({ "type": "function", "function": { "name": "search" } }),
/// );
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToolChoice {
    /// The model will not call any tool.
    None,
    /// The model decides whether to call a tool.
    Auto,
    /// The model must call some tool.
    Required,
    /// The model must call the named function.
    Function {
        /// The function the model is required to call.
        name: FunctionName,
    },
}

impl Serialize for ToolChoice {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        match self {
            Self::None => s.serialize_str("none"),
            Self::Auto => s.serialize_str("auto"),
            Self::Required => s.serialize_str("required"),
            Self::Function { name } => {
                let mut map = s.serialize_map(Some(2))?;
                map.serialize_entry("type", "function")?;
                // Serialize nested {"name": "<fn>"} as the "function" value.
                map.serialize_entry("function", &serde_json::json!({ "name": name.as_str() }))?;
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

    fn fname(s: &str) -> FunctionName {
        FunctionName::new(s).unwrap()
    }

    // --- ToolType ---

    #[test]
    fn tool_type_serializes_lowercase() {
        assert_eq!(
            serde_json::to_value(ToolType::Function).unwrap(),
            json!("function"),
        );
    }

    // --- FunctionDef ---

    #[test]
    fn function_def_minimal_omits_optionals() {
        let def = FunctionDef::new(fname("get_weather"));
        assert_eq!(
            serde_json::to_value(&def).unwrap(),
            json!({ "name": "get_weather" }),
        );
    }

    #[test]
    fn function_def_all_fields_serialize() {
        let def = FunctionDef {
            name: fname("search"),
            description: Some("Search for items".into()),
            parameters: Some(json!({ "type": "object", "properties": {} })),
            strict: Some(true),
        };
        assert_eq!(
            serde_json::to_value(&def).unwrap(),
            json!({
                "name": "search",
                "description": "Search for items",
                "parameters": { "type": "object", "properties": {} },
                "strict": true,
            }),
        );
    }

    // --- Tool ---

    #[test]
    fn tool_function_ctor_sets_type() {
        let tool = Tool::function(FunctionDef::new(fname("my_fn")));
        assert_eq!(tool.r#type, ToolType::Function);
        assert_eq!(tool.function.name.as_str(), "my_fn");
    }

    #[test]
    fn tool_serializes_to_correct_shape() {
        let tool = Tool::function(FunctionDef::new(fname("search_books")));
        assert_eq!(
            serde_json::to_value(&tool).unwrap(),
            json!({ "type": "function", "function": { "name": "search_books" } }),
        );
    }

    // --- ToolChoice ---

    #[test]
    fn tool_choice_none_serializes_to_string() {
        assert_eq!(
            serde_json::to_value(&ToolChoice::None).unwrap(),
            json!("none"),
        );
    }

    #[test]
    fn tool_choice_auto_serializes_to_string() {
        assert_eq!(
            serde_json::to_value(&ToolChoice::Auto).unwrap(),
            json!("auto"),
        );
    }

    #[test]
    fn tool_choice_required_serializes_to_string() {
        assert_eq!(
            serde_json::to_value(&ToolChoice::Required).unwrap(),
            json!("required"),
        );
    }

    #[test]
    fn tool_choice_function_serializes_to_object() {
        let tc = ToolChoice::Function {
            name: fname("search_books"),
        };
        assert_eq!(
            serde_json::to_value(&tc).unwrap(),
            json!({ "type": "function", "function": { "name": "search_books" } }),
        );
    }
}
