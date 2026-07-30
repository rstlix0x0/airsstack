# 13 — Sessions

Resume a session, fork one, and read the stored transcripts off disk.

## Run

```text
cargo run -p clauders --example agent_13_sessions
```

The example runs three short turns (new, resumed, forked) and then lists, reads,
renames, and tags what ended up in the store.

## Two different things called "session"

- **`SessionControl` on `Options`** decides what a *new* run does with prior history.
  It is startup configuration.
- **`SessionArchive`** is plain local file I/O over the on-disk transcript store. It
  never spawns the binary; it lists, reads, renames, and tags what is already there.

## Continuation intent

```rust
SessionControl::New                                    // the default: no prior history
SessionControl::Continue { fork: false }               // pick up the latest session for this cwd
SessionControl::Resume { id, fork: false, resume_at: None }   // resume a named session in place
SessionControl::Resume { id, fork: true,  resume_at: None }   // branch it into a new session
```

`fork: true` gives the new run the same history under a **new** session id, leaving the
original untouched — so two futures can diverge from one past. The example prints both
ids to make that concrete.

`resume_at: Some(MessageId)` resumes only the messages up to a given message uuid. It
lives on the `Resume` variant specifically because it requires resuming, which makes
"`resume_at` without a resume" unrepresentable rather than a runtime error.

A session is only resumable if it was persisted. That is the default;
`session_persistence(SessionPersistence::Disabled)` (example 09) opts out.

The session id comes off the result frame:

```rust
Message::Result(result) => id = Some(result.session_id),
```

## The archive

```rust
let archive = SessionArchive::new()?;
```

Roots at `CLAUDE_CONFIG_DIR`, else `$HOME/.claude`, and errors with
`SessionError::NoConfigRoot` when neither is set. `SessionArchive::with_base(path)`
takes an explicit root for tests and non-default installs. The handle is stateless and
cheap to clone — it holds only the path.

### List

```rust
let listing = archive.list(ListOptions {
    dir: Some(cwd.clone()),
    limit: Some(5),
    ..ListOptions::default()
}).await?;
```

Newest first. `dir: None` scans every project directory. `include_worktrees` and
`include_programmatic` both default to `true`. Per-file read failures are skipped
rather than failing the whole listing, matching the binary.

### Info

```rust
match archive.info(id, Some(&cwd.to_string_lossy())).await? {
    Some(info) => println!("{} — {}", info.session_id.as_str(), info.summary),
    None => println!("no stored session for that id"),
}
```

`None` means "no such session", not an error — a non-UUID id also yields `None`.
`summary` is the custom title if one was set, else the auto-summary, else the first
prompt.

### Messages

```rust
let messages = archive.messages(id, MessagesOptions {
    dir: Some(cwd.clone()),
    include_system_messages: false,
    limit: Some(6),
    offset: 0,
}).await?;

for entry in &messages {
    println!("{} parent={:?} at {}", entry.uuid, entry.parent_uuid, entry.timestamp);
}
```

**This deliberately differs from the official SDKs.** They hand back a reconstructed
single active thread; `clauders` returns the **full flat transcript**, deduped by uuid
and in file order, with each entry keeping its `parent_uuid`. The branch structure is
handed to you unresolved, so nothing is lost and reconstruction is your choice. Every
other operation here is at parity.

`SessionMessage::payload` reuses the streaming `Message` types verbatim, so the same
match arms work on a stored transcript as on a live stream.

### Rename and tag

```rust
archive.rename(id, "clauders example 13", Some(&cwd.to_string_lossy())).await?;
archive.tag(id, Some("example"), Some(&cwd.to_string_lossy())).await?;
archive.tag(id, None, Some(&cwd.to_string_lossy())).await?;   // clears it
```

Both append a record to the transcript file, so a later `list` reflects the change.
Both reject a non-UUID id (`SessionError::InvalidSessionId`), reject a blank value
(`SessionError::EmptyValue`), and fail when no such session file exists
(`SessionError::SessionNotFound`). Passing `None` to `tag` clears the tag.
