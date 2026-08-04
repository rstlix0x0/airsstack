//! Models resource (`GET /v1/models`, `GET /v1/models/{id}`).
//!
//! Exists as its own module tree so the model-discovery types and their HTTP
//! dispatch are scoped separately from the Messages API surface, which shares
//! no wire types with them.
//!
//! Responsibilities:
//! - Re-export [`ModelInfo`], [`ModelInfoKind`], [`ModelList`],
//!   [`ModelsResource`], and the [`ModelCapabilities`] tree as the public
//!   surface of this module.
//! - Declare the `capabilities`, `resource`, and `types` submodules that hold
//!   the implementations.
//!
//! Not responsible for:
//! - HTTP transport — that is owned by [`crate::transport`].
//! - Client construction — that is the builder's responsibility.
//!
//! Entry point: [`ModelsResource`], obtained via `client.models()`.

pub mod capabilities;
pub mod resource;
pub mod types;

#[doc(inline)]
pub use capabilities::{
    CapabilitySupport, ContextManagementCapability, EffortCapability, ModelCapabilities,
    ThinkingCapability, ThinkingTypes,
};
#[doc(inline)]
pub use resource::ModelsResource;
#[doc(inline)]
pub use types::{ModelInfo, ModelInfoKind, ModelList};
