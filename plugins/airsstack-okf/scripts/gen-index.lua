-- Regenerate the bundle-root index.md of an OKF bundle, deterministically.
--
-- Usage: gen-index.lua <bundle-root>
--   - preserves the existing leading frontmatter block (the okf_version marker) verbatim when it
--     is well-formed
--   - emits '# Index', then root-level concepts (no heading), then one '## <subdir>' section per
--     top-level subdirectory, entries sorted by path; nested subdirs are listed inside their top
--     section
--   - entry: '- [title](/path.md) — description'; title falls back to the filename stem, and a
--     missing description renders the bare link
--   - reserved files (index.md, log.md at any level) are never listed
--   - files with unparseable frontmatter are skipped with a stderr warning
--
-- Output is byte-reproducible: two consecutive runs are identical.
--
--   airsl run --policy confined --allow-read <bundle-root> --allow-write <bundle-root> \
--     scripts/gen-index.lua <bundle-root>

local genindex = require("lib.genindex")
local okf = require("lib.okf")
local path = airsstack.path

local target = arg[1]
if not target or target == "" then
  okf.die("usage: gen-index.lua <bundle-root>")
end

local root = okf.absolute_dir(target)
if not root then
  okf.die("usage: gen-index.lua <bundle-root>")
end

local index_file = path.join(root, "index.md")

local document = genindex.render(
  okf.markdown_files(root),
  okf.read(index_file),
  function(rel)
    return okf.read(path.join(root, rel))
  end,
  function(line)
    airsstack.stdio.error(line .. "\n")
  end
)

-- Atomic, so a reader never sees a half-written index. The shell original reached the same
-- property by writing `index.md.tmp` and renaming it.
airsstack.fs.atomic_write(index_file, document)
