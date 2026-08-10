-- Provision the SDD artifact tree. Idempotent: creates only what is missing, and never duplicates
-- the .gitignore line.
--
-- Two roots, split by artifact type — rfcs/ worktree-local, specs/ and plans/ HOME-global under a
-- stable per-repo key. `references/artifact-paths.md` is the prose mirror; the two MUST agree.
--
--   airsl run --fail-open --policy confined \
--     --allow-env AIRSSTACK_HOME --allow-env HOME \
--     --allow-read . --allow-write . \
--     --allow-read "$HOME_ROOT" --allow-write "$HOME_ROOT" \
--     --allow-exec git \
--     hooks/ensure-layout.lua

local layout = require("lib.layout")
local path = airsstack.path

local home = airsstack.env.get("AIRSSTACK_HOME")
if not home or home == "" then
  home = path.join(airsstack.env.get("HOME") or "", ".airsstack")
end

local created = layout.provision(path.absolute("."), home)

if #created == 0 then
  airsstack.stdio.write("airsstack-sdd layout already present; nothing to do.\n")
  return
end

local lines = { "airsstack-sdd layout provisioned:" }
for _, entry in ipairs(created) do
  lines[#lines + 1] = "  " .. entry
end
airsstack.stdio.write(table.concat(lines, "\n") .. "\n")
