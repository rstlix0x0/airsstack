-- The SessionStart cache delivery guard: worktree detection, version drift, and the report.
--
-- The PostToolUse hook only mirrors files it observes being edited, so it never backfills a commit,
-- a branch switch, or an edit that predates its own installation. That blind spot is how
-- `enforcement.json` reached the repo but no install cache, leaving the enforcement framework
-- silently dead. This closes it.

local cache = require("lib.cache")
local json = airsstack.json
local path = airsstack.path

local M = {}

M.RESTART_NOTE = "NOTE: enforcement.json is read at hook-fire time and takes effect now, but "
  .. "hooks.json and commands/ are read at plugin-load time — restart the session "
  .. "to pick up anything backfilled just now."

M.PUSH_NOTE = "NOTE: user-scope installs pull the marketplace from GitHub, so a version bump "
  .. "reaches them only after the commit is pushed to main. Neither the backfill nor "
  .. "this check substitutes for that."

-- True only in the repository's main working tree.
--
-- `rev-parse --show-toplevel` succeeds from every linked worktree, so it cannot be the gate on its
-- own: four checkouts at three commits share one version-keyed cache, and with add-and-update-only
-- semantics the cache would converge on a union of branches. Comparing the common git dir against
-- the toplevel's own `.git` answers "which branch is authoritative".
function M.is_main_worktree(cwd)
  local top = cache.git(cwd, "rev-parse", "--show-toplevel")
  local common = cache.git(cwd, "rev-parse", "--git-common-dir")
  if not top or not common then
    return false
  end
  -- `--git-common-dir` comes back cwd-relative in the main tree (`.git`, `../../.git` from a
  -- subdirectory) and absolute in a linked one, so the relative form is joined onto `cwd` first.
  if not path.is_absolute(common) then
    common = path.join(cwd, common)
  end

  local function resolved(target)
    local ok, found = pcall(airsstack.fs.canonicalize, target)
    return ok and found or target
  end

  return resolved(common) == resolved(path.join(top, ".git"))
end

-- Whether `top` hosts the airsstack marketplace manifest.
function M.is_airsstack_marketplace(top)
  local ok, text = pcall(airsstack.fs.read,
    path.join(top, ".claude-plugin", "marketplace.json"))
  if not ok then
    return false
  end
  local decoded, data = pcall(json.decode, text)
  return decoded and type(data) == "table" and data.name == cache.MARKETPLACE
end

