-- Tests for lib/enforce — the resolution pipeline and its gates.
--
--   airsl test --policy confined --allow-read / --allow-write "${TMPDIR:-/tmp}" \
--     --allow-exec git plugins/airsstack/hooks
--
-- The read grant is `/`, not `/tmp`: fixtures live under `fs.tempdir()`, but the guard below
-- also reads real source files anywhere under the repository's `plugins/` tree.

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

-- Splits `text` into lines. Every driver under `plugins/` (`enforce.lua`, `concise-tracker.lua`,
-- `session-start.lua`, ...) is not `require`-able as a module — each reads stdin/its payload and
-- runs at load time — so a driver's own emission call cannot be exercised directly from here.
-- Reading its source and asserting on it is the crude but honest alternative: the property being
-- guarded ("no driver may emit `permissionDecision`") is a textual one.
local function lines_of(text)
  local result = {}
  for line in (text .. "\n"):gmatch("(.-)\n") do
    result[#result + 1] = line
  end
  return result
end

-- The directory this test file was loaded from — `arg[0]` is the path the harness read to find
-- it, Lua's own convention for a standalone script (see `airsl::engine::set_arguments`), so
-- resolving it through `path.absolute` needs neither `git` nor any assumption about the caller's
-- cwd or which repository it sits in: it is arithmetic over the one path already known to be
-- correct, since it is the path this very file was just read from.
-- Anchored on a marker file, not on the directory's name. `plugins` is far too common a name to
-- match on: this file is copied verbatim into the plugin install cache, where it sits at
-- `~/.claude/plugins/cache/<marketplace>/<plugin>/<version>/hooks/`, and the first ancestor there
-- called `plugins` is `~/.claude/plugins` itself. Matching that would point the sweep at every
-- plugin of every marketplace on the machine and fail on a third party's hook, which is entitled
-- to emit whatever it likes. Requiring the suite's own manifest underneath keeps the walk inside
-- a checkout of this repository.
local function plugins_root()
  local dir = path.dirname(path.absolute(arg[0]))
  for _ = 1, 10 do
    if fs.is_file(path.join(dir, "airsstack", ".claude-plugin", "plugin.json")) then
      return dir
    end
    local parent = path.dirname(dir)
    if parent == dir then
      return nil
    end
    dir = parent
  end
  return nil
end

-- Every `.lua` file under `root`, relative-to-root paths, sorted.
local function all_lua_files(root)
  local ok, entries = pcall(fs.walk, root)
  assert(ok, "could not walk " .. root .. ": " .. tostring(entries))
  local found = {}
  for _, rel in ipairs(entries) do
    if rel:match("%.lua$") and fs.is_file(path.join(root, rel)) then
      found[#found + 1] = rel
    end
  end
  table.sort(found)
  return found
end

-- The code lines of `source`, one entry per source line, with every comment removed: both a `--`
-- line comment and a `--[[ ... ]]` block comment (which may open and close on the same line, or
-- span several). Aware of `'` and `"` string literals so a `--` embedded in one — this very file
-- uses exactly that construct, e.g. `enforce.lua`'s `"--show-toplevel"` — is not mistaken for a
-- comment opener, which is how the previous version of this guard missed code written after such
-- a string on the same line. Long bracket strings are tracked for the same reason, and they are
-- not hypothetical: `lib/concise.lua` builds its patterns with them (`regex.compile([[\bnormal
-- mode\b]])` and seven more), so a `--` inside one would truncate a real line of this suite.
--
-- Long brackets are tracked at their own level (`[[`, `[=[`, `[==[` … each closed only by a `]`
-- with the same run of `=`), in both forms Lua gives them: `--[==[ … ]==]` is a block comment and
-- drops out, while a bare `[==[ … ]==]` is a string literal whose text is KEPT. Keeping it is
-- deliberate and matches how `'`/`"` strings are treated here — a field name reaching the CLI
-- inside a string literal is still a field reaching the CLI, so a payload like
-- `json.decode([[{"permissionDecision":"defer"}]])` must not be able to hide in one.
--
-- One construct still slips through, accepted rather than chased: a key built by concatenation,
-- e.g. `s["permission" .. "Decision"] = "defer"`, never puts the substring "permissionDecision"
-- in a single token for a text scan to find.
local function strip_comments(source)
  local out = {}
  -- While inside a multi-line long bracket: its `=` count, and whether it is a comment.
  local long_level, long_is_comment = nil, false
  for _, line in ipairs(lines_of(source)) do
    local code = {}
    local i, n = 1, #line
    local quote = nil

    if long_level then
      local closer = "]" .. string.rep("=", long_level) .. "]"
      local close = line:find(closer, 1, true)
      if close then
        if not long_is_comment then
          code[#code + 1] = line:sub(1, close - 1)
        end
        i = close + #closer
        long_level, long_is_comment = nil, false
      else
        if not long_is_comment then
          code[#code + 1] = line
        end
        i = n + 1
      end
    end

    while i <= n do
      local ch = line:sub(i, i)
      local comment_eq = line:match("^%-%-%[(=*)%[", i)
      local string_eq = line:match("^%[(=*)%[", i)
      if quote then
        code[#code + 1] = ch
        if ch == "\\" then
          code[#code + 1] = line:sub(i + 1, i + 1)
          i = i + 2
        else
          if ch == quote then
            quote = nil
          end
          i = i + 1
        end
      elseif ch == '"' or ch == "'" then
        quote = ch
        code[#code + 1] = ch
        i = i + 1
      elseif comment_eq then
        local closer = "]" .. comment_eq .. "]"
        local close = line:find(closer, i + 4 + #comment_eq, true)
        if close then
          i = close + #closer
        else
          long_level, long_is_comment = #comment_eq, true
          i = n + 1
        end
      elseif line:sub(i, i + 1) == "--" then
        i = n + 1
      elseif string_eq then
        local closer = "]" .. string_eq .. "]"
        local close = line:find(closer, i + 2 + #string_eq, true)
        if close then
          code[#code + 1] = line:sub(i, close + #closer - 1)
          i = close + #closer
        else
          long_level, long_is_comment = #string_eq, false
          code[#code + 1] = line:sub(i)
          i = n + 1
        end
      else
        code[#code + 1] = ch
        i = i + 1
      end
    end
    out[#out + 1] = table.concat(code)
  end
  return out
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

  no_plugin_driver_ever_emits_a_permission_decision = function()
    -- Any hook that returns `permissionDecision` on PreToolUse can have the CLI swallow the tool
    -- call it fired on outright — no `tool_result` at all — when the session is non-interactive,
    -- the tool batch is solo, and the abort signal is not already set: a subagent's run ends
    -- reported `completed` having done nothing. That is the defect `hook.context` (see
    -- `enforce.lua`'s own comment above its emission call) exists to avoid, and it is not unique
    -- to `enforce.lua` — every hook driver under `plugins/` fires on some Claude Code event and
    -- so carries the same risk. This sweeps all of them rather than the one path a previous
    -- version of this test named.
    local root = plugins_root()
    assert(root, "could not resolve the plugin suite root from this test file's own path "
      .. "(arg[0] = " .. tostring(arg[0]) .. "): no ancestor directory holds "
      .. "`airsstack/.claude-plugin/plugin.json`. This guard sweeps a checkout of this "
      .. "repository; running it from the install cache has no suite source to check, and it "
      .. "refuses rather than sweeping whatever `plugins/` directory it happens to sit under")

    -- A named file, not a count: a threshold drifts with the suite's size, failing the day a
    -- plugin is removed rather than the day the root is wrong.
    assert(fs.is_file(path.join(root, "airsstack", "hooks", "enforce.lua")),
      "resolved the suite root to " .. root .. ", which holds no airsstack/hooks/enforce.lua")
    local files = all_lua_files(root)

    -- This test file itself is excluded: it names `permissionDecision` in code, in its own
    -- assertion messages and in the string literal every other file's source is searched for,
    -- and that is talking *about* the field, not emitting it. Guarding this file's own vocabulary
    -- is not the property under test.
    local self_path = path.absolute(arg[0])

    for _, rel in ipairs(files) do
      local file = path.join(root, rel)
      if file ~= self_path then
        local source = fs.read(file)
        for lineno, code in ipairs(strip_comments(source)) do
          assert(not code:find("permissionDecision", 1, true), file .. ":" .. lineno
            .. ": emits `permissionDecision` — a hook driver firing on PreToolUse (or any event) "
            .. "that returns this field can have the CLI swallow the tool call outright (no "
            .. "tool_result at all) in a non-interactive subagent session, stranding whatever the "
            .. "tool call was meant to do with the run still reported `completed`. Emit through "
            .. "`airsstack.hook.context`, never a hand-built `hook.emit` envelope carrying this "
            .. "field.")
        end
      end
    end
  end,
}
