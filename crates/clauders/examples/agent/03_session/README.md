# 03 — Session

Connect once, take several turns, then read what the handshake told you and what
the session cost.

## Run

```text
cargo run -p clauders --example agent_03_session
```

## What it shows

```rust
let client = Client::builder().options(options).connect().await?;

for prompt in ["Pick a number…", "Double the number you just picked."] {
    let mut stream = client.query(prompt).await?;
    while let Some(frame) = stream.next().await { /* … */ }
}
```

`Client::connect(options)` and `Client::builder().options(options).connect()` are
equivalent; the builder form is convenient when options are assembled somewhere
other than the call site.

The second prompt works because the subprocess is still alive and still holds the
first turn. That is the whole difference from example 01: `query` tears the session
down when its stream drops, a `Client` does not.

## Reading the handshake for free

The `initialize` round-trip happens during `connect`, and its response is retained.
These accessors answer from that copy without touching the wire:

```rust
client.supported_models();     // Vec<serde_json::Value>
client.supported_commands();   // the slash commands this binary offers
client.supported_agents();     // agent definitions found on disk
client.account_info();
client.capabilities();         // the flags the binary advertised
client.initialize_result();    // the whole retained response
```

Element shapes are `serde_json::Value` on purpose: they are the binary's own
catalogue, which changes independently of this SDK, so they are passed through
rather than typed. Read what you need:

```rust
for command in client.supported_commands().iter().take(5) {
    if let Some(name) = command.get("name").and_then(serde_json::Value::as_str) {
        println!("/{name}");
    }
}
```

`Capabilities` is an **open set** — `supports("hooks")` is false both when the flag
is absent and when the binary is too old to send a manifest at all. Absence means
"not available here", never "error".

`client.reinitialize()` (example 12) re-runs the handshake over the live channel
when you need a fresh copy.

## Session accounting

Two control requests, both round-trips:

```rust
let usage = client.get_usage().await?;
println!("{:?}", usage.subscription_type);   // Some("pro") / None for API keys
println!("{}", usage.rate_limits_available);
println!("{}", usage.session);               // cost + per-model token totals

let context = client.get_context_usage().await?;
println!("{} / {} ({}%)", context.total_tokens, context.max_tokens, context.percentage);
```

`UsageReport::session`, `rate_limits`, and `behaviors` are opaque values: the binary
itself marks that payload experimental, so the SDK carries it rather than modelling
a shape that is documented to change. `ContextUsage` types the stable counters
(`total_tokens`, `max_tokens`, `percentage`, `model`, `is_auto_compact_enabled`) and
leaves the per-category breakdown — which exists to render the CLI's own `/context`
view — opaque.

## Shutting down

```rust
drop(client);
```

Dropping the client closes the pipe and gives the subprocess `shutdown_grace` to
exit before it is killed.
