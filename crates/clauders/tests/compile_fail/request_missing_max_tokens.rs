fn main() {
    let _req = clauders::messages::MessageRequest::builder()
        .model(clauders::types::ModelId::claude_sonnet_4_5())
        .build();
}
