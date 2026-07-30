# 14 — Agent console

An interactive terminal UI over a live `claude` session. The capstone: everything the
earlier examples showed in isolation, wired into something usable.

## Run

```text
cargo run -p clauders --example agent_14_agent_console
```

Type a prompt and press Enter.

| Key | Does |
|---|---|
| `Enter` | send the typed line |
| `y` / `n` | answer a pending permission question |
| `Ctrl+X` | interrupt the running turn |
| `Ctrl+C` | quit |
| `↑` `↓` `PgUp` `PgDn` | scroll the transcript |
| `End` | resume following the newest line |

| Command | Does |
|---|---|
| `/model haiku` \| `sonnet` \| `opus` \| `<any id>` | switch model mid-session |
| `/quit` | close the session |

## What it shows

- a long-lived `Client` in a background task, streaming frames to the UI;
- a `PermissionPolicy` that **decides nothing itself** — it hands the request to the UI
  and waits for a keystroke;
- a `PreToolUse` hook logging every tool the agent reaches for;
- `interrupt()` bound to a key, so a runaway turn can be cut short;
- `set_model()` bound to a slash command;
- live cost and turn counters read off each result frame.

## Structure

Two tasks and two channels:

```text
        Command::{Prompt, Interrupt, SetModel, Quit}
   UI  ──────────────────────────────────────────────▶  agent task
       ◀──────────────────────────────────────────────
        UiEvent::{Line, Status, Permission, TurnDone}
```

The UI owns the terminal on the main thread and blocks on `event::poll`; the
multi-threaded runtime keeps servicing the agent task while it does. The agent task owns
the `Client`, because a `Client` is not `Clone` and this way nothing needs to be shared.

## Turning a permission request into a prompt

This is the part worth copying. The policy does not embed any rule — it forwards the
question and blocks on the answer:

```rust
async fn can_use_tool(&self, tool: &str, input: &serde_json::Value,
                      _ctx: PermissionContext, cancel: CancelSignal)
    -> Result<PermissionDecision, AgentError>
{
    let (reply_tx, reply_rx) = oneshot::channel();
    self.tx.send(UiEvent::Permission { tool: tool.to_owned(), detail: compact_json(input), reply: reply_tx })?;

    tokio::select! {
        answer = reply_rx => match answer {
            Ok(true)  => Ok(PermissionDecision::allow()),
            Ok(false) => Ok(PermissionDecision::deny("denied at the console")),
            Err(_)    => Ok(PermissionDecision::deny("console closed")),
        },
        () = cancel.cancelled() => Err(AgentError::Interrupted),
    }
}
```

Three things fall out of that shape:

- **The `oneshot` is the answer channel.** The UI stashes the sender next to the
  question and fires it on the keystroke.
- **A dropped sender means deny.** If the console closes with a question pending,
  `reply_rx` errors and the call is refused. Failing closed is the right default for a
  permission gate.
- **`cancel.cancelled()` is in the `select!`.** The binary can withdraw a request while
  the user is still thinking; without that arm the policy would block forever on a
  keystroke for a call nobody is waiting on any more. Cancellation is cooperative, so
  observing it is the handler's job.

## Staying responsive during a turn

The turn loop selects over the frame stream *and* the command channel:

```rust
tokio::select! {
    Some(command) = commands.recv() => match command {
        Command::Interrupt => { client.interrupt().await; }
        Command::Quit => break,
        Command::Prompt(_) | Command::SetModel(_) => {}   // dropped mid-turn
    },
    frame = stream.next() => match frame {
        None => break,
        Some(Ok(message)) => { if forward(message, tx) { return; } }
        Some(Err(error)) => { /* report, keep going */ }
    },
}
```

That is what lets `Ctrl+X` reach `interrupt()` while frames are still arriving. A prompt
typed mid-turn is dropped rather than queued — queueing it would be a small extension.

A per-frame `Err` is reported and the loop continues; only `None` (the producer closed)
or a result frame ends the turn.

## Rendering frames

`forward` maps each block kind to a coloured transcript line — text, thinking, tool
calls, tool results — and returns `true` on the result frame, which is what ends the
turn. `ContentBlock` is `#[non_exhaustive]`, so the wildcard arm is required and
unmodelled block kinds are simply not drawn.
