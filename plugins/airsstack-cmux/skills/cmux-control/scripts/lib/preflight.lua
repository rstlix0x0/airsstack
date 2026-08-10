-- The cmux control-surface preflight check.
--
-- Split from the driver so the report shape can be asserted without a cmux binary present: the
-- JSON form is what a caller parses to decide whether to proceed, and its field names are the
-- contract.

local cmux = require("lib.cmux")
local json = airsstack.json

local M = {}

-- The statuses this check can report, in the order they are ruled out.
M.UNAVAILABLE = "unavailable"
M.NO_SOCKET = "no-socket"
M.UNREACHABLE = "unreachable"
M.OK = "ok"

-- The default control socket, when CMUX_SOCKET_PATH does not name one.
function M.default_socket(home)
  return airsstack.path.join(home, ".local", "state", "cmux", "cmux.sock")
end

-- The machine-readable form.
function M.to_json(report)
  return json.encode({
    status = report.status,
    binary = report.binary,
    version = report.version,
    socket = report.socket,
    reachable = report.reachable,
    workspace = report.workspace,
    surface = report.surface,
  }) .. "\n"
end

-- The human-readable form.
function M.to_text(report)
  return table.concat({
    "cmux preflight: " .. report.status,
    "  binary:    " .. report.binary,
    "  version:   " .. report.version,
    "  socket:    " .. report.socket,
    "  reachable: " .. tostring(report.reachable),
    "  workspace: " .. report.workspace,
    "  surface:   " .. report.surface,
    "",
  }, "\n")
end

-- The preflight report. `probe` answers the three questions that need the outside world, so the
-- decision table itself stays testable.
function M.check(context, probe)
  local report = {
    status = M.UNAVAILABLE,
    binary = "",
    version = "",
    socket = context.socket,
    reachable = false,
    workspace = context.workspace,
    surface = context.surface,
  }

  local binary = probe.which()
  if not binary or binary == "" then
    return report
  end
  report.binary = binary
  report.version = probe.version() or "unknown"

  if not probe.socket_present(context.socket) then
    report.status = M.NO_SOCKET
    return report
  end

  if not probe.ping() then
    report.status = M.UNREACHABLE
    return report
  end

  report.status = M.OK
  report.reachable = true
  return report
end

-- The probe that talks to the real cmux.
function M.live_probe()
  return {
    which = function()
      local ok, found = pcall(airsstack.proc.which, "cmux")
      return ok and found or nil
    end,
    version = function()
      local result = cmux.run({ "version" }, false)
      if result.status ~= 0 then
        return nil
      end
      local line = cmux.first_line(result.stdout)
      return line ~= "" and line or nil
    end,
    -- `fs` reports a socket's kind as neither dir nor symlink, so this is presence plus
    -- "not a directory" rather than the shell's `[ -S ]`. A regular file sitting at the socket
    -- path therefore reports `unreachable` (the ping fails) where the shell said `no-socket`;
    -- both are non-zero and both name a socket that cannot be used.
    socket_present = function(socket)
      local ok, found = pcall(airsstack.fs.exists, socket)
      if not ok or not found then
        return false
      end
      local read, is_dir = pcall(airsstack.fs.is_dir, socket)
      return read and not is_dir
    end,
    ping = function()
      return cmux.run({ "ping" }, false).status == 0
    end,
  }
end

return M
