-- Mirroring plugin source into the install cache: the registry, the containment rule, and the
-- add-and-update-only sync.
--
-- Shared by both hooks. The PostToolUse hook mirrors one edited file; the SessionStart guard
-- backfills whole trees. Keeping the registry lookup and the containment check here is what stops
-- the two drifting apart on which marketplace or which records count — the guard delegating to the
-- sync hook's resolver was the original arrangement, for the same reason.

local fs = airsstack.fs
local json = airsstack.json
local path = airsstack.path
local proc = airsstack.proc

local M = {}

M.MARKETPLACE = "airsstack"

-- Names neither the mirror nor the extras report ever considers. Claude Code writes `.in_use` into
-- every cache directory, and without this the extras report is never empty and you learn to skim
-- past it.
M.IGNORED_NAMES = { [".in_use"] = true, [".DS_Store"] = true, [".git"] = true }

-- The install cache root every write is bounded by.
function M.cache_root(home)
  return path.join(home, ".claude", "plugins", "cache")
end

-- Where the installed-plugins registry lives.
function M.registry_path(home)
  return path.join(home, ".claude", "plugins", "installed_plugins.json")
end

-- Runs git in `dir`; returns trimmed stdout, or nil on any failure.
function M.git(dir, ...)
  local argv = { "git", "-C", dir }
  for _, value in ipairs({ ... }) do
    argv[#argv + 1] = value
  end
  local ok, result = pcall(proc.run, argv)
  if not ok or result.status ~= 0 then
    return nil
  end
  local text = result.stdout:gsub("%s+$", "")
  return text ~= "" and text or nil
end

-- Whether `target` exists, answering false where the question itself is refused.
function M.exists(target)
  local ok, found = pcall(fs.exists, target)
  return ok and found
end

-- Whether `target` is a file.
function M.is_file(target)
  local ok, found = pcall(fs.is_file, target)
  return ok and found
end

-- Whether `target` is a directory.
function M.is_dir(target)
  local ok, found = pcall(fs.is_dir, target)
  return ok and found
end

-- The decoded registry, or an empty table.
function M.load_registry(file)
  local ok, text = pcall(fs.read, file)
  if not ok then
    return {}
  end
  local decoded, data = pcall(json.decode, text)
  if not decoded or type(data) ~= "table" then
    return {}
  end
  return data
end

-- `(plugin, rel)` for a path under `plugins/<plugin>/`, else nil.
--
-- `rel` is the remainder after `plugins/<plugin>/`. A path naming a plugin directory with no file
-- remainder is not a mirrorable edit.
function M.extract_plugin_rel(target)
  local parts = {}
  for part in target:gmatch("[^/]+") do
    parts[#parts + 1] = part
  end

  for index, part in ipairs(parts) do
    if part == "plugins" and index + 2 <= #parts then
      local plugin = parts[index + 1]
      local rel = {}
      for tail = index + 2, #parts do
        rel[#rel + 1] = parts[tail]
      end
      if plugin ~= "" and #rel > 0 then
        return plugin, table.concat(rel, "/")
      end
      return nil
    end
  end
  return nil
end

-- Distinct install paths for `<plugin>@airsstack`, in first-seen order.
--
-- The `@airsstack` suffix is the marketplace gate: plugins from any other marketplace are never
-- selected. Distinct values only — several registry records commonly point at the same cache
-- directory, and mirroring once per record would multiply every reported line.
function M.install_paths(registry, plugin)
  local key = plugin .. "@" .. M.MARKETPLACE
  local plugins = type(registry.plugins) == "table" and registry.plugins or {}
  local entries = type(plugins[key]) == "table" and plugins[key] or {}

  local seen, found = {}, {}
  for _, entry in ipairs(entries) do
    if type(entry) == "table" and type(entry.installPath) == "string" and entry.installPath ~= "" then
      if not seen[entry.installPath] then
        seen[entry.installPath] = true
        found[#found + 1] = entry.installPath
      end
    end
  end
  return found
end

-- Whether `child` is `parent` or nested under it.
--
-- Compared segment by segment rather than as strings: `/cache-extra` begins with `/cache` as text
-- and is not inside it, and accepting that is the classic way a containment check turns out never
-- to have contained anything.
function M.is_within(child, parent)
  local function segments(target)
    local out = {}
    for part in path.normalize(target):gmatch("[^/]+") do
      out[#out + 1] = part
    end
    return out
  end

  local outer, inner = segments(parent), segments(child)
  if #inner < #outer then
    return false
  end
  for index = 1, #outer do
    if inner[index] ~= outer[index] then
      return false
    end
  end
  return true
end

-- Root-relative file paths under `root`, ignore-list applied, sorted.
--
-- Returns nil when the tree cannot be walked. That is distinct from an empty tree on purpose: an
-- unreadable source tree reading back as empty is how a delivery failure reports agreement.
function M.relative_files(root)
  local ok, entries = pcall(fs.walk, root)
  if not ok then
    return nil
  end

  local found = {}
  for _, rel in ipairs(entries) do
    local skip = false
    for component in rel:gmatch("[^/]+") do
      skip = skip or M.IGNORED_NAMES[component] == true
    end
    if not skip and M.is_file(path.join(root, rel)) then
      found[#found + 1] = rel
    end
  end
  table.sort(found)
  return found
end

-- Copies `source` to `<install_path>/<rel>` when the destination is inside `containment_root`.
--
-- Returns the destination on success, nil when containment refused it or the copy failed.
function M.sync_one(source, rel, install_path, containment_root)
  local dest = path.normalize(path.join(install_path, rel))
  if not M.is_within(dest, containment_root) then
    return nil, "outside the cache root"
  end
  if not pcall(fs.mkdir, path.dirname(dest)) then
    return nil, "cannot create the destination directory"
  end
  if not pcall(fs.copy, source, dest) then
    return nil, "copy failed"
  end
  return dest
end

-- Add-and-update-only mirror of `src_dir` into `cache_dir`.
--
-- Returns `copied` and `extras`. Extras are cache-only files; they are REPORTED, never deleted. An
-- unreferenced leftover is inert, whereas an unattended hook holding delete authority over Claude
-- Code's data directory is a larger risk than the cruft it would clean.
--
-- `containment_root` bounds every write. It is a parameter rather than a switch so that tests
-- re-point the boundary instead of disabling it: this guard is the only bound on an unattended
-- hook's writes, so it must stay live in the paths the tests exercise.
function M.sync_tree(src_dir, cache_dir, containment_root)
  local source_files = M.relative_files(src_dir) or {}
  local copied = {}

  for _, rel in ipairs(source_files) do
    local source = path.join(src_dir, rel)
    local dest = path.normalize(path.join(cache_dir, rel))
    if M.is_within(dest, containment_root) then
      local same = M.exists(dest) and select(2, pcall(fs.same_content, source, dest)) == true
      if not same and M.sync_one(source, rel, cache_dir, containment_root) then
        copied[#copied + 1] = rel
      end
    end
  end

  local known = {}
  for _, rel in ipairs(source_files) do
    known[rel] = true
  end

  local extras = {}
  for _, rel in ipairs(M.relative_files(cache_dir) or {}) do
    if not known[rel] then
      extras[#extras + 1] = rel
    end
  end

  return copied, extras
end

return M
