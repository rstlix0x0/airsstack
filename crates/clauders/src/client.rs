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
//!   resolves to the default reqwest transport.
//!
//! Not responsible for:
//! - Constructing the client — that is the builder's job.
//! - Sending requests — the request methods land in a later phase.

use std::sync::Arc;

use crate::auth::Auth;
use crate::config::Config;
use crate::retry::RetryPolicy;
use crate::transport::HttpTransport;
use crate::transport::ReqwestTransport;

/// `Client` specialized to the default reqwest transport.
pub type DefaultClient = Client<ReqwestTransport>;

/// SDK client generic over the HTTP transport.
///
/// `Client<T>` is the single handle every SDK call goes through. The
/// transport is a generic parameter (per the static-dispatch policy);
/// cloning the client shares state via an internal `Arc` rather than
/// duplicating it. The default type parameter is the placeholder transport
/// that resolves to `ReqwestTransport`.
pub struct Client<T = DefaultTransportPlaceholder>
where
    T: HttpTransport,
{
    pub(crate) inner: Arc<ClientInner<T>>,
}

/// Default transport substituted into `Client<T = DefaultTransportPlaceholder>`.
///
/// An alias for [`ReqwestTransport`]. Callers that want a different
/// transport supply their own at builder time via
/// [`Client::builder_with_transport`].
pub type DefaultTransportPlaceholder = ReqwestTransport;

pub(crate) struct ClientInner<T: HttpTransport> {
    pub(crate) config: Config,
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
    #[must_use]
    pub const fn messages(&self) -> crate::messages::MessagesResource<'_, T> {
        crate::messages::MessagesResource { client: self }
    }

    /// Return a resource handle for the Models API.
    ///
    /// The returned handle borrows `self` for its lifetime, so it is
    /// typically created inline at the call site rather than stored.
    #[must_use]
    pub const fn models(&self) -> crate::models::ModelsResource<'_, T> {
        crate::models::ModelsResource { client: self }
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
        let transport = ReqwestTransport::try_new_with_user_agent(concat!(
            "clauders/",
            env!("CARGO_PKG_VERSION")
        ))
        .map_err(|e| crate::error::BuildError::Transport(e.to_string()))?;
        Ok(crate::builder::ClientBuilder::new_with_transport(transport))
    }
}
