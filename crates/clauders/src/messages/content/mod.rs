//! Content block types for Messages API request and response bodies.
//!
//! Exists as its own module tree so each direction's content-block union is
//! defined apart from the other and each leaf block shape can be extended
//! independently without touching response decoding or request assembly.
//!
//! Responsibilities:
//! - [`ContentBlock`] (in [`block`]) — the union the API *returns*.
//! - [`ContentBlockParam`] (in [`param`]) — the union a caller *sends*.
//! - [`TextBlock`] / [`ThinkingBlock`] (in [`text`]) — leaf structs shared by
//!   both unions.
//!
//! Not responsible for:
//! - Request construction or response decoding — those live in `request.rs`
//!   and `response.rs` respectively.

pub mod block;
pub mod param;
pub mod text;

pub use block::ContentBlock;
pub use param::ContentBlockParam;
pub use text::{TextBlock, ThinkingBlock};
