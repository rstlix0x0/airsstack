-- airsstack concise — UserPromptSubmit hook.
--
-- Detects concise activation, level switch, and deactivation in the user prompt (slash command +
-- natural language), persists the active level to a brand-namespaced flag file, and re-injects the
-- active level's directive every turn so terse mode survives the whole session instead of drifting
-- back to verbose.
--
-- Must never throw or block the prompt: run it with --fail-open, and every step here is wrapped.
--
--   airsl run --fail-open --policy confined \
--     --allow-env AIRSSTACK_HOME --allow-env HOME \
--     --allow-read "$HOME_ROOT" --allow-write "$HOME_ROOT" \
--     hooks/concise-tracker.lua

local concise = require("lib.concise")
local path = airsstack.path

local home = airsstack.env.get("AIRSSTACK_HOME")
if not home or home == "" then
  home = path.join(airsstack.env.get("HOME") or "", ".airsstack")
end
local flag = concise.flag_path(home)

local ok, payload = pcall(airsstack.hook.payload)
local prompt = ok and type(payload) == "table" and payload.prompt or nil

if type(prompt) == "string" then
  local wanted = concise.classify(prompt)
  if wanted == "off" then
    concise.clear_level(flag)
  elseif wanted then
    pcall(concise.write_level, flag, wanted)
  end
end

-- Persistence: re-inject the active level's directive every turn, including the turns that said
-- nothing about concise mode. That re-injection is the whole reason this hook exists.
local active = concise.read_level(flag)
if active then
  airsstack.hook.context("UserPromptSubmit", concise.directive(active))
end
