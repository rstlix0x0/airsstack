-- Building a cmux workspace layout: argument parsing and the split/command plan.
--
-- Split from the driver so the argument grammar can be exercised without cmux present. That is
-- where the refusals live — `--provider` and its siblings are rejected rather than passed through,
-- because agent spawning is deliberately out of this script's scope and a silently accepted flag
-- would make it look supported.

local M = {}

M.DIRECTIONS = { left = true, right = true, up = true, down = true }

-- Flags rejected on sight: this script is pure geometry plus an optional per-pane command.
M.OUT_OF_SCOPE = { ["--provider"] = true, ["--agent"] = true, ["--claude-teams"] = true }

-- Parses the argument list into `{name, splits, cmds}`, or nil plus a reason.
function M.parse(args)
  local plan = { name = "", splits = {}, cmds = {} }
  local index = 1

  while index <= #args do
    local flag = args[index]
    if flag == "--name" then
      plan.name = args[index + 1] or ""
      index = index + 2
    elseif flag == "--split" then
      local direction = args[index + 1] or ""
      if not M.DIRECTIONS[direction] then
        return nil, "--split must be left|right|up|down"
      end
      plan.splits[#plan.splits + 1] = direction
      index = index + 2
    elseif flag == "--cmd" then
      plan.cmds[#plan.cmds + 1] = args[index + 1] or ""
      index = index + 2
    elseif M.OUT_OF_SCOPE[flag] then
      return nil, flag .. " is out of scope (no agent spawning)"
    else
      return nil, "unknown argument: " .. flag
    end
  end

  if plan.name == "" then
    return nil, "--name is required"
  end
  return plan
end

return M
