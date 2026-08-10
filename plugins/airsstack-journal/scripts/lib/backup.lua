-- Snapshotting the journal vault before a curator run, and pruning old snapshots.
--
-- Split from the driver so retention can be exercised without creating archives: the prune rule is
-- the part that deletes, and it is worth testing against a directory of empty files rather than
-- against ten real tarballs.

local vault = require("lib.vault")
local fs = airsstack.fs
local path = airsstack.path

local M = {}

M.DEFAULT_KEEP = 10
M.CONTENT_DIRS = { "daily", "sessions", "notes", "mocs" }

-- The content directories that actually exist under `root`.
--
-- `.index/` is derived and `.backups/` would make the archive recursive, so neither is a member.
function M.content_dirs(root)
  local present = {}
  for _, name in ipairs(M.CONTENT_DIRS) do
    local directory = path.join(root, name)
    if vault.exists(directory) and fs.is_dir(directory) then
      present[#present + 1] = name
    end
  end
  return present
end

-- The archive names under `backups`, oldest first.
--
-- The names carry a sortable timestamp, so lexical order is chronological and no `stat` is needed
-- to order them.
function M.archives(backups)
  local found = {}
  if not vault.exists(backups) then
    return found
  end
  for _, name in ipairs(fs.list(backups)) do
    if name:sub(-7) == ".tar.gz" then
      found[#found + 1] = name
    end
  end
  table.sort(found)
  return found
end

-- Removes all but the newest `keep` archives. Returns the names removed.
function M.prune(backups, keep)
  local names = M.archives(backups)
  local excess = #names - keep
  local removed = {}
  for index = 1, math.max(excess, 0) do
    local name = names[index]
    local ok = pcall(fs.remove, path.join(backups, name))
    if ok then
      removed[#removed + 1] = name
    end
  end
  return removed
end

-- Writes `<root>/.backups/<stamp>.tar.gz` covering the vault's content directories.
--
-- Returns the archive path, or nil plus a reason. `-C <root>` roots the archive at the vault so
-- member paths read `daily/...` rather than an absolute path that could not be restored elsewhere.
function M.create(root, stamp, keep)
  local dirs = M.content_dirs(root)
  if #dirs == 0 then
    return nil, nil -- an empty vault is a no-op rather than a failure
  end

  local backups = path.join(root, ".backups")
  local ok = pcall(fs.mkdir, backups)
  if not ok then
    return nil, "cannot create " .. backups
  end

  local archive = path.join(backups, stamp .. ".tar.gz")
  local argv = { "tar", "-czf", archive, "-C", root }
  for _, name in ipairs(dirs) do
    argv[#argv + 1] = name
  end

  local ran, result = pcall(airsstack.proc.run, argv)
  if not ran or result.status ~= 0 then
    pcall(fs.remove, archive)
    return nil, "tar failed"
  end

  M.prune(backups, keep)
  return archive
end

return M
