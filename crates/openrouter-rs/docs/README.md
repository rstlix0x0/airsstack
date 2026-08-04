# openrouter-rs documentation

Documentation for the `openrouter-rs` crate, organised by the
[Diátaxis](https://diataxis.fr/) framework. Each quadrant answers a different
question, so start from the one that matches what you are doing.

|  | Practical steps | Theoretical knowledge |
|---|---|---|
| **Study** | [Tutorials](#tutorials) — learn by building | [Explanation](#explanation) — understand why |
| **Work** | [How-to guides](#how-to-guides) — solve a task | [Reference](#reference) — look something up |

Everything here describes the crate **as implemented at version 0.1.0**. Where a
type exists but has no effect on the request path, the page that covers it says
so.

## Tutorials

Learning-oriented. Follow them in order; each one ends with a program that runs.

1. [Your first completion](tutorials/01-first-completion.md) — install, build a
   client, send a chat request, read the response.
2. [Streaming a response](tutorials/02-streaming-responses.md) — switch the same
   request to Server-Sent Events and print tokens as they arrive.

## How-to guides

Task-oriented. Each one assumes you already have a working client.

- [Configure the client](how-to/configure-the-client.md) — base URL, attribution
  headers, custom transport, timeouts.
- [Call tools](how-to/call-tools.md) — define a function, handle the model's
  call, feed the result back.
- [Request structured outputs](how-to/request-structured-outputs.md) — JSON
  object mode and JSON Schema mode.
- [Steer provider routing](how-to/steer-provider-routing.md) — pick, order, and
  exclude providers; cap price and latency.
- [Cache requests](how-to/cache-requests.md) — the provider prompt cache and the
  gateway edge cache.
- [Handle errors](how-to/handle-errors.md) — match failure domains, back off on
  rate limits, write your own retry loop.
- [Browse the model catalog](how-to/browse-the-model-catalog.md) — list models
  and read their pricing.
- [Test with a mock transport](how-to/test-with-a-mock-transport.md) — drive the
  SDK without a network.

## Reference

Information-oriented. Dry, complete, and organised around the code.

- [Client and configuration](reference/client-and-config.md)
- [Chat requests](reference/chat-requests.md)
- [Chat responses](reference/chat-responses.md)
- [Streaming](reference/streaming.md)
- [Caching](reference/caching.md)
- [Provider routing](reference/provider-routing.md)
- [Tools and structured outputs](reference/tools-and-structured-outputs.md)
- [Models catalog](reference/models-catalog.md)
- [Domain types](reference/domain-types.md)
- [Errors](reference/errors.md)

## Explanation

Understanding-oriented. Why the crate is shaped the way it is.

- [Architecture](explanation/architecture.md) — the four layers every call
  travels, and why the transport is a generic parameter.
- [Type-state builders](explanation/type-state-builders.md) — why a missing
  required field is a compile error rather than a runtime one.
- [Validated domain types](explanation/validated-domain-types.md) — parse, don't
  validate, at the newtype boundary.
- [The two caches](explanation/the-two-caches.md) — the single most confusable
  pair of concepts in the crate.
- [Errors and retries](explanation/errors-and-retries.md) — why the SDK
  classifies failures but never retries for you.

## What is not covered

The crate implements two endpoints: `POST /chat/completions` and
`GET /models`. Anything else in the OpenRouter API — generation metadata, key
management, credits, completions (non-chat) — has no Rust surface here, so
there is nothing to document. See
[reference/chat-requests.md](reference/chat-requests.md#not-modelled) for the
per-endpoint list of unmodelled fields.
