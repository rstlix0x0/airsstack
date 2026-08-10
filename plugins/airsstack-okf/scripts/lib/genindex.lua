-- Deterministic regeneration of an OKF bundle's root index.md.
--
-- Byte-reproducible is the property that matters: the index is committed, so two runs that differ
-- would show up as a diff nobody made. Everything here is therefore ordered explicitly — entries
-- by path in C-locale order, sections by directory name — rather than by whatever the filesystem
-- returned.

local okf = require("lib.okf")
local path = airsstack.path

local M = {}

-- Whether a bundle-relative path is a listable concept.
--
-- Reserved names carry structure rather than a concept, at any level; `.tmp` is the working file
-- this generator itself writes, and listing it would make the output depend on whether a previous
-- run was interrupted.
function M.listable(rel)
  local name = path.basename(rel)
  if okf.RESERVED[name] then
    return false
  end
  return rel:sub(-4) ~= ".tmp"
end

-- The concept paths of a bundle, in C-locale order.
function M.concepts(files)
  local found = {}
  for _, rel in ipairs(files) do
    if M.listable(rel) then
      found[#found + 1] = rel
    end
  end
  table.sort(found)
  return found
end

-- The top-level directories the concepts fall under, in order.
function M.sections(concepts)
  local seen, out = {}, {}
  for _, rel in ipairs(concepts) do
    local top = rel:match("^([^/]+)/")
    if top and not seen[top] then
      seen[top] = true
      out[#out + 1] = top
    end
  end
  table.sort(out)
  return out
end

-- One index line for a concept, or nil plus a warning when its frontmatter will not parse.
--
-- The title falls back to the filename stem, and a missing description renders the bare link:
-- an index that dropped a document because it lacked a description would hide it from every
-- consumer that reads only the index.
function M.entry(rel, text)
  local lines = okf.lines(text)
  if not okf.parseable(lines) then
    return nil, "gen-index: skipping unparseable frontmatter: " .. rel
  end

  local title = okf.field(lines, "title")
  if not title or title == "" then
    title = path.stem(rel)
  end

  local description = okf.field(lines, "description")
  if description and description ~= "" then
    return "- [" .. title .. "](/" .. rel .. ") — " .. description
  end
  return "- [" .. title .. "](/" .. rel .. ")"
end

-- The leading frontmatter block of the existing index, preserved verbatim.
--
-- That block carries the `okf_version` marker, which is what makes the directory a bundle at all;
-- regenerating the index must never be the thing that un-marks it.
function M.preserved_frontmatter(existing)
  if not existing then
    return nil
  end
  local lines = okf.lines(existing)
  local closing = okf.fence_end(lines)
  if not closing then
    return nil
  end
  local kept = {}
  for number = 1, closing do
    kept[#kept + 1] = lines[number]
  end
  return table.concat(kept, "\n") .. "\n"
end

-- The complete index document.
--
-- `read` takes a bundle-relative path and returns its text; `warn` receives one line per document
-- whose frontmatter would not parse.
function M.render(files, existing, read, warn)
  local out = {}

  local preserved = M.preserved_frontmatter(existing)
  if preserved then
    out[#out + 1] = preserved
    out[#out + 1] = "\n"
  end

  out[#out + 1] = "# Index\n"

  local concepts = M.concepts(files)

  local function emit(rel)
    local text = read(rel)
    if not text then
      return
    end
    local line, reason = M.entry(rel, text)
    if not line then
      warn(reason)
      return
    end
    out[#out + 1] = line .. "\n"
  end

  local first_root = true
  for _, rel in ipairs(concepts) do
    if not rel:find("/", 1, true) then
      if first_root then
        out[#out + 1] = "\n"
        first_root = false
      end
      emit(rel)
    end
  end

  for _, section in ipairs(M.sections(concepts)) do
    out[#out + 1] = "\n## " .. section .. "\n\n"
    for _, rel in ipairs(concepts) do
      if rel:sub(1, #section + 1) == section .. "/" then
        emit(rel)
      end
    end
  end

  return table.concat(out)
end

return M
