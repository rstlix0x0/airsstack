# 08 — Vision

Send an image and ask the model about it.

## Run

```text
ANTHROPIC_API_KEY=sk-ant-... cargo run -p clauders --example 08_vision
```

## What it shows

The image is **bundled next to the example** (`assets/sample.jpg`) and sent
**inline as base64**, so the run needs no external fetch:

```rust
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use clauders::messages::{ImageBlock, ImageMediaType, ImageSource, MessageContent, TextBlock};

const IMAGE_BYTES: &[u8] = include_bytes!("assets/sample.jpg");

let image = ImageBlock {
    source: ImageSource::Base64 {
        media_type: ImageMediaType::Jpeg,
        data: STANDARD.encode(IMAGE_BYTES),
    },
    cache_control: None,
};
```

**Compose a user turn** with the image block followed by the question:

```rust
let req = MessageRequest::builder()
    .model(ModelId::claude_sonnet_4_5())
    .max_tokens(MaxTokens::new(1024))
    .add_message(Role::User, MessageContent::Blocks(vec![
        ContentBlockParam::Image(image),
        ContentBlockParam::Text(TextBlock::new("Describe this image in one sentence.")),
    ]))
    .build();
```

## Notes

- **Base64 vs URL.** `ImageSource::Url { url }` is the other source form — it
  hands the API a URL to download. That is convenient but only works when the
  host lets the API's fetcher reach it; some hosts (e.g. Wikimedia) block it and
  the request fails with *"Unable to download the file"*. Bundling the bytes
  avoids that entirely.
- `ImageMediaType` covers `Jpeg`, `Png`, `Gif`, and `Webp`.
- `base64` is a dev-dependency, used only to encode the bundled bytes.
