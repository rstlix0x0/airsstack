-- SessionStart orchestration for airsstack-journal: provision, rebuild the derived index only when
-- it is stale or absent, then inject a project-scoped orientation card.
--
-- Fail-open throughout: never block the session, never invoke a model. Each fallible step is
-- wrapped, so a failure in one leaves the others to run.
--
--   airsl run --fail-open --policy confined \
--     --allow-env AIRSSTACK_HOME --allow-env HOME \
--     --allow-read "$AIRSSTACK_HOME" --allow-write "$AIRSSTACK_HOME" \
--     --allow-read . --allow-exec git \
--     scripts/session-start.lua

local index = require("lib.index")
local orientation = require("lib.orientation")
local vault = require("lib.vault")
local fs = airsstack.fs
local path = airsstack.path

local root = vault.root()
local marker = path.join(root, ".index", "summaries.tsv")

-- 1. Always provision. Cheap, and everything below assumes the directories exist.
for _, name in ipairs({ "daily", "sessions", "notes", "mocs", ".index" }) do
  pcall(fs.mkdir, path.join(root, name))
end

-- 2. Rebuild only when the corpus has moved since the index was written.
--
-- Modification times rather than content hashes: the index is derived from every note in the
-- vault, so hashing to decide whether to read them all would cost what it saves.
local function stale()
  if not vault.exists(marker) then
    return true
  end
  local ok, stamp = pcall(fs.stat, marker)
  if not ok then
    return true
  end
  for _, sub in ipairs(index.NOTE_DIRS) do
    local directory = path.join(root, sub)
    if vault.exists(directory) then
      local listed, names = pcall(fs.list, directory)
      if listed then
        for _, name in ipairs(names) do
          if name:sub(-3) == ".md" then
            local read, meta = pcall(fs.stat, path.join(directory, name))
            if read and meta.modified > stamp.modified then
              return true
            end
          end
        end
      end
    end
  end
  return false
end

local checked, needs_build = pcall(stale)
if checked and needs_build then
  pcall(index.rebuild, root, function() end)
end

-- 3. The orientation card, as SessionStart additional context. A vault with nothing to say for
--    this project emits nothing at all rather than an empty heading.
local keyed, project = pcall(vault.project_base)
if not keyed or project == "" then
  return
end

local built, card = pcall(orientation.card, root, project)
if built and card ~= "" then
  airsstack.hook.context("SessionStart", card)
end
