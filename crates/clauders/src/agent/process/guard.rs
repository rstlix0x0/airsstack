use super::ManagedProcess;
use super::supervisor::Supervisor;

impl Drop for ManagedProcess {
    /// Request teardown so a dropped handle never orphans the child.
    ///
    /// `Drop` is synchronous, so it only *signals* the detached supervisor
    /// task (via `Notify`); that task performs the async graceful→kill→reap
    /// sequence. If the runtime is already gone, `kill_on_drop(true)` on the
    /// spawn command is the final SIGKILL safety net.
    fn drop(&mut self) {
        Supervisor::request_shutdown(&self.supervisor);
    }
}
