# Errors and retries

The SDK classifies every failure and retries none of them. That is a deliberate
split of responsibility, and this page explains where the line falls and why.

## Three failure domains, not one

```
BuildError        before a request exists      (client construction)
Invalid*          before a request exists      (newtype rejection)
TransportError    the wire                      (network, TLS, timeout, framing)
Error             everything a caller sees      (wraps Transport and Build; adds API semantics)
```

The interesting boundary is between the newtype `Invalid*` errors and `Error`.
`InvalidModelId` is not an `Error` variant and does not convert with `?`. That
looks like friction until you consider what the alternative costs: every `match`
on an API response would also have to consider "the model id had a space in it",
a condition that cannot occur at that point in the program. Keeping them apart
means `Error` describes exactly one thing — what happened when you talked to the
API.

## Why match on variants, not strings

`Error` is a data enum, not a message carrier:

```rust
Error::RateLimit { retry_after: Option<Duration> }
Error::Moderation { reasons, flagged_input, provider_name, model_slug }
Error::Provider { provider_name, raw }
Error::Api { status, code, message, metadata }
```

Everything a program might branch on is a field. Nothing requires parsing the
`Display` output, which is free to change without breaking you. `Error` is
`#[non_exhaustive]`, so new variants are additive.

## Two axes of classification

Non-2xx responses are routed on **status** and on the **shape of the metadata
object**, not on status alone. A 403 with `{reasons, flagged_input,
provider_name, model_slug}` is a moderation block; a 403 without it is a generic
API error. Any status carrying `{provider_name, raw}` is an upstream provider
failure passed through.

That shape-based dispatch is why `Error::Moderation` can carry the reasons as a
`Vec<String>` rather than handing you an untyped blob, and why `Error::Provider`
keeps `raw` untyped — the provider's error format is the provider's business.

The full ladder is in
[reference/errors.md](../reference/errors.md#how-a-non-2xx-response-is-classified).

## No foreign types at the boundary

`reqwest` does not appear anywhere in the public surface. Its errors are
classified into `TransportError` variants — timeout, connect, TLS, request-build,
body, other — at the transport layer, in that order because `reqwest`'s
`is_request` flag also covers the more specific categories.

The consequence is concrete: bumping `reqwest` is not a semver break for this
crate's users. The same reasoning keeps `url::Url` private inside `BaseUrl`.

## Why there is no retry layer

`Error::is_retryable()` gives you the classification. `Error::retry_after()`
gives you the server's suggested delay. Neither of them retries anything, and
nothing else in the crate does either.

The reason is that a retry policy is an *application* decision, and every part of
it depends on context the SDK does not have:

- **How many attempts?** A background batch job and an interactive request want
  different answers.
- **What backoff curve?** Fixed, exponential, jittered — and jitter matters most
  under exactly the conditions that trigger retries.
- **What budget?** Retrying an LLM call costs money. The SDK does not know your
  ceiling.
- **Idempotency at what level?** Re-sending a chat completion is safe on the
  wire, but if your application already recorded a partial result, the second
  attempt may not be safe in your domain.
- **Where does the deadline live?** A retry loop inside the SDK cannot see the
  caller's overall deadline; a loop in the caller can.

An SDK that guesses at these produces a policy that is wrong for most callers and
invisible until it hurts. Exposing the classification and staying out of the way
produces a policy that is right by construction, at the cost of ten lines in the
caller — see
[how-to/handle-errors.md](../how-to/handle-errors.md#write-a-retry-loop).

`ChatRequest` derives `Clone` precisely so those ten lines are easy to write.

## What "retryable" means here

`is_retryable()` answers one narrow question: *is it safe to re-issue this exact
request?* It is not a prediction that a retry will succeed.

| Retryable | Reasoning |
|---|---|
| `Transport(Network)`, `Transport(Timeout)` | transient connectivity |
| `RateLimit` | explicitly a "try again later" |
| `Api` 408, 500, 502, 503 | server-side transients |

Everything else is a request-shape problem, a policy decision, or a client bug —
categories where the same request will fail the same way forever. Notably
`TransportError::Other` is treated as **not** retryable: without a known
category the SDK cannot prove a retry is safe, so it declines to.

## Streams are the exception

`Error::Stream` is not retryable and `ChatStream` is terminal — once it yields an
error, the next poll returns `None`. There is no resume protocol; recovering
means a fresh request from the start, which re-bills the whole prompt.

A streaming call therefore has two distinct failure points. `stream()` itself
fails for anything before the first body byte, including every non-2xx status;
those errors are ordinary, classifiable, sometimes retryable. Errors yielded *by*
the stream are the terminal kind, and whether to start over is a question only
the application can answer.

## Forward compatibility, on purpose

Several decisions in the error surface exist to keep future changes non-breaking:

- `Error`, `BuildError`, `TransportError`, and most `Invalid*` enums are
  `#[non_exhaustive]`.
- `FinishReason::Unknown` is `#[serde(other)]`, so a new server value decodes
  instead of failing.
- Response DTOs ignore unknown fields.
- `ClientBuilder::build()` returns `Result` despite being currently infallible.
- `BuildError::BaseUrl` and `BuildError::InvalidConfig` are declared but never
  constructed — reserved slots.

The pattern throughout: the client bends rather than breaks when the server
changes underneath it.
