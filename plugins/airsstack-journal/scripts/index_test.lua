-- Tests for lib/index — the derived recall index over the Markdown corpus.
--
--   airsl test --allow-read /tmp --allow-write /tmp plugins/airsstack-journal/scripts

local index = require("lib.index")
local vault = require("lib.vault")
local fs = airsstack.fs
local json = airsstack.json
local path = airsstack.path

local function vault_with(notes)
  local root = fs.tempdir()
  for _, dir in ipairs(index.NOTE_DIRS) do
    fs.mkdir(path.join(root, dir))
  end
  for rel, text in pairs(notes) do
    fs.write(path.join(root, rel), text)
  end
  return root
end

local function silent() end

-- Returns build's four results. Nothing may follow the call in the return list: a Lua call in
-- anything but the final position is truncated to one value, which silently hands the caller
-- `graph` and three nils.
local function built(notes)
  local root = vault_with(notes)
  return index.build(index.collect_notes(root, silent))
end

local function note(front, body)
  return "---\n" .. front .. "\n---\n" .. (body or "")
end

return {
  a_wikilink_in_a_fenced_block_is_not_an_edge = function()
    local body = "Real [[b]].\n\n```lua\n-- [[fake]]\n```\n"
    local graph = built({
      ["notes/a.md"] = note("type: concept", body),
      ["notes/b.md"] = note("type: concept"),
      ["notes/fake.md"] = note("type: concept"),
    })
    assert(json.encode(graph.a) == '["b"]', "got " .. json.encode(graph.a))
  end,

  a_wikilink_in_an_inline_span_is_not_an_edge = function()
    local graph = built({
      ["notes/a.md"] = note("type: concept", "Text `[[fake]]` here.\n"),
      ["notes/fake.md"] = note("type: concept"),
    })
    assert(json.encode(graph.a) == "[]", "got " .. json.encode(graph.a))
  end,

  a_note_with_no_links_encodes_an_empty_array_not_an_object = function()
    local graph = built({ ["notes/a.md"] = note("type: concept", "Nothing.\n") })
    assert(json.encode(graph) == '{"a":[]}', "got " .. json.encode(graph))
  end,

  an_unknown_target_lands_in_unresolved_rather_than_the_graph = function()
    local graph, _, _, idx = built({
      ["notes/a.md"] = note("type: concept", "Points at [[nowhere]].\n"),
    })
    assert(json.encode(graph.a) == "[]")
    assert(json.encode(idx.unresolved) == '[["a","nowhere"]]', json.encode(idx.unresolved))
  end,

  a_container_note_emits_contains_where_a_concept_emits_references = function()
    local _, _, _, idx = built({
      ["sessions/session-a.md"] = note("type: session", "Holds [[b]].\n"),
      ["notes/b.md"] = note("type: concept", "Cites [[session-a]].\n"),
    })
    local kinds = {}
    for _, edge in ipairs(idx.edges) do
      kinds[edge.from] = edge.type
    end
    assert(kinds["session-a"] == "contains", "session edge was " .. tostring(kinds["session-a"]))
    assert(kinds.b == "references", "concept edge was " .. tostring(kinds.b))
  end,

  a_typed_field_outranks_the_structural_edge_type = function()
    local _, _, _, idx = built({
      ["notes/a.md"] = note('type: concept\nsupersedes: "[[b]]"', "Also cites [[b]].\n"),
      ["notes/b.md"] = note("type: concept"),
    })
    assert(#idx.edges == 1, "expected one edge, got " .. #idx.edges)
    assert(idx.edges[1].type == "supersedes", "got " .. idx.edges[1].type)
  end,

  an_alias_and_an_anchor_are_stripped_from_a_target = function()
    assert(index.normalize_target("Some Note|shown as this") == "some note")
    assert(index.normalize_target("some-note#heading") == "some-note")
    assert(index.normalize_target("  Mixed Case  ") == "mixed case")
  end,

  a_block_style_list_parses_the_way_obsidian_writes_it = function()
    local _, _, _, idx = built({
      ["notes/a.md"] = note("type: concept\ntags:\n  - tokio\n  - async"),
    })
    assert(json.encode(idx.nodes.a.tags) == '["async","tokio"]' or
      json.encode(idx.nodes.a.tags) == '["tokio","async"]',
      "got " .. json.encode(idx.nodes.a.tags))
  end,

  an_inline_flow_list_parses_too = function()
    local _, _, _, idx = built({
      ["notes/a.md"] = note("type: concept\ndomains: [rust, async]"),
    })
    assert(json.encode(idx.nodes.a.domains) == '["rust","async"]',
      "got " .. json.encode(idx.nodes.a.domains))
  end,

  a_malformed_note_is_reported_and_skipped_while_the_rest_index = function()
    local root = vault_with({
      ["notes/broken.md"] = "---\ntype: concept\nno closing fence\n",
      ["notes/good.md"] = note("type: concept"),
    })
    local reported = {}
    local notes = index.collect_notes(root, function(line)
      reported[#reported + 1] = line
    end)
    assert(#notes == 1, "expected one usable note, got " .. #notes)
    assert(#reported == 1, "expected one diagnostic, got " .. #reported)
    assert(reported[1]:find("unterminated", 1, true), reported[1])
  end,

  a_backlink_is_recorded_against_the_target = function()
    local _, _, _, idx = built({
      ["notes/a.md"] = note("type: concept", "To [[b]].\n"),
      ["notes/b.md"] = note("type: concept"),
    })
    assert(json.encode(idx.backlinks.b) == '["a"]', json.encode(idx.backlinks))
  end,

  a_tab_in_a_summary_cannot_break_the_tsv_row = function()
    assert(index.tsv_clean("a\tb\nc\rd") == "a b c d")
  end,

  an_empty_vault_writes_an_index_rather_than_failing = function()
    local root = vault_with({})
    assert(index.rebuild(root, silent) == 0)
    assert(fs.read(path.join(root, ".index", "summaries.tsv")) == "")
    assert(json.decode(fs.read(path.join(root, ".index", "graph.json"))) ~= nil)
  end,

  the_same_corpus_indexes_to_the_same_bytes_every_run = function()
    local notes = {
      ["notes/a.md"] = note("type: concept\ntags: [x, y]", "To [[b]].\n"),
      ["notes/b.md"] = note("type: concept\ntags: [y, z]", "To [[a]].\n"),
    }
    local first = vault_with(notes)
    index.rebuild(first, silent)
    local second = vault_with(notes)
    index.rebuild(second, silent)
    for _, name in ipairs({ "graph.json", "tags.json", "index.json", "summaries.tsv" }) do
      assert(
        fs.same_content(path.join(first, ".index", name), path.join(second, ".index", name)),
        name .. " is not byte-reproducible"
      )
    end
  end,

  vault_array_encodes_empty_as_an_array = function()
    assert(json.encode(vault.array({})) == "[]")
    assert(json.encode(vault.array({ "a" })) == '["a"]')
  end,
}
