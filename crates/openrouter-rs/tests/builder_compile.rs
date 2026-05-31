//! Compile-fail proofs for the type-state client builder.
#![cfg(feature = "transport-reqwest")]

#[test]
fn type_state_rejects_build_without_api_key() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/client_missing_api_key.rs");
}
