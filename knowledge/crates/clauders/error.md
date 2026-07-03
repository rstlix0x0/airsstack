---
type: Rust Module
title: clauders::error
description: Layered SDK error hierarchy — Error wraps TransportError, ApiError, BuildError, and SDK-internal decode/protocol failures behind one Result<T, Error> return type.
tags: [rust, sdk, error-handling]
timestamp: 2026-07-03T00:00:00Z
resource: crates/clauders/src/error.rs
---

Every fallible public SDK call returns `Result<T, Error>`. The error surface
is layered so callers can match on the failure domain without parsing
strings.

# Schema

- `TransportError` — re-exported from `airs_transport`; network/TLS/timeout/framing failures.
- `ApiError` — non-2xx response with a decoded `{"type":"error","error":{...}}` envelope.
  Fields: `status: StatusCode`, `body: ApiErrorBody`, `request_id: Option<RequestId>`,
  `organization_id: Option<OrganizationId>`, `retry_after: Option<Duration>`. `#[non_exhaustive]`.
  `is_retryable()` — true for `RateLimitError`, `OverloadedError`, `ApiError`.
- `ApiErrorBody` — `kind: ErrorType`, `message: String`.
- `ErrorType` — `#[non_exhaustive]` enum: `InvalidRequestError`, `AuthenticationError`,
  `PermissionError`, `NotFoundError`, `RequestTooLarge`, `RateLimitError`, `ApiError`,
  `OverloadedError`, `Unknown` (serde catch-all via `#[serde(other)]`, forward-compat).
- `BuildError` — `#[non_exhaustive]`: `BaseUrl(String)`, `Transport(String)`, `InvalidConfig(String)`.
- `Error` — top-level `#[non_exhaustive]` wrapper:
  `Transport(TransportError)`, `Api(ApiError)`,
  `UndecodableApiError { status, detail, request_id }`,
  `Serde { context: &'static str, source: serde_json::Error }`,
  `InvalidRequest(String)`, `Build(BuildError)`,
  `Stream(String)` (feature `messages-streaming`),
  `JsonLines(String)` (feature `messages-batches`).

## Methods on `Error`

- `is_retryable() -> bool` — delegates to the wrapped `TransportError`/`ApiError`
  classification; all other variants are non-retryable.
- `retry_after() -> Option<Duration>`, `request_id() -> Option<&RequestId>`,
  `organization_id() -> Option<&OrganizationId>` — inspect retry/correlation
  metadata without matching variants by hand.

# Examples

```rust
use clauders::{Error, BuildError};
let e: Error = BuildError::BaseUrl("not a url".into()).into();
assert!(!e.is_retryable());
```

Related: [RetryPolicy](/crates/clauders/retry.md) (consumes `is_retryable`
via the request layer), [messages resource](/crates/clauders/messages/resource.md)
(the primary producer of `Error` variants), [RequestId/OrganizationId](/crates/clauders/types/ids.md).

# Citations

1. `crates/clauders/src/error.rs`
