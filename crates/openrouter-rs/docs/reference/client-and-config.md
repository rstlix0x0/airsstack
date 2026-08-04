# Reference — Client and configuration

## `Client<T>`

```rust
pub struct Client<T = DefaultTransportPlaceholder> where T: HttpTransport
```

The single handle every SDK call goes through. State lives in an internal
`Arc<ClientInner<T>>`, so `Clone` is a refcount bump. `Debug` prints only
`config` and is `finish_non_exhaustive` — credentials never appear.

| Alias | Expands to |
|---|---|
| `DefaultClient` | `Client<ReqwestTransport>` |
| `DefaultTransportPlaceholder` | `ReqwestTransport` |

### Methods

| Method | Signature | Notes |
|---|---|---|
| `builder` | `fn builder() -> Result<ClientBuilder<Missing, ReqwestTransport>, BuildError>` | Only on `Client<ReqwestTransport>`. Constructs the default transport with User-Agent `openrouter-rs/<crate version>`. |
| `builder_with_transport` | `const fn builder_with_transport(t: T) -> ClientBuilder<Missing, T>` | Infallible. |
| `chat` | `const fn chat(&self) -> ChatResource<'_, T>` | Short-lived borrow; create at the call site. |
| `models` | `const fn models(&self) -> ModelsResource<'_, T>` | Same. |
| `config` | `fn config(&self) -> &Config` | |
| `auth` | `fn auth(&self) -> &Auth` | Exposed mainly for tests and debugging. |
| `ref_count` | `fn ref_count(&self) -> usize` | Live `Arc` strong count. Diagnostic only. |

`builder()` fails only when the underlying `reqwest::Client` cannot be built —
in practice a TLS-backend initialisation failure — surfacing as
`BuildError::Transport`.

## `ClientBuilder<Key, T>`

```rust
pub struct ClientBuilder<Key, T> where Key: BuilderApiKeyState, T: HttpTransport
```

`Key` is the type-state parameter: `Missing` or `Present`. `BuilderApiKeyState`
is sealed, so downstream crates cannot add states.

### Setters available in any state

| Method | Type | Effect |
|---|---|---|
| `http_referer` | `impl Into<String>` | `HTTP-Referer` header value |
| `app_title` | `impl Into<String>` | `X-Title` header value |
| `timeout` | `Duration` | stored on `Config`; **see the caveat below** |
| `base_url` | `BaseUrl` | overrides the default endpoint |

### The transition

| Method | Available on | Returns |
|---|---|---|
| `api_key(ApiKey)` | `ClientBuilder<Missing, T>` | `ClientBuilder<Present, T>` |
| `build()` | `ClientBuilder<Present, T>` | `Result<Client<T>, BuildError>` |

`build()` cannot fail in the current implementation — every value it holds was
validated at construction. The `Result` reserves the failure path so a future
cross-field validation is not a breaking change.

Optional fields set *before* `api_key` survive the transition. All mutable
builder data lives in one private non-generic struct that the transition moves
whole, so adding a field never touches the transition code.

## `Config`

Fields are crate-private; read them through the accessors.

| Accessor | Type | Default |
|---|---|---|
| `base_url()` | `&BaseUrl` | `https://openrouter.ai/api/v1/` |
| `http_referer()` | `Option<&str>` | `None` |
| `app_title()` | `Option<&str>` | `None` |
| `timeout()` | `Duration` | 60 seconds |

The default base URL ends in `/` because path joining is additive only for a
base whose path ends in a slash. See
[domain-types.md](domain-types.md#baseurl).

### ⚠️ `timeout` is stored, not applied

Nothing in the request path reads `Config::timeout`. The `reqwest::Client` built
by `Client::builder()` is constructed with a User-Agent and nothing else, and
neither `ChatResource` nor `ModelsResource` passes a deadline to the transport.
`ClientBuilder::timeout` therefore changes the value returned by
`client.config().timeout()` and nothing else.

To enforce a wall-clock deadline, build a `reqwest::Client` with `.timeout(..)`
and pass it via `ReqwestTransport::from_client` +
`Client::builder_with_transport`. See
[how-to/configure-the-client.md](../how-to/configure-the-client.md#enforce-a-request-timeout).

## `Auth`

```rust
pub enum Auth { Bearer(ApiKey) }
```

A closed set with one variant today. `api_key(&self) -> Option<&ApiKey>` returns
the key when the scheme is `Bearer`. Match arms should carry a `_` arm so a
future scheme is not a breaking change. `Debug` does not leak the secret,
because `ApiKey` masks its own.

## Headers sent on every request

| Header | When |
|---|---|
| `authorization: Bearer <key>` | whenever `auth().api_key()` is `Some` |
| `accept: application/json` | non-streaming chat, and `GET /models` |
| `accept: text/event-stream` | streaming chat |
| `content-type: application/json` | chat only (`GET /models` sends an empty body and no content type) |
| `http-referer: <value>` | when configured |
| `x-title: <value>` | when configured |
| `user-agent` | set by the transport, not by the SDK |
| `x-openrouter-cache*` | only on `send_cached` / `stream_cached` — see [caching.md](caching.md) |

## Transport surface

`openrouter_rs::transport` is a `#[doc(inline)]` re-export of the
`airs_transport` crate:

| Item | Kind |
|---|---|
| `Transport` | trait — send one request, get one response |
| `HttpTransport` | marker sub-trait; blanket impl for any `Transport` with the HTTP associated types |
| `ReqwestTransport` | the default implementation |
| `BodyStream` | `Pin<Box<dyn Stream<Item = Result<Bytes, TransportError>> + Send>>` |
| `TransportError` | transport failure enum |
| `collect_body` | drain a `BodyStream` with a byte cap |
| `MAX_RESPONSE_BODY_BYTES` | `16 * 1024 * 1024` |

`ReqwestTransport` constructors: `try_new()` (UA `airs-transport/<version>`),
`try_new_with_user_agent(&str)`, and `from_client(reqwest::Client)` (const,
infallible).

The 16 MiB cap applies to every body drained in full: non-streaming chat
responses, `GET /models`, and non-2xx error bodies on the streaming paths.
A streaming 2xx body is not drained and is not capped.
