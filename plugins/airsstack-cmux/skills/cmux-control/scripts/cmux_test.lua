-- Tests for lib/cmux — reading refs out of cmux output.
--
--   airsl test --allow-read /tmp --allow-write /tmp plugins/airsstack-cmux/skills/cmux-control/scripts

local cmux = require("lib.cmux")

return {
  a_ref_is_read_out_of_a_human_line = function()
    assert(cmux.first_ref("OK workspace:3\n", "workspace") == "workspace:3")
    assert(cmux.first_ref("created surface:12 in workspace:3", "surface") == "surface:12")
  end,

  the_first_ref_wins = function()
    assert(cmux.first_ref("surface:1 surface:2", "surface") == "surface:1")
  end,

  output_carrying_no_ref_yields_nothing = function()
    assert(cmux.first_ref("OK\n", "workspace") == nil)
    assert(cmux.first_ref("", "surface") == nil)
  end,

  a_ref_of_another_kind_is_not_matched = function()
    assert(cmux.first_ref("OK workspace:3\n", "surface") == nil)
  end,

  the_first_line_is_taken_and_trimmed = function()
    assert(cmux.first_line("cmux 0.64.17 (97)\nextra\n") == "cmux 0.64.17 (97)")
    assert(cmux.first_line("trailing   \n") == "trailing")
    assert(cmux.first_line("") == "")
  end,
}
