-- Resolve the Claude Code transcript JSONL path for a session id.
--
-- Usage: transcript-path.lua <session_id>
-- Prints the transcript path and exits 0 when it exists; prints nothing and exits 1 when no
-- transcript can be located. The store slug is the working directory with every non-alphanumeric
-- character replaced by '-' (the same munge Claude Code uses for ~/.claude/projects/<slug>/).
-- Tries the logical working directory first, then the symlink-resolved one. Honours
-- CLAUDE_CONFIG_DIR.
--
--   airsl run --policy confined \
--     --allow-env CLAUDE_CONFIG_DIR --allow-env HOME --allow-env PWD \
--     --allow-read . --allow-read "$HOME/.claude" \
--     scripts/transcript-path.lua <session_id>

local vault = require("lib.vault")
local env = airsstack.env
local path = airsstack.path
local regex = airsstack.regex

local session_id = arg[1]
if not session_id or session_id == "" then
  error("transcript-path: usage: transcript-path.lua <session_id>", 0)
end

local config_dir = env.get("CLAUDE_CONFIG_DIR")
if not config_dir or config_dir == "" then
  config_dir = path.join(env.get("HOME") or "", ".claude")
end

-- The logical working directory first — a session started under a symlinked path is stored under
-- the slug of the name the user typed, not of what it resolves to. The resolved one is the
-- fallback for the opposite case.
local physical = vault.realpath(".") or vault.cwd()
local candidates = {}
local logical = env.get("PWD")
if logical and logical ~= "" then
  candidates[#candidates + 1] = logical
end
candidates[#candidates + 1] = physical

for _, directory in ipairs(candidates) do
  local slug = regex.replace_all("[^A-Za-z0-9]", directory, "-")
  local target = path.join(config_dir, "projects", slug, session_id .. ".jsonl")
  if vault.exists(target) then
    airsstack.stdio.write(target .. "\n")
    return
  end
end

-- Nothing on stdout, and a failing exit: the caller distinguishes "no transcript" from a path.
error("transcript-path: no transcript for " .. session_id, 0)
