-- Context Handoff session manager for the airsstack orchestration.
--
-- Single source of truth (code side) for the handoff tree path, the session liveness lease, and
-- pruning. Prose mirror: `skills/process-guidelines/references/context-handoff.md`. The two MUST
-- agree — change one, change the other.
--
-- Subcommands:
--   init                 resolve base, mint a session, write .active, prune, print dir + id
--   beat <session-dir>   refresh the session's .active lease (heartbeat)
--   end  <session-dir>   remove the session's .active lease (clean close)
--
--   airsl run --policy confined \
--     --allow-env AIRSSTACK_HANDOFF_KEEP --allow-env AIRSSTACK_HANDOFF_GRACE \
--     --allow-read . --allow-write . --allow-exec git \
--     scripts/handoff.lua {init|beat <dir>|end <dir>}

local handoff = require("lib.handoff")
local env = airsstack.env
local stdio = airsstack.stdio

local function die(message)
  stdio.error(message .. "\n")
  error(message, 0)
end

local function number_from(name, fallback)
  local value = tonumber(env.get(name) or "")
  if not value or value < 0 then
    return fallback
  end
  return math.floor(value)
end

local command = arg[1]

if command == "init" then
  local directory, id = handoff.init(
    airsstack.path.absolute("."),
    os.date("%Y%m%d-%H%M%S"),
    number_from("AIRSSTACK_HANDOFF_KEEP", handoff.DEFAULT_KEEP),
    airsstack.time.now(),
    number_from("AIRSSTACK_HANDOFF_GRACE", handoff.DEFAULT_GRACE_MINUTES)
  )
  stdio.write(directory .. "\n" .. id .. "\n")
elseif command == "beat" then
  if not arg[2] or arg[2] == "" then
    die("beat: missing <session-dir>")
  end
  if not handoff.beat(arg[2]) then
    die("beat: no such session dir: " .. arg[2])
  end
elseif command == "end" then
  if not arg[2] or arg[2] == "" then
    die("end: missing <session-dir>")
  end
  handoff.close(arg[2])
else
  die("usage: handoff.lua {init|beat <dir>|end <dir>}")
end
