//! Type-state builder for [`crate::client::Client`].
//!
//! Encodes the "`api_key` must be set before `build()`" requirement in the
//! type system: the `build` method only exists once the builder's first
//! type parameter is the `Present` marker. Callers never see a runtime
//! `BuilderError::MissingApiKey` — `build()` simply is not callable on a
//! `ClientBuilder<Missing, _>`.
//!
//! Responsibilities:
//! - Declare the sealed [`BuilderApiKeyState`] trait and its two
//!   inhabitants ([`Missing`], [`Present`]) so downstream crates cannot
//!   extend the state set.
//! - Declare [`ClientBuilder`], generic over the api-key state and the
//!   transport type.
//! - Provide setter methods that compose regardless of api-key state.
//! - Provide [`ClientBuilder::api_key`] as the state transition
//!   `Missing → Present`.
//! - Provide [`ClientBuilder::build`] on `Present` only.
//!
//! Not responsible for:
//! - Constructing the transport — that is the caller's responsibility,
//!   either via `Client::builder()` (which materializes a default
//!   `ReqwestTransport`) or `Client::builder_with_transport(t)`.

use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;

use crate::auth::Auth;
use crate::client::{Client, ClientInner};
use crate::config::Config;
use crate::retry::RetryPolicy;
use crate::transport::HttpTransport;
use crate::types::{AnthropicVersion, ApiKey, BaseUrl, BetaHeader};

mod sealed {
    pub trait Sealed {}
}

/// Closed set of builder api-key states.
///
/// Sealed so downstream crates cannot invent new states; the only
/// inhabitants are [`Missing`] and [`Present`]. These marker names are
/// scoped to this builder — other builders in the crate declare their own
/// state markers under the same sealed-trait pattern, so the generic names
/// never collide across builder types.
pub trait BuilderApiKeyState: sealed::Sealed {}

/// Builder state indicating no API key has been supplied yet.
///
/// `build()` is not callable in this state.
#[derive(Default)]
pub struct Missing;

/// Builder state indicating the API key has been supplied.
///
/// `build()` is callable in this state.
#[derive(Default)]
pub struct Present;

impl sealed::Sealed for Missing {}
impl sealed::Sealed for Present {}
impl BuilderApiKeyState for Missing {}
impl BuilderApiKeyState for Present {}

// All mutable data fields live in one private struct so that the type-state
// transition (`api_key`) moves this whole value without enumerating individual
// fields. Adding a new field requires edits only to: this struct, its
// `new_with_transport` initializer, the relevant setter, and `build()`.
struct ClientBuilderFields {
    api_key: Option<ApiKey>,
    version: AnthropicVersion,
    beta: Vec<BetaHeader>,
    timeout: Option<Duration>,
    retry: Option<RetryPolicy>,
    base_url: Option<BaseUrl>,
}

impl ClientBuilderFields {
    const fn new() -> Self {
        Self {
            api_key: None,
            version: AnthropicVersion::V_2023_06_01,
            beta: Vec::new(),
            timeout: None,
            retry: None,
            base_url: None,
        }
    }
}

/// Builder for [`Client<T>`].
///
/// Construct via [`Client::builder`] (feature-gated default reqwest
/// transport) or [`Client::builder_with_transport`] (any custom
/// transport). The first type parameter encodes whether the API key has
/// been supplied; `build()` only exists once it reaches `Present`.
pub struct ClientBuilder<Key, T>
where
    Key: BuilderApiKeyState,
    T: HttpTransport,
{
    fields: ClientBuilderFields,
    transport: T,
    _key: PhantomData<Key>,
}

impl<T: HttpTransport> ClientBuilder<Missing, T> {
    pub(crate) const fn new_with_transport(transport: T) -> Self {
        Self {
            fields: ClientBuilderFields::new(),
            transport,
            _key: PhantomData,
        }
    }

    /// Supply the API key. Transitions the builder from `Missing` to
    /// `Present`, making `build()` callable.
    #[must_use]
    pub fn api_key(self, key: ApiKey) -> ClientBuilder<Present, T> {
        let mut fields = self.fields;
        fields.api_key = Some(key);
        ClientBuilder {
            fields,
            transport: self.transport,
            _key: PhantomData,
        }
    }
}

impl<Key: BuilderApiKeyState, T: HttpTransport> ClientBuilder<Key, T> {
    /// Override the `anthropic-version` header value.
    #[must_use]
    pub fn anthropic_version(mut self, v: AnthropicVersion) -> Self {
        self.fields.version = v;
        self
    }

