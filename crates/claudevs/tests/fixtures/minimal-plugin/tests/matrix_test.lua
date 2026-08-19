-- Generated data cases + one scripted case: the four case shapes in one file.
local cases = {}
for _, f in ipairs({ "Cargo.lock", "poetry.lock" }) do
  cases["blocks_" .. f:gsub("%.", "_")] = {
    event = "PreToolUse",
    payload = { tool_input = { file_path = f } },
    expect = { decision = "deny" },
  }
end

cases.rust_files_get_the_guideline_pointer = function(t)
  local reply = t.hook("PreToolUse", { tool_input = { file_path = "src/main.rs" } })
  assert(reply.decision == "defer", "expected defer, got " .. tostring(reply.decision))
  assert(reply.context:find("rust%-guidelines"), "context should name the guideline")
end

return cases
