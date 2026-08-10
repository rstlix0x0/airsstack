-- Tests for lib/health — orphans, hubs and broken links over the derived index.
--
--   airsl test --allow-read /tmp --allow-write /tmp plugins/airsstack-journal/scripts

local health = require("lib.health")
local vault = require("lib.vault")
local json = airsstack.json

local function index_with(nodes, edges, unresolved)
  return {
    nodes = nodes or {},
    edges = vault.array(edges or {}),
    unresolved = vault.array(unresolved or {}),
  }
end

return {
  a_node_with_no_edges_is_an_orphan = function()
    local report = health.analyze(index_with({ a = { type = "concept" } }), 12)
    assert(json.encode(report.orphans) == '["a"]', json.encode(report.orphans))
  end,

  a_daily_container_is_never_reported_as_an_orphan = function()
    local report = health.analyze(index_with({ d = { type = "daily" } }), 12)
    assert(json.encode(report.orphans) == "[]", json.encode(report.orphans))
  end,

  a_session_with_no_edges_is_still_an_orphan = function()
    -- Only `daily` is exempt: a session nobody links to is worth surfacing.
    local report = health.analyze(index_with({ s = { type = "session" } }), 12)
    assert(json.encode(report.orphans) == '["s"]', json.encode(report.orphans))
  end,

  both_ends_of_an_edge_count_toward_degree = function()
    local report = health.analyze(index_with(
      { a = { type = "concept" }, b = { type = "concept" } },
      { { from = "a", to = "b", type = "references" } }
    ), 1)
    assert(json.encode(report.orphans) == "[]", json.encode(report.orphans))
    assert(#report.hubs == 2, "both ends should reach the threshold")
  end,

  an_edge_to_an_unknown_node_does_not_invent_a_degree = function()
    local report = health.analyze(index_with(
      { a = { type = "concept" } },
      { { from = "a", to = "ghost", type = "references" } }
    ), 12)
    assert(json.encode(report.orphans) == "[]", "a still has one edge")
    assert(report.hubs[1] == nil)
  end,

  hubs_are_most_connected_first_with_the_stem_breaking_a_tie = function()
    local report = health.analyze(index_with(
      { a = { type = "c" }, b = { type = "c" }, z = { type = "c" } },
      {
        { from = "a", to = "b", type = "references" },
        { from = "a", to = "z", type = "references" },
      }
    ), 1)
    assert(report.hubs[1].stem == "a" and report.hubs[1].degree == 2, json.encode(report.hubs))
    assert(report.hubs[2].stem == "b", json.encode(report.hubs))
    assert(report.hubs[3].stem == "z", json.encode(report.hubs))
  end,

  the_threshold_is_inclusive = function()
    local report = health.analyze(index_with(
      { a = { type = "c" }, b = { type = "c" } },
      { { from = "a", to = "b", type = "references" } }
    ), 2)
    assert(#report.hubs == 0, "degree 1 must not reach a threshold of 2")
  end,

  unresolved_pairs_are_reported_sorted = function()
    local report = health.analyze(index_with({}, {}, { { "b", "x" }, { "a", "y" } }), 12)
    assert(json.encode(report.broken) == '[["a","y"],["b","x"]]', json.encode(report.broken))
  end,

  an_empty_report_renders_none_in_every_section = function()
    local text = health.render(health.empty())
    local count = 0
    for _ in text:gmatch("_none_") do
      count = count + 1
    end
    assert(count == 3, "expected three empty sections, got " .. count)
  end,

  the_health_block_carries_the_machine_readable_report = function()
    local text = health.render(health.analyze(index_with({ a = { type = "c" } }), 12))
    local block = text:match("```health\n(.-)\n```")
    assert(block, "the fenced health block must be present:\n" .. text)
    assert(json.decode(block).orphans[1] == "a", block)
  end,

  every_empty_signal_encodes_as_an_array_not_an_object = function()
    local report = health.empty()
    assert(json.encode(report) == '{"broken":[],"hubs":[],"orphans":[]}', json.encode(report))
  end,
}
