# clauders documentation

`clauders` is an unofficial Rust client for Anthropic's official SDK surfaces. It ships two
independent clients in one crate — a Messages API client and an Agent SDK that drives the `claude`
Code CLI as a subprocess — and they share no code path beyond a handful of value types.

These docs follow [Diátaxis](https://diataxis.fr/): four modes, kept separate on purpose. A tutorial
that stops to explain becomes a bad tutorial *and* a bad explanation, so each document commits to one
job.

## Start here

Pick the client you need. They are unrelated: one talks HTTP to `api.anthropic.com` and wants an API
key, the other spawns a binary and wants no key at all.

| | Messages API | Agent SDK |
|---|---|---|
| What it is | typed client over `POST /v1/messages` and friends | drives the `claude` Code CLI over its control protocol |
| Needs | `ANTHROPIC_API_KEY` | a `claude` binary 2.0.0+ on `PATH` |
| Module | `clauders::messages`, `clauders::models` | `clauders::agent` |
| Learn it | [tutorial](messages-sdk/tutorial.md) | [tutorial](agent-sdk/tutorial.md) |
| Do a specific thing | [how-to](messages-sdk/how-to.md) | [how-to](agent-sdk/how-to.md) |
| Understand the design | [explanation](messages-sdk/explanation.md) | [explanation](agent-sdk/explanation.md) |
| Check parity with the official SDK | [feature parity](messages-sdk/feature-parity.md) | [feature parity](agent-sdk/feature-parity.md) |

## Cross-cutting

Read these when the question spans both clients, or before your first edit to the crate.

- **[Architecture](architecture.md)** — the three parity pillars, the Agent SDK's four layers, how
  the Messages client is generic over its transport, and the crate-wide conventions worth knowing
  before you change anything.
- **[Divergences](divergences.md)** — every place clauders deliberately behaves differently from the
  official SDKs, with the reasoning. Read it before "fixing" something that looks wrong.

## The four modes, and which file is which

| Mode | Question it answers | Where it lives |
|---|---|---|
| Tutorial | "I am new — get me something working." | `*/tutorial.md` |
| How-to | "I know the basics. How do I do *this*?" | `*/how-to.md`, plus the 25 runnable examples under `examples/` |
| Reference | "What exactly does this do, and does it match the official SDK?" | `*/feature-parity.md`, plus the rustdoc |
| Explanation | "Why is it built this way?" | `*/explanation.md`, `architecture.md`, `divergences.md` |

The API reference is the rustdoc, not a file here. It is generated from the source, gated by
`RUSTDOCFLAGS="-D warnings"`, and cannot drift from the code:

```bash
cargo doc -p clauders --no-deps --open
```

The parity documents are the reference material rustdoc *cannot* produce — what the official Python
and TypeScript SDKs do, what clauders does, and exactly where the two differ.

## Examples

25 runnable programs, each in its own directory with a `main.rs` and a `README.md`:

- [`examples/messages/`](../examples/messages/README.md) — 11 programs, simplest first.
- [`examples/agent/`](../examples/agent/README.md) — 14 programs, simplest first.

Both `how-to.md` files index these by the goal you arrive with rather than by number.
