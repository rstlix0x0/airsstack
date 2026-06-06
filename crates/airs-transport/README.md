# airs-transport

Generic async transport substrate shared by the airsstack SDK crates
(`clauders`, `openrouter-rs`).

Layered:

- `Transport` — the generic send-one-request contract (associated
  `Request`/`Response`/`Error`). Names no HTTP concept.
- `HttpTransport` — a `Transport` fixed to the HTTP types; a marker sub-trait
  with a blanket impl.
- `ReqwestTransport` — the concrete HTTP implementer (feature `transport-reqwest`).

It carries **zero domain knowledge**. Boundary test: *does the code name a
provider, an endpoint, an API-key format, a model catalog, a sampling range,
or a wire error envelope?* If yes, it belongs in a consumer SDK; if no, it is
eligible here.

## Public surface

- `Transport`, `HttpTransport` — the trait layers.
- `BodyStream` — incremental response-body stream type.
- `TransportError` — wire-level failure categories with `is_retryable`.
- `collect_body` / `MAX_RESPONSE_BODY_BYTES` — drain a body stream with a cap.
- `ReqwestTransport` (feature `transport-reqwest`) — default backend.
- `MockHttpTransport` (feature `__test-mocks`) — `mockall` fake.
