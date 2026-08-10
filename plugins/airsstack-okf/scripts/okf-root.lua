-- Resolve the OKF bundle root. Resolution order:
--   1. explicit path argument (wins; must be an existing directory)
--   2. <repo-root>/knowledge/index.md carrying okf_version: frontmatter
--   3. bounded scan of git-visible index.md files (tracked + untracked, not ignored) for the
--      okf_version marker; no-git falls back to a walk
--
-- Prints the absolute bundle-root path on stdout and exits 0. Bad usage, no bundle, or an
-- ambiguous scan goes to stderr and exits non-zero.
--
--   airsl run --policy confined --allow-read . --allow-exec git \
--     scripts/okf-root.lua [path]

local okf = require("lib.okf")

local root, reason = okf.resolve_root(arg[1], airsstack.path.absolute("."))
if not root then
  okf.die("okf-root: " .. reason)
end

airsstack.stdio.write(root .. "\n")
