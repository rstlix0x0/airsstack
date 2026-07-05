//! A fixed-concurrency limiter backed by a tokio semaphore.

use std::sync::Arc;

use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use crate::agent::orchestration::core::limiter::Limiter;

/// Admits at most a fixed number of concurrent jobs.
#[derive(Clone)]
pub struct SemaphoreLimiter {
    semaphore: Arc<Semaphore>,
    permits: usize,
}

impl SemaphoreLimiter {
    /// A limiter that admits at most `permits` jobs concurrently.
    #[must_use]
    pub fn new(permits: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(permits)),
            permits,
        }
    }

    /// The configured concurrency bound.
    #[must_use]
    pub const fn permits(&self) -> usize {
        self.permits
    }
}

impl Limiter for SemaphoreLimiter {
    type Permit = OwnedSemaphorePermit;

    async fn acquire(&self) -> Self::Permit {
        match self.semaphore.clone().acquire_owned().await {
            Ok(permit) => permit,
            // The semaphore is owned solely by this limiter and is never
            // closed, so acquisition cannot fail; park rather than panic to
            // keep library code panic-free under the workspace lint bar.
            Err(_) => std::future::pending().await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SemaphoreLimiter;
    use crate::agent::orchestration::core::limiter::Limiter;

    #[test]
    fn reports_its_bound() {
        assert_eq!(SemaphoreLimiter::new(4).permits(), 4);
    }

    #[tokio::test]
    async fn admits_up_to_the_bound_then_blocks() {
        let limiter = SemaphoreLimiter::new(2);
        let _a = limiter.acquire().await;
        let _b = limiter.acquire().await;
        // A third acquire must not resolve while two permits are held.
        let pending_is_err =
            tokio::time::timeout(std::time::Duration::from_millis(20), limiter.acquire())
                .await
                .is_err();
        assert!(pending_is_err, "third acquire blocks at the bound");
    }

    #[tokio::test]
    async fn releasing_a_permit_admits_a_waiter() {
        let limiter = SemaphoreLimiter::new(1);
        let first = limiter.acquire().await;
        drop(first);
        // With the only permit freed, the next acquire resolves promptly.
        let second_is_ok =
            tokio::time::timeout(std::time::Duration::from_millis(20), limiter.acquire())
                .await
                .is_ok();
        assert!(second_is_ok, "freed permit admits the next waiter");
    }
}
