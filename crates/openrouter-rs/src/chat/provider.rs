//! Provider routing-preference types for the OpenRouter chat API.
//!
//! Exists as its own file to keep provider-routing types separate from tool
//! definitions, response-format types, and message types, which evolve
//! independently.
//!
//! Responsibilities:
//! - Boolean-flag enums: [`FallbackPolicy`], [`ParameterRequirement`],
//!   [`ZeroDataRetention`] — each serializes to a bare JSON bool.
//! - String-enum routing controls: [`DataCollection`], [`ProviderSort`],
//!   [`Quantization`] — each serializes to its wire string.
//! - Chat-local scalar newtypes: [`ThroughputFloor`], [`LatencyCeiling`] —
//!   non-negative, finite `f64` values serialized as JSON numbers.
//! - [`MaxPrice`] — partial price object for per-token-type budget limits.
//! - [`ProviderPreferences`] — the complete `"provider"` request-body object,
//!   derived `Serialize` with per-field skip-none.
//! - [`ProviderPreferencesBuilder`] — a non-typestate fluent builder (all
//!   fields optional).
//!
//! Not responsible for sending requests or decoding responses — the resource
//! layer dispatches the built [`ProviderPreferences`] via [`crate::chat::ChatRequest`].

use serde::Serialize;

use crate::types::{Price, ProviderSlug};

// ---------------------------------------------------------------------------
// Boolean-flag enums
// ---------------------------------------------------------------------------

/// Controls whether OpenRouter may fall back to other providers.
///
/// Serializes to a bare JSON bool: `Allow` → `true`, `Deny` → `false`.
/// When the field is omitted the API defaults to `true` (fallbacks allowed).
///
/// # Examples
///
/// ```
/// use openrouter_rs::chat::FallbackPolicy;
///
/// assert!(FallbackPolicy::Allow.as_bool());
/// assert!(!FallbackPolicy::Deny.as_bool());
/// assert_eq!(serde_json::to_value(FallbackPolicy::Allow).unwrap(), true);
/// assert_eq!(serde_json::to_value(FallbackPolicy::Deny).unwrap(), false);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FallbackPolicy {
    /// Provider fallbacks are permitted.
    Allow,
    /// Provider fallbacks are not permitted; only the specified providers are used.
    Deny,
}

impl FallbackPolicy {
    /// Convert to the wire boolean representation.
    #[must_use]
    pub const fn as_bool(self) -> bool {
        matches!(self, Self::Allow)
    }
}

impl Serialize for FallbackPolicy {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_bool(self.as_bool())
    }
}

/// Controls whether only providers supporting all requested parameters are used.
///
/// Serializes to a bare JSON bool: `Required` → `true`, `Optional` → `false`.
/// When omitted the API defaults to `false`.
///
/// # Examples
///
/// ```
/// use openrouter_rs::chat::ParameterRequirement;
///
/// assert!(ParameterRequirement::Required.as_bool());
/// assert!(!ParameterRequirement::Optional.as_bool());
/// assert_eq!(serde_json::to_value(ParameterRequirement::Required).unwrap(), true);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParameterRequirement {
    /// Only providers that support all request parameters are eligible.
    Required,
    /// Providers that ignore unsupported parameters are also eligible.
    Optional,
}

impl ParameterRequirement {
    /// Convert to the wire boolean representation.
    #[must_use]
    pub const fn as_bool(self) -> bool {
        matches!(self, Self::Required)
    }
}

impl Serialize for ParameterRequirement {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_bool(self.as_bool())
    }
}

/// Controls whether only zero-data-retention providers are used.
///
/// Serializes to a bare JSON bool: `Enabled` → `true`, `Disabled` → `false`.
///
/// # Examples
///
/// ```
/// use openrouter_rs::chat::ZeroDataRetention;
///
/// assert!(ZeroDataRetention::Enabled.as_bool());
/// assert!(!ZeroDataRetention::Disabled.as_bool());
/// assert_eq!(serde_json::to_value(ZeroDataRetention::Enabled).unwrap(), true);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ZeroDataRetention {
    /// Request that the provider retains no data.
    Enabled,
    /// No zero-data-retention constraint is applied.
    Disabled,
}

