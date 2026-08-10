-- Compiling a path glob into an anchored regular expression.
--
-- Deliberately not `airsstack.glob`. The two now agree on `*`, `?` and `**` — the host module's
-- `*` used to cross `/`, and that was fixed rather than worked around — but `globset` accepts a
-- strictly larger grammar than the manifests were written against. Brace alternation is the case
-- that matters: `*.{lua,rs}` matches nothing here and matches `a.rs` there, so delegating would
-- start enforcing rules over files whose manifests never selected them. Widening enforcement is
-- the one direction this must not drift in, and a manifest is a contract with plugin authors
-- outside this repository, so its grammar changes when someone decides to change it.
--
-- One further divergence, harmless and recorded so it is not mistaken for a bug: an unclosed `[`
-- is a literal here and a compile error in `globset`, where `matches_any` would disable that one
-- pattern.
--
-- `**/` matches ZERO or more leading segments, which is what makes `**/Cargo.toml` match a
-- root-level `Cargo.toml` — this repository's most important Rust file.

local regex = airsstack.regex

local M = {}

-- One character, escaped so the regex engine reads it literally.
local function escaped(character)
  if character:match("^[%w_]$") then
    return character
  end
  return "\\" .. character
end

-- The anchored regex source for `pattern`.
function M.to_regex(pattern)
  local out = {}
  local index, length = 1, #pattern

  while index <= length do
    local character = pattern:sub(index, index)

    if pattern:sub(index, index + 2) == "**/" then
      out[#out + 1] = "(?:[^/]+/)*"
      index = index + 3
    elseif pattern:sub(index, index + 1) == "**" then
      out[#out + 1] = ".*"
      index = index + 2
    elseif character == "*" then
      out[#out + 1] = "[^/]*"
      index = index + 1
    elseif character == "?" then
      out[#out + 1] = "[^/]"
      index = index + 1
    elseif character == "[" then
      local scan = index + 1
      local negated = pattern:sub(scan, scan)
      if negated == "!" or negated == "^" then
        scan = scan + 1
      end
      -- A `]` immediately after the opening bracket is a literal member, not the close.
      if pattern:sub(scan, scan) == "]" then
        scan = scan + 1
      end
      while scan <= length and pattern:sub(scan, scan) ~= "]" do
        scan = scan + 1
      end

      if scan > length then
        out[#out + 1] = "\\[" -- an unclosed bracket is a literal
        index = index + 1
      else
        local body = pattern:sub(index + 1, scan - 1):gsub("\\", "\\\\")
        if body:sub(1, 1) == "!" then
          body = "^" .. body:sub(2)
        end
        out[#out + 1] = "[" .. body .. "]"
        index = scan + 1
      end
    else
      out[#out + 1] = escaped(character)
      index = index + 1
    end
  end

  return "^" .. table.concat(out) .. "$"
end

-- Whether `candidate` matches at least one of `patterns`.
--
-- A malformed glob disables itself rather than the whole manifest: one unusable pattern must not
-- take the rules it sits beside down with it.
function M.matches_any(candidate, patterns)
  for _, pattern in ipairs(patterns or {}) do
    local ok, matched = pcall(regex.is_match, M.to_regex(tostring(pattern)), candidate)
    if ok and matched then
      return true
    end
  end
  return false
end

return M
