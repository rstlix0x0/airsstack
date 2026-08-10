-- Tests for lib/daily — creates the daily note, links idempotently.
--
--   airsl test --allow-read /tmp --allow-write /tmp plugins/airsstack-journal/scripts

local daily = require("lib.daily")
local fs = airsstack.fs
local path = airsstack.path

local DATE = "2026-06-23"
local NOW = "2026-06-23 09:41"

local function vault()
  return fs.tempdir()
end

local function note(root)
  return fs.read(path.join(root, "daily", DATE .. ".md"))
end

local function occurrences(text, needle)
  local count, from = 0, 1
  while true do
    local found = text:find(needle, from, true)
    if not found then
      return count
    end
    count = count + 1
    from = found + 1
  end
end

return {
  creates_the_daily_note_with_frontmatter_and_the_link = function()
    local root = vault()
    assert(daily.link(root, DATE, "tokio-cancellation-safety", NOW) == true)

    local text = note(root)
    assert(text:find("\ntype: daily\n", 1, true), "daily note must declare type: daily")
    assert(text:find("[[tokio-cancellation-safety]]", 1, true), "the stem must be linked")
  end,

  linking_the_same_stem_twice_adds_one_link = function()
    local root = vault()
    daily.link(root, DATE, "tokio-cancellation-safety", NOW)
    assert(daily.link(root, DATE, "tokio-cancellation-safety", NOW) == false)

    local found = occurrences(note(root), "[[tokio-cancellation-safety]]")
    assert(found == 1, "expected exactly one link, found " .. found)
  end,

  a_second_distinct_stem_also_appears = function()
    local root = vault()
    daily.link(root, DATE, "tokio-cancellation-safety", NOW)
    daily.link(root, DATE, "session-abc12345", NOW)

    local text = note(root)
    assert(text:find("[[tokio-cancellation-safety]]", 1, true))
    assert(text:find("[[session-abc12345]]", 1, true))
  end,

  adding_a_link_bumps_updated_but_creation_does_not_move_created = function()
    local root = vault()
    daily.link(root, DATE, "first", NOW)
    daily.link(root, DATE, "second", "2026-06-24 11:02")

    local text = note(root)
    assert(text:find("\ncreated: " .. NOW .. "\n", 1, true), "created must stay at first write")
    assert(text:find("\nupdated: 2026-06-24 11:02\n", 1, true), "updated must move")
  end,

  a_note_already_carrying_the_link_is_left_untouched = function()
    local root = vault()
    daily.link(root, DATE, "only", NOW)
    local before = note(root)
    daily.link(root, DATE, "only", "2026-12-31 23:59")
    assert(note(root) == before, "an idempotent call must not rewrite the file")
  end,
}
