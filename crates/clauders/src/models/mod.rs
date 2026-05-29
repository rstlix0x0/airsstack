//! Models resource (`GET /v1/models`, `GET /v1/models/{id}`).
//!
//! Exists as a feature-gated module so the models types and HTTP dispatch
//! are only compiled when the `models` Cargo feature is enabled.
//!
//! Responsibilities:
//! - Re-export [`ModelInfo`], [`ModelInfoKind`], [`ModelList`], and
//!   [`ModelsResource`] as the public surface of this module.
//! - Declare the `resource` and `types` submodules that hold the
//!   implementations.
//!
//! Not responsible for:
//! - HTTP transport — that is owned by [`crate::transport`].
//! - Client construction — that is the builder's responsibility.
//!
//! Entry point: [`ModelsResource`], obtained via `client.models()`.

pub mod resource;
pub mod types;

#[doc(inline)]
pub use resource::ModelsResource;
#[doc(inline)]
pub use types::{ModelInfo, ModelInfoKind, ModelList};
