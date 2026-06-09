#![cfg(all(unix, feature = "agent"))]
#![expect(clippy::expect_used, reason = "tests assert via expect")]

//! Lifecycle tests for `clauders::agent::process` against the controllable
//! `clauders-agent-testchild` binary. Unix-only: reaping is asserted with
//! `nix::sys::signal::kill(pid, None)` returning `ESRCH`.

use std::time::Duration;

use clauders::agent::process::{ManagedProcess, ProcessConfig};
use nix::errno::Errno;
use nix::sys::signal::kill;
use nix::unistd::Pid;
use tokio::time::{sleep, timeout};

const TESTCHILD: &str = env!("CARGO_BIN_EXE_clauders-agent-testchild");

/// True once `pid` is fully reaped (kill(pid, 0) -> ESRCH). A live process
/// or an unreaped zombie returns `Ok`.
fn is_reaped(pid: u32) -> bool {
    let Ok(raw) = i32::try_from(pid) else {
        return true;
    };
    matches!(kill(Pid::from_raw(raw), None), Err(Errno::ESRCH))
}

/// Poll `is_reaped` until true or the deadline elapses.
async fn await_reaped(pid: u32, within: Duration) -> bool {
    let poll = async {
        loop {
            if is_reaped(pid) {
                return true;
            }
            sleep(Duration::from_millis(20)).await;
        }
    };
    timeout(within, poll).await.unwrap_or(false)
}

// Graceful-path tests use a generous grace: it is only the upper bound on the
// wait for a child to exit on its own, so a child that exits promptly returns
// immediately regardless of this value. Kill-path tests (escalation, drop)
// override `shutdown_grace` with a short value so the forced kill fires fast.
fn config(args: &[&str]) -> ProcessConfig {
    let mut cfg = ProcessConfig::new(TESTCHILD);
    cfg.args = args.iter().map(|s| (*s).to_string()).collect();
    cfg.shutdown_grace = Duration::from_secs(2);
    cfg
}

#[tokio::test]
async fn reaps_child_on_normal_exit() {
    let (proc, io) = ManagedProcess::spawn(&config(&[])).expect("spawn");
    let pid = proc.id().expect("pid");

    // Drop stdin -> EOF -> the default child echoes nothing and exits 0.
    drop(io.stdin);

    let status = proc.shutdown().await.expect("shutdown");
    assert!(status.success(), "expected clean exit, got {status:?}");
    assert!(
        await_reaped(pid, Duration::from_secs(2)).await,
        "child not reaped (zombie)"
    );
}

#[tokio::test]
async fn escalates_to_kill_when_child_ignores_eof() {
    // Short grace so the forced kill fires well within 2 s.
    let mut cfg = config(&["--ignore-eof"]);
    cfg.shutdown_grace = Duration::from_millis(300);
    let (proc, io) = ManagedProcess::spawn(&cfg).expect("spawn");
    let pid = proc.id().expect("pid");

    // EOF is ignored by this child, so graceful wait must time out and the
    // supervisor must escalate to a forced kill.
    drop(io.stdin);

    let started = std::time::Instant::now();
    let status = proc.shutdown().await.expect("shutdown");
    assert!(!status.success(), "killed child should not report success");
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "shutdown took too long; escalation did not fire"
    );
    assert!(
        await_reaped(pid, Duration::from_secs(2)).await,
        "child not reaped after kill"
    );
}

#[tokio::test]
async fn dropping_handle_kills_child_without_explicit_shutdown() {
    let mut cfg = config(&["--ignore-eof"]);
    cfg.shutdown_grace = Duration::from_millis(300);
    let (proc, io) = ManagedProcess::spawn(&cfg).expect("spawn");
    let pid = proc.id().expect("pid");
    drop(io.stdin);

    // Drop the handle without calling shutdown(): the Drop bridge must
    // signal the supervisor, which kills and reaps the child.
    drop(proc);

    assert!(
        await_reaped(pid, Duration::from_secs(2)).await,
        "child orphaned after handle drop"
    );
}

#[tokio::test]
async fn captures_stderr_flood_without_deadlock() {
    // Child spams ~256 KiB to stderr (past the OS pipe buffer) then exits
    // on EOF. If stderr were not drained, the child would block and the
    // test would hang; the timeout guards against that.
    let (proc, io) = ManagedProcess::spawn(&config(&["--spam-stderr"])).expect("spawn");
    let stderr = io.stderr.clone();
    drop(io.stdin);

    let status = timeout(Duration::from_secs(3), proc.shutdown())
        .await
        .expect("shutdown timed out — stderr drain deadlock")
        .expect("shutdown");
    assert!(status.success());

    let captured = stderr.snapshot();
    assert!(!captured.is_empty(), "expected captured stderr");
    assert!(
        captured.bytes().all(|b| b == b'E'),
        "unexpected stderr content"
    );
}

#[tokio::test]
async fn shutdown_is_idempotent() {
    let (proc, io) = ManagedProcess::spawn(&config(&[])).expect("spawn");
    let pid = proc.id().expect("pid");
    drop(io.stdin);

    let first = proc.shutdown().await.expect("first shutdown");
    let second = proc.shutdown().await.expect("second shutdown");
    assert_eq!(
        first.code(),
        second.code(),
        "repeated shutdown changed the outcome"
    );

    // A third teardown via Drop must also be a no-op (no panic).
    drop(proc);
    assert!(await_reaped(pid, Duration::from_secs(2)).await);
}
