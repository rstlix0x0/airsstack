-- OKF v0.1 conformance lint. Usage: okf-lint.lua <bundle-root>
--
-- HARD BAR (any hit → non-zero exit), exactly the spec's conformance rules:
--   - every non-reserved .md: leading --- fence, closing --- fence, and a non-empty type: inside
--     the block (bounded structural check — full YAML validation is deliberately out of scope)
--   - log.md: no frontmatter; every '## ' heading is ISO YYYY-MM-DD; every other non-blank line is
--     a '# ' title, a '- ' list item, or an indented continuation
--   - index.md: no frontmatter — EXCEPT the bundle-root index.md, whose block may contain only
--     okf_version lines
--
-- WARNINGS (never affect the exit code — permissive consumption):
--   - broken bundle-relative links: [text](/path.md) with no such file
--   - missing recommended fields: title, description, timestamp
--
--   airsl run --policy confined --allow-read <bundle-root> \
--     scripts/okf-lint.lua <bundle-root>

local lint = require("lib.lint")
local okf = require("lib.okf")
local fs = airsstack.fs
local path = airsstack.path

local target = arg[1]
if not target or target == "" then
  okf.die("usage: okf-lint.lua <bundle-root>")
end

local root = okf.absolute_dir(target)
if not root then
  okf.die("usage: okf-lint.lua <bundle-root>")
end

local findings = lint.check(
  okf.markdown_files(root),
  function(rel)
    return okf.read(path.join(root, rel))
  end,
  function(rel)
    local ok, found = pcall(fs.exists, path.join(root, rel))
    return ok and found
  end
)

airsstack.stdio.write(lint.render(findings))

local fails = lint.tally(findings)
if fails > 0 then
  -- A conformance failure is a failing exit, and the report above already named every one of
  -- them; `error` here only sets the status.
  error("okf-lint: " .. fails .. " conformance failure(s)", 0)
end
