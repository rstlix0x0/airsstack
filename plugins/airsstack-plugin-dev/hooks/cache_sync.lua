-- airsstack-plugin-dev — PostToolUse cache-sync hook.
--
-- Mirrors an edited plugins/<plugin>/<rel> file into that plugin's install cache
-- (~/.claude/plugins/cache/<marketplace>/<plugin>/<version>/<rel>) so plugin development at a
-- fixed version needs no manual cp and no reinstall.
--
-- Only plugins installed from the `airsstack` marketplace are touched. Every failure mode no-ops;
-- the hook never blocks the triggering tool.
--
--   airsl run --fail-open --policy confined \
--     --allow-env HOME --allow-env AIRSSTACK_PLUGIN_DEV_DEBUG \
--     --allow-read / --allow-write "$HOME/.claude/plugins/cache" \
--     hooks/cache_sync.lua

local cache = require("lib.cache")
local path = airsstack.path

local home = airsstack.env.get("HOME") or ""
local debugging = airsstack.env.get("AIRSSTACK_PLUGIN_DEV_DEBUG")

local function note(message)
  if debugging and debugging ~= "" then
    airsstack.stdio.error("[cache-sync] " .. message .. "\n")
  end
end

local ok, payload = pcall(airsstack.hook.payload)
if not ok or type(payload) ~= "table" then
  return
end

local tool_input = type(payload.tool_input) == "table" and payload.tool_input or {}
local source = tool_input.file_path
if type(source) ~= "string" or source == "" then
  return
end

source = path.absolute(source)
if not cache.is_file(source) then
  return
end

local plugin, rel = cache.extract_plugin_rel(source)
if not plugin then
  return
end

local registry = cache.load_registry(cache.registry_path(home))
local root = cache.cache_root(home)

for _, install_path in ipairs(cache.install_paths(registry, plugin)) do
  local dest, reason = cache.sync_one(source, rel, install_path, root)
  if dest then
    note("synced: " .. source .. " -> " .. dest)
  else
    note("skip (" .. reason .. "): " .. path.join(install_path, rel))
  end
end
