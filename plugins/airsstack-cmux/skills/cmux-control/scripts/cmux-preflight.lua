-- cmux-preflight — assert the cmux control surface is usable before automation.
--
-- Checks: cmux binary on PATH; control socket present and reachable (cmux ping); reports version
-- and caller workspace/surface context. Human summary by default, machine JSON with --json.
-- Exits 0 when healthy, non-zero (with the summary on stderr) when cmux is unavailable, so a
-- caller can stop early.
--
--   airsl run --policy confined \
--     --allow-env HOME --allow-env CMUX_SOCKET_PATH --allow-env CMUX_WORKSPACE_ID \
--     --allow-env CMUX_SURFACE_ID --allow-env CMUX_QUIET \
--     --allow-read "$HOME/.local/state/cmux" --allow-exec cmux \
--     scripts/cmux-preflight.lua [--json]

local preflight = require("lib.preflight")
local env = airsstack.env

local as_json = arg[1] == "--json"

local socket = env.get("CMUX_SOCKET_PATH")
if not socket or socket == "" then
  socket = preflight.default_socket(env.get("HOME") or "")
end

local report = preflight.check({
  socket = socket,
  workspace = env.get("CMUX_WORKSPACE_ID") or "",
  surface = env.get("CMUX_SURFACE_ID") or "",
}, preflight.live_probe())

local rendered = as_json and preflight.to_json(report) or preflight.to_text(report)

if report.status == preflight.OK then
  airsstack.stdio.write(rendered)
  return
end

-- The summary goes to stderr on every unhealthy path, so a caller reading stdout sees nothing it
-- could mistake for a healthy surface.
airsstack.stdio.error(rendered)
error("cmux preflight: " .. report.status, 0)
