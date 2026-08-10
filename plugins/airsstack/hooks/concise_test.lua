-- Tests for lib/concise — prompt classification and the persisted level.
--
--   airsl test --allow-read / --allow-write /tmp --allow-exec git plugins/airsstack/hooks

local concise = require("lib.concise")
local fs = airsstack.fs
local path = airsstack.path

local function flag()
  return concise.flag_path(fs.tempdir())
end

return {
  a_bare_slash_command_selects_the_default_level = function()
    assert(concise.classify("/concise") == concise.DEFAULT_LEVEL)
    assert(concise.classify("/airsstack:concise") == concise.DEFAULT_LEVEL)
  end,

  a_slash_command_with_a_level_selects_it = function()
    assert(concise.classify("/concise ultra") == "ultra")
    assert(concise.classify("/airsstack:concise lite") == "lite")
  end,

  a_slash_command_turning_it_off_says_off = function()
    for _, word in ipairs({ "off", "stop", "disable" }) do
      assert(concise.classify("/concise " .. word) == "off", word)
    end
  end,

  an_unknown_argument_is_not_an_instruction = function()
    -- Distinct from "off": the flag is left exactly as it was rather than guessed at.
    assert(concise.classify("/concise bogus") == nil)
  end,

  deactivation_beats_activation_in_the_same_sentence = function()
    -- Both rules match "stop being concise"; if activation ran first it would enable the mode the
    -- user just asked to leave.
    assert(concise.classify("stop being concise") == "off")
    assert(concise.classify("please turn off concise mode") == "off")
    assert(concise.classify("concise off") == "off")
  end,

  normal_and_verbose_mode_deactivate = function()
    assert(concise.classify("switch to normal mode") == "off")
    assert(concise.classify("verbose mode please") == "off")
  end,

  natural_language_activation_needs_both_a_subject_and_a_verb = function()
    assert(concise.classify("please be concise") == concise.DEFAULT_LEVEL)
    assert(concise.classify("be terse") == concise.DEFAULT_LEVEL)
    assert(concise.classify("the concise history of rust") == nil,
      "a subject with no instruction verb must not activate")
  end,

  a_level_named_in_prose_is_honoured = function()
    assert(concise.classify("use ultra concise mode") == "ultra")
    assert(concise.classify("go lite concise") == "lite")
  end,

  a_prompt_about_something_else_says_nothing = function()
    assert(concise.classify("what does this function do?") == nil)
    assert(concise.classify("") == nil)
  end,

  a_slash_command_is_anchored_at_the_start = function()
    -- Mentioning the command mid-sentence describes it rather than issuing it.
    assert(concise.classify("run /concise ultra to switch level") == nil)
  end,

  a_level_round_trips_through_the_flag_file = function()
    local file = flag()
    concise.write_level(file, "ultra")
    assert(concise.read_level(file) == "ultra")
  end,

  clearing_removes_the_level = function()
    local file = flag()
    concise.write_level(file, "full")
    concise.clear_level(file)
    assert(concise.read_level(file) == nil)
  end,

  clearing_an_absent_flag_is_not_an_error = function()
    concise.clear_level(flag())
  end,

  an_absent_flag_reads_as_no_level = function()
    assert(concise.read_level(flag()) == nil)
  end,

  a_flag_holding_an_unknown_level_reads_as_none = function()
    local file = flag()
    fs.mkdir(path.dirname(file))
    fs.write(file, '{"level":"screaming"}\n')
    assert(concise.read_level(file) == nil)
  end,

  a_flag_that_is_not_json_reads_as_none = function()
    local file = flag()
    fs.mkdir(path.dirname(file))
    fs.write(file, "not json at all\n")
    assert(concise.read_level(file) == nil)
  end,

  an_oversized_flag_is_refused_rather_than_parsed = function()
    local file = flag()
    fs.mkdir(path.dirname(file))
    fs.write(file, '{"level":"full","pad":"' .. string.rep("x", concise.MAX_FLAG_BYTES) .. '"}')
    assert(concise.read_level(file) == nil, "something else is writing here")
  end,

  every_level_has_a_directive_carrying_the_common_clause = function()
    for _, level in ipairs(concise.LEVELS) do
      local text = concise.directive(level)
      assert(text:find(level:upper(), 1, true), level)
      assert(text:find("Keep ALL technical substance", 1, true), level)
      assert(text:find("security warnings", 1, true), level)
    end
  end,
}
