# claudevs (engine)

Engine library for `claudevs`, the Claude Code plugin lifecycle CLI. Holds the
canonical case model (YAML and Lua front-ends), the deterministic test harness
that spawns a plugin's hooks and scripts the way the Claude Code runtime would,
native-suite delegation, and report rendering. The `claudevs-cli` crate is the
binary; this crate is everything it calls.

```rust,no_run
let report = claudevs::run_suite(std::path::Path::new("plugins/my-plugin"), &claudevs::SuiteOptions::default())?;
println!("{}", claudevs::render_human(&report));
# Ok::<(), claudevs::Error>(())
```
