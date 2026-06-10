//! Opt-in end-to-end test against a real backend binary.
//!
//! Excluded from the default gate (the test is `#[ignore]`d) and additionally
//! guarded by `CLAUDERS_AGENT_E2E=1`. Run explicitly with a real `claude`
//! binary on `PATH`:
//!
//! ```text
//! CLAUDERS_AGENT_E2E=1 cargo test -p clauders --all-features --test agent_e2e -- --ignored
//! ```
#![cfg(feature = "agent")]
#![expect(clippy::expect_used, reason = "test assertions use expect for context")]

use clauders::agent::{Message, Options, query};
use futures_util::StreamExt;

#[tokio::test]
#[ignore = "requires a real claude binary; opt in with CLAUDERS_AGENT_E2E=1"]
async fn one_shot_query_reaches_a_result_frame() {
    if std::env::var("CLAUDERS_AGENT_E2E").is_err() {
        eprintln!("skipping: set CLAUDERS_AGENT_E2E=1 to run");
        return;
    }
    let mut stream = query("Say hi in one word.", Options::default())
        .await
        .expect("connect and send prompt");
    let mut saw_result = false;
    while let Some(item) = stream.next().await {
        if let Ok(Message::Result(_)) = item {
            saw_result = true;
            break;
        }
    }
    assert!(saw_result, "expected a terminal Result frame");
}
