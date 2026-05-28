fn main() {
    // `Client::builder()` returns `Result<ClientBuilder<Missing, _>, _>`.
    // Unwrap to expose the `Missing` builder; calling `.build()` on it must
    // fail to compile because `build()` is not in scope for `Missing`.
    let _ = clauders::Client::builder().unwrap().build();
}
