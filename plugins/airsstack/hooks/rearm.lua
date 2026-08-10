-- airsstack enforcement re-arm — SessionStart(compact) hook.
--
-- Compaction drops the injected additionalContext out of the window, but the session_id survives
-- it (measured: one sessionId across a transcript spanning a compact event). Without this the
-- dispatcher's one-shot-per-context sentinel would keep the rule suppressed for the rest of the
-- session. Unlinking this session's sentinels lets the pointer re-enter context on the next
-- Read/Edit.
--
--   airsl run --fail-open --policy confined \
--     --allow-env TMPDIR --allow-write "$TMPDIR" --allow-read "$TMPDIR" \
--     hooks/rearm.lua

local enforce = require("lib.enforce")

local ok, payload = pcall(airsstack.hook.payload)
if not ok or type(payload) ~= "table" then
  return
end

local session_id = type(payload.session_id) == "string" and payload.session_id or ""
enforce.clear_session(enforce.sentinel_dir(airsstack.env), session_id)
