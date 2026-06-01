//! Compile-fail proofs for the type-state client builder.
#![cfg(feature = "transport-reqwest")]

#[test]
fn type_state_rejects_build_without_api_key() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/client_missing_api_key.rs");
}

#[test]
fn type_state_rejects_chat_build_without_required_fields() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/chat_missing_model.rs");
    t.compile_fail("tests/compile_fail/chat_missing_messages.rs");
    t.pass("tests/compile_pass/full_chat_request.rs");
}
