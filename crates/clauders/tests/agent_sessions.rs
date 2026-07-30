#![expect(clippy::expect_used, reason = "tests assert via expect")]

//! End-to-end round-trip over a temp config root: create a session file,
//! then list / info / messages / rename / tag it.

use clauders::agent::{ListOptions, MessagesOptions, SessionArchive};

#[tokio::test]
async fn full_session_file_round_trip() {
    let tmp = tempfile::tempdir().expect("tmp");
    let id = "f28ced56-9bd4-41f8-a37d-2a496c7d0e35";
    let dir = tmp.path().join("projects").join("-repo-demo");
    tokio::fs::create_dir_all(&dir).await.expect("mkdir");
    let jsonl = concat!(
        r#"{"type":"user","uuid":"u1","parentUuid":null,"sessionId":"sess","timestamp":"2026-07-23T09:37:06.000Z","cwd":"/repo/demo","message":{"role":"user","content":"first prompt"}}"#,
        "\n",
        r#"{"type":"assistant","uuid":"a1","parentUuid":"u1","sessionId":"sess","message":{"id":"m1","content":[{"type":"text","text":"a reply"}]}}"#,
        "\n",
    );
    tokio::fs::write(dir.join(format!("{id}.jsonl")), jsonl)
        .await
        .expect("write");

    let archive = SessionArchive::with_base(tmp.path());

    // list
    let listed = archive.list(ListOptions::default()).await.expect("list");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].session_id.as_str(), id);
    assert_eq!(listed[0].summary, "first prompt");

    // info
    let info = archive.info(id, None).await.expect("info").expect("some");
    assert_eq!(info.first_prompt.as_deref(), Some("first prompt"));
    assert_eq!(info.created_at, Some(1_784_799_426_000));

    // messages
    let msgs = archive
        .messages(id, MessagesOptions::default())
        .await
        .expect("messages");
    assert_eq!(msgs.len(), 2);

    // rename + tag reflected in info
    archive
        .rename(id, "Demo Session", None)
        .await
        .expect("rename");
    archive.tag(id, Some("demo"), None).await.expect("tag");
    let after = archive.info(id, None).await.expect("info").expect("some");
    assert_eq!(after.custom_title.as_deref(), Some("Demo Session"));
    assert_eq!(after.tag.as_deref(), Some("demo"));
}
