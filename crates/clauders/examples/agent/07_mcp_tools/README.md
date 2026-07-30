# 07 — MCP tools

Give the agent Rust functions to call. No server process, no transport, no schema
file — the tool runs in this program.

## Run

```text
cargo run -p clauders --example agent_07_mcp_tools
```

## What it shows

```rust
let word_count = tool(
    "word_count",
    "Count the words in a piece of text.",
    serde_json::json!({
        "type": "object",
        "properties": { "text": { "type": "string" } },
        "required": ["text"]
    }),
    |args| async move {
        let text = args.get("text").and_then(serde_json::Value::as_str).unwrap_or_default();
        Ok(ToolResult::text(text.split_whitespace().count().to_string()))
    },
);

let server = SdkMcpServer::builder("calc").version("1.0.0").tool(word_count).build();

let options = Options::builder()
    .sdk_mcp_server(server)
    .allowed_tools(vec!["mcp__calc__word_count".to_owned()])
    .build();
```

`tool(name, description, schema, handler)` is the common path. The description is
what the model reads when deciding whether to call it, so it is worth writing
carefully. For anything richer — state, shared resources, dynamic schemas —
implement the `Tool` trait directly instead.

## Naming

The model sees in-process tools under the MCP convention
**`mcp__<server>__<tool>`**. That is also how they are spelled in `allowed_tools`
and `disallowed_tools`. The server name comes from `SdkMcpServer::builder(name)`.

## Results

```rust
ToolResult::text("42")                       // success, one text block
ToolResult::error("cannot divide by zero")   // tool-level failure (isError: true)
ToolResult::image(base64, "image/png")
ToolResult::audio(base64, "audio/wav")
```

`ToolResult::error` reports a failure to *the model*, not to the session: the model
reads the message and can retry or change approach. Returning `Err(AgentError)` from
the handler does the same thing — it becomes a model-visible error result rather than
killing the run.

For anything more elaborate, build `ToolResult { content, is_error }` directly.
`ToolContent` covers `Text`, `Image`, `Audio`, `ResourceLink`, and an embedded
`Resource` carrying `ResourceContents::Text` or `ResourceContents::Blob`.

## Annotations

```rust
.with_annotations(ToolAnnotations {
    read_only_hint: Some(true),
    idempotent_hint: Some(true),
    ..ToolAnnotations::default()
})
```

Four optional MCP hints — `read_only_hint`, `destructive_hint`, `idempotent_hint`,
`open_world_hint`. They are declarative wire flags describing what kind of operation
the tool is; unset fields are omitted entirely.

## In-process versus external servers

Two different fields:

- `sdk_mcp_server(SdkMcpServer)` — tools that run here, in this process.
- `mcp_servers(Vec<McpServerConfig>)` — external servers, forwarded to the binary as
  opaque JSON config. `McpServerConfig::new(name, config)` carries the config
  untouched, so a newer binary's config shape needs no SDK change.

`strict_mcp_config(true)` (example 09) restricts the session to the servers you
declared, ignoring project, user, and plugin MCP config.

## Elicitation

Elicitation is how an MCP server asks the **user** for structured input in the middle
of a tool call. Register a policy to answer:

```rust
#[async_trait::async_trait]
impl ElicitationPolicy for AutoAnswer {
    async fn elicit(&self, request: ElicitationRequest, _cancel: CancelSignal)
        -> Result<ElicitationResponse, AgentError>
    {
        match request.mode {
            Some(ElicitationMode::Form) => Ok(ElicitationResponse::Accept(
                serde_json::json!({ "confirmed": true })
            )),
            Some(ElicitationMode::Url) => Ok(ElicitationResponse::Decline),
            _ => Ok(ElicitationResponse::Cancel),
        }
    }
}
```

Three outcomes: `Accept(value)` supplies values conforming to
`request.requested_schema`, `Decline` actively refuses, `Cancel` means dismissed or
timed out. With **no** policy registered the runtime declines, matching the official
SDKs.

Two modes:

- `Form` — `requested_schema` holds the JSON Schema the answer must satisfy.
- `Url` — the user must visit `request.url`, typically an OAuth flow.

`mode` is `Option`: `None` means the binary did not say, and
`Some(ElicitationMode::Unknown)` means it sent a mode this release does not model.
Decline either rather than guessing.

The in-process tools in this example never elicit, so the policy here shows the shape
rather than firing. An external MCP server that does elicit routes to it.