-- Names of every plugin directory in the source repo, sorted.
function M.source_plugins(top)
  local root = path.join(top, "plugins")
  local ok, names = pcall(airsstack.fs.list, root)
  if not ok then
    return {}
  end
  local found = {}
  for _, name in ipairs(names) do
    if cache.is_file(path.join(root, name, ".claude-plugin", "plugin.json")) then
      found[#found + 1] = name
    end
  end
  table.sort(found)
  return found
end

-- The `version` field of `rel_path` at `rev`, or nil.
local function version_at(top, rev, rel_path)
  local raw = cache.git(top, "show", rev .. ":" .. rel_path)
  if not raw then
    return nil
  end
  local ok, data = pcall(json.decode, raw)
  if not ok or type(data) ~= "table" then
    return nil
  end
  return data.version
end

-- The newest commit where plugin.json's `version` VALUE changed.
--
-- Not "the last commit touching plugin.json". That naive rule reported 5 of 7 plugins stale on its
-- first run — content newer than the last bump is the normal state of a plugin under development —
-- and had three proven false negatives: a manifest edited without changing the version, a squash
-- merge collapsing bump and content into one commit (this repo's entire history), and uncommitted
-- work invisible to `git log`.
function M.last_bump_commit(top, plugin)
  local rel = path.join("plugins", plugin, ".claude-plugin", "plugin.json")
  local log = cache.git(top, "log", "--format=%H", "--", rel)
  if not log then
    return nil
  end
  for commit in (log .. "\n"):gmatch("([^\n]+)\n") do
    if version_at(top, commit, rel) ~= version_at(top, commit .. "^", rel) then
      return commit
    end
  end
  return nil
end

-- "ok", "stale" or "unknown" for one plugin's committed content.
function M.version_drift(top, plugin)
  local bump = M.last_bump_commit(top, plugin)
  if not bump then
    return "unknown"
  end
  local newer = cache.git(top, "rev-list", bump .. "..HEAD", "--", path.join("plugins", plugin))
  return newer and "stale" or "ok"
end

-- Whether the working tree has changes under `plugins/<plugin>/`.
function M.has_uncommitted(top, plugin)
  return cache.git(top, "status", "--porcelain", "--", path.join("plugins", plugin)) ~= nil
end

-- `a, b, c` with a `(+N more)` tail once the list runs past `limit`.
function M.listing(names, limit)
  limit = limit or 5
  local shown = {}
  for index = 1, math.min(#names, limit) do
    shown[index] = names[index]
  end
  local text = table.concat(shown, ", ")
  if #names <= limit then
    return text
  end
  return text .. " (+" .. (#names - limit) .. " more)"
end

-- The sorted, deduplicated form of a list.
local function unique(values)
  local seen, out = {}, {}
  for _, value in ipairs(values) do
    if not seen[value] then
      seen[value] = true
      out[#out + 1] = value
    end
  end
  table.sort(out)
  return out
end

-- Per-plugin backfill and drift results. `write = false` reports only.
function M.run(top, registry, write, containment_root)
  local results = {}
  for _, plugin in ipairs(M.source_plugins(top)) do
    local targets = cache.install_paths(registry, plugin)
    if #targets > 0 then
      local src_dir = path.join(top, "plugins", plugin)
      local copied, extras = {}, {}

      for _, cache_dir in ipairs(targets) do
        if write then
          local synced, found = cache.sync_tree(src_dir, cache_dir, containment_root)
          for _, rel in ipairs(synced) do
            copied[#copied + 1] = rel
          end
          for _, rel in ipairs(found) do
            extras[#extras + 1] = rel
          end
        else
          local known = {}
          for _, rel in ipairs(cache.relative_files(src_dir) or {}) do
            known[rel] = true
          end
          for _, rel in ipairs(cache.relative_files(cache_dir) or {}) do
            if not known[rel] then
              extras[#extras + 1] = rel
            end
          end
        end
      end

      results[#results + 1] = {
        plugin = plugin,
        copied = unique(copied),
        extras = unique(extras),
        drift = M.version_drift(top, plugin),
        uncommitted = M.has_uncommitted(top, plugin),
      }
    end
  end
  return results
end

-- Report lines for SessionStart stdout; empty when there is nothing to say.
--
-- Backfills, extras and uncommitted edits are per-plugin: each names one thing to look at.
-- Staleness is not — it is a PUBLICATION reminder, and "content committed after the last version
-- bump" is the normal state of a plugin under active development. Locally the backfill has already
-- corrected the content; the only consumer of the stale signal is another machine pulling the
-- marketplace from GitHub, and that needs saying once. So the count is aggregated into a single
-- line rather than one line per plugin, which would print a wall on every start and train you to
-- skim past the whole report.
function M.format_report(active, results)
  local body, copied_any, stale = {}, false, 0

  for _, item in ipairs(results) do
    if #item.copied > 0 then
      copied_any = true
      body[#body + 1] = "  " .. item.plugin .. ": backfilled " .. #item.copied
        .. " file(s): " .. M.listing(item.copied)
    end
    if #item.extras > 0 then
      body[#body + 1] = "  " .. item.plugin .. ": " .. #item.extras
        .. " cache-only file(s), not deleted: " .. M.listing(item.extras)
    end
    if item.drift == "stale" then
      stale = stale + 1
    end
    if item.uncommitted then
      body[#body + 1] = "  " .. item.plugin .. ": uncommitted edits in the working tree"
    end
  end

  if stale > 0 then
    body[#body + 1] = "  " .. stale .. " of " .. #results .. " "
      .. (#results == 1 and "plugin" or "plugins")
      .. " stale — content committed after the last version bump"
  end

  if #body == 0 then
    return {}
  end

  local lines = {
    active and "airsstack cache guard:"
      or "airsstack cache guard (linked worktree — reporting only, nothing written):",
  }
  for _, line in ipairs(body) do
    lines[#lines + 1] = line
  end
  if copied_any then
    lines[#lines + 1] = M.RESTART_NOTE
  end
  if stale > 0 then
    lines[#lines + 1] = M.PUSH_NOTE
  end
  return lines
end

return M
