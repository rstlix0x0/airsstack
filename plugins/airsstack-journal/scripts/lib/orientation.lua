-- The project-scoped recent-activity card rendered from the derived summaries index.
--
-- Reads `.index/summaries.tsv` rather than the Markdown corpus: the card is built on every session
-- start, and re-parsing every note to render six lines is the one thing that would make it worth
-- skipping. Pure — no model, no clock, no writes.

local vault = require("lib.vault")

local M = {}

M.SESSION_LIMIT = 3
M.NOTE_LIMIT = 5

-- summaries.tsv column order, as `lib/index` writes it.
local STEM, TITLE, SUMMARY, PROJECT, HELPED, UPDATED = 1, 2, 3, 4, 5, 6

-- One TSV line split into its six cells; short rows keep empty strings rather than nils.
function M.cells(line)
  local out = {}
  for cell in (line .. "\t"):gmatch("([^\t]*)\t") do
    out[#out + 1] = cell
  end
  for index = 1, 6 do
    out[index] = out[index] or ""
  end
  return out
end

-- Whether `project` appears in a ", "-separated project column.
--
-- Membership rather than equality: a note's `project` field can carry several, and the shell
-- original split the same way.
function M.member(column, project)
  for candidate in (column .. ", "):gmatch("(.-), ") do
    if candidate == project then
      return true
    end
  end
  return false
end

-- Whether a stem names a session note.
function M.is_session(stem)
  return stem:sub(1, 8) == "session-"
end

-- The rows for one kind, newest first, capped at `limit`.
--
-- "Newest" is a descending sort on the whole `updated<TAB>stem<TAB>summary` line, which is what
-- `sort -r` did — the timestamps are ISO, so lexical order is chronological, and the stem breaks a
-- tie deterministically.
function M.select(lines, project, kind, limit)
  local rows = {}
  for _, line in ipairs(lines) do
    if line ~= "" then
      local cell = M.cells(line)
      if M.member(cell[PROJECT], project) then
        local session = M.is_session(cell[STEM])
        if (kind == "session") == session then
          rows[#rows + 1] = {
            key = cell[UPDATED] .. "\t" .. cell[STEM] .. "\t" .. cell[SUMMARY],
            stem = cell[STEM],
            summary = cell[SUMMARY],
          }
        end
      end
    end
  end

  table.sort(rows, function(left, right)
    return left.key > right.key
  end)

  local capped = {}
  for index = 1, math.min(limit, #rows) do
    capped[index] = rows[index]
  end
  return capped
end

-- The card for `project`, or the empty string when nothing matches.
--
-- An empty string rather than a placeholder: the caller injects this as session context, and a
-- heading announcing that there is nothing to say costs the reader more than silence.
function M.render(lines, project)
  local sessions = M.select(lines, project, "session", M.SESSION_LIMIT)
  local notes = M.select(lines, project, "note", M.NOTE_LIMIT)
  if #sessions == 0 and #notes == 0 then
    return ""
  end

  local out = { "## Journal — recent activity (" .. project .. ")" }

  local function section(title, rows)
    if #rows == 0 then
      return
    end
    out[#out + 1] = ""
    out[#out + 1] = "**" .. title .. ":**"
    for _, row in ipairs(rows) do
      out[#out + 1] = "- [[" .. row.stem .. "]] — " .. row.summary
    end
  end

  section("Sessions", sessions)
  section("Notes", notes)
  return table.concat(out, "\n") .. "\n"
end

-- The card for `project` from the vault's summaries index, or "" when there is no index yet.
function M.card(root, project)
  local tsv = airsstack.path.join(root, ".index", "summaries.tsv")
  if not vault.exists(tsv) then
    return ""
  end
  return M.render(airsstack.fs.read_lines(tsv), project)
end

return M