impl ZeroDataRetention {
    /// Convert to the wire boolean representation.
    #[must_use]
    pub const fn as_bool(self) -> bool {
        matches!(self, Self::Enabled)
    }
}

impl Serialize for ZeroDataRetention {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_bool(self.as_bool())
    }
}

// ---------------------------------------------------------------------------
// String-enum routing controls
// ---------------------------------------------------------------------------

/// Whether the provider may collect request data for training purposes.
///
/// Serializes to `"allow"` or `"deny"`.
///
/// # Examples
///
/// ```
/// use openrouter_rs::chat::DataCollection;
///
/// assert_eq!(
///     serde_json::to_value(DataCollection::Allow).unwrap(),
///     serde_json::json!("allow"),
/// );
/// assert_eq!(
///     serde_json::to_value(DataCollection::Deny).unwrap(),
///     serde_json::json!("deny"),
/// );
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DataCollection {
    /// Data collection by the provider is permitted.
    Allow,
    /// Data collection by the provider is not permitted.
    Deny,
}

/// The criterion by which OpenRouter sorts eligible providers.
///
/// Serializes to its lowercase wire token. The object form (`{by, partition}`)
/// is not supported in this release; use the string variants only.
///
/// # Examples
///
/// ```
/// use openrouter_rs::chat::ProviderSort;
///
/// assert_eq!(
///     serde_json::to_value(ProviderSort::Price).unwrap(),
///     serde_json::json!("price"),
/// );
/// assert_eq!(
///     serde_json::to_value(ProviderSort::Exacto).unwrap(),
///     serde_json::json!("exacto"),
/// );
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderSort {
    /// Sort by cost (cheapest first).
    Price,
    /// Sort by throughput (tokens per second, highest first).
    Throughput,
    /// Sort by latency (time to first token, lowest first).
    Latency,
    /// Exact provider ordering with no fallback reordering.
    Exacto,
}

/// The quantization level to require from the provider.
///
/// Serializes to its verbatim wire token (e.g. `"int4"`, `"bf16"`).
///
/// # Examples
///
/// ```
/// use openrouter_rs::chat::Quantization;
///
/// assert_eq!(
///     serde_json::to_value(Quantization::Int4).unwrap(),
///     serde_json::json!("int4"),
/// );
/// assert_eq!(
///     serde_json::to_value(Quantization::Bf16).unwrap(),
///     serde_json::json!("bf16"),
/// );
/// assert_eq!(
///     serde_json::to_value(Quantization::Unknown).unwrap(),
///     serde_json::json!("unknown"),
/// );
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Quantization {
    /// 4-bit integer quantization.
    #[serde(rename = "int4")]
    Int4,
    /// 8-bit integer quantization.
    #[serde(rename = "int8")]
    Int8,
    /// 4-bit floating-point quantization.
    #[serde(rename = "fp4")]
    Fp4,
    /// 6-bit floating-point quantization.
    #[serde(rename = "fp6")]
    Fp6,
    /// 8-bit floating-point quantization.
    #[serde(rename = "fp8")]
    Fp8,
    /// 16-bit floating-point quantization.
    #[serde(rename = "fp16")]
    Fp16,
    /// 16-bit brain floating-point quantization.
    #[serde(rename = "bf16")]
    Bf16,
    /// 32-bit floating-point (full precision).
    #[serde(rename = "fp32")]
    Fp32,
    /// Quantization level is unknown or unspecified by the provider.
    #[serde(rename = "unknown")]
    Unknown,
}

// ---------------------------------------------------------------------------
// Chat-local scalar newtypes
// ---------------------------------------------------------------------------

/// Minimum acceptable throughput in tokens per second.
///
/// Rejects `NaN`, infinite, and negative values at construction. Serializes
/// as a JSON number.
///
/// # Examples
///
/// ```
/// use openrouter_rs::chat::ThroughputFloor;
///
/// let t = ThroughputFloor::new(50.0).expect("valid");
/// assert_eq!(serde_json::to_value(t).unwrap(), serde_json::json!(50.0));
///
/// assert!(ThroughputFloor::new(-1.0).is_err());
/// assert!(ThroughputFloor::new(f64::NAN).is_err());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize)]
#[serde(transparent)]
pub struct ThroughputFloor(f64);

