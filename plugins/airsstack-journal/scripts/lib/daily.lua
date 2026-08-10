-- Linking a note stem into a day's daily structure note.
--
-- Separate from the `daily-link.lua` driver so the behaviour can be exercised against a temporary
-- vault by `airsl test` without spawning a process. The driver supplies the vault root and the
-- timestamp; nothing here reads the environment or the clock, which is what makes the output
-- assertable byte for byte.

local vault = require("lib.vault")
local fs = airsstack.fs
local path = airsstack.path

local M = {}

-- The daily note a fresh `date` starts as.
function M.template(date, now)
  return table.concat({
    "---",
    "title: " .. date,
    "type: daily",
    "created: " .. now,
    "updated: " .. now,
    "helped: 0",
    "---",
    "",
    "## Notes",
    "",
  }, "\n")
end

-- Links `stem` into `root`'s daily note for `date`, creating the note when absent.
--
-- Returns true when the link was added, false when it was already there. Idempotence is the whole
-- contract: the caller is a hook that fires again on every capture of the same note.
function M.link(root, date, stem, now)
  local daily = path.join(root, "daily")
  local file = path.join(daily, date .. ".md")

  fs.mkdir(daily)
  if not vault.exists(file) then
    fs.write(file, M.template(date, now))
  end

  local link = "[[" .. stem .. "]]"
  local lines = fs.read_lines(file)
  for _, line in ipairs(lines) do
    if line:find(link, 1, true) then
      return false
    end
  end

  lines[#lines + 1] = "- " .. link

  local bumped = false
  for index, line in ipairs(lines) do
    if not bumped and line:sub(1, 8) == "updated:" then
      lines[index] = "updated: " .. now
      bumped = true
    end
  end

  -- Atomic: a reader sees either the note without the link or the note with it, never a truncated
  -- file. The shell original reached the same property through mktemp plus mv.
  fs.atomic_write(file, table.concat(lines, "\n") .. "\n")
  return true
end

return M
