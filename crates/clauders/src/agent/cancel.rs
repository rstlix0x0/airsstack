//! The cooperative cancellation signal handed to an in-flight handler.

use tokio_util::sync::CancellationToken;

/// Signals that the binary withdrew the request a handler is servicing.
///
/// Cancellation is **cooperative**, matching the official SDKs: the handler
/// is not killed. A handler that ignores this runs to completion and its
/// response is still written; a handler that observes it should stop work and
/// return an error.
///
/// # Examples
///
/// ```
/// use clauders::agent::CancelSignal;
///
/// let signal = CancelSignal::new();
/// assert!(!signal.is_cancelled());
/// ```
#[derive(Clone, Debug, Default)]
pub struct CancelSignal(CancellationToken);

impl CancelSignal {
    /// A signal that has not been cancelled.
    ///
    /// Public so a downstream handler implementation can be unit tested
    /// without a live session.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether cancellation has been requested.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.0.is_cancelled()
    }

    /// Resolves once cancellation is requested; otherwise never resolves.
    pub async fn cancelled(&self) {
        self.0.cancelled().await;
    }

    /// Requests cancellation, waking every clone's [`cancelled`](Self::cancelled) future.
    pub(crate) fn cancel(&self) {
        self.0.cancel();
    }
}

#[cfg(test)]
mod tests {
    use super::CancelSignal;

    #[test]
    fn a_fresh_signal_is_not_cancelled() {
        assert!(!CancelSignal::new().is_cancelled());
    }

    #[tokio::test]
    async fn cancelled_resolves_after_cancel() {
        let signal = CancelSignal::new();
        let observer = signal.clone();
        signal.cancel();
        observer.cancelled().await;
        assert!(observer.is_cancelled());
    }

    #[test]
    fn clones_share_one_state() {
        let signal = CancelSignal::new();
        let clone = signal.clone();
        signal.cancel();
        assert!(
            clone.is_cancelled(),
            "a clone must observe the cancellation"
        );
    }
}
