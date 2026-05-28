//! Strongly-typed domain primitives.
//!
//! Each primitive is a newtype with a validating constructor: invariants
//! are enforced once at construction and downstream code trusts the type
//! as proof. Constructors return per-type `Invalid*` errors implementing
//! [`std::error::Error`].

mod api_key;

pub use api_key::{ApiKey, InvalidApiKey};
