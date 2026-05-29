//! `Client<T>` — the SDK handle constructed by the upcoming client builder.
//!
//! Lives in its own module so the handle, its inner state, and its
//! accessors are scoped together while the builder, retry policy, and
//! request methods stay in their own modules. The handle is generic over
//! the transport `T` (per the static-dispatch policy) and shares state
//! through an internal `Arc<ClientInner<T>>` so cloning is cheap.
//!
//! Responsibilities:
//! - Declare [`Client`] (generic over `T: HttpTransport`) and the
//!   internal `ClientInner` state.
//! - Implement `Clone` via `Arc::clone` and `Debug` such that the
//!   credential material is omitted from formatted output.
//! - Expose narrow read accessors for [`Config`], [`Auth`], and
//!   [`RetryPolicy`].
//! - Provide a [`DefaultClient`] alias and a [`DefaultTransportPlaceholder`]
//!   so the type signature `Client<T = DefaultTransportPlaceholder>`
//!   resolves under both default-feature and `--no-default-features`
//!   builds.
//!
//! Not responsible for:
//! - Constructing the client — that is the builder's job.
//! - Sending requests — the request methods land in a later phase.

use std::sync::Arc;

use crate::auth::Auth;
use crate::config::Config;
use crate::retry::RetryPolicy;
use crate::transport::HttpTransport;

#[cfg(feature = "transport-reqwest")]
use crate::transport::ReqwestTransport;

/// `Client` specialized to the default reqwest transport.
///
/// Available only when the `transport-reqwest` feature is enabled.
#[cfg(feature = "transport-reqwest")]
#[cfg_attr(docsrs, doc(cfg(feature = "transport-reqwest")))]
pub type DefaultClient = Client<ReqwestTransport>;

/// SDK client generic over the HTTP transport.
///
/// `Client<T>` is the single handle every SDK call goes through. The
/// transport is a generic parameter (per the static-dispatch policy);
/// cloning the client shares state via an internal `Arc` rather than
/// duplicating it. The default type parameter is the placeholder transport
/// that resolves to `ReqwestTransport` when the `transport-reqwest`
/// feature is on.
pub struct Client<T = DefaultTransportPlaceholder>
where
    T: HttpTransport,
{
    pub(crate) inner: Arc<ClientInner<T>>,
}

/// Default transport substituted into `Client<T = DefaultTransportPlaceholder>`.
///
/// When the `transport-reqwest` feature is enabled this is an alias for
/// [`ReqwestTransport`]. When the feature is disabled this is a
/// stand-in unit struct whose `send` always errors — callers that want a
/// working client without the reqwest feature must supply their own
/// transport at builder time.
#[cfg(feature = "transport-reqwest")]
#[cfg_attr(docsrs, doc(cfg(feature = "transport-reqwest")))]
pub type DefaultTransportPlaceholder = ReqwestTransport;

/// Default-transport stand-in used when no transport feature is enabled.
///
/// Implements [`HttpTransport`] but every `send` call returns
/// [`crate::error::TransportError::Other`] explaining that no transport
/// is configured. The struct exists so the default type parameter on
/// [`Client`] resolves to a concrete `HttpTransport` impl regardless of
/// feature configuration.
///
/// Keeping a single default type parameter across both feature
/// configurations — rather than removing it when `transport-reqwest` is
/// off — keeps `Client<T>`'s signature stable for rustdoc, IDE tooling,
/// and generic downstream code. Callers that build without a transport
/// feature must supply their own transport via
/// [`Client::builder_with_transport`]; the clear `send`-time error guides
/// anyone who reaches this stand-in by mistake.
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
    // `dead_code` fires on `--no-default-features` builds (no `messages` feature, no callers)
    // but NOT on the default build (resource.rs reads this field). Per M-LINT-OVERRIDE-EXPECT,
    // `#[allow]` is correct for conditionally-firing lints where `#[expect]` would warn on the
    // passing configuration.
    #[allow(dead_code)]
    pub(crate) transport: T,
    pub(crate) auth: Auth,
    pub(crate) retry: RetryPolicy,
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
            .field("retry", &self.inner.retry)
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
    /// Exposed primarily for tests and debugging; production callers do
    /// not normally need to inspect the credential after construction.
    #[must_use]
    pub fn auth(&self) -> &Auth {
        &self.inner.auth
    }

    /// Borrow the retry policy.
    #[must_use]
    pub fn retry(&self) -> &RetryPolicy {
        &self.inner.retry
    }

    /// Number of `Client` handles currently sharing the same internal state.
    ///
    /// Cloning a `Client` is a refcount bump on an internal `Arc`; this
    /// returns the live count. Useful for diagnostics and for tests that
    /// want to verify clones do not duplicate the underlying state.
    ///
    /// # Notes
    /// The count is read non-atomically: other threads may clone or drop
    /// a `Client` between observation and use. Treat the value as a
    /// best-effort diagnostic, not a synchronization primitive.
    #[must_use]
    pub fn ref_count(&self) -> usize {
        Arc::strong_count(&self.inner)
    }

    /// Return a resource handle for the Messages API.
    ///
    /// The returned handle borrows `self` for its lifetime, so it is
    /// typically created inline at the call site rather than stored.
    #[cfg(feature = "messages")]
    #[cfg_attr(docsrs, doc(cfg(feature = "messages")))]
    #[must_use]
    pub const fn messages(&self) -> crate::messages::MessagesResource<'_, T> {
        crate::messages::MessagesResource { client: self }
    }

    /// Begin building a client with the supplied transport.
    ///
    /// Infallible — callers who already hold a configured transport
    /// (custom implementations, pre-tuned `ReqwestTransport`, test mocks)
    /// reach for this entry point instead of [`Client::builder`], which
    /// is fallible because it materializes a default `ReqwestTransport`
    /// and TLS-backend initialization can fail.
    #[must_use]
    pub const fn builder_with_transport(
        transport: T,
    ) -> crate::builder::ClientBuilder<crate::builder::Missing, T> {
        crate::builder::ClientBuilder::new_with_transport(transport)
    }
}

#[cfg(feature = "transport-reqwest")]
#[cfg_attr(docsrs, doc(cfg(feature = "transport-reqwest")))]
impl Client<ReqwestTransport> {
    /// Begin building a client with the default `ReqwestTransport`.
    ///
    /// Constructs the underlying transport via
    /// [`ReqwestTransport::try_new`]; failures (typically TLS-backend
    /// initialization) surface as [`crate::error::BuildError::Transport`].
    ///
    /// # Errors
    /// Returns [`crate::error::BuildError`] when the underlying
    /// `reqwest::Client` cannot be constructed.
    pub fn builder() -> Result<
        crate::builder::ClientBuilder<crate::builder::Missing, ReqwestTransport>,
        crate::error::BuildError,
    > {
        let transport = ReqwestTransport::try_new()
            .map_err(|e| crate::error::BuildError::Transport(e.to_string()))?;
        Ok(crate::builder::ClientBuilder::new_with_transport(transport))
    }
}
