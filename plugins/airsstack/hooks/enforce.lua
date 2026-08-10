-- airsstack rule-enforcement dispatcher — PreToolUse(Read|Edit|Write) hook.
--
-- Reads the installed-plugins registry, keeps only airsstack-marketplace plugins, loads each one's
-- enforcement.json, and — for the file being edited — surfaces the matching guideline skill via
-- additionalContext.
--
-- Fail-open: never blocks, never denies. Run it with --fail-open, which is what stops a broken
-- hook from turning into a blocked Read. `--explain <path>` runs the same pipeline with a trace
-- attached and prints it.
--
--   airsl run --fail-open --policy confined \
--     --allow-env HOME --allow-env TMPDIR --allow-env AIRSSTACK_HOME \
--     --allow-env AIRSSTACK_ENFORCE_REGISTRY \
--     --allow-read / --allow-write "$TMPDIR" --allow-exec git \
--     hooks/enforce.lua

local enforce = require("lib.enforce")
local env = airsstack.env
local path = airsstack.path

-- The environment this run resolves against, read once.
local function settings()
  local home = env.get("AIRSSTACK_HOME")
  if not home or home == "" then
    home = path.join(env.get("HOME") or "", ".airsstack")
  end
  return {
    home = home,
    registry = enforce.registry_path(env),
    sentinel_dir = enforce.sentinel_dir(env),
  }
end

-- The doctor: a human-readable trace of the resolution pipeline for one path.
--
-- Drives the same `resolve` the hook drives, with a trace collector attached. The framework has
-- several paths that end in silence and are mutually indistinguishable from outside; this names
-- the one that was taken.
--
-- `resolve` claims sentinels as a side effect, so a second `--explain` of the same
-- session/agent/stack/phase reports "already claimed" from the doctor's own prior run.
local function explain(file_path)
  local config = settings()
  local cwd = path.absolute(".")
  local session, agent = "enforce-doctor", "doctor"

  local lines = {
    "runtime:  airsl (Lua)",
    "path:     " .. path.absolute(file_path),
    "cwd:      " .. cwd,
    "registry: " .. config.registry,
    "sdd root: " .. enforce.sdd_root(config.home),
    "sentinel dir: " .. config.sentinel_dir,
  }

  local held = enforce.held_sentinels(config.sentinel_dir, session, agent)
  lines[#lines + 1] = "sentinels held: " .. (#held > 0 and table.concat(held, ", ") or "none")

  local trace = {}
  local pointers = enforce.resolve({
    file_path = file_path,
    cwd = cwd,
    session_id = session,
    agent = agent,
    registry = config.registry,
    home = config.home,
    sentinel_dir = config.sentinel_dir,
  }, trace)

  for _, entry in ipairs(trace) do
    lines[#lines + 1] = "  " .. entry
  end
  lines[#lines + 1] = "outcome: " .. #pointers .. " pointer(s)"
  for _, text in ipairs(pointers) do
    lines[#lines + 1] = "  -> " .. text
  end

  local top = enforce.git(cwd, "rev-parse", "--show-toplevel")
  if top then
    local drift = enforce.parity_report(top, enforce.read_registry(config.registry))
    if #drift > 0 then
      -- `drift` also carries "source tree unreadable, parity unknown" lines. Counting those into
      -- the headline would claim the file IS out of sync when the very next line says that is
      -- precisely unknown, so only comparison results are counted.
      local out_of_sync = 0
      for _, entry in ipairs(drift) do
        if entry:find("MISSING from cache", 1, true) or entry:find("DIFFERS from source", 1, true) then
          out_of_sync = out_of_sync + 1
        end
      end
      lines[#lines + 1] = "parity: " .. out_of_sync .. " file(s) out of sync between repo and cache"
      for index = 1, math.min(#drift, 20) do
        lines[#lines + 1] = "  " .. drift[index]
      end
      if #drift > 20 then
        lines[#lines + 1] = "  (+" .. (#drift - 20) .. " more)"
      end
      lines[#lines + 1] = "  -> the dispatcher runs from the cache, so anything MISSING there "
        .. "is invisible to it. Start a session in the plugin repo's main "
        .. "worktree to let the cache guard backfill, or reinstall the plugin."
    elseif enforce.exists(path.join(top, "plugins")) then
      lines[#lines + 1] = "parity: repo and cache agree"
    end
  end

  return table.concat(lines, "\n") .. "\n"
end

-- The hook itself.
local function main()
  local payload = airsstack.hook.payload()
  if type(payload) ~= "table" then
    return
  end

  local tool_input = type(payload.tool_input) == "table" and payload.tool_input or {}
  local file_path = tool_input.file_path
  if type(file_path) ~= "string" or file_path == "" then
    return
  end

  local config = settings()
  local cwd = type(payload.cwd) == "string" and payload.cwd ~= "" and payload.cwd
    or path.absolute(".")

  enforce.prune_sentinels(config.sentinel_dir, airsstack.time.now())

  local pointers = enforce.resolve({
    file_path = file_path,
    cwd = cwd,
    session_id = type(payload.session_id) == "string" and payload.session_id or "",
    agent = type(payload.agent_id) == "string" and payload.agent_id ~= "" and payload.agent_id
      or "main",
    registry = config.registry,
    home = config.home,
    sentinel_dir = config.sentinel_dir,
  })

  if #pointers == 0 then
    return
  end

  airsstack.hook.emit({
    hookSpecificOutput = {
      hookEventName = "PreToolUse",
      additionalContext = table.concat(pointers, "\n"),
      permissionDecision = "defer",
    },
  })
end

if arg[1] == "--explain" then
  if not arg[2] or arg[2] == "" then
    airsstack.stdio.error("usage: enforce.lua --explain <path>\n")
    return
  end
  airsstack.stdio.write(explain(arg[2]))
else
  main()
end