/// Reasons [`ThroughputFloor::new`] can reject input.
#[derive(Debug, Clone, Copy, thiserror::Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum InvalidThroughputFloor {
    /// Input was `NaN`, infinite, or negative.
    #[error("throughput floor must be a non-negative, finite number")]
    Invalid,
}

impl ThroughputFloor {
    /// Validate and wrap an `f64`.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidThroughputFloor::Invalid`] when `n` is `NaN`,
    /// infinite, or negative.
    pub fn new(n: f64) -> Result<Self, InvalidThroughputFloor> {
        if n.is_finite() && n >= 0.0 {
            Ok(Self(n))
        } else {
            Err(InvalidThroughputFloor::Invalid)
        }
    }

    /// The inner value, for wire-format use.
    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }
}

/// Maximum acceptable latency in seconds (time to first token).
///
/// Rejects `NaN`, infinite, and negative values at construction. Serializes
/// as a JSON number.
///
/// # Examples
///
/// ```
/// use openrouter_rs::chat::LatencyCeiling;
///
/// let l = LatencyCeiling::new(1.5).expect("valid");
/// assert_eq!(serde_json::to_value(l).unwrap(), serde_json::json!(1.5));
///
/// assert!(LatencyCeiling::new(-0.1).is_err());
/// assert!(LatencyCeiling::new(f64::INFINITY).is_err());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize)]
#[serde(transparent)]
pub struct LatencyCeiling(f64);

/// Reasons [`LatencyCeiling::new`] can reject input.
#[derive(Debug, Clone, Copy, thiserror::Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum InvalidLatencyCeiling {
    /// Input was `NaN`, infinite, or negative.
    #[error("latency ceiling must be a non-negative, finite number")]
    Invalid,
}

impl LatencyCeiling {
    /// Validate and wrap an `f64`.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidLatencyCeiling::Invalid`] when `n` is `NaN`,
    /// infinite, or negative.
    pub fn new(n: f64) -> Result<Self, InvalidLatencyCeiling> {
        if n.is_finite() && n >= 0.0 {
            Ok(Self(n))
        } else {
            Err(InvalidLatencyCeiling::Invalid)
        }
    }

    /// The inner value, for wire-format use.
    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }
}

// ---------------------------------------------------------------------------
// MaxPrice
// ---------------------------------------------------------------------------

/// Per-token-type budget limits for provider routing.
///
/// Each field is a price cap in USD per million tokens. All fields are
/// optional; unset fields impose no constraint. The wire key `"request"` caps
/// the cost per API request rather than per token.
///
/// Build with [`MaxPrice::new`] (all-`None`) then set individual fields, or
/// use [`ProviderPreferencesBuilder::max_price`].
///
/// # Examples
///
/// ```
/// use openrouter_rs::chat::MaxPrice;
/// use openrouter_rs::types::Price;
///
/// let mut mp = MaxPrice::new();
/// mp.prompt = Some(Price::new(1.0).unwrap());
/// assert_eq!(
///     serde_json::to_value(&mp).unwrap(),
///     serde_json::json!({ "prompt": 1.0 }),
/// );
///
/// // All None → empty object.
/// assert_eq!(
///     serde_json::to_value(&MaxPrice::new()).unwrap(),
///     serde_json::json!({}),
/// );
/// ```
#[derive(Clone, Debug, PartialEq, Serialize, Default)]
pub struct MaxPrice {
    /// Maximum prompt-token cost, USD per million tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<Price>,
    /// Maximum completion-token cost, USD per million tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion: Option<Price>,
    /// Maximum image-token cost, USD per million tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<Price>,
    /// Maximum audio-token cost, USD per million tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio: Option<Price>,
    /// Maximum per-request cost in USD.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request: Option<Price>,
}

