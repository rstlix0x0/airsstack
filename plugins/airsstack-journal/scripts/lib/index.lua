-- Building the airsstack-journal derived recall index from the Markdown corpus.
--
-- Separate from the `build-index.lua` driver so the whole pipeline — collect, build, render — can
-- be exercised against a temporary vault by `airsl test`. The Markdown corpus is the sole source
-- of truth and every output here is fully reconstructible from it.
--
-- Fail-open: a malformed note is skipped with a stderr diagnostic and the rest still index.

local vault = require("lib.vault")
local fs = airsstack.fs
local json = airsstack.json
local path = airsstack.path
local regex = airsstack.regex

local M = {}

M.NOTE_DIRS = { "daily", "sessions", "notes", "mocs" }
M.UNRESOLVED_KEY = "_unresolved"
M.CONTAINER_TYPES = { session = true, daily = true }
M.EDGE_PRIORITY = { supersedes = 4, ["depends-on"] = 3, contains = 2, references = 1 }

-- Two patterns rather than the Python original's one: its `(```|~~~).*?\1` closes the fence with a
-- backreference, and the `regex` crate has none by design. Alternating over the two fence
-- characters separately says the same thing without one.
local FENCED_BACKTICK = regex.compile("(?s)```.*?```")
local FENCED_TILDE = regex.compile("(?s)~~~.*?~~~")
local INLINE_CODE = regex.compile("`[^`\n]*`")

