-- Snapshot the airsstack-journal vault before a /journal-review run so any curator edit is
-- reversible.
--
-- Tars daily/ sessions/ notes/ mocs/ into .backups/<timestamp>.tar.gz and prunes to the newest
-- AIRSSTACK_JOURNAL_BACKUP_KEEP backups. Excludes .index/ (derived) and .backups/ (recursive).
-- An empty or absent vault is a no-op. Any tar or IO failure goes to stderr and exits non-zero so
-- the review aborts before any write. Prints the archive path on success.
--
--   airsl run --policy confined \
--     --allow-env AIRSSTACK_HOME --allow-env HOME --allow-env AIRSSTACK_JOURNAL_BACKUP_KEEP \
--     --allow-read "$AIRSSTACK_HOME" --allow-write "$AIRSSTACK_HOME" --allow-exec tar \
--     scripts/journal-backup.lua

local backup = require("lib.backup")
local vault = require("lib.vault")

local root = vault.root()
if not vault.exists(root) then
  return -- nothing to protect yet
end

local keep = tonumber(airsstack.env.get("AIRSSTACK_JOURNAL_BACKUP_KEEP") or "")
if not keep or keep < 0 then
  keep = backup.DEFAULT_KEEP
end

local archive, reason = backup.create(root, os.date("%Y-%m-%d-%H%M%S"), math.floor(keep))
if reason then
  vault.die("journal-backup: " .. reason)
end
if archive then
  airsstack.stdio.write(archive .. "\n")
end
