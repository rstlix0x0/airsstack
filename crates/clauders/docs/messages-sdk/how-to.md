# How-to guides: Messages API

Recipes indexed by what you are trying to do. Each one names a runnable example — every example is a
complete program with its own `README.md` walking through the calls it makes.

If you have not used the client before, start with the [tutorial](tutorial.md) instead.

## Prerequisites for every recipe

An API key in `ANTHROPIC_API_KEY`, and model access on that key for the model the example names. Swap
the `ModelId::claude_*()` call if a model is not available to you.

Examples are registered by name in `Cargo.toml` and run from anywhere in the workspace:

```bash
ANTHROPIC_API_KEY=sk-ant-... cargo run -p clauders --example 01_hello
```

---

## Getting a reply

### Send one request and read the answer

```bash
cargo run -p clauders --example 01_hello
```

`Client::builder()`, `MessageRequest::builder()`, `client.messages().create(req)`. Reads text out of
`Message.content` and prints `stop_reason` and `usage`.
→ [`examples/messages/01_hello/`](../../examples/messages/01_hello/README.md)

### Show the answer as it is generated

```bash
cargo run -p clauders --example 02_streaming
```

`client.messages().stream(req)` yields `StreamEvent`s. Match
`StreamEvent::ContentBlockDelta { delta: ContentDelta::TextDelta { text }, .. }` and print each
fragment.
→ [`examples/messages/02_streaming/`](../../examples/messages/02_streaming/README.md)

### Let Claude call your code

```bash
cargo run -p clauders --example 03_tools
```

Declare a `Tool` with a JSON Schema, set `ToolChoice::Auto`, find the `ContentBlock::ToolUse` in the
reply, answer with a `ToolResultBlock`, send the whole history back.
→ [`examples/messages/03_tools/`](../../examples/messages/03_tools/README.md)

### Run tools in a loop until the model is done

```bash
cargo run -p clauders --example 07_agentic_tool_loop
```

The multi-turn generalisation of the previous recipe: keep answering tool calls until `stop_reason`
is no longer `ToolUse`.
→ [`examples/messages/07_agentic_tool_loop/`](../../examples/messages/07_agentic_tool_loop/README.md)

---

## Controlling cost and latency

### Reuse a long prompt prefix across calls

```bash
cargo run -p clauders --example 04_caching
```

Mark a breakpoint with `CacheControl` and watch `usage.cache_creation_input_tokens` on the first call
become `usage.cache_read_input_tokens` on the second.
→ [`examples/messages/04_caching/`](../../examples/messages/04_caching/README.md)

### Give the model room to reason before answering

```bash
cargo run -p clauders --example 06_thinking
```

`ThinkingConfig` on the request. Thinking output arrives as `ContentBlock::Thinking` blocks alongside
the text.
→ [`examples/messages/06_thinking/`](../../examples/messages/06_thinking/README.md)

### Find out what a request will cost before sending it

`client.messages().count_tokens(req)` sends `POST /v1/messages/count_tokens` with the fields that
affect the count — including `thinking`, which changes it. No example directory; it is a single call.

### Submit a lot of work cheaply

```bash
cargo run -p clauders --example 10_batches
```

`client.messages().batches()` → `create`, `get`, `list`, `results`, `cancel`, `delete`. The example
submits a batch, polls it, and streams the result rows.
→ [`examples/messages/10_batches/`](../../examples/messages/10_batches/README.md)

---

## Sending richer input

### Send an image

```bash
cargo run -p clauders --example 08_vision
```

`ContentBlockParam::Image` with a base64 source. Media type is closed to the four the API accepts.
→ [`examples/messages/08_vision/`](../../examples/messages/08_vision/README.md)

### Send a PDF and get citations back

```bash
cargo run -p clauders --example 09_document_citations
```

`ContentBlockParam::Document` with citations enabled; the reply's `TextBlock.citations` point back
into the source.
→ [`examples/messages/09_document_citations/`](../../examples/messages/09_document_citations/README.md)

---

## Getting structured results

### Force the reply into a JSON Schema

```bash
cargo run -p clauders --example 05_structured_output
```

`OutputConfig` on the request constrains generation to your schema.
→ [`examples/messages/05_structured_output/`](../../examples/messages/05_structured_output/README.md)

Note there is no typed `parse()` helper — the official Python and TypeScript SDKs have one, this
client does not. You get schema-conforming JSON in the first text block and deserialize it yourself.

---

## Discovering what a model supports

`client.models().list()` and `client.models().get(&model_id)` return `ModelInfo`, which carries
`max_input_tokens`, `max_tokens`, and a `ModelCapabilities` tree covering batch, citations, code
execution, image input, PDF input, structured outputs, effort levels, thinking types, and context
management.

Use it instead of hardcoding a table — it is the API's own answer to "does this model take `xhigh`
effort or adaptive thinking".

---

## Building something larger

### A complete coding agent with a terminal UI

```bash
cargo run -p clauders --example 11_coding_agent
```

A `ratatui` TUI driving real file and `cargo` tools through the tool loop. The program to read if you
are wiring this client into an application rather than a script.
→ [`examples/messages/11_coding_agent/`](../../examples/messages/11_coding_agent/README.md)

---

## Swapping the HTTP transport

`Client::builder()` builds a default `reqwest` transport. When you already hold a configured one — a
tuned `ReqwestTransport`, a custom implementation, or a test double — use
`Client::builder_with_transport(t)` instead. It is infallible, since nothing needs constructing.

`Client<T>` is generic over the transport, so this costs no dynamic dispatch.
