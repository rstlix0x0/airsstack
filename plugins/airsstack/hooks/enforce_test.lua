-- Tests for lib/enforce — the resolution pipeline and its gates.
--
--   airsl test --allow-read /tmp --allow-write /tmp --allow-exec git plugins/airsstack/hooks

local enforce = require("lib.enforce")
local fs = airsstack.fs
local json = airsstack.json
local path = airsstack.path

local MANIFEST = {
  stack = "rust",
  detect = { "Cargo.toml" },
  match = { "**/*.rs", "**/Cargo.toml" },
  skill = "airsstack-guideline-rust:rust-guidelines",
  phase = { "code", "design" },
}

-- A plugin install cache carrying `manifest`.
local function cache(manifest)
  local dir = fs.tempdir()
  fs.mkdir(path.join(dir, ".claude-plugin"))
  fs.write(path.join(dir, ".claude-plugin", "plugin.json"), '{"name":"x"}\n')
  if manifest then
    fs.write(path.join(dir, "enforcement.json"), json.encode(manifest))
  end
  return dir
end

-- A git repository with a Cargo.toml and a Rust file.
local function repo()
  local dir = fs.tempdir()
  enforce.git(dir, "init", "-q")
  enforce.git(dir, "-c", "user.email=t@t", "-c", "user.name=t", "commit", "-q",
    "--allow-empty", "-m", "init")
  fs.mkdir(path.join(dir, "src"))
  fs.write(path.join(dir, "Cargo.toml"), "[package]\n")
  fs.write(path.join(dir, "src", "main.rs"), "fn main() {}\n")
  fs.write(path.join(dir, "README.md"), "# notes\n")
  return dir
end

local function registry(entries)
  local file = path.join(fs.tempdir(), "installed_plugins.json")
  fs.write(file, json.encode({ plugins = entries }))
  return file
end

local function resolve(overrides)
  local context = {
    session_id = "s",
    agent = "main",
    home = fs.tempdir(),
    sentinel_dir = fs.tempdir(),
  }
  for key, value in pairs(overrides) do
    context[key] = value
  end
  return enforce.resolve(context)
end

