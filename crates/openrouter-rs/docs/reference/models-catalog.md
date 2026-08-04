# Reference — Models catalog

`GET /models`. Path constant `models`, joined onto the configured base URL.

## `ModelsResource<'a, T>`

Obtained from `Client::models()`. Borrows the client; do not construct directly.

```rust
pub async fn list(&self) -> Result<Vec<Model>, Error>
```

One method. It issues the GET, drains the body (subject to
`MAX_RESPONSE_BODY_BYTES`, 16 MiB), decodes the `{"data": [...]}` envelope, and
returns the `data` array. The envelope wrapper is private; you never see it.

### Request shape

| Header | Value |
|---|---|
| `accept` | `application/json` |
| `authorization` | `Bearer <key>`, when configured |
| `http-referer` | when configured |
| `x-title` | when configured |

Method `GET`, empty body, and **no `content-type` header** — the one place the
two endpoints differ in request construction.

### Errors

Identical routing to the chat endpoint, with one difference in the `Serde`
context string: a 2xx body that fails to decode reports
`context: "ModelsResponse"`. See [errors.md](errors.md).

## `Model`

```rust
pub struct Model {
    pub id: ModelId,
    pub name: String,
    pub context_length: u64,
    pub pricing: Pricing,
}
```

Derives `Debug`, `Clone`, `PartialEq`, `Eq`, `Deserialize`.

`id` is a validated `ModelId`, so a catalog entry whose id contains whitespace
would fail to decode. `name` is the human-readable display name
(`"Anthropic: Claude Sonnet 4.5"`).

## `Pricing`

```rust
pub struct Pricing {
    pub prompt: PricePerToken,
    pub completion: PricePerToken,
    pub input_cache_read: Option<PricePerToken>,
    pub input_cache_write: Option<PricePerToken>,
    pub image: Option<PricePerToken>,
    pub web_search: Option<PricePerToken>,
    pub internal_reasoning: Option<PricePerToken>,
    pub audio: Option<PricePerToken>,
}
```

`prompt` and `completion` are required. The six optionals cover token categories
only some models expose.

## `PricePerToken`

Prices arrive as decimal **strings** (`"0.0000003"`) and are kept that way —
`f64` cannot represent them exactly.

| Method | Returns |
|---|---|
| `new(impl Into<String>)` | `Result<Self, InvalidPricePerToken>` |
| `as_str()` | the original string, byte for byte |
| `to_f64()` | lossy conversion; cannot fail (the constructor validated it) |
| `Display` | same as `as_str()` |

Deserialises via `#[serde(try_from = "String")]`, so the validating constructor
always runs on decode.

Rejection reasons (`InvalidPricePerToken`, `#[non_exhaustive]`): `Empty`,
`NotANumber(String)`, `Negative`, `NonFinite`.

`PartialEq` and `Hash` compare the **string**, so `"0.5" != "0.50"` even though
the numbers are equal. Compare `to_f64()` if you want numeric equality.

> Not to be confused with `Price`, the `f64`-backed type used for provider
> routing caps. `PricePerToken` is catalog data coming in; `Price` is a limit
> going out.

## What is dropped on decode

The catalog returns roughly eighteen fields per entry. Four are modelled;
everything else — `description`, `architecture`, `top_provider`,
`supported_parameters`, `per_request_limits`, and the rest — is **silently
ignored**. There is no accessor and no raw-value escape hatch.

The upside: new server-side fields never break decoding.

## No client-side caching

Every `list()` call is a fresh HTTP request. Nothing memoises the catalog. Fetch
once and hold the `Vec` if you need it repeatedly.