-- Every `[[target]]` in `text`.
function M.wikilinks(text)
  local found = {}
  for target in text:gmatch("%[%[([^%]]+)%]%]") do
    found[#found + 1] = target
  end
  return found
end

-- Fenced and inline code removed, so a `[[link]]` shown as an example is not indexed as an edge.
function M.strip_code_spans(text)
  local stripped = FENCED_BACKTICK.replace_all(text, " ")
  stripped = FENCED_TILDE.replace_all(stripped, " ")
  return INLINE_CODE.replace_all(stripped, " ")
end

-- A wikilink target reduced to the stem it names: alias and anchor dropped, lowercased.
function M.normalize_target(text)
  local stem = text:gsub("|.*$", ""):gsub("#.*$", "")
  return stem:match("^%s*(.-)%s*$"):lower()
end

-- The note stem a file path names.
function M.stem_of(file)
  return path.stem(file):lower()
end

-- The `nodes` entry for one note.
function M.node_record(rel, frontmatter)
  local helped = tonumber(vault.scalar(frontmatter.helped)) or 0

  local function trimmed(values)
    local out = {}
    for _, value in ipairs(vault.as_list(values)) do
      local text = value:match("^%s*(.-)%s*$")
      if text ~= "" then
        out[#out + 1] = text
      end
    end
    return out
  end

  return {
    type = vault.scalar(frontmatter.type),
    title = vault.scalar(frontmatter.title),
    summary = vault.scalar(frontmatter.summary),
    project = vault.scalar(frontmatter.project),
    domains = vault.array(trimmed(frontmatter.domains)),
    tags = vault.array(trimmed(frontmatter.tags)),
    helped = math.floor(helped),
    updated = vault.scalar(frontmatter.updated),
    path = rel,
  }
end

-- Every readable note under the vault's four note directories, in path order.
--
-- `report` receives one diagnostic per malformed note; the driver points it at stderr.
function M.collect_notes(root, report)
  local notes = {}
  for _, sub in ipairs(M.NOTE_DIRS) do
    local directory = path.join(root, sub)
    if vault.exists(directory) and fs.is_dir(directory) then
      local names = fs.list(directory)
      for _, name in ipairs(names) do
        if name:sub(-3) == ".md" then
          local file = path.join(directory, name)
          local ok, text = pcall(fs.read, file)
          if not ok then
            report("journal: skipping unreadable note " .. file)
          else
            local frontmatter, body = vault.frontmatter(text)
            if not frontmatter then
              report("journal: skipping malformed note " .. file .. ": " .. body)
            else
              notes[#notes + 1] = {
                rel = sub .. "/" .. name,
                stem = M.stem_of(name),
                frontmatter = frontmatter,
                body = body,
              }
            end
          end
        end
      end
    end
  end
  return notes
end

-- The `(target, edge_type)` pairs the typed frontmatter fields declare.
local function typed_link_targets(frontmatter)
  local typed = {}
  for _, field in ipairs({ "supersedes", "depends-on" }) do
    for _, raw in ipairs(vault.as_list(frontmatter[field])) do
      for _, target in ipairs(M.wikilinks(raw)) do
        typed[#typed + 1] = { target = target, kind = field }
      end
    end
  end
  return typed
end

-- Every wikilink a note declares, from its `links` field and from its body.
local function link_targets(frontmatter, body)
  local targets = {}
  for _, raw in ipairs(vault.as_list(frontmatter.links)) do
    for _, target in ipairs(M.wikilinks(raw)) do
      targets[#targets + 1] = target
    end
  end
  for _, target in ipairs(M.wikilinks(M.strip_code_spans(body))) do
    targets[#targets + 1] = target
  end
  return targets
end

-- The four derived artefacts, from the collected notes.
function M.build(notes)
  local known = {}
  for _, note in ipairs(notes) do
    known[note.stem] = true
  end

  local graph, tags, nodes, backlinks = {}, {}, {}, {}
  local edges, rows = {}, {}
  local unresolved, unresolved_seen = {}, {}

  local function note_unresolved(stem, target)
    local key = stem .. "\0" .. target
    if not unresolved_seen[key] then
      unresolved_seen[key] = true
      unresolved[#unresolved + 1] = { stem, target }
    end
  end

  for _, note in ipairs(notes) do
    local stem = note.stem
    nodes[stem] = M.node_record(note.rel, note.frontmatter)

    local source_type = nodes[stem].type:match("^%s*(.-)%s*$"):lower()
    local edge_type = M.CONTAINER_TYPES[source_type] and "contains" or "references"

    local resolved, resolved_seen = {}, {}
    local edge_best = {}

    for _, raw in ipairs(link_targets(note.frontmatter, note.body)) do
      local target = M.normalize_target(raw)
      if target ~= "" and target ~= stem then
        if known[target] then
          if not resolved_seen[target] then
            resolved_seen[target] = true
            resolved[#resolved + 1] = target
          end
          edge_best[target] = edge_type
        else
          note_unresolved(stem, target)
        end
      end
    end

    for _, typed in ipairs(typed_link_targets(note.frontmatter)) do
      local target = M.normalize_target(typed.target)
      if target ~= "" and target ~= stem then
        if known[target] then
          local current = M.EDGE_PRIORITY[edge_best[target]] or 0
          if M.EDGE_PRIORITY[typed.kind] > current then
            edge_best[target] = typed.kind
          end
        else
          note_unresolved(stem, target)
        end
      end
    end

    for _, target in ipairs(vault.sorted_keys(edge_best)) do
      edges[#edges + 1] = { from = stem, to = target, type = edge_best[target] }
      backlinks[target] = backlinks[target] or {}
      local seen = false
      for _, existing in ipairs(backlinks[target]) do
        seen = seen or existing == stem
      end
      if not seen then
        table.insert(backlinks[target], stem)
      end
    end

    table.sort(resolved)
    graph[stem] = vault.array(resolved)

    local labels = vault.as_list(note.frontmatter.tags)
    for _, domain in ipairs(vault.as_list(note.frontmatter.domains)) do
      labels[#labels + 1] = domain
    end
    for _, label in ipairs(labels) do
      local tag = label:match("^%s*(.-)%s*$"):lower()
      if tag ~= "" then
        tags[tag] = tags[tag] or {}
        local seen = false
        for _, existing in ipairs(tags[tag]) do
          seen = seen or existing == stem
        end
        if not seen then
          table.insert(tags[tag], stem)
        end
      end
    end

    rows[#rows + 1] = {
      stem,
      vault.scalar(note.frontmatter.title),
      vault.scalar(note.frontmatter.summary),
      vault.scalar(note.frontmatter.project),
      note.frontmatter.helped == nil and "0" or vault.scalar(note.frontmatter.helped),
      vault.scalar(note.frontmatter.updated),
    }
  end

  vault.sort_rows(unresolved)
  if #unresolved > 0 then
    local pairs_out = {}
    for _, pair in ipairs(unresolved) do
      pairs_out[#pairs_out + 1] = vault.array(pair)
    end
    graph[M.UNRESOLVED_KEY] = vault.array(pairs_out)
  end

  for tag, stems in pairs(tags) do
    table.sort(stems)
    tags[tag] = vault.array(stems)
  end

  local sorted_backlinks = {}
  for target, stems in pairs(backlinks) do
    table.sort(stems)
    sorted_backlinks[target] = vault.array(stems)
  end

  table.sort(edges, function(left, right)
    if left.from ~= right.from then
      return left.from < right.from
    end
    if left.to ~= right.to then
      return left.to < right.to
    end
    return left.type < right.type
  end)

  vault.sort_rows(rows)

  local unresolved_array = {}
  for _, pair in ipairs(unresolved) do
    unresolved_array[#unresolved_array + 1] = vault.array(pair)
  end

  local index = {
    nodes = nodes,
    edges = vault.array(edges),
    backlinks = sorted_backlinks,
    unresolved = vault.array(unresolved_array),
  }

  return graph, tags, rows, index
end

-- A tab-separated cell: the three characters that would break the row are flattened to spaces.
function M.tsv_clean(value)
  return (value:gsub("[\t\r\n]", " "))
end

-- Writes the four derived artefacts under `<root>/.index`.
function M.write_outputs(root, graph, tags, rows, index)
  local directory = path.join(root, ".index")
  fs.mkdir(directory)

  fs.write(path.join(directory, "graph.json"), json.encode_pretty(graph))
  fs.write(path.join(directory, "tags.json"), json.encode_pretty(tags))
  fs.write(path.join(directory, "index.json"), json.encode_pretty(index))

  local lines = {}
  for _, row in ipairs(rows) do
    local cells = {}
    for _, cell in ipairs(row) do
      cells[#cells + 1] = M.tsv_clean(cell)
    end
    lines[#lines + 1] = table.concat(cells, "\t")
  end
  local text = table.concat(lines, "\n")
  if #lines > 0 then
    text = text .. "\n"
  end
  fs.write(path.join(directory, "summaries.tsv"), text)
end

-- Collect, build and write, in one call.
function M.rebuild(root, report)
  local notes = M.collect_notes(root, report)
  local graph, tags, rows, index = M.build(notes)
  M.write_outputs(root, graph, tags, rows, index)
  return #notes
end

return M
