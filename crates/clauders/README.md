# clauders

Unofficial Rust client for Anthropic's official SDK surfaces. Not affiliated with Anthropic.

Two independent clients ship in this crate. They share almost no code, and which one you want depends
on what you are building:

| | Messages API | Agent SDK |
|---|---|---|
| What it is | typed client over `POST /v1/messages` and its companion endpoints | drives the `claude` Code CLI as a subprocess over its control protocol |
| Reach for it when | you want a model to answer, with tools and streaming under your control | you want an agent that reads files, runs commands, and manages its own loop |
| Needs | `ANTHROPIC_API_KEY` | a `claude` binary 2.0.0+ on `PATH` — and no API key |
| Module | `clauders::messages`, `clauders::models` | `clauders::agent` |

A third pillar, Managed Agents (`/v1/agents`, `/v1/sessions`), is **not started** — there is no code
for it.

## Quick start — Messages API

```rust,no_run
use clauders::messages::ContentBlock;
use clauders::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder()?
        .api_key(ApiKey::new(std::env::var("ANTHROPIC_API_KEY")?)?)
        .build()?;

    let req = MessageRequest::builder()
        .model(ModelId::claude_sonnet_5())
        .max_tokens(MaxTokens::new(1024))
        .add_user_text("Say hi.")
        .build();

    let msg = client.messages().create(req).await?;

    for block in &msg.content {
        if let ContentBlock::Text(t) = block {
            println!("{}", t.text);
        }
    }
    Ok(())
}
```

## Quick start — Agent SDK

```rust,no_run
use clauders::agent::{ContentBlock, Message, Options, query};
use futures_util::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = query("Say hi in one short sentence.", Options::default()).await?;

    while let Some(frame) = stream.next().await {
        match frame? {
            Message::Assistant(a) => {
                for block in &a.content {
                    if let ContentBlock::Text { text } = block {
                        println!("{text}");
                    }
                }
            }
            Message::Result(r) => println!("session {}", r.session_id.as_str()),
            _ => {}
        }
    }
    Ok(())
}
```

No API key in the second one. The Agent SDK never calls the HTTP API — it spawns the binary and reuses
whatever credentials that binary already holds.

## Documentation

The docs follow [Diátaxis](https://diataxis.fr/): four modes, kept separate. Start at
[`docs/README.md`](docs/README.md), or go straight to what you need:

| | Messages API | Agent SDK |
|---|---|---|
| Learn it | [tutorial](docs/messages-sdk/tutorial.md) | [tutorial](docs/agent-sdk/tutorial.md) |
| Do one thing | [how-to](docs/messages-sdk/how-to.md) | [how-to](docs/agent-sdk/how-to.md) |
| Understand it | [explanation](docs/messages-sdk/explanation.md) | [explanation](docs/agent-sdk/explanation.md) |
| Check parity | [feature parity](docs/messages-sdk/feature-parity.md) | [feature parity](docs/agent-sdk/feature-parity.md) |

Cross-cutting: [architecture](docs/architecture.md) for the pillar map and the Agent SDK's internal
layering; [divergences](docs/divergences.md) for every place this crate deliberately differs from the
official SDKs.

The API reference is the rustdoc — `cargo doc -p clauders --no-deps --open`.

## What is implemented

The crate declares no Cargo features; everything below is always compiled.

**Messages API** — `POST /v1/messages` with SSE streaming and a full accumulator; custom tools with
`strict` and all four `tool_choice` forms; prompt caching with both TTL tiers and per-tier usage
accounting; extended thinking and effort levels; JSON-Schema structured output; image and PDF input;
citations; token counting; the Message Batches API; `GET /v1/models` with the capability-discovery
payload.

**Agent SDK** — one-shot `query` and a stateful `Client`; 45 `Options` fields lowered to the binary's
flags and handshake; in-process MCP tools written in Rust; hooks; the full six-mode permission system
with a `PermissionPolicy` callback; MCP elicitation; programmatic subagents; session continue, resume,
fork and resume-at-a-message; on-disk session inspection; warm start; and 22 mid-session control and
introspection operations.

Neither is at complete parity. The two feature-parity documents say exactly where, graded against the
shipped official artifacts rather than their documentation.

## Examples

25 runnable programs, each in its own directory with a `README.md`:

```bash
ANTHROPIC_API_KEY=sk-ant-... cargo run -p clauders --example 01_hello
cargo run -p clauders --example agent_01_query
```

- [`examples/messages/`](examples/messages/README.md) — 11 programs, simplest first.
- [`examples/agent/`](examples/agent/README.md) — 14 programs, simplest first.

Both how-to guides index these by the goal you arrive with.

## License

Apache-2.0. See the workspace root `LICENSE` file.
