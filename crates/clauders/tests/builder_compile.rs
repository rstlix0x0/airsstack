//! `trybuild` fixtures that lock the type-state behaviour of `ClientBuilder`.
//!
//! Compile-fail proves `build()` is unavailable on a `Missing`-state
//! builder; compile-pass proves the full happy-path chain (fallible
//! `Client::builder()` plus the type-state transition via `api_key`)
//! compiles end-to-end.

#![cfg(all(feature = "transport-reqwest", feature = "messages"))]

#[test]
fn type_state_compile_contracts() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/builder_missing_api_key.rs");
    t.pass("tests/compile_pass/full_builder.rs");
    t.compile_fail("tests/compile_fail/request_missing_model.rs");
    t.compile_fail("tests/compile_fail/request_missing_max_tokens.rs");
}
