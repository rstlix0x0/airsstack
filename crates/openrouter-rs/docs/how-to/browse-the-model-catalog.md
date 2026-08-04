# How to browse the model catalog

`GET /models` returns every model OpenRouter can route to. One call, one method.

```rust
use openrouter_rs::prelude::*;

let models = client.models().list().await?;
println!("catalog has {} models", models.len());
```

`list()` returns `Vec<Model>` — the contents of the response's `data` array,
with the envelope stripped.

## Read an entry

```rust
for m in &models {
    println!(
        "{:<40} {:>8} ctx   prompt {}/tok",
        m.id.as_str(),
        m.context_length,
        m.pricing.prompt.as_str(),
    );
}
```

`Model` carries four fields: `id` (a validated `ModelId`), `name`,
`context_length` (`u64`), and `pricing`.

## Prices are decimal strings

`PricePerToken` preserves the wire representation exactly, because a price like
`"0.0000003"` loses precision the moment it becomes an `f64`.

```rust
let p = &models[0].pricing;
p.prompt.as_str();   // "0.0000003" — exactly what the server sent
p.prompt.to_f64();   // 3e-7 — convenient, lossy
```

`as_str()` for display, storage, and comparison; `to_f64()` when you are about to
do arithmetic and can accept the rounding. Note that `PartialEq` compares the
string, so `"0.5"` and `"0.50"` are **not** equal.

`prompt` and `completion` are always present. Six more are `Option`, populated
only by models that expose them:

| Field | Covers |
|---|---|
| `input_cache_read` | tokens read from the prompt cache |
| `input_cache_write` | tokens written to the prompt cache |
| `image` | image tokens |
| `web_search` | web-search operations |
| `internal_reasoning` | hidden reasoning tokens |
| `audio` | audio tokens |

## Find models that support prompt caching

```rust
let cacheable: Vec<_> = models
    .iter()
    .filter(|m| m.pricing.input_cache_read.is_some())
    .map(|m| m.id.as_str())
    .collect();
```

## Filter by context window

```rust
let long_context: Vec<_> = models
    .iter()
    .filter(|m| m.context_length >= 200_000)
    .collect();
```

## What is not in the entry

The catalog returns roughly eighteen fields per model. This crate decodes four.
`architecture`, `top_provider`, `description`, `supported_parameters` and the
rest are **silently dropped** at decode — they are not exposed and not stored.
If you need them, hit the endpoint directly rather than reaching for a hidden
accessor; there isn't one.

The upside of that same behaviour: a new field appearing server-side never
breaks decoding.

## Cost of the call

There is no client-side caching of the catalog. Every `list()` is a fresh HTTP
request, and the full response is a few hundred kilobytes. Fetch it once at
startup and keep the `Vec` if you need it repeatedly.
