# How to handle errors

Every fallible SDK call returns `Result<T, openrouter_rs::error::Error>`. Match
on the variant; never parse the display string.

## Match the failure domain

```rust
use openrouter_rs::error::Error;

match client.chat().send(req).await {
    Ok(completion) => { /* ... */ }

    Err(Error::RateLimit { retry_after }) => {
        // 429. retry_after is Some only when the server sent Retry-After.
    }
    Err(Error::Moderation { reasons, provider_name, .. }) => {
        // 403 with moderation metadata. Your input was flagged — do not retry.
        eprintln!("blocked by {provider_name}: {reasons:?}");
    }
    Err(Error::Provider { provider_name, raw }) => {
        // Upstream provider error passed through. `raw` is the provider's payload.
        eprintln!("{provider_name} said: {raw}");
    }
    Err(Error::Api { status, code, message, .. }) => {
        // Any other non-2xx with a decodable error envelope.
        eprintln!("api {status}/{code}: {message}");
    }
    Err(Error::UndecodableApiError { status, detail }) => {
        // Non-2xx whose body was not a recognised envelope — a proxy or an
        // HTML error page, usually. `detail` is the raw body text.
    }
    Err(Error::Transport(e)) => {
        // Network, TLS, timeout, or body-framing failure.
    }
    Err(Error::Stream(msg)) => {
        // SSE framing failure or a mid-stream error event.
    }
    Err(Error::Serde { context, source }) => {
        // The SDK could not encode the request or decode the response.
        // `context` names the type: "ChatRequest", "ChatCompletion", "StreamChunk", …
    }
    Err(Error::InvalidRequest(msg)) => {
        // Rejected before the network: bad URL join, unbuildable HTTP request.
    }
    Err(Error::Build(e)) => { /* client construction failed */ }
}
```

`Error` is `#[non_exhaustive]`, so a `_` arm is required and new variants are not
a breaking change.

## Write a retry loop

**The SDK has no retry layer.** It classifies failures and hands you the
decision. `Error::is_retryable()` is the signal:

| Retryable | Not retryable |
|---|---|
| `Transport(Network)`, `Transport(Timeout)` | `Transport(Tls \| BodyStream \| Build \| Other)` |
| `RateLimit` | `Moderation`, `Provider` |
| `Api` with status 408, 500, 502, 503 | `Api` with any other status |
| | `Stream`, `Serde`, `InvalidRequest`, `Build` |

`Error::retry_after()` returns the server-supplied delay when the failure is a
`RateLimit` and the `Retry-After` header held an integer number of seconds; it
is `None` for every other variant, and `None` when the header used the HTTP-date
form (which this API does not send).

```rust
use std::time::Duration;

async fn send_with_retry<T: openrouter_rs::transport::HttpTransport>(
    client: &openrouter_rs::Client<T>,
    req: openrouter_rs::ChatRequest,
    max_attempts: u32,
) -> Result<openrouter_rs::ChatCompletion, Error> {
    let mut backoff = Duration::from_millis(500);

    for attempt in 1..=max_attempts {
        match client.chat().send(req.clone()).await {
            Ok(c) => return Ok(c),
            Err(e) if e.is_retryable() && attempt < max_attempts => {
                let delay = e.retry_after().unwrap_or(backoff);
                tokio::time::sleep(delay).await;
                backoff *= 2;
            }
            Err(e) => return Err(e),
        }
    }
    unreachable!("loop returns on the final attempt")
}
```

`ChatRequest` is `Clone`, so re-sending the same request is straightforward.

## Do not retry a stream mid-flight

`Error::Stream` is not retryable and `ChatStream` is terminal — once it yields an
error, the next poll returns `None`. Recovering means issuing a fresh request
from the start, which re-bills the whole prompt. Decide at the application level
whether that is worth it.

Note that a streaming call has two distinct failure points. `stream()` itself
returns `Err` for anything that goes wrong before the first byte of the body,
including every non-2xx status — those errors *are* classifiable and possibly
retryable. Errors yielded *by* the stream are the terminal kind.

## What `Error` does not absorb

The newtype rejection errors — `InvalidApiKey`, `InvalidModelId`,
`InvalidFunctionName`, `InvalidPrice`, and the rest — are **not** variants of
`Error`. They implement `std::error::Error`, but `?` will not convert them into
`Error`, so this does not compile:

```rust
fn build() -> Result<ChatRequest, openrouter_rs::error::Error> {
    let model = ModelId::custom("openai/gpt-4o")?;   // ❌ InvalidModelId is not an Error variant
    // ...
}
```

Handle them where you construct the value: map them into your own error type, or
return `Box<dyn std::error::Error>` as the examples do. This is a deliberate
split — those failures are programmer input errors caught before any request
exists, not API failures.

## Transport errors in detail

`Error::Transport` wraps `airs_transport::TransportError`, re-exported as
`openrouter_rs::error::TransportError`:

| Variant | Cause | Retryable |
|---|---|---|
| `Network(String)` | connection refused, reset, DNS | ✅ |
| `Timeout { elapsed }` | request exceeded the configured timeout | ✅ |
| `Tls(String)` | handshake or certificate failure | ❌ |
| `BodyStream(String)` | body failed after headers, or exceeded the 16 MiB cap | ❌ |
| `Build(String)` | outgoing request could not be constructed | ❌ |
| `Other(String)` | uncategorised — treated as unsafe to retry | ❌ |

`reqwest` never appears in the public surface. Its failures are classified into
these variants at the transport boundary, so a `reqwest` version bump is not a
breaking change for you.
