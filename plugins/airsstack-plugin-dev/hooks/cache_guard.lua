-- airsstack-plugin-dev — SessionStart cache delivery guard.
--
-- On session start, in the MAIN worktree of the plugin source repo only, copies every source file
-- that is missing or differing in the cache, reports cache-only extras without deleting them, and
-- reports version drift. Fail-open throughout; it never blocks a session.
--
--   airsl run --fail-open --policy confined \
--     --allow-env HOME \
--     --allow-read / --allow-write "$HOME/.claude/plugins/cache" \
--     --allow-exec git \
--     hooks/cache_guard.lua

local cache = require("lib.cache")
local guard = require("lib.guard")

local home = airsstack.env.get("HOME") or ""

local ok, payload = pcall(airsstack.hook.payload)
local cwd
if ok and type(payload) == "table" and type(payload.cwd) == "string" and payload.cwd ~= "" then
  cwd = payload.cwd
else
  cwd = airsstack.path.absolute(".")
end

local top = cache.git(cwd, "rev-parse", "--show-toplevel")
if not top or not guard.is_airsstack_marketplace(top) then
  return -- not the plugin source repo: nothing to guard
end

local active = guard.is_main_worktree(cwd)
local registry = cache.load_registry(cache.registry_path(home))
local lines = guard.format_report(active, guard.run(top, registry, active, cache.cache_root(home)))

if #lines > 0 then
  airsstack.stdio.write(table.concat(lines, "\n") .. "\n")
end
