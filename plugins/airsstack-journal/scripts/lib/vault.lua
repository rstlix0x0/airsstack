-- Shared vault helpers for the airsstack-journal scripts.
--
-- Required by every script in this directory (`require("lib.vault")`), which the confined
-- `require` resolves against the script's own directory. Nothing here reaches outside the
-- capabilities the calling script was granted: each function raises the runtime's own denial
-- message when the policy did not grant what it needs.

local fs = airsstack.fs
local path = airsstack.path
local proc = airsstack.proc
local regex = airsstack.regex

local M = {}

-- Every character outside [A-Za-z0-9._-] becomes '-', matching `tr -c 'A-Za-z0-9._-' '-'`.
function M.sanitize(text)
  return regex.replace_all("[^A-Za-z0-9._-]", text or "", "-")
end

-- The working directory, as an absolute path. `path.absolute` needs no grant.
function M.cwd()
  return path.absolute(".")
end

-- Runs git in `dir` and returns its trimmed stdout, or nil on any failure.
--
-- `-C` rather than a working directory on the child: `proc.run` takes an argv array and nothing
-- else, so the directory has to travel as an argument. Every git invocation in this suite accepts
-- it.
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
  if text == "" then
    return nil
  end
  return text
end

-- The repository basename, with linked worktrees collapsing onto the main repo.
--
-- `--git-common-dir` is what collapses them: it answers with the main repo's `.git` from every
-- linked worktree, so its grandparent is the one directory every worktree of a repo shares.
-- Outside a repository the working directory's own basename is the floor.
function M.project_base(dir)
  local base = dir or M.cwd()
  local common = M.git(base, "rev-parse", "--git-common-dir")
  if not common then
    local absolute = M.realpath(base) or base
    return M.sanitize(path.basename(absolute)), absolute
  end

  if not path.is_absolute(common) then
    common = path.join(base, common)
  end
  -- Canonicalise the parent rather than the whole path: `.git` is a file rather than a directory
  -- in a linked worktree, and the shell original resolved the parent with `pwd -P` for the same
  -- reason. `absolute` is the main repository's `.git`, so its parent's name is the repo's.
  local parent = M.realpath(path.dirname(common)) or path.dirname(common)
  local absolute = path.join(parent, path.basename(common))
  return M.sanitize(path.basename(path.dirname(absolute))), absolute
end

-- `fs.canonicalize` where the policy allows it, nil where it does not or the path is absent.
function M.realpath(target)
  local ok, resolved = pcall(fs.canonicalize, target)
  if ok then
    return resolved
  end
  return nil
end

-- The vault root: `$AIRSSTACK_HOME/journal`, defaulting to `~/.airsstack/journal`.
function M.root()
  local home = airsstack.env.get("AIRSSTACK_HOME")
  if not home or home == "" then
    home = path.join(airsstack.env.get("HOME") or "", ".airsstack")
  end
  return path.join(home, "journal")
end

