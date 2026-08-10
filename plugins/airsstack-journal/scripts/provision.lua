-- Provision the airsstack-journal vault directories. Idempotent, zero-dependency.
--
--   airsl run --policy confined \
--     --allow-env AIRSSTACK_HOME --allow-env HOME \
--     --allow-write "$AIRSSTACK_HOME" \
--     scripts/provision.lua

local vault = require("lib.vault")

local root = vault.root()

for _, directory in ipairs({ "daily", "sessions", "notes", "mocs", ".index" }) do
  airsstack.fs.mkdir(airsstack.path.join(root, directory))
end

airsstack.stdio.write("journal: vault provisioned at " .. root .. "\n")
