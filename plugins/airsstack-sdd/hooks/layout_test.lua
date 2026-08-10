-- Tests for lib/layout — the two-root SDD tree and the stable per-repo key.
--
--   airsl test --allow-read /tmp --allow-write /tmp --allow-exec git plugins/airsstack-sdd/hooks

local layout = require("lib.layout")
local fs = airsstack.fs
local path = airsstack.path

local function repo()
  local dir = fs.tempdir()
  layout.git(dir, "init", "-q")
  layout.git(dir, "-c", "user.email=t@t", "-c", "user.name=t", "commit", "-q",
    "--allow-empty", "-m", "init")
  return dir
end

local function is_dir(...)
  local target = path.join(...)
  return fs.exists(target) and fs.is_dir(target)
end

return {
  rfcs_stay_worktree_local_and_specs_go_home_global = function()
    local dir, home = repo(), fs.tempdir()
    local _, key = layout.provision(dir, home)

    assert(is_dir(dir, layout.RFC_LOCAL_ROOT, "rfcs"), "rfcs/ must be worktree-local")
    assert(is_dir(layout.home_root(home, key), "specs"), "specs/ must be HOME-global")
    assert(is_dir(layout.home_root(home, key), "plans", "_archive"), "plans/_archive must exist")
    assert(not is_dir(dir, layout.RFC_LOCAL_ROOT, "specs"), "specs/ must NOT be worktree-local")
  end,

  the_first_run_reports_what_it_created_and_the_second_reports_nothing = function()
    local dir, home = repo(), fs.tempdir()
    assert(#layout.provision(dir, home) > 0, "the first run creates the tree")
    assert(#layout.provision(dir, home) == 0, "the second run is a no-op")
  end,

  a_linked_worktree_resolves_to_the_same_key = function()
    local dir, home = repo(), fs.tempdir()
    local linked = path.join(fs.tempdir(), "wt")
    layout.git(dir, "worktree", "add", "-q", linked, "-b", "feature")

    local main = layout.project_key(dir)
    local other = layout.project_key(linked)
    assert(main == other, "worktrees must collapse: " .. main .. " vs " .. other)
  end,

  a_subdirectory_resolves_to_the_repository_key = function()
    local dir, home = repo(), fs.tempdir()
    local nested = path.join(dir, "crates", "sub")
    fs.mkdir(nested)
    assert(layout.project_key(nested) == layout.project_key(dir))
  end,

  without_git_the_key_comes_from_the_working_directory = function()
    local dir = fs.tempdir()
    local key = layout.project_key(dir)
    -- Compared by prefix rather than by pattern: a temp directory's name contains `-`, which is a
    -- quantifier to `string.find` and would match far more than intended.
    local want = layout.sanitize(path.basename(dir))
    assert(key:sub(1, #want + 1) == want .. "-", key .. " should begin with " .. want)
  end,

  two_projects_are_never_given_the_same_key = function()
    assert(layout.project_key(fs.tempdir()) ~= layout.project_key(fs.tempdir()))
  end,

  a_disallowed_character_is_replaced_in_the_readable_component = function()
    local parent = fs.tempdir()
    local dir = path.join(parent, "plain@test")
    fs.mkdir(dir)
    local key = layout.project_key(dir)
    assert(key:find("^plain%-test%-"), key)
  end,

  the_hash_distinguishes_names_that_sanitise_to_one_token = function()
    -- `plain@test` and `plain-test` both sanitise to `plain-test`; the digest is taken from the
    -- full path, so the keys still differ.
    local parent = fs.tempdir()
    local first = path.join(parent, "plain@test")
    local second = path.join(parent, "plain-test")
    fs.mkdir(first)
    fs.mkdir(second)
    assert(layout.project_key(first) ~= layout.project_key(second))
  end,

  a_missing_gitignore_is_created_with_the_ignore_line = function()
    local dir = fs.tempdir()
    assert(layout.ensure_gitignore(dir))
    assert(fs.read(path.join(dir, ".gitignore")) == layout.IGNORE_LINE .. "\n")
  end,

  an_existing_gitignore_keeps_its_content_and_gains_the_line = function()
    local dir = fs.tempdir()
    fs.write(path.join(dir, ".gitignore"), "target/\n")
    layout.ensure_gitignore(dir)

    local lines = fs.read_lines(path.join(dir, ".gitignore"))
    assert(lines[1] == "target/", "existing content must survive")
    assert(lines[2] == layout.IGNORE_LINE, "the ignore line must be appended")
  end,

  the_ignore_line_is_never_added_twice = function()
    local dir = fs.tempdir()
    layout.ensure_gitignore(dir)
    assert(layout.ensure_gitignore(dir) == nil, "a second call reports no change")

    local found = 0
    for _, line in ipairs(fs.read_lines(path.join(dir, ".gitignore"))) do
      if line == layout.IGNORE_LINE then
        found = found + 1
      end
    end
    assert(found == 1, "expected one ignore line, found " .. found)
  end,

  a_negation_containing_the_text_does_not_count_as_the_line = function()
    -- `!.airsstack/keep` contains the text without ignoring the tree, so a substring test would
    -- leave the worktree-local root committed.
    local dir = fs.tempdir()
    fs.write(path.join(dir, ".gitignore"), "!.airsstack/keep\n")
    assert(layout.ensure_gitignore(dir), "the ignore line is still missing and must be added")
  end,
}