return {
  a_matching_rust_file_emits_one_pointer = function()
    local project, install = repo(), cache(MANIFEST)
    local pointers = resolve({
      file_path = path.join(project, "src", "main.rs"),
      cwd = project,
      registry = registry({ ["rust@airsstack"] = { { installPath = install } } }),
    })
    assert(#pointers == 1, "expected one pointer, got " .. #pointers)
    assert(pointers[1]:find("rust-guidelines", 1, true), pointers[1])
    assert(pointers[1]:find("Definition of Done", 1, true), "code phase names the DoD")
  end,

  a_root_cargo_toml_matches_the_zero_segment_glob = function()
    local project, install = repo(), cache(MANIFEST)
    local pointers = resolve({
      file_path = path.join(project, "Cargo.toml"),
      cwd = project,
      registry = registry({ ["rust@airsstack"] = { { installPath = install } } }),
    })
    assert(#pointers == 1, "a root Cargo.toml must match **/Cargo.toml")
  end,

  a_file_matching_no_glob_emits_nothing = function()
    local project, install = repo(), cache(MANIFEST)
    local pointers = resolve({
      file_path = path.join(project, "README.md"),
      cwd = project,
      registry = registry({ ["rust@airsstack"] = { { installPath = install } } }),
    })
    assert(#pointers == 0, "gate 3 should have stopped this")
  end,

  a_repository_without_the_detect_marker_emits_nothing = function()
    local project, install = fs.tempdir(), cache(MANIFEST)
    fs.mkdir(path.join(project, "src"))
    fs.write(path.join(project, "src", "main.rs"), "fn main() {}\n")
    local pointers = resolve({
      file_path = path.join(project, "src", "main.rs"),
      cwd = project,
      registry = registry({ ["rust@airsstack"] = { { installPath = install } } }),
    })
    assert(#pointers == 0, "gate 2 should have stopped this")
  end,

  a_plugin_from_another_marketplace_is_never_read = function()
    local project, install = repo(), cache(MANIFEST)
    local pointers = resolve({
      file_path = path.join(project, "src", "main.rs"),
      cwd = project,
      registry = registry({ ["rust@elsewhere"] = { { installPath = install } } }),
    })
    assert(#pointers == 0, "the marketplace suffix is the scope guard")
  end,

  a_record_bound_to_another_project_contributes_nothing = function()
    -- The anti-leak property: a plugin installed only for repo A must be silent in repo B.
    local other, project, install = repo(), repo(), cache(MANIFEST)
    local file = registry({
      ["rust@airsstack"] = { { installPath = install, projectPath = other } },
    })

    assert(#resolve({
      file_path = path.join(other, "src", "main.rs"), cwd = other, registry = file,
    }) == 1, "the bound project still fires")

    assert(#resolve({
      file_path = path.join(project, "src", "main.rs"), cwd = project, registry = file,
    }) == 0, "another project must not inherit it")
  end,

  a_user_scope_record_is_the_fallback = function()
    local other, project, install = repo(), repo(), cache(MANIFEST)
    local file = registry({
      ["rust@airsstack"] = {
        { installPath = install, projectPath = other },
        { installPath = install },
      },
    })
    assert(#resolve({
      file_path = path.join(project, "src", "main.rs"), cwd = project, registry = file,
    }) == 1, "the unbound record governs every other project")
  end,

  a_missing_or_malformed_manifest_skips_that_plugin = function()
    local project = repo()
    for _, install in ipairs({ cache(nil), cache({ stack = "rust" }) }) do
      local pointers = resolve({
        file_path = path.join(project, "src", "main.rs"),
        cwd = project,
        registry = registry({ ["rust@airsstack"] = { { installPath = install } } }),
      })
      assert(#pointers == 0, "an unusable manifest must not emit")
    end
  end,

  a_missing_registry_resolves_to_nothing = function()
    local project = repo()
    assert(#resolve({
      file_path = path.join(project, "src", "main.rs"),
      cwd = project,
      registry = "/nonexistent/installed_plugins.json",
    }) == 0)
  end,

  the_sentinel_makes_the_pointer_one_shot_per_context = function()
    local project, install = repo(), cache(MANIFEST)
    local context = {
      file_path = path.join(project, "src", "main.rs"),
      cwd = project,
      registry = registry({ ["rust@airsstack"] = { { installPath = install } } }),
      session_id = "s1",
      agent = "main",
      home = fs.tempdir(),
      sentinel_dir = fs.tempdir(),
    }
    assert(#enforce.resolve(context) == 1, "the first call emits")
    assert(#enforce.resolve(context) == 0, "the second call is suppressed")
  end,

  a_subagent_does_not_consume_the_main_threads_pointer = function()
    -- Subagents inherit the parent's session_id, so without the agent component an explorer
    -- reading one .rs file would spend the pointer the main thread exists to receive.
    local project, install = repo(), cache(MANIFEST)
    local shared = {
      file_path = path.join(project, "src", "main.rs"),
      cwd = project,
      registry = registry({ ["rust@airsstack"] = { { installPath = install } } }),
      session_id = "s1",
      home = fs.tempdir(),
      sentinel_dir = fs.tempdir(),
    }

    local function with(agent)
      local context = { agent = agent }
      for key, value in pairs(shared) do
        context[key] = value
      end
      return enforce.resolve(context)
    end

    assert(#with("explorer-1") == 1, "the subagent gets its own pointer")
    assert(#with("main") == 1, "the main thread still gets one")
  end,

  a_design_doc_gets_the_architecture_pointer_rather_than_the_dod = function()
    local project, install = repo(), cache(MANIFEST)
    local home = fs.tempdir()
    local spec = path.join(enforce.sdd_root(home), "key-abc", "specs", "a.md")
    fs.mkdir(path.dirname(spec))
    fs.write(spec, "# spec\n")

    local pointers = resolve({
      file_path = spec,
      cwd = project,
      registry = registry({ ["rust@airsstack"] = { { installPath = install } } }),
      home = home,
    })
    assert(#pointers == 1, "a design doc under the SDD root should fire")
    assert(pointers[1]:find("architecture rules to this design", 1, true), pointers[1])
    assert(not pointers[1]:find("Definition of Done", 1, true),
      "a design has nothing to build, so the DoD is a demand the reader cannot satisfy")
  end,

  a_specs_segment_deeper_in_the_tree_is_not_a_design_doc = function()
    -- A substring test on '/specs/' matched `<key>/a/specs/b/plans/c.md` by accident, which swaps
    -- the Definition of Done for the architecture rules.
    local home = fs.tempdir()
    local root = enforce.sdd_root(home)
    assert(enforce.is_design_doc(path.join(root, "key", "specs", "a.md"), home) == true)
    assert(enforce.is_design_doc(path.join(root, "key", "plans", "a.md"), home) == true)
    assert(enforce.is_design_doc(path.join(root, "key", "a", "specs", "b", "c.md"), home) == false)
    assert(enforce.is_design_doc(path.join(root, "key", "a.md"), home) == false)
    assert(enforce.is_design_doc("/elsewhere/specs/a.md", home) == false)
  end,

  a_manifest_that_does_not_declare_the_phase_is_skipped = function()
    local project = repo()
    local install = cache({
      stack = "rust", skill = "s", detect = { "Cargo.toml" },
      match = { "**/*.rs" }, phase = { "design" },
    })
    assert(#resolve({
      file_path = path.join(project, "src", "main.rs"),
      cwd = project,
      registry = registry({ ["rust@airsstack"] = { { installPath = install } } }),
    }) == 0, "a design-only manifest must not fire on a code file")
  end,

  the_match_path_is_relative_to_the_repository_root = function()
    local project = repo()
    assert(enforce.path_for_matching(path.join(project, "src", "main.rs"), project)
      == "src/main.rs")
  end,

  outside_a_repository_the_match_path_is_the_basename = function()
    local loose = path.join(fs.tempdir(), "loose.rs")
    fs.write(loose, "fn main() {}\n")
    assert(enforce.path_for_matching(loose, path.dirname(loose)) == "loose.rs")
  end,

  every_worktree_of_one_repository_shares_a_key = function()
    local project = repo()
    local linked = path.join(fs.tempdir(), "wt")
    enforce.git(project, "worktree", "add", "-q", linked, "-b", "feature")
    assert(enforce.project_key(project) == enforce.project_key(linked))
  end,

  a_sentinel_is_claimed_exactly_once = function()
    local directory = fs.tempdir()
    local file = enforce.sentinel_path(directory, "s", "main", "rust", "code")
    assert(enforce.claim(file) == true, "the first claim wins")
    assert(enforce.claim(file) == false, "the second finds it taken")
  end,

  clearing_a_session_leaves_other_sessions_alone = function()
    local directory = fs.tempdir()
    enforce.claim(enforce.sentinel_path(directory, "mine", "main", "rust", "code"))
    enforce.claim(enforce.sentinel_path(directory, "yours", "main", "rust", "code"))

    assert(enforce.clear_session(directory, "mine") == 1)
    assert(#enforce.held_sentinels(directory, "mine", "main") == 0)
    assert(#enforce.held_sentinels(directory, "yours", "main") == 1)
  end,

  a_stale_sentinel_is_pruned_and_a_fresh_one_is_kept = function()
    local directory = fs.tempdir()
    local file = enforce.sentinel_path(directory, "s", "main", "rust", "code")
    enforce.claim(file)

    local now = airsstack.time.now()
    enforce.prune_sentinels(directory, now)
    assert(fs.exists(file), "a fresh sentinel must survive")

    enforce.prune_sentinels(directory, now + enforce.SENTINEL_MAX_AGE + 60)
    assert(not fs.exists(file), "a sentinel past its age must be pruned")
  end,

  a_file_that_is_not_a_sentinel_is_never_pruned = function()
    local directory = fs.tempdir()
    local other = path.join(directory, "unrelated.txt")
    fs.write(other, "x")
    enforce.prune_sentinels(directory, airsstack.time.now() + 10 * enforce.SENTINEL_MAX_AGE)
    assert(fs.exists(other), "only this hook's own sentinels are prunable")
  end,

  a_held_sentinel_is_reported_as_stack_colon_phase = function()
    local directory = fs.tempdir()
    enforce.claim(enforce.sentinel_path(directory, "s", "main", "rust", "code"))
    local held = enforce.held_sentinels(directory, "s", "main")
    assert(held[1] == "rust:code", tostring(held[1]))
  end,

  the_trace_names_the_silent_exit_that_was_taken = function()
    local project = repo()
    local trace = {}
    enforce.resolve({
      file_path = path.join(project, "src", "main.rs"),
      cwd = project,
      registry = "/nonexistent/installed_plugins.json",
      session_id = "s", agent = "main",
      home = fs.tempdir(), sentinel_dir = fs.tempdir(),
    }, trace)
    assert(#trace > 0, "the doctor needs a trace to explain silence")
    assert(table.concat(trace, "\n"):find("no @airsstack plugins", 1, true),
      table.concat(trace, "\n"))
  end,
}