impl MaxPrice {
    /// Build an all-`None` price cap.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

// ---------------------------------------------------------------------------
// ProviderPreferences + ProviderPreferencesBuilder
// ---------------------------------------------------------------------------

/// Routing preferences for the `"provider"` request-body field.
///
/// Controls which model providers OpenRouter routes to and how it selects
/// among them. All fields are optional; unset fields inherit API defaults.
///
/// Build with [`ProviderPreferences::builder`].
///
/// # Examples
///
/// ```
/// use openrouter_rs::chat::{FallbackPolicy, ProviderPreferences, ProviderSort};
///
/// // Empty preferences serializes to an empty object.
/// let prefs = ProviderPreferences::builder().build();
/// assert_eq!(serde_json::to_value(&prefs).unwrap(), serde_json::json!({}));
///
/// // A minimal example with sort and fallback policy.
/// let prefs = ProviderPreferences::builder()
///     .sort(ProviderSort::Price)
///     .allow_fallbacks(FallbackPolicy::Allow)
///     .build();
/// let v = serde_json::to_value(&prefs).unwrap();
/// assert_eq!(v["sort"], serde_json::json!("price"));
/// assert_eq!(v["allow_fallbacks"], serde_json::json!(true));
/// ```
#[derive(Clone, Debug, PartialEq, Serialize, Default)]
pub struct ProviderPreferences {
    /// Ordered list of providers to try first.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) order: Option<Vec<ProviderSlug>>,
    /// Restrict routing to only these providers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) only: Option<Vec<ProviderSlug>>,
    /// Providers to exclude from routing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) ignore: Option<Vec<ProviderSlug>>,
    /// Whether provider fallbacks are permitted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) allow_fallbacks: Option<FallbackPolicy>,
    /// Whether only providers supporting all parameters are eligible.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) require_parameters: Option<ParameterRequirement>,
    /// Whether only zero-data-retention providers are eligible.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) zdr: Option<ZeroDataRetention>,
    /// Data-collection policy constraint.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) data_collection: Option<DataCollection>,
    /// Quantization levels to accept.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) quantizations: Option<Vec<Quantization>>,
    /// Criterion used to sort eligible providers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) sort: Option<ProviderSort>,
    /// Per-token-type price caps.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) max_price: Option<MaxPrice>,
    /// Minimum acceptable throughput in tokens per second.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) preferred_min_throughput: Option<ThroughputFloor>,
    /// Maximum acceptable latency in seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) preferred_max_latency: Option<LatencyCeiling>,
}

impl ProviderPreferences {
    /// Start building provider preferences.
    #[must_use]
    pub fn builder() -> ProviderPreferencesBuilder {
        ProviderPreferencesBuilder::default()
    }
}

/// Non-typestate fluent builder for [`ProviderPreferences`].
///
/// All fields are optional; call only the setters you need, then [`build`].
///
/// # Examples
///
/// ```
/// use openrouter_rs::chat::{
///     DataCollection, FallbackPolicy, ProviderPreferences, ProviderSort, Quantization,
///     ZeroDataRetention,
/// };
/// use openrouter_rs::types::ProviderSlug;
///
/// let prefs = ProviderPreferences::builder()
///     .order(vec![ProviderSlug::new("openai").unwrap()])
///     .allow_fallbacks(FallbackPolicy::Deny)
///     .sort(ProviderSort::Throughput)
///     .data_collection(DataCollection::Deny)
///     .quantizations(vec![Quantization::Fp16, Quantization::Bf16])
///     .zdr(ZeroDataRetention::Enabled)
///     .build();
///
/// let v = serde_json::to_value(&prefs).unwrap();
/// assert_eq!(v["sort"], serde_json::json!("throughput"));
/// assert_eq!(v["allow_fallbacks"], serde_json::json!(false));
/// assert_eq!(v["data_collection"], serde_json::json!("deny"));
/// assert_eq!(v["zdr"], serde_json::json!(true));
/// ```
///
/// [`build`]: ProviderPreferencesBuilder::build
#[derive(Clone, Debug, Default)]
pub struct ProviderPreferencesBuilder {
    inner: ProviderPreferences,
}

impl ProviderPreferencesBuilder {
    /// Set the ordered provider list.
    #[must_use]
    pub fn order(mut self, order: Vec<ProviderSlug>) -> Self {
        self.inner.order = Some(order);
        self
    }

    /// Restrict routing to only the specified providers.
    #[must_use]
    pub fn only(mut self, only: Vec<ProviderSlug>) -> Self {
        self.inner.only = Some(only);
        self
    }

    /// Exclude the specified providers from routing.
    #[must_use]
    pub fn ignore(mut self, ignore: Vec<ProviderSlug>) -> Self {
        self.inner.ignore = Some(ignore);
        self
    }

