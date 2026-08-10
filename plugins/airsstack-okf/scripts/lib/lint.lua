-- OKF v0.1 conformance checks.
--
-- Split from the driver so every rule can be exercised against a string rather than a file tree.
-- The distinction the format draws, and that this module keeps, is between the **hard bar** — the
-- structural rules a consumer relies on — and **warnings**, which describe a bundle that is
-- merely less useful than it could be. Only the first affects the exit code, because a permissive
-- consumer is a design goal rather than a concession.

local okf = require("lib.okf")
local path = airsstack.path

local M = {}

-- Fields a concept document should carry but need not.
M.RECOMMENDED = { "title", "description", "timestamp" }

-- Appends a finding.
local function add(findings, severity, rel, message)
  findings[#findings + 1] = { severity = severity, file = rel, message = message }
end

-- The root `index.md` may carry frontmatter, containing only `okf_version` lines.
local function check_root_index(findings, rel, lines)
  local closing = okf.fence_end(lines)
  local last = closing and closing - 1 or #lines
  for number = 2, last do
    local line = lines[number]
    if line ~= "" and not line:match("^okf_version:") then
      add(findings, "FAIL", rel,
        "root index.md frontmatter may contain only okf_version (found: " .. line .. ")")
      return
    end
  end
end

-- `index.md` at any other level must not carry frontmatter at all.
function M.check_index(findings, rel, lines)
  if not okf.has_fence(lines) then
    return
  end
  if rel == "index.md" then
    check_root_index(findings, rel, lines)
  else
    add(findings, "FAIL", rel, "index.md must not carry frontmatter")
  end
end

-- `log.md` carries no frontmatter, ISO date headings, and list-shaped entries.
function M.check_log(findings, rel, lines)
  if okf.has_fence(lines) then
    add(findings, "FAIL", rel, "log.md must not carry frontmatter")
  end

  for _, line in ipairs(lines) do
    if line:sub(1, 3) == "## " and not line:match("^## %d%d%d%d%-%d%d%-%d%d$") then
      add(findings, "FAIL", rel, "non-ISO date heading: " .. line)
      break
    end
  end

  for _, line in ipairs(lines) do
    local shaped = line == ""
      or line:sub(1, 3) == "## "
      or line:sub(1, 2) == "# "
      or line:sub(1, 2) == "- "
      or line:match("^%s")
    if not shaped then
      add(findings, "FAIL", rel, "log entry is not a list item: " .. line)
      break
    end
  end
end

-- A concept document: a well-formed fence, a non-empty `type`, and the recommended fields.
function M.check_concept(findings, rel, lines, text, exists)
  if not okf.parseable(lines) then
    add(findings, "FAIL", rel, "missing or unclosed frontmatter fence")
    return
  end

  local kind = okf.field(lines, "type")
  if not kind or kind == "" then
    add(findings, "FAIL", rel, "missing or empty required field: type")
  end

  for _, key in ipairs(M.RECOMMENDED) do
    if okf.field(lines, key) == nil then
      add(findings, "WARN", rel, "missing recommended field: " .. key)
    end
  end

  for _, target in ipairs(okf.absolute_links(text)) do
    if not exists(target) then
      add(findings, "WARN", rel, "broken link: /" .. target)
    end
  end
end

-- Every finding for one bundle, in file order.
--
-- `exists` answers whether a bundle-relative path names a file; the driver binds it to the bundle
-- root so this function never has to know where the bundle lives.
function M.check(files, read, exists)
  local findings = {}
  for _, rel in ipairs(files) do
    local text = read(rel)
    if text then
      local lines = okf.lines(text)
      local name = path.basename(rel)
      if name == "index.md" then
        M.check_index(findings, rel, lines)
      elseif name == "log.md" then
        M.check_log(findings, rel, lines)
      else
        M.check_concept(findings, rel, lines, text, exists)
      end
    end
  end
  return findings
end

-- Counts by severity.
function M.tally(findings)
  local fails, warns = 0, 0
  for _, finding in ipairs(findings) do
    if finding.severity == "FAIL" then
      fails = fails + 1
    else
      warns = warns + 1
    end
  end
  return fails, warns
end

-- The report body, one line per finding, in the shell original's shape.
function M.render(findings)
  local lines = {}
  for _, finding in ipairs(findings) do
    lines[#lines + 1] = finding.severity .. ": " .. finding.file .. ": " .. finding.message
  end
  local fails, warns = M.tally(findings)
  lines[#lines + 1] = string.format("okf-lint: %d failure(s), %d warning(s)", fails, warns)
  return table.concat(lines, "\n") .. "\n"
end

return M
