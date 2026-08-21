-- The skill documents the command; the case runs exactly what it documents.
return {
  skill_command_runs_what_the_skill_documents = function(t)
    local command = t.skill_command("demo", 1)
    local expected = [[sh "${CLAUDE_PLUGIN_ROOT}/hooks/gate.sh"]]
    assert(
      command == expected,
      "unexpected command: " .. tostring(command)
    )

    local result = t.script({ "sh", "-c", command }, { stdin = '{"tool_name":"Read"}' })
    assert(result.exit == 0, "documented command failed: exit " .. tostring(result.exit))
  end,
}