    /// Set the fallback policy.
    #[must_use]
    pub const fn allow_fallbacks(mut self, policy: FallbackPolicy) -> Self {
        self.inner.allow_fallbacks = Some(policy);
        self
    }

    /// Set the parameter-support requirement.
    #[must_use]
    pub const fn require_parameters(mut self, req: ParameterRequirement) -> Self {
        self.inner.require_parameters = Some(req);
        self
    }

    /// Set the zero-data-retention constraint.
    #[must_use]
    pub const fn zdr(mut self, zdr: ZeroDataRetention) -> Self {
        self.inner.zdr = Some(zdr);
        self
    }

    /// Set the data-collection policy.
    #[must_use]
    pub const fn data_collection(mut self, policy: DataCollection) -> Self {
        self.inner.data_collection = Some(policy);
        self
    }

    /// Set the accepted quantization levels.
    #[must_use]
    pub fn quantizations(mut self, q: Vec<Quantization>) -> Self {
        self.inner.quantizations = Some(q);
        self
    }

    /// Set the provider-sort criterion.
    #[must_use]
    pub const fn sort(mut self, sort: ProviderSort) -> Self {
        self.inner.sort = Some(sort);
        self
    }

    /// Set the per-token-type price caps.
    #[must_use]
    pub const fn max_price(mut self, mp: MaxPrice) -> Self {
        self.inner.max_price = Some(mp);
        self
    }

    /// Set the minimum throughput floor in tokens per second.
    #[must_use]
    pub const fn preferred_min_throughput(mut self, t: ThroughputFloor) -> Self {
        self.inner.preferred_min_throughput = Some(t);
        self
    }

    /// Set the maximum latency ceiling in seconds.
    #[must_use]
    pub const fn preferred_max_latency(mut self, l: LatencyCeiling) -> Self {
        self.inner.preferred_max_latency = Some(l);
        self
    }

