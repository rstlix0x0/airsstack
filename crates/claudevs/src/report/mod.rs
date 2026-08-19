//! Rendering a suite report for people and for machines, and its exit code.
//!
//! Responsibilities: [`render_human`], [`render_json`], [`exit_code`].

mod render;

pub use render::{exit_code, render_human, render_json};
