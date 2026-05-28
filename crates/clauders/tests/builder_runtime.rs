//! Runtime smoke tests for the type-state builder happy path.
//!
//! Verifies the full builder chain materializes a usable `Client` and
//! that cloning a `Client` shares the internal `Arc` rather than
//! duplicating state.

#![cfg(feature = "transport-reqwest")]
#![allow(clippy::expect_used)]

use std::time::Duration;

use clauders::retry::RetryPolicy;
use clauders::types::{AnthropicVersion, ApiKey, BetaHeader};

#[test]
fn happy_path_constructs_client() {
    let key = ApiKey::new("sk-test-abc").expect("valid key");
    let beta = BetaHeader::new("prompt-caching-2024-07-31").expect("valid beta");

    let client = clauders::Client::builder()
        .expect("builder")
        .api_key(key)
        .anthropic_version(AnthropicVersion::V_2023_06_01)
        .anthropic_beta([beta])
        .timeout(Duration::from_secs(30))
        .retry(RetryPolicy::Disabled)
        .build()
        .expect("happy path");

    assert_eq!(client.config().timeout, Duration::from_secs(30));
    assert_eq!(
        client.config().anthropic_version,
        AnthropicVersion::V_2023_06_01
    );
    assert_eq!(client.config().anthropic_beta.len(), 1);
    assert!(matches!(client.retry(), RetryPolicy::Disabled));
}

#[test]
fn client_clone_shares_inner() {
    let key = ApiKey::new("sk-test-abc").expect("valid key");
    let c1 = clauders::Client::builder()
        .expect("builder")
        .api_key(key)
        .build()
        .expect("happy path");

    assert_eq!(c1.ref_count(), 1);

    let c2 = c1.clone();
    assert_eq!(c1.ref_count(), 2);
    assert_eq!(c2.ref_count(), 2);

    drop(c2);
    assert_eq!(c1.ref_count(), 1);
}
