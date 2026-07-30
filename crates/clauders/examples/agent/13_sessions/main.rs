//! Sessions: resume one, fork one, and read the stored transcripts.
//!
//! Two separate things share the word "session":
//!
//! - `SessionControl` on `Options` decides what a *new* run does with prior
//!   history — start fresh, continue the latest, resume a named one, or fork
//!   either into a new branch.
//! - `SessionArchive` is plain local file I/O over the on-disk transcript store.
//!   It never spawns the binary; it lists, reads, renames, and tags what is
//!   already on disk.
//!
//! Run:
//!
//! ```text
//! cargo run -p clauders --example agent_13_sessions
//! ```

use clauders::agent::types::{SessionControl, SessionId};
use clauders::agent::{
    ContentBlock, ListOptions, Message, MessagesOptions, Options, PermissionMode, SessionArchive,
    query,
};
use futures_util::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cwd = std::env::current_dir()?;

    // 1. A fresh session. `SessionControl::New` is the default.
    println!("=== new session ===");
    let first = run("Remember this word: pangolin. Reply with just 'ok'.", None).await?;
    println!("session id: {}", first.as_str());

    // 2. Resume it in place. The agent sees the earlier turn.
    println!("=== resume ===");
    let resumed = run(
        "What word did I ask you to remember?",
        Some(SessionControl::Resume {
            id: first.clone(),
            fork: false,
            resume_at: None,
        }),
    )
    .await?;
    println!("still session: {}", resumed.as_str());

    // 3. Fork it. Same history, new session id — the original is untouched, so
    //    two futures can diverge from one past.
    println!("=== fork ===");
    let forked = run(
        "Same question again, and add an exclamation mark.",
        Some(SessionControl::Resume {
            id: first.clone(),
            fork: true,
            resume_at: None,
        }),
    )
    .await?;
    println!("forked into: {}", forked.as_str());
    println!("original was: {}", first.as_str());

    // `SessionControl::Continue { fork }` does the same without naming an id:
    // it picks up the most recent session for the working directory.

    // 4. Read the store. No subprocess involved from here on.
    println!("=== archive ===");
    let archive = SessionArchive::new()?;

    let listing = archive
        .list(ListOptions {
            dir: Some(cwd.clone()),
            limit: Some(5),
            ..ListOptions::default()
        })
        .await?;
    println!("{} recent session(s) for this directory:", listing.len());
    for info in &listing {
        println!("  {} — {}", info.session_id.as_str(), info.summary);
    }

    // Metadata for one session. `None` means "no such session", not an error.
    match archive
        .info(first.as_str(), Some(&cwd.to_string_lossy()))
        .await?
    {
        Some(info) => println!("info: {} — {}", info.session_id.as_str(), info.summary),
        None => println!("info: no stored session for that id yet"),
    }

    // The full flat transcript, each entry carrying its `parent_uuid`. This
    // deliberately differs from the official SDKs, which hand back a single
    // reconstructed thread; branch reconstruction is left to the caller.
    let messages = archive
        .messages(
            first.as_str(),
            MessagesOptions {
                dir: Some(cwd.clone()),
                include_system_messages: false,
                limit: Some(6),
                offset: 0,
            },
        )
        .await?;
    println!("{} transcript entry(ies):", messages.len());
    for entry in &messages {
        println!(
            "  {} parent={:?} at {}",
            entry.uuid, entry.parent_uuid, entry.timestamp
        );
    }

    // Rename and tag append records to the transcript file; a later `list`
    // shows the new title.
    archive
        .rename(
            first.as_str(),
            "clauders example 13",
            Some(&cwd.to_string_lossy()),
        )
        .await?;
    archive
        .tag(
            first.as_str(),
            Some("example"),
            Some(&cwd.to_string_lossy()),
        )
        .await?;
    println!("renamed and tagged {}", first.as_str());

    // Passing `None` clears the tag.
    archive
        .tag(first.as_str(), None, Some(&cwd.to_string_lossy()))
        .await?;
    println!("tag cleared");

    Ok(())
}

/// Run one prompt, print the reply, and return the session id it ran under.
async fn run(
    prompt: &str,
    session: Option<SessionControl>,
) -> Result<SessionId, Box<dyn std::error::Error>> {
    let mut builder = Options::builder()
        .system_prompt("Answer in as few words as possible.")
        .permission_mode(PermissionMode::Plan)
        .max_turns(2);
    if let Some(session) = session {
        builder = builder.session(session);
    }

    let mut stream = query(prompt, builder.build()).await?;
    let mut id = None;

    while let Some(frame) = stream.next().await {
        match frame? {
            Message::Assistant(assistant) => {
                for block in &assistant.content {
                    if let ContentBlock::Text { text } = block {
                        println!("{text}");
                    }
                }
            }
            Message::Result(result) => id = Some(result.session_id),
            _ => {}
        }
    }

    id.ok_or_else(|| "the turn ended without a result frame".into())
}
