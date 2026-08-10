-- The Context Handoff session tree: minting sessions, the liveness lease, and pruning.
--
-- Single source of truth (code side) for the handoff tree path, the session liveness lease, and
-- pruning. Prose mirror: `skills/process-guidelines/references/context-handoff.md`. The two MUST
-- agree — change one, change the other.
--
-- Split from the driver so pruning can be exercised against a directory of stub sessions. Pruning
-- is the part that deletes, and the lease rule that protects a live session from it is the part
-- most worth pinning.

local fs = airsstack.fs
local path = airsstack.path
local proc = airsstack.proc

local M = {}

M.HANDOFF_REL = ".airsstack/cc/plugins/airsstack/handoff"
M.DEFAULT_KEEP = 10

-- Minutes. A lease older than this no longer protects its session from pruning.
M.DEFAULT_GRACE_MINUTES = 120

M.IGNORE_LINE = ".airsstack/"
M.LEASE = ".active"

-- The worktree root, or the working directory when there is no repository.
function M.worktree_root(cwd)
  local ok, result = pcall(proc.run, { "git", "-C", cwd, "rev-parse", "--show-toplevel" })
  if ok and result.status == 0 then
    local text = result.stdout:gsub("%s+$", "")
    if text ~= "" then
      return text
    end
  end
  return cwd
end

-- The handoff tree for one worktree.
function M.base(cwd)
  return path.join(M.worktree_root(cwd), M.HANDOFF_REL)
end

-- Appends the ignore line to `<root>/.gitignore` unless it is already a whole line there.
function M.ensure_gitignore(root)
  local file = path.join(root, ".gitignore")
  if not fs.exists(file) then
    fs.write(file, M.IGNORE_LINE .. "\n")
    return
  end
  for _, line in ipairs(fs.read_lines(file)) do
    if line == M.IGNORE_LINE then
      return
    end
  end
  fs.append(file, M.IGNORE_LINE .. "\n")
end

-- A session id: a sortable timestamp plus a short random suffix.
--
-- The timestamp prefix is what makes lexical order chronological, which is what lets pruning order
-- sessions without stat-ing any of them. The suffix only has to separate two sessions minted in
-- the same second.
function M.session_id(stamp)
  local suffix = string.format("%02x%02x", math.random(0, 255), math.random(0, 255))
  return stamp .. "-" .. suffix
end

-- The session directory names under `base`, oldest first.
function M.sessions(base)
  local found = {}
  local ok, names = pcall(fs.list, base)
  if not ok then
    return found
  end
  for _, name in ipairs(names) do
    local target = path.join(base, name)
    local read, is_dir = pcall(fs.is_dir, target)
    if read and is_dir then
      found[#found + 1] = name
    end
  end
  table.sort(found)
  return found
end

-- Whether a session still holds a live lease.
--
-- A lease file that exists but has not been touched inside the grace window is treated as
-- abandoned: a session that ended without a clean close would otherwise pin its directory forever.
function M.leased(base, name, now, grace_minutes)
  local lease = path.join(base, name, M.LEASE)
  local ok, stamp = pcall(fs.stat, lease)
  if not ok then
    return false
  end
  return now - stamp.modified <= grace_minutes * 60
end

-- Removes the oldest sessions beyond `keep`, skipping any that still hold a live lease.
--
-- Returns the names removed. Only the oldest `#sessions - keep` are ever candidates, so a live
-- session near the front does not push the pruner further down the list than it should go.
function M.prune(base, keep, now, grace_minutes)
  local names = M.sessions(base)
  local candidates = #names - keep
  local removed = {}
  for index = 1, math.max(candidates, 0) do
    local name = names[index]
    if not M.leased(base, name, now, grace_minutes) then
      if pcall(fs.remove_dir, path.join(base, name)) then
        removed[#removed + 1] = name
      end
    end
  end
  return removed
end

-- Mints a session directory with a fresh lease, prunes, and returns the directory and its id.
function M.init(cwd, stamp, keep, now, grace_minutes)
  local root = M.worktree_root(cwd)
  local base = path.join(root, M.HANDOFF_REL)

  M.ensure_gitignore(root)

  local id = M.session_id(stamp)
  local directory = path.join(base, id)
  fs.mkdir(directory)
  M.touch(path.join(directory, M.LEASE))

  M.prune(base, keep, now, grace_minutes)
  return directory, id
end

-- Creates the file, or refreshes its modification time when it is already there.
--
-- A zero-byte write rather than `touch`: `fs` has no utimes, and rewriting an empty file moves the
-- modification time, which is the only thing the lease carries.
function M.touch(file)
  fs.write(file, "")
end

-- Refreshes a session's lease. Returns false when the directory is not there.
function M.beat(directory)
  local ok, is_dir = pcall(fs.is_dir, directory)
  if not ok or not is_dir then
    return false
  end
  M.touch(path.join(directory, M.LEASE))
  return true
end

-- Drops a session's lease, closing it cleanly. An absent lease is already closed.
function M.close(directory)
  pcall(fs.remove, path.join(directory, M.LEASE))
end

return M
