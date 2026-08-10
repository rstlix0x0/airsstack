-- Idempotently link a note stem into a day's daily structure note.
--
-- Usage: daily-link.lua <YYYY-MM-DD> <stem>
-- Creates daily/<date>.md (frontmatter + "## Notes" list) when absent; appends "- [[<stem>]]"
-- only when the link is not already present; bumps `updated`. Honours AIRSSTACK_HOME.
--
--   airsl run --policy confined \
--     --allow-env AIRSSTACK_HOME --allow-env HOME \
--     --allow-read "$AIRSSTACK_HOME" --allow-write "$AIRSSTACK_HOME" \
--     scripts/daily-link.lua <YYYY-MM-DD> <stem>

local daily = require("lib.daily")
local vault = require("lib.vault")

local date = arg[1]
local stem = arg[2]
if not date or date == "" or not stem or stem == "" then
  vault.die("daily-link: usage: daily-link.lua <YYYY-MM-DD> <stem>")
end

-- Local time, not UTC: a daily note is named for the day the author is living in, and
-- `time.format` renders UTC only.
daily.link(vault.root(), date, stem, os.date("%Y-%m-%d %H:%M"))
