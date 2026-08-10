-- cmux-snap — snapshot-then-act wrapper for cmux browser surfaces.
--
-- Captures a fresh interactive snapshot, then performs the requested browser action with
-- --snapshot-after, so the agent sees the view before and after acting and is less likely to act
-- on a stale snapshot.
--
-- Usage: cmux-snap.lua <surface> <action> [args...]   e.g. cmux-snap.lua surface:2 click "button"
--
--   airsl run --policy confined --allow-env CMUX_QUIET --allow-exec cmux \
--     scripts/cmux-snap.lua <surface> <action> [args...]

local cmux = require("lib.cmux")

local surface = arg[1]
local action = arg[2]
if not surface or surface == "" then
  cmux.die("cmux-snap: <surface> is required (e.g. surface:2)")
end
if not action or action == "" then
  cmux.die("cmux-snap: <action> is required (e.g. click)")
end

-- `proc.run` captures output rather than letting it through, so each step's output is re-emitted.
-- The snapshot IS the payload the agent reads; dropping it would leave the wrapper doing half its
-- job.
local function forward(result)
  if result.stdout ~= "" then
    airsstack.stdio.write(result.stdout)
  end
  if result.stderr ~= "" then
    airsstack.stdio.error(result.stderr)
  end
end

local snapshot = cmux.run({ "browser", surface, "snapshot", "--interactive" }, false)
forward(snapshot)
if snapshot.status ~= 0 then
  cmux.die("cmux-snap: pre-snapshot failed")
end

local argv = { "browser", surface, action }
for index = 3, #arg do
  argv[#argv + 1] = arg[index]
end
argv[#argv + 1] = "--snapshot-after"

local acted = cmux.run(argv, false)
forward(acted)
if acted.status ~= 0 then
  cmux.die("cmux-snap: action " .. action .. " failed")
end
