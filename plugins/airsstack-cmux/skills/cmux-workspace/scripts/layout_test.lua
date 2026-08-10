-- Tests for lib/layout — the cmux-layout argument grammar.
--
--   airsl test --allow-read /tmp --allow-write /tmp plugins/airsstack-cmux/skills/cmux-workspace/scripts

local layout = require("lib.layout")

return {
  a_name_alone_is_a_valid_plan = function()
    local plan = layout.parse({ "--name", "review" })
    assert(plan.name == "review", plan.name)
    assert(#plan.splits == 0 and #plan.cmds == 0)
  end,

  splits_keep_their_order = function()
    local plan = layout.parse({ "--name", "x", "--split", "right", "--split", "down" })
    assert(table.concat(plan.splits, ",") == "right,down", table.concat(plan.splits, ","))
  end,

  every_documented_direction_is_accepted = function()
    for direction in pairs(layout.DIRECTIONS) do
      assert(layout.parse({ "--name", "x", "--split", direction }), direction)
    end
  end,

  an_unknown_direction_is_refused = function()
    local plan, reason = layout.parse({ "--name", "x", "--split", "sideways" })
    assert(plan == nil)
    assert(reason:find("left|right|up|down", 1, true), reason)
  end,

  commands_keep_their_order_and_empty_slots = function()
    local plan = layout.parse({ "--name", "x", "--cmd", "vim", "--cmd", "", "--cmd", "top" })
    assert(#plan.cmds == 3, "an empty command still consumes its pane slot")
    assert(plan.cmds[1] == "vim" and plan.cmds[2] == "" and plan.cmds[3] == "top")
  end,

  a_missing_name_is_refused = function()
    local plan, reason = layout.parse({ "--split", "right" })
    assert(plan == nil)
    assert(reason:find("--name is required", 1, true), reason)
  end,

  agent_spawning_flags_are_rejected_rather_than_ignored = function()
    -- Silently accepting one would make agent spawning look supported when it is deliberately out
    -- of scope.
    for flag in pairs(layout.OUT_OF_SCOPE) do
      local plan, reason = layout.parse({ "--name", "x", flag, "claude" })
      assert(plan == nil, flag .. " must be refused")
      assert(reason:find("out of scope", 1, true), reason)
    end
  end,

  an_unknown_argument_is_refused = function()
    local plan, reason = layout.parse({ "--name", "x", "--nonsense" })
    assert(plan == nil)
    assert(reason:find("unknown argument", 1, true), reason)
  end,
}
