-- Shared helpers for the cmux control scripts: running the binary and reading its refs.
--
-- Every cmux invocation goes through `run`, which is the one place `CMUX_QUIET` is set. `proc.run`
-- takes an argv array and no per-call environment, so the quiet flag travels through the process
-- environment overlay — which `proc.run` hands to the child.

local M = {}

-- Runs cmux with `argv`, quietly. Returns the captured result.
function M.run(argv, quiet)
  if quiet then
    pcall(airsstack.env.set, "CMUX_QUIET", "1")
  end
  local command = { "cmux" }
  for _, value in ipairs(argv) do
    command[#command + 1] = value
  end
  local ok, result = pcall(airsstack.proc.run, command)
  if not ok then
    return { status = 127, stdout = "", stderr = tostring(result) }
  end
  return result
end

-- The first `<kind>:<n>` ref in `text`, or nil.
--
-- cmux reports refs inside a human line (`OK workspace:3`), and the ref is the only part a caller
-- may act on — matching it explicitly is what keeps targeting pinned to what a command created
-- rather than to whatever happens to be focused.
function M.first_ref(text, kind)
  return text:match("(" .. kind .. ":%d+)")
end

-- The first line of `text`, trimmed.
function M.first_line(text)
  local line = text:match("^([^\n]*)")
  return (line:gsub("%s+$", ""))
end

-- Writes `message` to stderr and fails the script, which the CLI turns into exit 1.
function M.die(message)
  airsstack.stdio.error(message .. "\n")
  error(message, 0)
end

return M
