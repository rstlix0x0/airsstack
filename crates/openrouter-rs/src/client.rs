//! `Client<T>` — the SDK handle constructed by the client builder.
//!
//! Lives in its own module so the handle, its inner state, and its accessors
//! are scoped together while the builder lives in its own file. The handle is
//! generic over the transport `T` (static-dispatch policy) and shares state
//! through an internal `Arc<ClientInner<T>>`, so cloning is a refcount bump.
//!
//! Responsibilities:
//! - Declare [`Client`] (generic over `T: HttpTransport`) and `ClientInner`.
//! - Implement `Clone` via `Arc::clone` and a `Debug` that omits credentials.
//! - Expose read accessors for [`Config`] and [`Auth`].
//! - Provide builder entry points and a [`DefaultClient`] alias.
//!
//! Not responsible for:
//! - Constructing the client — that is the builder's job.
//! - Sending requests — resource accessors and request methods are added by
//!   each capability module.

// The accessors (`config`, `auth`, `ref_count`) are trivial forwarders into
// the inner `Arc`. No inline tests per rust-unit-test-mandate exemption #1
// (trivial getter). The construct-and-read contract is exercised by the
// builder's inline runtime test and the trybuild fixture.

use std::sync::Arc;

use crate::auth::Auth;
use crate::config::Config;
use crate::transport::HttpTransport;

#[cfg(feature = "transport-reqwest")]
use crate::transport::ReqwestTransport;

/// `Client` specialized to the default reqwest transport.
#[cfg(feature = "transport-reqwest")]
#[cfg_attr(docsrs, doc(cfg(feature = "transport-reqwest")))]
pub type DefaultClient = Client<ReqwestTransport>;

/// SDK client generic over the HTTP transport.
///
/// `Client<T>` is the single handle every SDK call goes through. The transport
/// is a generic parameter (static-dispatch policy); cloning shares state via an
/// internal `Arc` rather than duplicating it.
pub struct Client<T = DefaultTransportPlaceholder>
where
    T: HttpTransport,
{
    pub(crate) inner: Arc<ClientInner<T>>,
}

/// Default transport substituted into `Client<T = DefaultTransportPlaceholder>`.
///
/// Aliases [`ReqwestTransport`] when `transport-reqwest` is enabled; otherwise
/// a stand-in whose `send` always errors, so the default type parameter
/// resolves to a concrete `HttpTransport` impl under every feature set.
#[cfg(feature = "transport-reqwest")]
#[cfg_attr(docsrs, doc(cfg(feature = "transport-reqwest")))]
pub type DefaultTransportPlaceholder = ReqwestTransport;

/// Default-transport stand-in used when no transport feature is enabled.
///
/// Every `send` returns [`crate::error::TransportError::Other`] explaining no
/// transport is configured. Keeping one default type parameter across both
/// feature configurations keeps `Client<T>`'s signature stable for rustdoc and
/// downstream generic code.
#[cfg(not(feature = "transport-reqwest"))]
pub struct DefaultTransportPlaceholder;

#[cfg(not(feature = "transport-reqwest"))]
#[async_trait::async_trait]
impl HttpTransport for DefaultTransportPlaceholder {
    async fn send(
        &self,
        _req: http::Request<bytes::Bytes>,
    ) -> Result<http::Response<crate::transport::BodyStream>, crate::error::TransportError> {
        Err(crate::error::TransportError::Other(
            "no transport configured: enable feature `transport-reqwest` or supply a custom transport via the client builder".into(),
        ))
    }
}

pub(crate) struct ClientInner<T: HttpTransport> {
    pub(crate) config: Config,
    // Dispatch target for resource request methods (e.g. the chat resource).
    pub(crate) transport: T,
    pub(crate) auth: Auth,
}

impl<T: HttpTransport> Clone for Client<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<T: HttpTransport> std::fmt::Debug for Client<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client")
            .field("config", &self.inner.config)
            .finish_non_exhaustive()
    }
}

impl<T: HttpTransport> Client<T> {
    /// Borrow the static request configuration.
    #[must_use]
    pub fn config(&self) -> &Config {
        &self.inner.config
    }

    /// Borrow the configured authentication scheme.
    ///
    /// Exposed primarily for tests and debugging.
    #[must_use]
    pub fn auth(&self) -> &Auth {
        &self.inner.auth
    }

    /// Number of `Client` handles currently sharing the same internal state.
    ///
    /// Cloning a `Client` is a refcount bump on the internal `Arc`; this reads
    /// the live count. Best-effort diagnostic, not a synchronization primitive.
    #[must_use]
    pub fn ref_count(&self) -> usize {
        Arc::strong_count(&self.inner)
    }

    /// Begin building a client with the supplied transport.
    ///
    /// Infallible — callers who already hold a configured transport (custom
    /// implementations, pre-tuned `ReqwestTransport`, test mocks) use this
    /// instead of [`Client::builder`], which materializes a default transport
    /// and can fail.
    #[must_use]
    pub const fn builder_with_transport(
        transport: T,
    ) -> crate::builder::ClientBuilder<crate::builder::Missing, T> {
        crate::builder::ClientBuilder::new_with_transport(transport)
    }

    /// Begin a chat-completions call.
    ///
    /// Returns a short-lived [`crate::chat::ChatResource`] borrowing this
    /// client; create it at the call site and drop it after the call.
    #[must_use]
    pub const fn chat(&self) -> crate::chat::ChatResource<'_, T> {
        crate::chat::ChatResource { client: self }
    }

    /// Begin a models-catalog call.
    ///
    /// Returns a short-lived [`crate::models::ModelsResource`] borrowing this
    /// client; create it at the call site and drop it after the call.
    #[must_use]
    pub const fn models(&self) -> crate::models::ModelsResource<'_, T> {
        crate::models::ModelsResource { client: self }
    }
}

#[cfg(feature = "transport-reqwest")]
#[cfg_attr(docsrs, doc(cfg(feature = "transport-reqwest")))]
impl Client<ReqwestTransport> {
    /// Begin building a client with the default `ReqwestTransport`.
    ///
    /// # Errors
    /// Returns [`crate::error::BuildError`] when the underlying
    /// `reqwest::Client` cannot be constructed (typically TLS-backend init).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use openrouter_rs::Client;
    /// use openrouter_rs::types::ApiKey;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::builder()?
    ///     .api_key(ApiKey::new("sk-or-v1-...")?)
    ///     .build()?;
    /// # let _ = client;
    /// # Ok(())
    /// # }
    /// ```
    pub fn builder() -> Result<
        crate::builder::ClientBuilder<crate::builder::Missing, ReqwestTransport>,
        crate::error::BuildError,
    > {
        let transport = ReqwestTransport::try_new()
            .map_err(|e| crate::error::BuildError::Transport(e.to_string()))?;
        Ok(crate::builder::ClientBuilder::new_with_transport(transport))
    }
}
