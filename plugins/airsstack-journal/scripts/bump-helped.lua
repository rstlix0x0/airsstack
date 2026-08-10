-- Increment the helped: counter of one journal note, then rebuild the index so ranking reflects
-- the new value.
--
-- Usage: bump-helped.lua <stem>
-- Resolves <stem>.md case-insensitively across notes/ sessions/ daily/ mocs/. Leaves `updated`
-- alone (a usage bump is not a content edit). Missing stem or non-integer helped: goes to stderr
-- and exits non-zero — this is a deliberate user action, so surfacing the error beats failing
-- silent.
--
--   airsl run --policy confined \
--     --allow-env AIRSSTACK_HOME --allow-env HOME \
--     --allow-read "$AIRSSTACK_HOME" --allow-write "$AIRSSTACK_HOME" \
--     scripts/bump-helped.lua <stem>

local index = require("lib.index")
local notes = require("lib.notes")
local vault = require("lib.vault")

local stem = arg[1]
if not stem or stem == "" then
  vault.die("bump-helped: usage: bump-helped.lua <stem>")
end

local root = vault.root()
local file, rel = notes.find(root, stem)
if not file then
  vault.die(
    "bump-helped: no note matching " .. stem .. ".md under notes/ sessions/ daily/ mocs/"
  )
end

local bumped, reason = notes.bump_helped(file)
if not bumped then
  vault.die("bump-helped: " .. reason)
end

index.rebuild(root, function(line)
  airsstack.stdio.error(line .. "\n")
end)

airsstack.stdio.write("bumped helped to " .. bumped .. " in " .. rel .. "\n")
