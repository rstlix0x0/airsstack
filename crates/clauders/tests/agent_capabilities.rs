#![expect(clippy::expect_used, reason = "tests assert via expect")]

//! Wiring coverage for `CliRuntime::connect` -> the reader task ->
//! `CliRuntime::capabilities()`, driven through the real control protocol
//! rather than reimplemented in miniature. Complements the inline unit test
//! in `runtime.rs` that pins `reader_loop`'s own store-on-`Some` composition
//! directly (that test cannot reach `connect()` or the public accessor,
//! since `CARGO_BIN_EXE_*` is only set for integration tests under `tests/`,
//! not for a unit test module inside `src/`).

use std::time::Duration;

use clauders::agent::{CliRuntime, Options, Runtime};

const TESTCHILD: &str = env!("CARGO_BIN_EXE_clauders-agent-testchild");

/// Poll `capabilities()` until it reports `flag` or `within` elapses.
async fn capabilities_reporting(
    runtime: &CliRuntime,
    flag: &str,
    within: Duration,
) -> clauders::agent::Capabilities {
    let deadline = tokio::time::Instant::now() + within;
    loop {
        let caps = runtime.capabilities();
        if caps.supports(flag) || tokio::time::Instant::now() >= deadline {
            return caps;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

#[tokio::test]
async fn connect_drives_the_init_frame_into_the_public_capabilities_accessor() {
    // The testchild answers the initialize handshake, then emits a
    // system/init message frame carrying `capabilities`. Going through the
    // real `connect()` pins `connect()`'s own `Arc::clone` of the capability
    // store into the reader task, `reader_loop`'s write into it, and
    // `capabilities()` reading the same `Arc` back — none of which a
    // `src/`-local unit test can reach.
    //
    // This test pins the read path ONLY. The testchild also emits a second,
    // key-less init frame, but the poll below returns as soon as the flag
    // appears, so nothing here synchronises on that second frame — the
    // "a key-less frame must not erase a populated manifest" property is
    // pinned by `reader_loop_stores_first_init_frame_and_a_keyless_second_frame_does_not_erase_it`
    // in `src/agent/runtime/cli/runtime.rs`, which drives both frames in order.
    let opts = Options::builder()
        .path_to_executable(TESTCHILD)
        .executable_args(vec![
            "--init-caps".to_string(),
            "interrupt_receipt_v1,msg_lifecycle_v1".to_string(),
        ])
        .build();

    let runtime = CliRuntime::connect(opts).await.expect("connect");

    let caps =
        capabilities_reporting(&runtime, "interrupt_receipt_v1", Duration::from_secs(2)).await;
    assert!(
        caps.supports("interrupt_receipt_v1"),
        "capabilities() must report the flags the init frame carried, read back through \
         connect()'s own Arc"
    );
    assert!(caps.supports("msg_lifecycle_v1"));
}
