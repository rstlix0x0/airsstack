# claudevs (engine)

Engine library for `claudevs`, the Claude Code plugin lifecycle CLI. Holds the
canonical case model (YAML and Lua front-ends), the deterministic test harness
that spawns a plugin's hooks and scripts the way the Claude Code runtime would,
native-suite delegation, static wiring checks, install-layout simulation, and
report rendering. The `claudevs-cli` crate is the binary; this crate is
everything it calls.

```rust,no_run
let report = claudevs::run_suite(std::path::Path::new("plugins/my-plugin"), &claudevs::SuiteOptions::default())?;
println!("{}", claudevs::render_human(&report));
# Ok::<(), claudevs::Error>(())
```

## Commands

| Command | What it does |
| --- | --- |
| `test [PATH]` | Runs every discovered case plus the native suites declared in `claudevs.toml`. |
| `test --installed [PATH]` | The same cases against a throwaway copy of the plugin in the shape it has once installed, so a path that resolves only in the checkout fails here. |
| `check [PATH]` | The gate: delegated manifest validation, then wiring, then `test`, then `test --installed` — stopping at the first failing stage. |
| `doctor [PATH]` | Names what this environment can and cannot do, one line per probe. |
| `migrate <case.yaml>` | Mechanical conversion of a YAML case to its data-Lua form. |

Every command that produces a report — `test`, `check`, `doctor` — takes
`--json` for the machine-readable form. `migrate` writes Lua, so it does not.

Exit codes read the same way everywhere: `0` when nothing was wrong, `1` for
verdict failures or findings, `2` when claudevs itself could not run. What
counts as "could not run" is per command, and the differences are deliberate.
`test` exits 2 on a usage error, an unreadable plugin directory, or a suite with
no cases to discover — a plugin with no cases is a broken discovery convention,
not a green suite. Inside `check` that same condition is an environment gap for
one stage, so it skips with a reason and the run can still end at 0. `doctor`
never exits 2 at all: every failure it can meet is something it reports as a
gap, which is a 1.

## The wiring checkers

`check`'s wiring stage runs three static checkers. None of them executes
anything in the plugin.

- **refs** — every `${CLAUDE_PLUGIN_ROOT}/…` occurrence anywhere in the plugin
  must resolve to a file that exists. A `..` segment that leaves the plugin root
  is a finding even when the path it names happens to resolve today: the file is
  not shipped with the plugin and will not be there once it is installed.
- **invocations** — fenced command blocks in skill markdown are parsed into
  invocations by the crate's one fenced-command parser, the same one
  `t.skill_command` uses. Scripts that exist in the plugin but are named by
  nothing else in it are reported as dead files. That one is a **warning**, not
  an error: it does not fail the stage. Case files are exempt, because the suite
  runner finds them by naming convention rather than by any reference.
- **matchers** — `hooks.json` event names must be known events, and each
  `matcher` must compile. Compilation uses the `regex` crate, which has no
  lookaround and no backreferences, so a pattern relying on either is reported
  here even where the runtime might accept it. The finding says which engine
  rejected the pattern.

## The validation stage and its absence

`check`'s first stage shells out to `claude plugin validate --strict` rather
than reimplementing manifest validation. The binary is not a requirement: when
it cannot be run, that stage reports itself skipped with the reason, and the
three deterministic stages still gate. `doctor` names the same gap directly.

A skip always means the *environment* is missing something — no `claude`, no
marketplace manifest above the plugin to key an install path by, no case files
yet. It never means the plugin is wrong. A malformed `plugin.json` fails its
stage rather than skipping it, so a plugin that cannot be installed cannot pass
the gate on a machine where the validation stage had already skipped.
