//! The admission seam: a [`Limiter`] gates how many jobs run at once.

use std::future::Future;

/// Admission control for the orchestration engine.
///
/// The engine calls [`Limiter::acquire`] before starting each job and holds the
/// returned [`Limiter::Permit`] for that job's lifetime; dropping the permit
/// releases the slot. Implementors decide what "a slot" means — a fixed
/// concurrency bound, a rate budget, a token budget.
pub trait Limiter {
    /// The RAII slot handle. Dropping it releases the admission.
    type Permit: Send + 'static;

    /// Wait until a slot is available, then claim it.
    fn acquire(&self) -> impl Future<Output = Self::Permit> + Send;
}

#[cfg(test)]
mod tests {
    use super::Limiter;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};

    /// A trivial limiter that counts live permits, used only to prove the
    /// trait shape is usable and that a permit releases on drop.
    #[derive(Clone)]
    struct Counting {
        live: Arc<AtomicUsize>,
    }

    struct CountPermit {
        live: Arc<AtomicUsize>,
    }

    impl Drop for CountPermit {
        fn drop(&mut self) {
            self.live.fetch_sub(1, SeqCst);
        }
    }

    impl Limiter for Counting {
        type Permit = CountPermit;

        async fn acquire(&self) -> Self::Permit {
            self.live.fetch_add(1, SeqCst);
            CountPermit {
                live: self.live.clone(),
            }
        }
    }

    #[tokio::test]
    async fn permit_bumps_live_then_releases_on_drop() {
        let limiter = Counting {
            live: Arc::new(AtomicUsize::new(0)),
        };
        let permit = limiter.acquire().await;
        assert_eq!(limiter.live.load(SeqCst), 1);
        drop(permit);
        assert_eq!(limiter.live.load(SeqCst), 0);
    }
}
