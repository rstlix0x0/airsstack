//! Models-catalog endpoint for `GET /models`.
//!
//! Exists as a dedicated endpoint module so the catalog DTOs and the resource
//! dispatch logic evolve together, mirroring the `chat/` module layout.
//!
//! Responsibilities:
//! - [`Model`] / [`Pricing`] — the v0 subset of the catalog entry wire format.
//! - [`ModelsResource`] — the short-lived handle vended by
//!   [`crate::client::Client::models`] that dispatches `GET /models`.
//!
//! Not responsible for:
//! - Sending requests beyond this endpoint.
//! - The full catalog entry: fields such as `architecture`, `top_provider`,
//!   and `description` are not modeled and are dropped on decode.

pub mod model;
pub mod resource;

pub use model::{Model, Pricing};
pub use resource::ModelsResource;
