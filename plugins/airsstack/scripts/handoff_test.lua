-- Tests for lib/handoff — session minting, the liveness lease, and pruning.
--
--   airsl test --allow-read /tmp --allow-write /tmp --allow-exec git plugins/airsstack/scripts

local handoff = require("lib.handoff")
local fs = airsstack.fs
local path = airsstack.path

-- Real time, not a fixed epoch: the lease files these tests create carry a real modification
-- time, and judging them against a constant would make every lease look ancient.
local NOW = airsstack.time.now()

-- A handoff base holding `names`, each with an optional lease age in minutes.
local function base_with(names)
  local base = fs.tempdir()
  for name, lease_age in pairs(names) do
    fs.mkdir(path.join(base, name))
    if lease_age then
      fs.write(path.join(base, name, handoff.LEASE), "")
    end
  end
  return base
end

return {
  a_session_id_carries_a_sortable_timestamp_and_a_suffix = function()
    local id = handoff.session_id("20260101-120000")
    assert(id:sub(1, 15) == "20260101-120000", id)
    assert(id:match("^%d+%-%d+%-%x%x%x%x$"), id)
  end,

  two_ids_from_one_timestamp_differ = function()
    -- The suffix only has to separate two sessions minted in the same second.
    local seen = {}
    for _ = 1, 32 do
      seen[handoff.session_id("20260101-120000")] = true
    end
    local distinct = 0
    for _ in pairs(seen) do
      distinct = distinct + 1
    end
    assert(distinct > 1, "the suffix must vary")
  end,

  init_mints_a_directory_with_a_live_lease = function()
    local project = fs.tempdir()
    local directory, id = handoff.init(project, "20260101-120000", 10, NOW, 120)
    assert(fs.is_dir(directory), directory)
    assert(fs.exists(path.join(directory, handoff.LEASE)), "a new session holds its lease")
    assert(directory:sub(-#id) == id, directory .. " should end in " .. id)
  end,

  init_ignores_the_handoff_tree_in_git = function()
    local project = fs.tempdir()
    handoff.init(project, "20260101-120000", 10, NOW, 120)
    local lines = fs.read_lines(path.join(project, ".gitignore"))
    assert(lines[1] == handoff.IGNORE_LINE, tostring(lines[1]))
  end,

  the_ignore_line_is_never_added_twice = function()
    local project = fs.tempdir()
    handoff.init(project, "20260101-120000", 10, NOW, 120)
    handoff.init(project, "20260101-120001", 10, NOW, 120)

    local found = 0
    for _, line in ipairs(fs.read_lines(path.join(project, ".gitignore"))) do
      if line == handoff.IGNORE_LINE then
        found = found + 1
      end
    end
    assert(found == 1, "expected one ignore line, found " .. found)
  end,

  an_existing_gitignore_keeps_its_content = function()
    local project = fs.tempdir()
    fs.write(path.join(project, ".gitignore"), "target/\n")
    handoff.init(project, "20260101-120000", 10, NOW, 120)
    local lines = fs.read_lines(path.join(project, ".gitignore"))
    assert(lines[1] == "target/" and lines[2] == handoff.IGNORE_LINE)
  end,

  sessions_are_listed_oldest_first = function()
    local base = base_with({ ["20260103-000000-aa"] = false, ["20260101-000000-bb"] = false })
    local names = handoff.sessions(base)
    assert(names[1] == "20260101-000000-bb", names[1])
  end,

  a_loose_file_is_not_a_session = function()
    local base = base_with({ ["20260101-000000-aa"] = false })
    fs.write(path.join(base, "notes.txt"), "x")
    assert(#handoff.sessions(base) == 1)
  end,

  pruning_removes_only_the_oldest_beyond_the_keep_count = function()
    local base = base_with({
      ["20260101-000000-a"] = false,
      ["20260102-000000-b"] = false,
      ["20260103-000000-c"] = false,
      ["20260104-000000-d"] = false,
    })
    local removed = handoff.prune(base, 2, NOW, 120)
    assert(#removed == 2, "expected two pruned, got " .. #removed)
    assert(removed[1] == "20260101-000000-a", removed[1])
    assert(#handoff.sessions(base) == 2)
  end,

  pruning_below_the_keep_count_removes_nothing = function()
    local base = base_with({ ["20260101-000000-a"] = false })
    assert(#handoff.prune(base, 10, NOW, 120) == 0)
  end,

  a_live_lease_protects_its_session_from_pruning = function()
    local base = base_with({
      ["20260101-000000-a"] = true,
      ["20260102-000000-b"] = false,
      ["20260103-000000-c"] = false,
    })
    local removed = handoff.prune(base, 1, NOW, 120)
    assert(#removed == 1 and removed[1] == "20260102-000000-b", table.concat(removed, ","))
    assert(fs.is_dir(path.join(base, "20260101-000000-a")), "the leased session must survive")
  end,

  a_lease_older_than_the_grace_window_no_longer_protects = function()
    local base = base_with({ ["20260101-000000-a"] = true, ["20260102-000000-b"] = false })
    -- The lease was written now; judging it from beyond the grace window makes it stale.
    local removed = handoff.prune(base, 1, NOW + 121 * 60, 120)
    assert(#removed == 1 and removed[1] == "20260101-000000-a", table.concat(removed, ","))
  end,

  a_heartbeat_refreshes_a_lease_and_reports_a_missing_session = function()
    local project = fs.tempdir()
    local directory = handoff.init(project, "20260101-120000", 10, NOW, 120)
    assert(handoff.beat(directory) == true)
    assert(handoff.beat(path.join(project, "no-such-session")) == false)
  end,

  closing_drops_the_lease_and_leaves_the_session = function()
    local project = fs.tempdir()
    local directory = handoff.init(project, "20260101-120000", 10, NOW, 120)
    handoff.close(directory)
    assert(not fs.exists(path.join(directory, handoff.LEASE)), "the lease is gone")
    assert(fs.is_dir(directory), "the session's contents are not")
  end,

  closing_an_already_closed_session_is_not_an_error = function()
    local project = fs.tempdir()
    local directory = handoff.init(project, "20260101-120000", 10, NOW, 120)
    handoff.close(directory)
    handoff.close(directory)
  end,
}
