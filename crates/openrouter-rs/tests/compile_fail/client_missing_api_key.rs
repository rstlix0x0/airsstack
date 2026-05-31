//! `build()` must be uncallable until `api_key` transitions Missing -> Present.
fn main() {
    let transport = openrouter_rs::transport::ReqwestTransport::try_new()
        .expect("transport builds");
    // No `.api_key(...)` call: the builder is still `ClientBuilder<Missing, _>`,
    // which has no `build` method. This must fail to compile.
    let _client = openrouter_rs::Client::builder_with_transport(transport).build();
}
