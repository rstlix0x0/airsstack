-- Graph-health analysis over the derived journal index.
--
-- Three signals, no model and no writes: orphans (nodes with no edges either way), hubs (nodes
-- whose degree reaches a threshold), and broken links (unresolved targets). Reads `.index/
-- index.json`, which `lib/index` wrote, so the analysis never re-parses the corpus.

local vault = require("lib.vault")

local M = {}

M.DEFAULT_HUB_DEGREE = 12

-- Nodes that are containers by nature are exempt from the orphan signal: a daily note nobody links
-- to is the normal case, not a defect.
local EXEMPT_FROM_ORPHANS = { daily = true }

-- The three signals for one decoded index.
function M.analyze(index, threshold)
  local nodes = index.nodes or {}
  local edges = index.edges or {}
  local unresolved = index.unresolved or {}

  local degree = {}
  for stem in pairs(nodes) do
    degree[stem] = 0
  end
  for _, edge in ipairs(edges) do
    if degree[edge.from] then
      degree[edge.from] = degree[edge.from] + 1
    end
    if degree[edge.to] then
      degree[edge.to] = degree[edge.to] + 1
    end
  end

  local orphans, hubs = {}, {}
  for _, stem in ipairs(vault.sorted_keys(degree)) do
    local count = degree[stem]
    local kind = (nodes[stem] or {}).type or ""
    if count == 0 and not EXEMPT_FROM_ORPHANS[kind] then
      orphans[#orphans + 1] = stem
    end
    if count >= threshold then
      hubs[#hubs + 1] = { stem = stem, degree = count }
    end
  end

  -- Most connected first, ties broken by stem so two runs over one index agree.
  table.sort(hubs, function(left, right)
    if left.degree ~= right.degree then
      return left.degree > right.degree
    end
    return left.stem < right.stem
  end)

  local broken = {}
  for _, pair in ipairs(unresolved) do
    broken[#broken + 1] = { pair[1], pair[2] }
  end
  vault.sort_rows(broken)

  local hub_records = {}
  for index_, hub in ipairs(hubs) do
    hub_records[index_] = { stem = hub.stem, degree = hub.degree }
  end

  local broken_records = {}
  for index_, pair in ipairs(broken) do
    broken_records[index_] = vault.array(pair)
  end

  return {
    orphans = vault.array(orphans),
    hubs = vault.array(hub_records),
    broken = vault.array(broken_records),
  }
end

-- The Markdown report, with the machine-readable block the curator parses appended.
function M.render(report)
  local out = { "# Journal graph-health report", "", "## Orphans (no links in or out)" }

  local function section(rows, line)
    if #rows == 0 then
      out[#out + 1] = "_none_"
      return
    end
    for _, row in ipairs(rows) do
      out[#out + 1] = line(row)
    end
  end

  section(report.orphans, function(stem)
    return "- [[" .. stem .. "]]"
  end)

  out[#out + 1] = ""
  out[#out + 1] = "## Hubs (over-connected)"
  section(report.hubs, function(hub)
    return "- [[" .. hub.stem .. "]] — degree " .. hub.degree
  end)

  out[#out + 1] = ""
  out[#out + 1] = "## Broken links"
  section(report.broken, function(pair)
    return "- [[" .. pair[1] .. "]] → " .. pair[2] .. " (missing)"
  end)

  out[#out + 1] = ""
  out[#out + 1] = "```health"
  -- `encode_pretty` already terminates with a newline, which the fence line supplies here.
  out[#out + 1] = (airsstack.json.encode_pretty(report):gsub("\n$", ""))
  out[#out + 1] = "```"
  return table.concat(out, "\n") .. "\n"
end

-- The empty report, which an absent or malformed index produces.
function M.empty()
  return {
    orphans = vault.array({}),
    hubs = vault.array({}),
    broken = vault.array({}),
  }
end

-- The report for the vault at `root`, falling back to the empty one.
function M.report(root, threshold)
  local file = airsstack.path.join(root, ".index", "index.json")
  local ok, text = pcall(airsstack.fs.read, file)
  if not ok then
    return M.empty()
  end
  local decoded, index = pcall(airsstack.json.decode, text)
  if not decoded or type(index) ~= "table" then
    return M.empty()
  end
  return M.analyze(index, threshold)
end

return M
