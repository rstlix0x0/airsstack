-- Shared Open Knowledge Format helpers: frontmatter, bundle discovery, and file enumeration.
--
-- Required by the three OKF scripts (`require("lib.okf")`), which the confined `require` resolves
-- against the script's own directory. Everything here is pure text and filesystem work — no
-- format decision lives in a shell pipeline any more, which is what made the previous awk-based
-- versions hard to change without changing behaviour by accident.

local fs = airsstack.fs
local path = airsstack.path
local proc = airsstack.proc
local regex = airsstack.regex

local M = {}

-- Reserved filenames, which carry structure rather than a concept and are never listed.
M.RESERVED = { ["index.md"] = true, ["log.md"] = true }

-- Files and directories the bundle walk never descends into or reports.
local function hidden(component)
  return component:sub(1, 1) == "."
end

-- Reads `file`, or nil when it cannot be read.
function M.read(file)
  local ok, text = pcall(fs.read, file)
  return ok and text or nil
end

-- The lines of `text`, without a trailing empty element for a terminating newline.
function M.lines(text)
  local out = {}
  for line in (text .. "\n"):gmatch("([^\n]*)\n") do
    out[#out + 1] = line
  end
  if out[#out] == "" then
    out[#out] = nil
  end
  return out
end

-- Where the leading frontmatter block ends, or nil when there is not a well-formed one.
--
-- Bounded structural check, exactly what the shell original did: the first line must be `---` and
-- a closing `---` must exist. Full YAML validation is deliberately out of scope — the format's
-- conformance rules are structural.
function M.fence_end(lines)
  if lines[1] ~= "---" then
    return nil
  end
  for number = 2, #lines do
    if lines[number] == "---" then
      return number
    end
  end
  return nil
end

-- Whether `lines` opens with a well-formed frontmatter block.
function M.parseable(lines)
  return M.fence_end(lines) ~= nil
end

-- Whether the file opens with a `---` line at all, well-formed or not.
function M.has_fence(lines)
  return lines[1] == "---"
end

-- The first frontmatter value for `key`, surrounding quotes stripped, or nil.
function M.field(lines, key)
  local closing = M.fence_end(lines)
  if not closing then
    return nil
  end
  for number = 2, closing - 1 do
    local line = lines[number]
    if line:sub(1, #key + 1) == key .. ":" then
      local value = line:sub(#key + 2):match("^%s*(.-)%s*$")
      local inner = value:match('^"(.*)"$') or value:match("^'(.*)'$")
      return inner or value
    end
  end
  return nil
end

-- Whether `file` is an index.md carrying the `okf_version` bundle marker.
function M.has_marker(file)
  local text = M.read(file)
  if not text then
    return false
  end
  local lines = M.lines(text)
  local closing = M.fence_end(lines)
  if not closing then
    return false
  end
  for number = 2, closing - 1 do
    if lines[number]:match("^okf_version:%s*%S") then
      return true
    end
  end
  return false
end

-- Every `.md` file under `root`, as bundle-relative paths, in C-locale order.
--
-- Hidden directories are skipped the way `find -not -path '*/.*'` skipped them, so a `.git` or a
-- `.obsidian` never reaches the linter.
function M.markdown_files(root)
  local found = {}
  for _, rel in ipairs(fs.walk(root)) do
    if rel:sub(-3) == ".md" then
      local skip = false
      for component in rel:gmatch("[^/]+") do
        skip = skip or hidden(component)
      end
      if not skip then
        found[#found + 1] = rel
      end
    end
  end
  table.sort(found)
  return found
end

-- Runs git in `dir`, returning trimmed stdout or nil.
function M.git(dir, ...)
  local argv = { "git", "-C", dir }
  for _, value in ipairs({ ... }) do
    argv[#argv + 1] = value
  end
  local ok, result = pcall(proc.run, argv)
  if not ok or result.status ~= 0 then
    return nil
  end
  local text = result.stdout:gsub("%s+$", "")
  return text ~= "" and text or nil
end

-- The absolute, symlink-resolved form of `dir`, or nil when it is not a directory.
function M.absolute_dir(dir)
  local ok, resolved = pcall(fs.canonicalize, dir)
  if not ok then
    return nil
  end
  local is_dir, found = pcall(fs.is_dir, resolved)
  if not is_dir or not found then
    return nil
  end
  return resolved
end

-- The bundle root, by the documented resolution order.
--
-- Returns the root, or nil plus the reason. The three failures are distinct on purpose: "you
-- pointed at something that is not a directory", "there is no bundle here", and "there are
-- several" need different actions from the caller.
function M.resolve_root(explicit, cwd)
  if explicit and explicit ~= "" then
    local resolved = M.absolute_dir(explicit)
    if resolved then
      return resolved
    end
    return nil, "explicit path is not a directory: " .. explicit
  end

  local base = M.git(cwd, "rev-parse", "--show-toplevel") or M.absolute_dir(cwd) or cwd

  -- The conventional default wins outright when it is marked: a repository with a `knowledge/`
  -- bundle should not become ambiguous because a vendored tree also carries one.
  if M.has_marker(path.join(base, "knowledge", "index.md")) then
    return path.join(base, "knowledge")
  end

  local candidates = {}
  local tracked = M.git(base, "ls-files", "-co", "--exclude-standard")
  if tracked then
    for line in (tracked .. "\n"):gmatch("([^\n]*)\n") do
      if line:match("(^|/)index%.md$") or line == "index.md" or line:match("/index%.md$") then
        candidates[#candidates + 1] = line
      end
    end
  else
    for _, rel in ipairs(M.markdown_files(base)) do
      if rel == "index.md" or rel:match("/index%.md$") then
        candidates[#candidates + 1] = rel
      end
    end
  end

  local hits, seen = {}, {}
  for _, rel in ipairs(candidates) do
    local file = path.join(base, rel)
    if M.has_marker(file) then
      local directory = path.dirname(file)
      if not seen[directory] then
        seen[directory] = true
        hits[#hits + 1] = directory
      end
    end
  end
  table.sort(hits)

  if #hits == 1 then
    return hits[1]
  end
  if #hits == 0 then
    return nil, "no OKF bundle found — run /airsstack-okf:okf-setup or pass an explicit path"
  end
  return nil, "multiple bundle candidates — pass an explicit path:\n" .. table.concat(hits, "\n")
end

-- The bundle-relative link targets a document declares, as `[text](/path.md)`.
--
-- Only the recommended absolute form is checked, which is what the shell original grepped for.
function M.absolute_links(text)
  local found, seen = {}, {}
  for target in text:gmatch("%]%((/[^%)]*%.md)%)") do
    local rel = target:sub(2)
    if not seen[rel] then
      seen[rel] = true
      found[#found + 1] = rel
    end
  end
  table.sort(found)
  return found
end

-- Writes `message` to stderr and fails the script, which the CLI turns into exit 1.
function M.die(message)
  airsstack.stdio.error(message .. "\n")
  error(message, 0)
end

-- Kept for callers that want the regex module's escaping rules rather than Lua patterns.
M.regex = regex

return M
