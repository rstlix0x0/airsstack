# 10 — Message Batches

Submit many requests as one batch, poll until it finishes, then stream the
per-row results.

## Run

```text
ANTHROPIC_API_KEY=sk-ant-... cargo run -p clauders --example 10_batches
```

## What it shows

The Batches sub-resource is reached with `client.messages().batches()`.

**Build and submit** a batch — each row carries a caller-chosen `custom_id` and a
full `MessageRequest`:

```rust
use clauders::messages::{BatchRequest, BatchResult, BatchStatus};
use clauders::types::CustomRequestId;

let batch_req = BatchRequest::builder()
    .add(CustomRequestId::new("greeting")?, one("Say hello in French."))
    .add(CustomRequestId::new("math")?, one("What is 6 times 7? Reply with the number only."))
    .build();

let batch = client.messages().batches().create(batch_req).await?;
```

**Poll** until the batch reaches a terminal state:

```rust
let ended = loop {
    let b = client.messages().batches().get(&batch.id).await?;
    if b.processing_status == BatchStatus::Ended {
        break b;
    }
    tokio::time::sleep(Duration::from_secs(5)).await;
};
```

**Stream the results.** `results()` returns a `BatchResultStream`
(`Stream<Item = Result<BatchResultRow, Error>>`); each row's `result` is one of
four outcomes:

```rust
let mut stream = client.messages().batches().results(&ended.id).await?;
// poll_next in a loop...
match row.result {
    BatchResult::Succeeded { message } => { /* message.content */ }
    BatchResult::Errored { error }     => println!("errored: {}", error.message),
    BatchResult::Canceled              => println!("canceled"),
    BatchResult::Expired               => println!("expired"),
}
```

## Notes

- Batches run **asynchronously server-side**. A small batch usually finishes in
  well under a minute, but it can take longer; the poll loop runs until the batch
  ends.
- Other operations on the sub-resource: `list()`, `cancel(&id)`, `delete(&id)`.
- `custom_id` is how you correlate each result row back to the request you
  submitted — result order is not guaranteed to match submission order.
