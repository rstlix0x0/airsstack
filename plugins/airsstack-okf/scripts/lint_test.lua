-- Tests for lib/lint — the OKF v0.1 conformance rules and their severities.
--
--   airsl test --allow-read /tmp --allow-write /tmp --allow-exec git plugins/airsstack-okf/scripts

local lint = require("lib.lint")

-- Runs the checks over an in-memory bundle. `present` names the paths that exist, so link checking
-- needs no filesystem at all.
local function check(files, present)
  local names = {}
  for rel in pairs(files) do
    names[#names + 1] = rel
  end
  table.sort(names)

  local exists = {}
  for _, rel in ipairs(present or {}) do
    exists[rel] = true
  end

  return lint.check(names, function(rel)
    return files[rel]
  end, function(rel)
    return exists[rel] == true or files[rel] ~= nil
  end)
end

local function messages(findings, severity)
  local out = {}
  for _, finding in ipairs(findings) do
    if finding.severity == severity then
      out[#out + 1] = finding.file .. ": " .. finding.message
    end
  end
  return out
end

local COMPLETE = "---\ntype: concept\ntitle: T\ndescription: D\ntimestamp: 2026-01-01\n---\nBody\n"

return {
  a_complete_concept_document_produces_nothing = function()
    local findings = check({ ["a.md"] = COMPLETE })
    assert(#findings == 0, table.concat(messages(findings, "FAIL"), "; ")
      .. table.concat(messages(findings, "WARN"), "; "))
  end,

  a_missing_fence_is_a_hard_failure = function()
    local fails = messages(check({ ["a.md"] = "no frontmatter\n" }), "FAIL")
    assert(#fails == 1 and fails[1]:find("missing or unclosed", 1, true), fails[1])
  end,

  an_unclosed_fence_is_a_hard_failure = function()
    local fails = messages(check({ ["a.md"] = "---\ntype: concept\nbody\n" }), "FAIL")
    assert(#fails == 1 and fails[1]:find("missing or unclosed", 1, true), fails[1])
  end,

  an_empty_type_is_a_hard_failure = function()
    local fails = messages(check({ ["a.md"] = "---\ntype:\n---\n" }), "FAIL")
    assert(#fails == 1 and fails[1]:find("required field: type", 1, true), fails[1])
  end,

  a_missing_recommended_field_is_only_a_warning = function()
    local findings = check({ ["a.md"] = "---\ntype: concept\n---\n" })
    assert(select(1, lint.tally(findings)) == 0, "recommended fields must not fail the bar")
    assert(#messages(findings, "WARN") == 3, "one warning per recommended field")
  end,

  a_broken_absolute_link_is_only_a_warning = function()
    local findings = check({ ["a.md"] = COMPLETE:gsub("Body", "See [x](/nope.md)") })
    assert(select(1, lint.tally(findings)) == 0)
    local warns = messages(findings, "WARN")
    assert(#warns == 1 and warns[1]:find("broken link: /nope.md", 1, true), warns[1])
  end,

  a_link_that_resolves_is_not_reported = function()
    local findings = check({
      ["a.md"] = COMPLETE:gsub("Body", "See [x](/b.md)"),
      ["b.md"] = COMPLETE,
    })
    assert(#messages(findings, "WARN") == 0, table.concat(messages(findings, "WARN"), "; "))
  end,

  a_nested_index_may_not_carry_frontmatter = function()
    local fails = messages(check({ ["sub/index.md"] = "---\ntype: x\n---\n" }), "FAIL")
    assert(#fails == 1 and fails[1]:find("must not carry frontmatter", 1, true), fails[1])
  end,

  a_nested_index_without_frontmatter_is_fine = function()
    assert(#check({ ["sub/index.md"] = "# Section\n" }) == 0)
  end,

  the_root_index_may_carry_only_okf_version = function()
    assert(#check({ ["index.md"] = '---\nokf_version: "0.1"\n---\n# Index\n' }) == 0)

    local fails = messages(check({
      ["index.md"] = '---\nokf_version: "0.1"\nstray: value\n---\n',
    }), "FAIL")
    assert(#fails == 1 and fails[1]:find("only okf_version", 1, true), fails[1])
  end,

  the_root_index_without_frontmatter_is_fine = function()
    assert(#check({ ["index.md"] = "# Index\n" }) == 0)
  end,

  a_log_may_not_carry_frontmatter = function()
    local fails = messages(check({ ["log.md"] = "---\ntype: log\n---\n" }), "FAIL")
    assert(fails[1]:find("must not carry frontmatter", 1, true), fails[1])
  end,

  a_log_heading_must_be_an_iso_date = function()
    local fails = messages(check({ ["log.md"] = "## 2026-13\n- entry\n" }), "FAIL")
    assert(#fails == 1 and fails[1]:find("non-ISO date heading", 1, true), fails[1])
  end,

  a_conformant_log_produces_nothing = function()
    local text = "# Log\n\n## 2026-01-02\n\n- did a thing\n  continued here\n"
    assert(#check({ ["log.md"] = text }) == 0)
  end,

  a_loose_line_in_a_log_is_a_hard_failure = function()
    local fails = messages(check({ ["log.md"] = "## 2026-01-02\nloose prose\n" }), "FAIL")
    assert(#fails == 1 and fails[1]:find("not a list item", 1, true), fails[1])
  end,

  the_summary_line_counts_both_severities = function()
    local findings = check({ ["a.md"] = "---\ntype: concept\n---\n", ["b.md"] = "bare\n" })
    local report = lint.render(findings)
    assert(report:find("okf-lint: 1 failure(s), 3 warning(s)", 1, true), report)
  end,

  a_clean_bundle_still_prints_a_summary = function()
    assert(lint.render({}) == "okf-lint: 0 failure(s), 0 warning(s)\n")
  end,
}
