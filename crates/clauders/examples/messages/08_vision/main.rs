//! Vision — send an image and ask the model about it.
//!
//! Builds a user turn holding an image block plus a text question. The image
//! is bundled with the example and sent inline as base64, so the run needs no
//! external fetch. The other source form is `ImageSource::Url`, which hands the
//! API a URL to download — convenient, but only when the host allows the API's
//! fetcher to reach it.
//!
//! Run:
//!
//! ```text
//! ANTHROPIC_API_KEY=sk-... cargo run --example 08_vision
//! ```

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use clauders::messages::{ImageBlock, ImageMediaType, ImageSource, MessageContent, TextBlock};
use clauders::prelude::*;

// A small JPEG bundled next to this example.
const IMAGE_BYTES: &[u8] = include_bytes!("assets/sample.jpg");

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = ApiKey::new(std::env::var("ANTHROPIC_API_KEY")?)?;
    let client = Client::builder()?.api_key(api_key).build()?;
    let max_tokens = MaxTokens::new(1024);

    let image = ImageBlock {
        source: ImageSource::Base64 {
            media_type: ImageMediaType::Jpeg,
            data: STANDARD.encode(IMAGE_BYTES),
        },
        cache_control: None,
    };

    // One user turn carrying the image followed by the question.
    let req = MessageRequest::builder()
        .model(ModelId::claude_sonnet_4_5())
        .max_tokens(max_tokens)
        .add_message(
            Role::User,
            MessageContent::Blocks(vec![
                ContentBlockParam::Image(image),
                ContentBlockParam::Text(TextBlock::new("Describe this image in one sentence.")),
            ]),
        )
        .build();

    let msg = client.messages().create(req).await?;

    for block in &msg.content {
        if let ContentBlock::Text(t) = block {
            println!("{}", t.text);
        }
    }

    Ok(())
}
