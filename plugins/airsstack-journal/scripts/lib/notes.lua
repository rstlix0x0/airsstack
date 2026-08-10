-- Locating a note by stem and maintaining its `helped:` counter.
--
-- The `helped` counter is what ranks recall results, so a bump that silently missed the note would
-- degrade every later search quietly. Every failure here is therefore reported rather than
-- swallowed — unlike the session-start path, where silence is the right answer.

local vault = require("lib.vault")
local fs = airsstack.fs
local path = airsstack.path

local M = {}

-- The directories a stem is resolved against, in precedence order.
M.SEARCH_DIRS = { "notes", "sessions", "daily", "mocs" }

-- The note file whose stem matches `stem`, case-insensitively, or nil.
--
-- Case-insensitive because the stem usually arrives from a person typing it back out of a recall
-- result, and `Tokio-Cancellation-Safety` naming the same note as `tokio-cancellation-safety` is
-- what they will expect.
function M.find(root, stem)
  local want = stem:lower()
  for _, sub in ipairs(M.SEARCH_DIRS) do
    local directory = path.join(root, sub)
    if vault.exists(directory) and fs.is_dir(directory) then
      for _, name in ipairs(fs.list(directory)) do
        if name:sub(-3) == ".md" and path.stem(name):lower() == want then
          return path.join(directory, name), sub .. "/" .. name
        end
      end
    end
  end
  return nil
end

-- Increments the note's `helped:` counter and returns the new value.
--
-- `updated` is deliberately left alone: a usage bump records that the note was useful, not that
-- its content changed, and moving `updated` would push it up the recent-activity card for no
-- reason a reader could see.
function M.bump_helped(file)
  local lines = fs.read_lines(file)

  local target, current
  for number, line in ipairs(lines) do
    local value = line:match("^helped:%s*(.*)$")
    if value then
      target, current = number, value:match("^%s*(.-)%s*$")
      break
    end
  end

  if not target or not current:match("^%d+$") then
    return nil, file .. " has no integer helped:"
  end

  local bumped = tonumber(current) + 1
  lines[target] = "helped: " .. bumped
  fs.atomic_write(file, table.concat(lines, "\n") .. "\n")
  return bumped
end

return M