-- Splits a note into its frontmatter table and its body.
--
-- Returns nil plus a reason when the leading fence is unterminated, which is the one malformed
-- shape the index builder has to report rather than guess at. A note with no leading `---` is not
-- malformed — it simply has no frontmatter.
function M.frontmatter(text)
  if text:sub(1, 3) ~= "---" then
    return {}, text
  end
  local lines = {}
  for line in (text .. "\n"):gmatch("([^\n]*)\n") do
    lines[#lines + 1] = line
  end

  local closing
  for i = 2, #lines do
    if lines[i]:match("^%s*(.-)%s*$") == "---" then
      closing = i
      break
    end
  end
  if not closing then
    return nil, "unterminated frontmatter fence"
  end

  local fields = {}
  local pending
  for i = 2, closing - 1 do
    local line = lines[i]
    if line:match("^%s*$") then
      pending = nil
    else
      local item = line:match("^%s*%-%s+(.*)$")
      if item and pending then
        -- Block-style list continuation, which is what Obsidian's Properties UI emits.
        local list = fields[pending]
        list[#list + 1] = M.unquote(item)
      else
        local key, value = line:match("^([^:]+):%s*(.*)$")
        if not key then
          return nil, "frontmatter line without ':' -> " .. line
        end
        key = key:match("^%s*(.-)%s*$")
        if value == "" then
          -- Either an empty scalar or the header of a block list; the next line decides.
          fields[key] = {}
          pending = key
        else
          fields[key] = M.parse_value(value)
          pending = nil
        end
      end
    end
  end

  local body = {}
  for i = closing + 1, #lines do
    body[#body + 1] = lines[i]
  end
  return fields, table.concat(body, "\n")
end

-- One surrounding pair of single or double quotes removed.
function M.unquote(value)
  local text = value:match("^%s*(.-)%s*$")
  local inner = text:match('^"(.*)"$') or text:match("^'(.*)'$")
  return inner or text
end

-- A frontmatter scalar, or an inline `[a, b, c]` flow list as a table.
function M.parse_value(value)
  local text = value:match("^%s*(.-)%s*$")
  local inner = text:match("^%[(.*)%]$")
  if not inner then
    return M.unquote(text)
  end
  if inner:match("^%s*$") then
    return {}
  end
  local items = {}
  for item in (inner .. ","):gmatch("([^,]*),") do
    items[#items + 1] = M.unquote(item)
  end
  return items
end

-- A frontmatter value as a list of strings, whatever shape it arrived in.
function M.as_list(value)
  if value == nil then
    return {}
  end
  if type(value) == "table" then
    local out = {}
    for _, item in ipairs(value) do
      out[#out + 1] = tostring(item)
    end
    return out
  end
  return { tostring(value) }
end

-- A frontmatter value as one string; a list joins with ", " the way the shell scripts render it.
function M.scalar(value)
  if value == nil then
    return ""
  end
  if type(value) == "table" then
    local parts = {}
    for _, item in ipairs(value) do
      parts[#parts + 1] = tostring(item)
    end
    return table.concat(parts, ", ")
  end
  return tostring(value)
end

-- A table that encodes as a JSON array even when it is empty.
--
-- Lua cannot tell an empty sequence from an empty map, so `airsstack.json` encodes `{}` as an
-- object. Decoding `[]` hands back a table the encoder has already marked as a sequence, and it
-- stays marked as elements are added — which is the only way to emit `"orphans": []` rather than
-- `"orphans": {}` from a script.
function M.array(items)
  local out = airsstack.json.decode("[]")
  for index, item in ipairs(items or {}) do
    out[index] = item
  end
  return out
end

-- Sorts a list of lists, comparing element by element like Python's tuple ordering.
function M.sort_rows(rows)
  table.sort(rows, function(left, right)
    for index = 1, math.max(#left, #right) do
      local a, b = left[index], right[index]
      if a == nil then
        return true
      end
      if b == nil then
        return false
      end
      if a ~= b then
        return a < b
      end
    end
    return false
  end)
  return rows
end

-- The keys of `map`, sorted.
function M.sorted_keys(map)
  local keys = {}
  for key in pairs(map) do
    keys[#keys + 1] = key
  end
  table.sort(keys)
  return keys
end

-- Writes `message` to stderr and fails the script, which the CLI turns into exit 1.
--
-- `error` with level 0 so the runtime's own trailer does not repeat a position the message
-- already carries.
function M.die(message)
  airsstack.stdio.error(message .. "\n")
  error(message, 0)
end

-- Creates `directory` and every missing parent, like `mkdir -p`.
function M.mkdir(directory)
  fs.mkdir(directory)
end

-- Whether `target` exists, answering false where the policy refuses the question.
--
-- Only for paths a script may legitimately not be granted; anywhere the grant is part of the
-- contract, let the denial raise.
function M.exists(target)
  local ok, found = pcall(fs.exists, target)
  return ok and found
end

return M
