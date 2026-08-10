-- Deterministic graph-health report over the airsstack-journal index.
--
-- Reads .index/index.json and reports three signals with NO model and NO writes:
--   orphans — nodes with zero in+out edges (excluding type: daily containers)
--   hubs    — nodes whose total degree (in+out) >= AIRSSTACK_JOURNAL_HUB_DEGREE (default 12)
--   broken  — unresolved [stem, missing-target] pairs
--
-- Emits a human Markdown section plus a fenced ```health JSON block the curator parses.
-- Absent/empty/malformed index -> empty report, exit 0 (fail-open).
--
--   airsl run --policy confined \
--     --allow-env AIRSSTACK_HOME --allow-env HOME --allow-env AIRSSTACK_JOURNAL_HUB_DEGREE \
--     --allow-read "$AIRSSTACK_HOME" \
--     scripts/graph-health.lua

local health = require("lib.health")
local vault = require("lib.vault")

local configured = tonumber(airsstack.env.get("AIRSSTACK_JOURNAL_HUB_DEGREE") or "")
local threshold = health.DEFAULT_HUB_DEGREE
if configured and configured > 0 then
  threshold = math.floor(configured)
end

airsstack.stdio.write(health.render(health.report(vault.root(), threshold)))
