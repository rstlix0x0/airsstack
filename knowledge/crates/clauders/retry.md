---
type: Rust Module
title: clauders::retry
description: RetryPolicy and exponential-backoff arithmetic for the SDK request path — pure, deterministic, no I/O or clock.
tags: [rust, sdk, retry, backoff]
timestamp: 2026-07-03T00:00:00Z
resource: crates/clauders/src/retry.rs
---

Retry behaviour is orthogonal to [Config](/crates/clauders/config.md): a
caller may pin one `Config` and swap the retry strategy per call site. The
arithmetic here is pure (no I/O, no clock) so every branch is unit-testable
deterministically; honouring server `Retry-After` headers and sleeping
between attempts are the request layer's job, not this module's.

# Schema

- `RetryPolicy` — `Disabled` | `ExponentialBackoff(ExpBackoff)`. Default: `ExponentialBackoff(ExpBackoff::default())`.
- `ExpBackoff` — crate-private-field struct built only via `try_new`:
  `max_attempts: NonZeroU32`, `initial: Duration`, `max: Duration`,
  `multiplier: f32`, `jitter: Jitter`. Default: 3 attempts, 250ms..8s, ×2.0, `Jitter::Full`.
- `Jitter` — `None` | `Equal` (half-fixed/half-random) | `Full` (uniform in `[0, curve]`).
- `InvalidExpBackoff` — `#[non_exhaustive]`: `NonFiniteMultiplier(f32)`,
  `NonPositiveMultiplier(f32)`, `InitialExceedsMax { initial, max }`.

## Methods

- `RetryPolicy::backoff(&self, attempt: u32) -> Duration` — delay before retry
  index `attempt` (0-based, `attempt = 0` is the first retry); `Disabled`
  always returns `Duration::ZERO`. Growth caps at `max` even on overflow/NaN
  inputs (never panics).
- `RetryPolicy::max_attempts(&self) -> u32` — total attempts including the
  original request; `Disabled` returns `1`.
- `ExpBackoff::try_new(...) -> Result<Self, InvalidExpBackoff>` — the only
  public constructor; validates `multiplier` is finite/positive and `initial <= max`.

# Examples

```rust
use clauders::RetryPolicy;
let p = RetryPolicy::default();
assert!(p.backoff(0) < p.backoff(1));
assert_eq!(RetryPolicy::Disabled.max_attempts(), 1);
```

Related: [Error::is_retryable / retry_after](/crates/clauders/error.md),
[ClientBuilder::retry](/crates/clauders/builder.md).

# Citations

1. `crates/clauders/src/retry.rs`
