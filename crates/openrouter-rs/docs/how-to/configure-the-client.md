# How to configure the client

Everything configurable lives on `ClientBuilder`. The optional setters compose
in any state, so they may come before or after `.api_key(...)`.

## Set attribution headers

OpenRouter uses `HTTP-Referer` and `X-Title` to attribute traffic to your app in
its public rankings. Both are optional and neither is sent when unset.

```rust
let client = Client::builder()?
    .api_key(api_key)
    .http_referer("https://myapp.example")
    .app_title("My App")
    .build()?;
```

They are attached to every request the client makes, on both endpoints.

## Point at a different base URL

Use this for a local proxy, a recording server, or a staging gateway.

```rust
use openrouter_rs::types::BaseUrl;

let base = BaseUrl::parse("http://127.0.0.1:8080/api/v1/")?;
let client = Client::builder()?.api_key(api_key).base_url(base).build()?;
```

`BaseUrl::parse` accepts only `http` and `https`; `file`, `data`, `ftp` and the
rest are rejected at construction.

**End the path with a slash.** Path joining follows RFC 3986, which replaces the
final segment of a base whose path does not end in `/`:

```text
"https://host/api/v1/" + "chat/completions" → "https://host/api/v1/chat/completions"   ✅
"https://host/api/v1"  + "chat/completions" → "https://host/api/chat/completions"      ❌ drops v1
"https://host"         + "chat/completions" → "https://host/chat/completions"          ✅
```

The default base URL is `https://openrouter.ai/api/v1/` — note the trailing
slash — and endpoint paths carry no leading slash.

`BaseUrl` is not in the prelude; import it from `openrouter_rs::types`.

## Enforce a request timeout

⚠️ **`ClientBuilder::timeout` does not currently affect requests.** The value is
stored on `Config` and readable through `client.config().timeout()` (default 60
seconds), but nothing in the request path passes it to the transport, and the
`reqwest::Client` built by `Client::builder()` is constructed without a timeout.

To get a wall-clock timeout today, configure it on a `reqwest::Client` and
supply that transport yourself:

```rust
use std::time::Duration;
use openrouter_rs::Client;
use openrouter_rs::transport::ReqwestTransport;

let http = reqwest::Client::builder()
    .timeout(Duration::from_secs(30))
    .user_agent("my-app/1.0")
    .build()?;

let client = Client::builder_with_transport(ReqwestTransport::from_client(http))
    .api_key(api_key)
    .build()?;
```

A timeout raised this way surfaces as
`Error::Transport(TransportError::Timeout { elapsed })`, which
`Error::is_retryable()` reports as retryable.

## Supply a custom transport

`Client::builder_with_transport` is the general entry point and is infallible —
the caller already holds a constructed transport, so there is nothing left to
fail. Use it for a pre-tuned `reqwest::Client` (proxies, custom TLS roots,
connection-pool settings, instrumentation), or for your own implementation of
`airs_transport::Transport` fixed to the HTTP associated types.

```rust
use openrouter_rs::transport::ReqwestTransport;

let transport = ReqwestTransport::try_new_with_user_agent("my-app/1.0")?;
let client = Client::builder_with_transport(transport).api_key(api_key).build()?;
```

The transport is a **generic parameter**, not a trait object, so the resulting
handle is `Client<MyTransport>` rather than `Client`. Functions that accept a
client should be generic:

```rust
use openrouter_rs::transport::HttpTransport;

async fn ask<T: HttpTransport>(client: &Client<T>, q: &str) -> Result<String, Error> {
    // ...
}
```

`DefaultClient` is the alias for `Client<ReqwestTransport>` if you only ever use
the default.

## Share one client

`Client` holds its state in an internal `Arc`, so `Clone` is a refcount bump and
every clone shares the same transport and connection pool. Clone freely across
tasks rather than building a second client.

```rust
let worker = client.clone();
tokio::spawn(async move { worker.chat().send(req).await });
```

`client.ref_count()` reports how many handles currently share the state. It is a
diagnostic, not a synchronisation primitive.

## What the client will not do for you

- **Retry.** There is no retry or backoff layer. See
  [how to handle errors](handle-errors.md).
- **Leak the key.** `Debug` on `Client`, `Auth`, and `ApiKey` all mask the
  credential; `ApiKey` prints `ApiKey("***")`.
- **Validate late.** Every setter takes an already-validated value, so `build()`
  cannot reject your configuration. It returns `Result` only to reserve the
  failure path for future cross-field checks.
