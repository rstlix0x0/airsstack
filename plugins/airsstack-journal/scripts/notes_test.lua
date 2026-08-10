-- Tests for lib/notes and lib/backup — stem resolution, the helped counter, and retention.
--
--   airsl test --allow-read /tmp --allow-write /tmp plugins/airsstack-journal/scripts

local backup = require("lib.backup")
local notes = require("lib.notes")
local fs = airsstack.fs
local path = airsstack.path

local function vault_with(files)
  local root = fs.tempdir()
  for _, dir in ipairs({ "notes", "sessions", "daily", "mocs" }) do
    fs.mkdir(path.join(root, dir))
  end
  for rel, text in pairs(files or {}) do
    fs.write(path.join(root, rel), text)
  end
  return root
end

local function note(helped)
  return "---\ntype: concept\nhelped: " .. helped .. "\nupdated: 2026-01-01 00:00\n---\nBody.\n"
end

return {
  a_stem_resolves_case_insensitively = function()
    local root = vault_with({ ["notes/tokio-cancellation.md"] = note(0) })
    local file, rel = notes.find(root, "TOKIO-Cancellation")
    assert(file, "the note should resolve whatever the case")
    assert(rel == "notes/tokio-cancellation.md", rel)
  end,

  an_absent_stem_resolves_to_nothing = function()
    assert(notes.find(vault_with({}), "nowhere") == nil)
  end,

  notes_are_searched_before_sessions = function()
    local root = vault_with({
      ["notes/same.md"] = note(0),
      ["sessions/same.md"] = note(0),
    })
    local _, rel = notes.find(root, "same")
    assert(rel == "notes/same.md", rel)
  end,

  bumping_increments_the_counter_and_leaves_updated_alone = function()
    local root = vault_with({ ["notes/a.md"] = note(3) })
    local file = notes.find(root, "a")
    assert(notes.bump_helped(file) == 4)

    local text = fs.read(file)
    assert(text:find("\nhelped: 4\n", 1, true), text)
    assert(text:find("\nupdated: 2026-01-01 00:00\n", 1, true), "updated must not move")
  end,

  a_non_integer_counter_is_refused_rather_than_coerced = function()
    local root = vault_with({
      ["notes/a.md"] = "---\ntype: concept\nhelped: many\n---\nBody.\n",
    })
    local bumped, reason = notes.bump_helped(notes.find(root, "a"))
    assert(bumped == nil, "a non-integer counter must not bump")
    assert(reason:find("integer helped", 1, true), reason)
  end,

  a_note_with_no_counter_at_all_is_refused = function()
    local root = vault_with({ ["notes/a.md"] = "---\ntype: concept\n---\nBody.\n" })
    local bumped = notes.bump_helped(notes.find(root, "a"))
    assert(bumped == nil)
  end,

  only_the_content_directories_that_exist_are_archived = function()
    local root = fs.tempdir()
    fs.mkdir(path.join(root, "notes"))
    fs.mkdir(path.join(root, ".index"))
    local dirs = backup.content_dirs(root)
    assert(#dirs == 1 and dirs[1] == "notes", table.concat(dirs, ","))
  end,

  an_empty_vault_is_a_no_op_rather_than_a_failure = function()
    local archive, reason = backup.create(fs.tempdir(), "2026-01-01-000000", 10)
    assert(archive == nil and reason == nil, "an empty vault must neither archive nor fail")
  end,

  retention_keeps_the_newest_and_prunes_the_rest = function()
    local backups = fs.tempdir()
    for _, stamp in ipairs({ "2026-01-01", "2026-01-02", "2026-01-03", "2026-01-04" }) do
      fs.write(path.join(backups, stamp .. ".tar.gz"), "x")
    end
    local removed = backup.prune(backups, 2)
    assert(#removed == 2, "expected two pruned, got " .. #removed)
    assert(removed[1] == "2026-01-01.tar.gz", removed[1])

    local left = backup.archives(backups)
    assert(#left == 2 and left[1] == "2026-01-03.tar.gz", table.concat(left, ","))
  end,

  retention_below_the_keep_count_removes_nothing = function()
    local backups = fs.tempdir()
    fs.write(path.join(backups, "2026-01-01.tar.gz"), "x")
    assert(#backup.prune(backups, 10) == 0)
  end,

  a_file_that_is_not_an_archive_is_never_pruned = function()
    local backups = fs.tempdir()
    fs.write(path.join(backups, "notes.md"), "x")
    fs.write(path.join(backups, "2026-01-01.tar.gz"), "x")
    backup.prune(backups, 0)
    assert(fs.exists(path.join(backups, "notes.md")), "only archives are retention candidates")
  end,
}
