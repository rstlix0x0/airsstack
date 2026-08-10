-- Resolve the human-readable `project` floor for a journal note: the repository basename.
--
-- Linked worktrees collapse onto the main repo (via git-common-dir); no git falls back to the
-- working directory's basename. The token is sanitised so it is safe as a frontmatter scalar.
-- Deterministic, no side effects, always prints one line, always exits 0.
--
--   airsl run --policy confined --allow-read . --allow-exec git scripts/project-key.lua

local vault = require("lib.vault")

local base = vault.project_base()
airsstack.stdio.write(base .. "\n")
