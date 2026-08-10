-- Tests for lib/preflight — the decision table and the report shape.
--
--   airsl test --allow-read /tmp --allow-write /tmp plugins/airsstack-cmux/skills/cmux-control/scripts

local preflight = require("lib.preflight")
local json = airsstack.json

local CONTEXT = { socket = "/run/cmux.sock", workspace = "workspace:1", surface = "surface:2" }

-- A probe answering exactly what a case needs.
local function probe(overrides)
  local answers = {
    which = function() return "/usr/bin/cmux" end,
    version = function() return "cmux 0.64.17 (97)" end,
    socket_present = function() return true end,
    ping = function() return true end,
  }
  for key, value in pairs(overrides or {}) do
    answers[key] = value
  end
  return answers
end

return {
  a_healthy_surface_reports_ok_and_reachable = function()
    local report = preflight.check(CONTEXT, probe())
    assert(report.status == preflight.OK, report.status)
    assert(report.reachable == true)
    assert(report.version == "cmux 0.64.17 (97)", report.version)
  end,

  a_missing_binary_reports_unavailable_with_no_version = function()
    local report = preflight.check(CONTEXT, probe({ which = function() return nil end }))
    assert(report.status == preflight.UNAVAILABLE, report.status)
    assert(report.binary == "" and report.version == "", "nothing may be claimed about an absent binary")
    assert(report.reachable == false)
  end,

  an_absent_socket_reports_no_socket_and_stops_before_pinging = function()
    local pinged = false
    local report = preflight.check(CONTEXT, probe({
      socket_present = function() return false end,
      ping = function() pinged = true; return true end,
    }))
    assert(report.status == preflight.NO_SOCKET, report.status)
    assert(pinged == false, "there is nothing to ping without a socket")
  end,

  a_present_but_dead_socket_reports_unreachable = function()
    local report = preflight.check(CONTEXT, probe({ ping = function() return false end }))
    assert(report.status == preflight.UNREACHABLE, report.status)
    assert(report.reachable == false)
    assert(report.binary ~= "", "the binary was still found")
  end,

  a_binary_that_will_not_report_a_version_says_unknown = function()
    local report = preflight.check(CONTEXT, probe({ version = function() return nil end }))
    assert(report.version == "unknown", report.version)
    assert(report.status == preflight.OK, "an unreadable version does not make the surface unusable")
  end,

  the_caller_context_is_carried_through_untouched = function()
    local report = preflight.check(CONTEXT, probe({ which = function() return nil end }))
    assert(report.workspace == "workspace:1", report.workspace)
    assert(report.surface == "surface:2", report.surface)
    assert(report.socket == "/run/cmux.sock", report.socket)
  end,

  the_json_form_carries_every_documented_field = function()
    local decoded = json.decode(preflight.to_json(preflight.check(CONTEXT, probe())))
    for _, field in ipairs({ "status", "binary", "version", "socket", "workspace", "surface" }) do
      assert(type(decoded[field]) == "string", field .. " must be a string")
    end
    assert(decoded.reachable == true, "reachable is a boolean, not a string")
  end,

  the_text_form_labels_every_field = function()
    local text = preflight.to_text(preflight.check(CONTEXT, probe()))
    assert(text:find("cmux preflight: ok", 1, true), text)
    for _, label in ipairs({ "binary:", "version:", "socket:", "reachable:", "workspace:", "surface:" }) do
      assert(text:find(label, 1, true), label .. " missing from:\n" .. text)
    end
  end,

  the_default_socket_sits_under_the_state_directory = function()
    assert(preflight.default_socket("/home/me") == "/home/me/.local/state/cmux/cmux.sock")
  end,
}
