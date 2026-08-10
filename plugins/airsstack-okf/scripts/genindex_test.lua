-- Tests for lib/genindex — the deterministic bundle index.
--
--   airsl test --allow-read /tmp --allow-write /tmp --allow-exec git plugins/airsstack-okf/scripts

local genindex = require("lib.genindex")

local function render(files, existing)
  local names = {}
  for rel in pairs(files) do
    names[#names + 1] = rel
  end
  table.sort(names)

  local warnings = {}
  local text = genindex.render(names, existing, function(rel)
    return files[rel]
  end, function(line)
    warnings[#warnings + 1] = line
  end)
  return text, warnings
end

local function doc(title, description)
  local out = "---\ntype: concept\n"
  if title then
    out = out .. "title: " .. title .. "\n"
  end
  if description then
    out = out .. "description: " .. description .. "\n"
  end
  return out .. "---\nBody\n"
end

return {
  a_root_concept_is_listed_under_no_heading = function()
    local text = render({ ["a.md"] = doc("Alpha", "the first") })
    assert(text == "# Index\n\n- [Alpha](/a.md) — the first\n", string.format("%q", text))
  end,

  a_missing_description_renders_the_bare_link = function()
    local text = render({ ["a.md"] = doc("Alpha") })
    assert(text:find("- [Alpha](/a.md)\n", 1, true), string.format("%q", text))
  end,

  a_missing_title_falls_back_to_the_filename_stem = function()
    local text = render({ ["some-concept.md"] = doc(nil, "d") })
    assert(text:find("- [some-concept](/some-concept.md) — d", 1, true), text)
  end,

  a_subdirectory_becomes_its_own_section = function()
    local text = render({ ["sub/a.md"] = doc("A", "d") })
    assert(text:find("\n## sub\n\n- [A](/sub/a.md) — d\n", 1, true), string.format("%q", text))
  end,

  a_nested_subdirectory_is_listed_inside_its_top_section = function()
    local text = render({ ["sub/deep/a.md"] = doc("A", "d") })
    assert(text:find("## sub", 1, true), text)
    assert(not text:find("## sub/deep", 1, true), "only top-level directories get a section")
    assert(text:find("- [A](/sub/deep/a.md) — d", 1, true), text)
  end,

  sections_and_entries_are_ordered_deterministically = function()
    local text = render({
      ["z.md"] = doc("Z", "d"),
      ["a.md"] = doc("A", "d"),
      ["two/b.md"] = doc("B", "d"),
      ["one/c.md"] = doc("C", "d"),
    })
    assert(text:find("/a.md", 1, true) < text:find("/z.md", 1, true), "root entries sort by path")
    assert(text:find("## one", 1, true) < text:find("## two", 1, true), "sections sort by name")
  end,

  reserved_files_are_never_listed = function()
    local text = render({
      ["index.md"] = "# Index\n",
      ["log.md"] = "# Log\n",
      ["sub/index.md"] = "# Sub\n",
      ["sub/log.md"] = "# Sub log\n",
      ["a.md"] = doc("A", "d"),
    })
    assert(not text:find("index.md)", 1, true), text)
    assert(not text:find("log.md)", 1, true), text)
    assert(text:find("/a.md", 1, true), text)
  end,

  the_generators_own_temp_file_is_never_listed = function()
    local text = render({ ["index.md.tmp"] = doc("T", "d"), ["a.md"] = doc("A", "d") })
    assert(not text:find(".tmp", 1, true), text)
  end,

  unparseable_frontmatter_is_warned_about_and_skipped = function()
    local text, warnings = render({
      ["broken.md"] = "---\ntype: concept\nno close\n",
      ["a.md"] = doc("A", "d"),
    })
    assert(not text:find("broken.md", 1, true), text)
    assert(#warnings == 1 and warnings[1]:find("unparseable", 1, true), warnings[1])
  end,

  an_existing_marker_block_is_preserved_verbatim = function()
    local existing = '---\nokf_version: "0.1"\n---\n\n# Index\n\n- [stale](/gone.md)\n'
    local text = render({ ["a.md"] = doc("A", "d") }, existing)
    local marker = '---\nokf_version: "0.1"\n---\n'
    assert(text:sub(1, #marker) == marker, string.format("%q", text:sub(1, #marker)))
    assert(not text:find("stale", 1, true), "the old body must not survive")
  end,

  an_index_with_no_frontmatter_gains_none = function()
    local text = render({ ["a.md"] = doc("A", "d") }, "# Index\n\n- [stale](/gone.md)\n")
    assert(text:sub(1, 1) ~= "-", text)
    assert(text:sub(1, 7) == "# Index", string.format("%q", text:sub(1, 10)))
  end,

  an_unclosed_marker_block_is_not_preserved = function()
    -- Preserving it would copy a broken fence forward and make the generated index itself
    -- non-conformant.
    local text = render({ ["a.md"] = doc("A", "d") }, "---\nokf_version: 0.1\n")
    assert(text:sub(1, 7) == "# Index", string.format("%q", text:sub(1, 10)))
  end,

  an_empty_bundle_still_renders_a_heading = function()
    assert(render({}) == "# Index\n")
  end,

  the_same_bundle_renders_the_same_bytes_every_time = function()
    local files = {
      ["a.md"] = doc("A", "d"),
      ["sub/b.md"] = doc("B", "e"),
      ["sub/deep/c.md"] = doc("C", "f"),
    }
    local first = render(files)
    for _ = 1, 8 do
      assert(render(files) == first, "the index must be byte-reproducible")
    end
  end,
}
