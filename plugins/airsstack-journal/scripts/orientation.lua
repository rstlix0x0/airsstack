-- Print a project-scoped recent-activity orientation card from summaries.tsv.
--
-- Usage: orientation.lua [project]   (project defaults to the repository basename)
-- Fail-open: any failure (no tsv, no match, empty vault) prints nothing and exits 0, so it never
-- blocks a session.
--
--   airsl run --policy confined \
--     --allow-env AIRSSTACK_HOME --allow-env HOME \
--     --allow-read "$AIRSSTACK_HOME" --allow-read . --allow-exec git \
--     scripts/orientation.lua [project]

local orientation = require("lib.orientation")
local vault = require("lib.vault")

local project = arg[1]
if not project or project == "" then
  local ok, resolved = pcall(vault.project_base)
  if not ok or resolved == "" then
    return
  end
  project = resolved
end

local ok, card = pcall(orientation.card, vault.root(), project)
if ok and card ~= "" then
  airsstack.stdio.write(card)
end
