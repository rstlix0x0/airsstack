//! Unofficial Rust SDK for the Anthropic Claude Messages API.
//!
//! This crate is a work in progress. Phase 1 ships the workspace skeleton
//! only — the SDK surface is added in subsequent phases. See the project
//! README and the design spec under `.superpowers/specs/` for the full plan.
//!
//! All public items will be added behind feature flags as documented in the
//! design spec; the Phase 1 build deliberately compiles a crate with no
//! public items so the workspace lints, formatter, and rustdoc pipeline can
//! be wired in cleanly before any production code lands.
#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]