    /// Set the `anthropic-beta` header values, replacing any previously
    /// configured list.
    ///
    /// Use [`ClientBuilder::add_anthropic_beta`] to append to the existing
    /// list instead of replacing it.
    #[must_use]
    pub fn set_anthropic_beta(mut self, hs: impl IntoIterator<Item = BetaHeader>) -> Self {
        self.fields.beta = hs.into_iter().collect();
        self
    }

    /// Append a single value to the `anthropic-beta` header list, keeping
    /// any previously configured values.
    #[must_use]
    pub fn add_anthropic_beta(mut self, h: BetaHeader) -> Self {
        self.fields.beta.push(h);
        self
    }

    /// Override the per-request timeout.
    #[must_use]
    pub const fn timeout(mut self, t: Duration) -> Self {
        self.fields.timeout = Some(t);
        self
    }

    /// Override the retry policy.
    #[must_use]
    pub const fn retry(mut self, p: RetryPolicy) -> Self {
        self.fields.retry = Some(p);
        self
    }

    /// Override the base URL.
    ///
    /// Construct the [`BaseUrl`] via [`BaseUrl::parse`], which validates the
    /// scheme up front; the builder therefore never has to reject it.
    #[must_use]
    pub fn base_url(mut self, base_url: BaseUrl) -> Self {
        self.fields.base_url = Some(base_url);
        self
    }
}

impl<T: HttpTransport> ClientBuilder<Present, T> {
    /// Build the configured [`Client<T>`].
    ///
    /// # Errors
    /// Returns [`crate::error::BuildError`] when configuration values fail
    /// validation. The current implementation cannot fail at this point —
    /// every value has been validated at construction — but the signature
    /// reserves the failure path so future validations (e.g. cross-field
    /// constraints) do not become breaking changes.
    ///
    /// # Panics
    /// Does not panic in practice: the type-state `Present` guarantees that
    /// `api_key` was set before `build()` became callable. The `expect` is
    /// an unreachable safety net.
    pub fn build(self) -> Result<Client<T>, crate::error::BuildError> {
        #[expect(
            clippy::expect_used,
            reason = "type-state Present guarantees api_key is set; this branch is unreachable"
        )]
        let api_key = self
            .fields
            .api_key
            .expect("invariant: type-state Present guarantees api_key is set");

        let mut config = Config {
            anthropic_version: self.fields.version,
            ..Config::default()
        };
        if !self.fields.beta.is_empty() {
            config.anthropic_beta = self.fields.beta;
        }
        if let Some(t) = self.fields.timeout {
            config.timeout = t;
        }
        if let Some(u) = self.fields.base_url {
            config.base_url = u;
        }

        let retry = self.fields.retry.unwrap_or_default();
        let auth = Auth::ApiKey(api_key);

        Ok(Client {
            inner: Arc::new(ClientInner {
                config,
                transport: self.transport,
                auth,
                retry,
            }),
        })
    }
}

#[cfg(test)]
mod tests {
    /// Verify that optional fields set BEFORE [`super::ClientBuilder::api_key`]
    /// survive the `Missing → Present` type-state transition.
    ///
    /// This is the regression guard for the field-copy pattern: if a new field
    /// is added to the builder but the transition method forgets to copy it,
    /// this test catches the omission.
    #[cfg(feature = "__test-mocks")]
    #[test]
    fn optional_fields_survive_api_key_transition() {
        #![expect(
            clippy::unwrap_used,
            reason = "tests unwrap known-valid fixtures; a panic is the intended failure signal"
        )]

        use std::time::Duration;

        use crate::retry::RetryPolicy;
        use crate::transport::MockHttpTransport;
        use crate::types::{AnthropicVersion, ApiKey, BaseUrl, BetaHeader};

        let transport = MockHttpTransport::new();
        let key = ApiKey::new("sk-test-abc").unwrap();
        let beta = BetaHeader::new("prompt-caching-2024-07-31").unwrap();
        let base_url = BaseUrl::parse("https://api.example.com").unwrap();

        let client = super::ClientBuilder::new_with_transport(transport)
            .anthropic_version(AnthropicVersion::V_2023_06_01)
            .set_anthropic_beta([beta])
            .timeout(Duration::from_secs(42))
            .retry(RetryPolicy::Disabled)
            .base_url(base_url)
            .api_key(key) // type-state transition: Missing → Present
            .build()
            .unwrap();

        assert_eq!(client.config().timeout(), Duration::from_secs(42));
        assert_eq!(
            client.config().anthropic_version(),
            &AnthropicVersion::V_2023_06_01
        );
        assert_eq!(client.config().anthropic_beta().len(), 1);
        assert!(matches!(client.retry(), RetryPolicy::Disabled));
        assert_eq!(
            client.config().base_url().as_str(),
            "https://api.example.com/"
        );
    }
}
