-- Tests for lib/cache and lib/guard — the registry gate, containment, and the mirror.
--
--   airsl test --allow-read / --allow-write /tmp --allow-exec git plugins/airsstack-plugin-dev/hooks

local cache = require("lib.cache")
local guard = require("lib.guard")
local fs = airsstack.fs
local json = airsstack.json
local path = airsstack.path

local function tree(files)
  local root = fs.tempdir()
  for rel, text in pairs(files or {}) do
    local target = path.join(root, rel)
    fs.mkdir(path.dirname(target))
    fs.write(target, text)
  end
  return root
end

local function registry(entries)
  return { plugins = entries }
end

return {
  a_path_under_plugins_yields_the_plugin_and_the_remainder = function()
    local plugin, rel = cache.extract_plugin_rel("/repo/plugins/airsstack/hooks/enforce.lua")
    assert(plugin == "airsstack", tostring(plugin))
    assert(rel == "hooks/enforce.lua", tostring(rel))
  end,

  a_path_naming_only_a_plugin_directory_is_not_an_edit = function()
    assert(cache.extract_plugin_rel("/repo/plugins/airsstack") == nil)
    assert(cache.extract_plugin_rel("/repo/crates/airsl/src/lib.rs") == nil)
  end,

  the_first_plugins_segment_wins = function()
    local plugin, rel = cache.extract_plugin_rel("/a/plugins/one/plugins/two/x.md")
    assert(plugin == "one", tostring(plugin))
    assert(rel == "plugins/two/x.md", tostring(rel))
  end,

  only_the_airsstack_marketplace_resolves_to_install_paths = function()
    local reg = registry({
      ["airsstack@airsstack"] = { { installPath = "/cache/a" } },
      ["airsstack@elsewhere"] = { { installPath = "/cache/b" } },
    })
    local found = cache.install_paths(reg, "airsstack")
    assert(#found == 1 and found[1] == "/cache/a", table.concat(found, ","))
  end,

  duplicate_install_paths_are_collapsed = function()
    -- Several registry records commonly point at one cache directory; mirroring per record would
    -- multiply every reported line.
    local reg = registry({
      ["p@airsstack"] = {
        { installPath = "/cache/a" }, { installPath = "/cache/a" }, { installPath = "/cache/b" },
      },
    })
    local found = cache.install_paths(reg, "p")
    assert(#found == 2, table.concat(found, ","))
    assert(found[1] == "/cache/a" and found[2] == "/cache/b", "first-seen order")
  end,

  an_unknown_plugin_resolves_to_nothing = function()
    assert(#cache.install_paths(registry({}), "absent") == 0)
    assert(#cache.install_paths({}, "absent") == 0)
  end,

  containment_compares_segments_rather_than_string_prefixes = function()
    assert(cache.is_within("/cache/plugin/file", "/cache") == true)
    assert(cache.is_within("/cache", "/cache") == true)
    assert(cache.is_within("/cache-extra/file", "/cache") == false,
      "a string prefix test would accept this and contain nothing")
    assert(cache.is_within("/elsewhere/file", "/cache") == false)
  end,

  a_traversal_escaping_the_cache_root_is_refused = function()
    assert(cache.is_within("/cache/../etc/passwd", "/cache") == false)
  end,

  a_destination_outside_the_cache_root_is_never_written = function()
    local source = path.join(tree({ ["a.txt"] = "x" }), "a.txt")
    local outside = fs.tempdir()
    local dest, reason = cache.sync_one(source, "a.txt", outside, "/nonexistent-cache-root")
    assert(dest == nil, "containment must refuse this")
    assert(reason:find("outside", 1, true), reason)
    assert(not fs.exists(path.join(outside, "a.txt")), "and nothing may be written")
  end,

  a_file_inside_the_cache_root_is_copied_with_its_directories = function()
    local source = path.join(tree({ ["a.txt"] = "hello" }), "a.txt")
    local root = fs.tempdir()
    local install = path.join(root, "plugin", "1.0.0")
    local dest = cache.sync_one(source, "hooks/a.txt", install, root)
    assert(dest, "the copy should have been permitted")
    assert(fs.read(dest) == "hello")
  end,

  the_walk_skips_the_ignored_names = function()
    local root = tree({ ["a.txt"] = "x", [".in_use"] = "x", ["sub/.DS_Store"] = "x" })
    local found = cache.relative_files(root)
    assert(table.concat(found, ",") == "a.txt", table.concat(found, ","))
  end,

  an_unwalkable_tree_is_nil_rather_than_empty = function()
    -- An unreadable source tree reading back as an empty one is how a delivery failure reports
    -- agreement.
    assert(cache.relative_files("/nonexistent-source-tree") == nil)
  end,

  the_mirror_copies_what_is_missing_and_what_differs = function()
    local src = tree({ ["a.txt"] = "new", ["b.txt"] = "same", ["c.txt"] = "added" })
    local root = fs.tempdir()
    local dest = path.join(root, "plugin", "1.0.0")
    fs.mkdir(dest)
    fs.write(path.join(dest, "a.txt"), "old")
    fs.write(path.join(dest, "b.txt"), "same")

    local copied = cache.sync_tree(src, dest, root)
    assert(table.concat(copied, ",") == "a.txt,c.txt", table.concat(copied, ","))
    assert(fs.read(path.join(dest, "a.txt")) == "new")
  end,

  a_cache_only_file_is_reported_and_never_deleted = function()
    local src = tree({ ["a.txt"] = "x" })
    local root = fs.tempdir()
    local dest = path.join(root, "plugin", "1.0.0")
    fs.mkdir(dest)
    fs.write(path.join(dest, "stale.txt"), "left over")

    local _, extras = cache.sync_tree(src, dest, root)
    assert(table.concat(extras, ",") == "stale.txt", table.concat(extras, ","))
    assert(fs.exists(path.join(dest, "stale.txt")), "extras are reported, never removed")
  end,

  a_wholly_missing_cache_directory_is_backfilled_rather_than_skipped = function()
    local src = tree({ ["a.txt"] = "x", ["sub/b.txt"] = "y" })
    local root = fs.tempdir()
    local copied = cache.sync_tree(src, path.join(root, "plugin", "1.0.0"), root)
    assert(#copied == 2, table.concat(copied, ","))
  end,

  the_mirror_writes_nothing_outside_the_containment_root = function()
    local src = tree({ ["a.txt"] = "x" })
    local elsewhere = fs.tempdir()
    local copied = cache.sync_tree(src, elsewhere, "/nonexistent-cache-root")
    assert(#copied == 0, "every write must be refused")
    assert(not fs.exists(path.join(elsewhere, "a.txt")))
  end,

  a_marketplace_manifest_is_recognised_by_its_name = function()
    local top = tree({ [".claude-plugin/marketplace.json"] = '{"name":"airsstack"}' })
    assert(guard.is_airsstack_marketplace(top) == true)

    local other = tree({ [".claude-plugin/marketplace.json"] = '{"name":"something-else"}' })
    assert(guard.is_airsstack_marketplace(other) == false)
    assert(guard.is_airsstack_marketplace(fs.tempdir()) == false)
  end,

  only_directories_carrying_a_plugin_manifest_count_as_plugins = function()
    local top = tree({
      ["plugins/one/.claude-plugin/plugin.json"] = "{}",
      ["plugins/two/.claude-plugin/plugin.json"] = "{}",
      ["plugins/notaplugin/README.md"] = "x",
    })
    local found = guard.source_plugins(top)
    assert(table.concat(found, ",") == "one,two", table.concat(found, ","))
  end,

  the_main_worktree_is_distinguished_from_a_linked_one = function()
    -- `rev-parse --show-toplevel` succeeds from both, so it cannot be the gate on its own: several
    -- checkouts share one version-keyed cache, and the cache would converge on a union of branches.
    local repo = fs.tempdir()
    cache.git(repo, "init", "-q")
    cache.git(repo, "-c", "user.email=t@t", "-c", "user.name=t", "commit", "-q",
      "--allow-empty", "-m", "init")
    local linked = path.join(fs.tempdir(), "wt")
    cache.git(repo, "worktree", "add", "-q", linked, "-b", "feature")

    assert(guard.is_main_worktree(repo) == true)
    assert(guard.is_main_worktree(linked) == false)
  end,

  a_directory_with_no_git_is_not_a_main_worktree = function()
    assert(guard.is_main_worktree(fs.tempdir()) == false)
  end,

  the_listing_caps_at_five_and_counts_the_rest = function()
    assert(guard.listing({ "a", "b" }) == "a, b")
    local many = {}
    for index = 1, 8 do
      many[index] = "f" .. index
    end
    assert(guard.listing(many) == "f1, f2, f3, f4, f5 (+3 more)", guard.listing(many))
  end,

  a_report_with_nothing_to_say_is_empty = function()
    assert(#guard.format_report(true, {}) == 0)
    assert(#guard.format_report(true, {
      { plugin = "p", copied = {}, extras = {}, drift = "ok", uncommitted = false },
    }) == 0, "an in-sync plugin prints nothing")
  end,

  a_backfill_is_reported_per_plugin_with_the_restart_note = function()
    local lines = guard.format_report(true, {
      { plugin = "p", copied = { "a.md" }, extras = {}, drift = "ok", uncommitted = false },
    })
    local text = table.concat(lines, "\n")
    assert(text:find("p: backfilled 1 file(s): a.md", 1, true), text)
    assert(text:find(guard.RESTART_NOTE, 1, true), "a backfill needs the restart note")
    assert(not text:find(guard.PUSH_NOTE, 1, true), "and not the publication one")
  end,

  staleness_is_aggregated_into_one_line_with_the_push_note = function()
    -- One line per stale plugin would print a wall on every session start and train you to skim
    -- past the whole report.
    local results = {}
    for index = 1, 5 do
      results[index] = {
        plugin = "p" .. index, copied = {}, extras = {}, drift = "stale", uncommitted = false,
      }
    end
    local text = table.concat(guard.format_report(true, results), "\n")
    assert(text:find("5 of 5 plugins stale", 1, true), text)
    assert(text:find(guard.PUSH_NOTE, 1, true), "staleness needs the publication note")
  end,

  a_linked_worktree_says_it_wrote_nothing = function()
    local lines = guard.format_report(false, {
      { plugin = "p", copied = {}, extras = { "x" }, drift = "ok", uncommitted = false },
    })
    assert(lines[1]:find("reporting only, nothing written", 1, true), lines[1])
  end,

  an_uncommitted_edit_is_reported_per_plugin = function()
    local text = table.concat(guard.format_report(true, {
      { plugin = "p", copied = {}, extras = {}, drift = "ok", uncommitted = true },
    }), "\n")
    assert(text:find("p: uncommitted edits in the working tree", 1, true), text)
  end,

  a_version_bump_is_found_by_value_rather_than_by_touch = function()
    -- "The last commit touching plugin.json" reports a manifest edited without a version change as
    -- a bump, which is one of the three false negatives that rule had.
    local repo = fs.tempdir()
    local manifest = path.join(repo, "plugins", "p", ".claude-plugin", "plugin.json")
    fs.mkdir(path.dirname(manifest))
    cache.git(repo, "init", "-q")

    local function commit(version, note)
      fs.write(manifest, json.encode({ name = "p", version = version, note = note }))
      cache.git(repo, "add", "-A")
      cache.git(repo, "-c", "user.email=t@t", "-c", "user.name=t", "commit", "-q", "-m", note)
    end

    commit("1.0.0", "first")
    commit("1.1.0", "bump")
    commit("1.1.0", "touch without bumping")

    local bump = guard.last_bump_commit(repo, "p")
    assert(bump, "a bump should have been found")
    local subject = cache.git(repo, "log", "-1", "--format=%s", bump)
    assert(subject == "bump", "found " .. tostring(subject) .. " rather than the bump commit")
  end,

  content_committed_after_the_last_bump_reads_as_stale = function()
    local repo = fs.tempdir()
    local manifest = path.join(repo, "plugins", "p", ".claude-plugin", "plugin.json")
    fs.mkdir(path.dirname(manifest))
    cache.git(repo, "init", "-q")

    local function commit(message)
      cache.git(repo, "add", "-A")
      cache.git(repo, "-c", "user.email=t@t", "-c", "user.name=t", "commit", "-q", "-m", message)
    end

    fs.write(manifest, json.encode({ name = "p", version = "1.0.0" }))
    commit("first")
    assert(guard.version_drift(repo, "p") == "ok", "nothing has moved since the bump")

    fs.write(path.join(repo, "plugins", "p", "README.md"), "content\n")
    commit("content after the bump")
    assert(guard.version_drift(repo, "p") == "stale")
  end,

  a_plugin_with_no_bump_at_all_reads_as_unknown = function()
    local repo = fs.tempdir()
    cache.git(repo, "init", "-q")
    assert(guard.version_drift(repo, "absent") == "unknown")
  end,
}
