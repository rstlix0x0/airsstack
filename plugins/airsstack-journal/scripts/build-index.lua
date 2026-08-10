-- Build the airsstack-journal derived recall index from the Markdown corpus.
--
-- Scans daily/, sessions/, notes/, mocs/ under the vault and writes .index/graph.json,
-- .index/tags.json, .index/summaries.tsv, and the enriched .index/index.json (nodes +
-- structurally-typed edges + backlinks + unresolved) consumed by the recall subagent.
--
-- Fail-open: a malformed note is skipped with a stderr diagnostic; the rest still index. A
-- `--force` argument is accepted; the builder always performs a full rebuild, so it is a marker of
-- intent rather than a mode switch.
--
--   airsl run --policy confined \
--     --allow-env AIRSSTACK_HOME --allow-env HOME \
--     --allow-read "$AIRSSTACK_HOME" --allow-write "$AIRSSTACK_HOME" \
--     scripts/build-index.lua

local index = require("lib.index")
local vault = require("lib.vault")

index.rebuild(vault.root(), function(line)
  airsstack.stdio.error(line .. "\n")
end)
