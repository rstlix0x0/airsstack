//! Type-state builder for [`crate::client::Client`].
//!
//! Encodes "`api_key` must be set before `build()`" in the type system: the
//! `build` method exists only once the first type parameter is `Present`.
//! Callers never see a runtime "missing api key" error — `build()` simply is
//! not callable on a `ClientBuilder<Missing, _>`.
//!
//! Responsibilities:
//! - Declare the sealed [`BuilderApiKeyState`] trait and its inhabitants
//!   ([`Missing`], [`Present`]).
//! - Declare [`ClientBuilder`], generic over the api-key state and transport.
//! - Provide setters that compose regardless of state, the `api_key`
//!   transition, and `build()` on `Present` only.
//!
//! Not responsible for:
//! - Constructing the transport — the caller supplies it via
//!   `Client::builder()` or `Client::builder_with_transport(t)`.

use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;

use crate::auth::Auth;
use crate::client::{Client, ClientInner};
use crate::config::Config;
use crate::transport::HttpTransport;
use crate::types::{ApiKey, BaseUrl};

mod sealed {
    pub trait Sealed {}
}

/// Closed set of builder api-key states. Sealed so downstream crates cannot
/// invent new states; the only inhabitants are [`Missing`] and [`Present`].
pub trait BuilderApiKeyState: sealed::Sealed {}

/// Builder state: no API key supplied yet. `build()` is not callable.
#[derive(Default)]
pub struct Missing;

/// Builder state: API key supplied. `build()` is callable.
#[derive(Default)]
pub struct Present;

impl sealed::Sealed for Missing {}
impl sealed::Sealed for Present {}
impl BuilderApiKeyState for Missing {}
impl BuilderApiKeyState for Present {}

// All mutable data lives in one private non-generic struct so the type-state
// transition moves this whole value without enumerating fields. Adding a field
// touches only: this struct, `new`, the setter, and `build()` — never the
// transition, which is the field-drop risk.
struct ClientBuilderFields {
    api_key: Option<ApiKey>,
    http_referer: Option<String>,
    app_title: Option<String>,
    timeout: Option<Duration>,
    base_url: Option<BaseUrl>,
}

impl ClientBuilderFields {
    const fn new() -> Self {
        Self {
            api_key: None,
            http_referer: None,
            app_title: None,
            timeout: None,
            base_url: None,
        }
    }
}

/// Builder for [`Client<T>`].
///
/// Construct via [`Client::builder`] (default reqwest transport) or
/// [`Client::builder_with_transport`] (any custom transport). The first type
/// parameter encodes whether the API key has been supplied; `build()` exists
/// only once it reaches `Present`.
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

    /// Supply the API key. Transitions `Missing` to `Present`, making `build()`
    /// callable.
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
    /// Set the `HTTP-Referer` attribution header value.
    #[must_use]
    pub fn http_referer(mut self, referer: impl Into<String>) -> Self {
        self.fields.http_referer = Some(referer.into());
        self
    }

    /// Set the `X-Title` attribution header value.
    #[must_use]
    pub fn app_title(mut self, title: impl Into<String>) -> Self {
        self.fields.app_title = Some(title.into());
        self
    }

    /// Override the per-request timeout.
    #[must_use]
    pub const fn timeout(mut self, t: Duration) -> Self {
        self.fields.timeout = Some(t);
        self
    }

    /// Override the base URL. Construct the [`BaseUrl`] via [`BaseUrl::parse`],
    /// which validates the scheme up front.
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
    /// validation. The current implementation cannot fail here — every value
    /// is validated at construction — but the signature reserves the failure
    /// path so future cross-field validations are non-breaking.
    ///
    /// # Panics
    /// Does not panic in practice: the `Present` type-state guarantees
    /// `api_key` was set before `build()` became callable.
    pub fn build(self) -> Result<Client<T>, crate::error::BuildError> {
        #[expect(
            clippy::expect_used,
            reason = "type-state Present guarantees api_key is set; this branch is unreachable"
        )]
        let api_key = self
            .fields
            .api_key
            .expect("invariant: type-state Present guarantees api_key is set");

        let mut config = Config::default();
        if let Some(u) = self.fields.base_url {
            config.base_url = u;
        }
        config.http_referer = self.fields.http_referer;
        config.app_title = self.fields.app_title;
        if let Some(t) = self.fields.timeout {
            config.timeout = t;
        }

        Ok(Client {
            inner: Arc::new(ClientInner {
                config,
                transport: self.transport,
                auth: Auth::Bearer(api_key),
            }),
        })
    }
}

#[cfg(test)]
mod tests {
    /// Optional fields set BEFORE the `api_key` transition must survive the
    /// `Missing -> Present` move. Regression guard for the field-move pattern.
    #[cfg(feature = "__test-mocks")]
    #[test]
    fn optional_fields_survive_api_key_transition() {
        #![expect(
            clippy::unwrap_used,
            reason = "tests unwrap known-valid fixtures; a panic is the intended failure signal"
        )]

        use std::time::Duration;

        use crate::transport::MockHttpTransport;
        use crate::types::{ApiKey, BaseUrl};

        let transport = MockHttpTransport::new();
        let key = ApiKey::new("sk-or-v1-abc").unwrap();
        let base_url = BaseUrl::parse("https://example.test/api/v1/").unwrap();

        let client = super::ClientBuilder::new_with_transport(transport)
            .http_referer("https://myapp.example")
            .app_title("My App")
            .timeout(Duration::from_secs(42))
            .base_url(base_url)
            .api_key(key) // type-state transition: Missing -> Present
            .build()
            .unwrap();

        assert_eq!(client.config().timeout(), Duration::from_secs(42));
        assert_eq!(
            client.config().http_referer(),
            Some("https://myapp.example")
        );
        assert_eq!(client.config().app_title(), Some("My App"));
        assert_eq!(
            client.config().base_url().as_str(),
            "https://example.test/api/v1/"
        );
        assert_eq!(
            client.auth().api_key().map(ApiKey::expose_secret),
            Some("sk-or-v1-abc")
        );
    }
}
