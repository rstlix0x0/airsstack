use openrouter_rs::chat::{ChatRequest, Message};

fn main() {
    // messages set, model missing — build() must not exist.
    let _ = ChatRequest::builder()
        .messages(vec![Message::user("hi")])
        .build();
}