    /// Assemble the preferences.
    #[must_use]
    pub fn build(self) -> ProviderPreferences {
        self.inner
    }
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::unwrap_used,
        reason = "tests unwrap known-valid fixtures; a panic is the intended failure signal"
    )]
    #![expect(
        clippy::float_cmp,
        reason = "transparent newtypes store and return exact bits; no arithmetic"
    )]

    use super::*;
    use crate::types::{Price, ProviderSlug};
    use serde_json::json;

    // --- FallbackPolicy ---

    #[test]
    fn fallback_allow_maps_to_true() {
        assert!(FallbackPolicy::Allow.as_bool());
        assert_eq!(
            serde_json::to_value(FallbackPolicy::Allow).unwrap(),
            json!(true)
        );
    }

    #[test]
    fn fallback_deny_maps_to_false() {
        assert!(!FallbackPolicy::Deny.as_bool());
        assert_eq!(
            serde_json::to_value(FallbackPolicy::Deny).unwrap(),
            json!(false)
        );
    }

    // --- ParameterRequirement ---

    #[test]
    fn parameter_required_maps_to_true() {
        assert!(ParameterRequirement::Required.as_bool());
        assert_eq!(
            serde_json::to_value(ParameterRequirement::Required).unwrap(),
            json!(true)
        );
    }

    #[test]
    fn parameter_optional_maps_to_false() {
        assert!(!ParameterRequirement::Optional.as_bool());
        assert_eq!(
            serde_json::to_value(ParameterRequirement::Optional).unwrap(),
            json!(false)
        );
    }

    // --- ZeroDataRetention ---

    #[test]
    fn zdr_enabled_maps_to_true() {
        assert!(ZeroDataRetention::Enabled.as_bool());
        assert_eq!(
            serde_json::to_value(ZeroDataRetention::Enabled).unwrap(),
            json!(true)
        );
    }

    #[test]
    fn zdr_disabled_maps_to_false() {
        assert!(!ZeroDataRetention::Disabled.as_bool());
        assert_eq!(
            serde_json::to_value(ZeroDataRetention::Disabled).unwrap(),
            json!(false)
        );
    }

    // --- DataCollection ---

    #[test]
    fn data_collection_allow_serializes_to_string() {
        assert_eq!(
            serde_json::to_value(DataCollection::Allow).unwrap(),
            json!("allow")
        );
    }

    #[test]
    fn data_collection_deny_serializes_to_string() {
        assert_eq!(
            serde_json::to_value(DataCollection::Deny).unwrap(),
            json!("deny")
        );
    }

    // --- ProviderSort ---

    #[test]
    fn provider_sort_all_variants_serialize_lowercase() {
        assert_eq!(
            serde_json::to_value(ProviderSort::Price).unwrap(),
            json!("price")
        );
        assert_eq!(
            serde_json::to_value(ProviderSort::Throughput).unwrap(),
            json!("throughput")
        );
        assert_eq!(
            serde_json::to_value(ProviderSort::Latency).unwrap(),
            json!("latency")
        );
        assert_eq!(
            serde_json::to_value(ProviderSort::Exacto).unwrap(),
            json!("exacto")
        );
    }

    // --- Quantization ---

    #[test]
    fn quantization_verbatim_wire_tokens() {
        assert_eq!(
            serde_json::to_value(Quantization::Int4).unwrap(),
            json!("int4")
        );
        assert_eq!(
            serde_json::to_value(Quantization::Int8).unwrap(),
            json!("int8")
        );
        assert_eq!(
            serde_json::to_value(Quantization::Fp4).unwrap(),
            json!("fp4")
        );
        assert_eq!(
            serde_json::to_value(Quantization::Fp6).unwrap(),
            json!("fp6")
        );
        assert_eq!(
            serde_json::to_value(Quantization::Fp8).unwrap(),
            json!("fp8")
        );
        assert_eq!(
            serde_json::to_value(Quantization::Fp16).unwrap(),
            json!("fp16")
        );
        assert_eq!(
            serde_json::to_value(Quantization::Bf16).unwrap(),
            json!("bf16")
        );
        assert_eq!(
            serde_json::to_value(Quantization::Fp32).unwrap(),
            json!("fp32")
        );
        assert_eq!(
            serde_json::to_value(Quantization::Unknown).unwrap(),
            json!("unknown")
        );
    }

    // --- ThroughputFloor ---

    #[test]
    fn throughput_floor_accepts_zero_and_positive() {
        assert_eq!(ThroughputFloor::new(0.0).unwrap().get(), 0.0);
        assert_eq!(ThroughputFloor::new(50.0).unwrap().get(), 50.0);
    }

    #[test]
    fn throughput_floor_rejects_negative_nan_inf() {
        assert_eq!(
            ThroughputFloor::new(-1.0).unwrap_err(),
            InvalidThroughputFloor::Invalid
        );
        assert_eq!(
            ThroughputFloor::new(f64::NAN).unwrap_err(),
            InvalidThroughputFloor::Invalid
        );
        assert_eq!(
            ThroughputFloor::new(f64::INFINITY).unwrap_err(),
            InvalidThroughputFloor::Invalid
        );
    }

    #[test]
    fn throughput_floor_serializes_as_number() {
        let t = ThroughputFloor::new(100.0).unwrap();
        assert_eq!(serde_json::to_value(t).unwrap(), json!(100.0));
    }

    // --- LatencyCeiling ---

    #[test]
    fn latency_ceiling_accepts_zero_and_positive() {
        assert_eq!(LatencyCeiling::new(0.0).unwrap().get(), 0.0);
        assert_eq!(LatencyCeiling::new(2.5).unwrap().get(), 2.5);
    }

    #[test]
    fn latency_ceiling_rejects_negative_nan_inf() {
        assert_eq!(
            LatencyCeiling::new(-0.1).unwrap_err(),
            InvalidLatencyCeiling::Invalid
        );
        assert_eq!(
            LatencyCeiling::new(f64::NAN).unwrap_err(),
            InvalidLatencyCeiling::Invalid
        );
        assert_eq!(
            LatencyCeiling::new(f64::INFINITY).unwrap_err(),
            InvalidLatencyCeiling::Invalid
        );
    }

    #[test]
    fn latency_ceiling_serializes_as_number() {
        let l = LatencyCeiling::new(1.5).unwrap();
        assert_eq!(serde_json::to_value(l).unwrap(), json!(1.5));
    }

    // --- MaxPrice ---

    #[test]
    fn max_price_all_none_is_empty_object() {
        let mp = MaxPrice::new();
        assert_eq!(serde_json::to_value(&mp).unwrap(), json!({}));
    }

    #[test]
    fn max_price_partial_omits_nones() {
        let mut mp = MaxPrice::new();
        mp.prompt = Some(Price::new(1.0).unwrap());
        mp.completion = Some(Price::new(2.0).unwrap());
        let v = serde_json::to_value(&mp).unwrap();
        assert_eq!(v["prompt"], json!(1.0));
        assert_eq!(v["completion"], json!(2.0));
        assert!(v.get("image").is_none());
        assert!(v.get("audio").is_none());
        assert!(v.get("request").is_none());
    }

    #[test]
    fn max_price_all_fields_serialize() {
        let mp = MaxPrice {
            prompt: Some(Price::new(1.0).unwrap()),
            completion: Some(Price::new(2.0).unwrap()),
            image: Some(Price::new(3.0).unwrap()),
            audio: Some(Price::new(4.0).unwrap()),
            request: Some(Price::new(0.5).unwrap()),
        };
        let v = serde_json::to_value(&mp).unwrap();
        assert_eq!(v["prompt"], json!(1.0));
        assert_eq!(v["completion"], json!(2.0));
        assert_eq!(v["image"], json!(3.0));
        assert_eq!(v["audio"], json!(4.0));
        assert_eq!(v["request"], json!(0.5));
    }

    // --- ProviderPreferences ---

    #[test]
    fn empty_preferences_serializes_to_empty_object() {
        let prefs = ProviderPreferences::builder().build();
        assert_eq!(serde_json::to_value(&prefs).unwrap(), json!({}));
    }

    #[test]
    fn preferences_all_fields_serialize_correctly() {
        let prefs = ProviderPreferences::builder()
            .order(vec![ProviderSlug::new("openai").unwrap()])
            .only(vec![ProviderSlug::new("anthropic").unwrap()])
            .ignore(vec![ProviderSlug::new("cohere").unwrap()])
            .allow_fallbacks(FallbackPolicy::Deny)
            .require_parameters(ParameterRequirement::Required)
            .zdr(ZeroDataRetention::Enabled)
            .data_collection(DataCollection::Deny)
            .quantizations(vec![Quantization::Fp16])
            .sort(ProviderSort::Throughput)
            .preferred_min_throughput(ThroughputFloor::new(50.0).unwrap())
            .preferred_max_latency(LatencyCeiling::new(2.0).unwrap())
            .build();
        let v = serde_json::to_value(&prefs).unwrap();
        assert_eq!(v["order"], json!(["openai"]));
        assert_eq!(v["only"], json!(["anthropic"]));
        assert_eq!(v["ignore"], json!(["cohere"]));
        assert_eq!(v["allow_fallbacks"], json!(false));
        assert_eq!(v["require_parameters"], json!(true));
        assert_eq!(v["zdr"], json!(true));
        assert_eq!(v["data_collection"], json!("deny"));
        assert_eq!(v["quantizations"], json!(["fp16"]));
        assert_eq!(v["sort"], json!("throughput"));
        assert_eq!(v["preferred_min_throughput"], json!(50.0));
        assert_eq!(v["preferred_max_latency"], json!(2.0));
    }

    #[test]
    fn preferences_with_max_price_field() {
        let mut mp = MaxPrice::new();
        mp.prompt = Some(Price::new(1.5).unwrap());
        let prefs = ProviderPreferences::builder().max_price(mp).build();
        let v = serde_json::to_value(&prefs).unwrap();
        assert_eq!(v["max_price"]["prompt"], json!(1.5));
        assert!(v["max_price"].get("completion").is_none());
    }

    #[test]
    fn builder_round_trip_preserves_all_set_fields() {
        let prefs = ProviderPreferences::builder()
            .sort(ProviderSort::Price)
            .allow_fallbacks(FallbackPolicy::Allow)
            .build();
        let v = serde_json::to_value(&prefs).unwrap();
        assert_eq!(v["sort"], json!("price"));
        assert_eq!(v["allow_fallbacks"], json!(true));
        // Unset fields must be absent.
        assert!(v.get("order").is_none());
        assert!(v.get("only").is_none());
        assert!(v.get("quantizations").is_none());
    }
}
