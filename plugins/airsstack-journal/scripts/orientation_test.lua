-- Tests for lib/orientation — the project-scoped recent-activity card.
--
--   airsl test --allow-read /tmp --allow-write /tmp plugins/airsstack-journal/scripts

local orientation = require("lib.orientation")

-- stem, title, summary, project, helped, updated
local function row(stem, summary, project, updated)
  return table.concat({ stem, "T", summary, project, "0", updated }, "\t")
end

return {
  a_row_for_another_project_is_not_shown = function()
    local card = orientation.render({ row("a", "s", "other", "2026-01-01") }, "airsstack")
    assert(card == "", "expected an empty card, got " .. string.format("%q", card))
  end,

  a_project_column_listing_several_projects_matches_each = function()
    local lines = { row("a", "s", "one, airsstack, two", "2026-01-01") }
    assert(orientation.render(lines, "airsstack"):find("[[a]]", 1, true))
    assert(orientation.render(lines, "one"):find("[[a]]", 1, true))
  end,

  a_project_name_is_matched_whole_rather_than_as_a_substring = function()
    assert(orientation.member("airsstack-extra", "airsstack") == false)
    assert(orientation.member("airsstack", "airsstack") == true)
  end,

  sessions_and_notes_are_split_by_the_stem_prefix = function()
    local lines = {
      row("session-abc", "ported scripts", "p", "2026-01-02"),
      row("tokio", "cancellation", "p", "2026-01-01"),
    }
    local card = orientation.render(lines, "p")
    assert(card:find("**Sessions:**", 1, true), card)
    assert(card:find("**Notes:**", 1, true), card)
    assert(card:find("- [[session-abc]] — ported scripts", 1, true), card)
    assert(card:find("- [[tokio]] — cancellation", 1, true), card)
  end,

  rows_are_newest_first = function()
    local lines = {
      row("old", "older", "p", "2026-01-01"),
      row("new", "newer", "p", "2026-06-01"),
    }
    local card = orientation.render(lines, "p")
    assert(card:find("new", 1, true) < card:find("old", 1, true), card)
  end,

  sessions_cap_at_three_and_notes_at_five = function()
    local lines = {}
    for n = 1, 9 do
      lines[#lines + 1] = row("session-" .. n, "s", "p", "2026-01-0" .. n)
      lines[#lines + 1] = row("note-" .. n, "s", "p", "2026-01-0" .. n)
    end
    local card = orientation.render(lines, "p")
    local sessions, notes = 0, 0
    for line in card:gmatch("[^\n]+") do
      if line:find("%[%[session%-") then
        sessions = sessions + 1
      elseif line:find("%[%[note%-") then
        notes = notes + 1
      end
    end
    assert(sessions == orientation.SESSION_LIMIT, "sessions: " .. sessions)
    assert(notes == orientation.NOTE_LIMIT, "notes: " .. notes)
  end,

  a_section_with_nothing_in_it_is_omitted_entirely = function()
    local card = orientation.render({ row("only-a-note", "s", "p", "2026-01-01") }, "p")
    assert(not card:find("**Sessions:**", 1, true), card)
    assert(card:find("**Notes:**", 1, true), card)
  end,

  a_short_row_does_not_raise = function()
    local cells = orientation.cells("stem\ttitle")
    assert(#cells == 6, "expected six cells, got " .. #cells)
    assert(cells[6] == "")
  end,

  an_empty_corpus_renders_nothing_rather_than_a_bare_heading = function()
    assert(orientation.render({}, "p") == "")
    assert(orientation.render({ "" }, "p") == "")
  end,
}
