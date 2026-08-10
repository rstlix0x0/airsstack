-- Tests for lib/okf — frontmatter structure, markers, walking and link extraction.
--
--   airsl test --allow-read /tmp --allow-write /tmp --allow-exec git plugins/airsstack-okf/scripts

local okf = require("lib.okf")
local fs = airsstack.fs
local path = airsstack.path

local function bundle(files)
  local root = fs.tempdir()
  for rel, text in pairs(files) do
    local directory = path.dirname(path.join(root, rel))
    fs.mkdir(directory)
    fs.write(path.join(root, rel), text)
  end
  return root
end

return {
  a_well_formed_fence_reports_where_it_closes = function()
    assert(okf.fence_end(okf.lines("---\ntype: concept\n---\nBody\n")) == 3)
  end,

  a_file_with_no_leading_fence_has_none = function()
    assert(okf.fence_end(okf.lines("Body only\n")) == nil)
  end,

  an_unclosed_fence_is_not_a_fence = function()
    assert(okf.fence_end(okf.lines("---\ntype: concept\nBody\n")) == nil)
    assert(okf.parseable(okf.lines("---\ntype: concept\nBody\n")) == false)
  end,

  a_fence_is_distinguished_from_an_unclosed_one = function()
    -- Both open with `---`; only one of them parses, and the linter reports them differently.
    local unclosed = okf.lines("---\nkey: value\n")
    assert(okf.has_fence(unclosed) == true)
    assert(okf.parseable(unclosed) == false)
  end,

  a_field_is_read_with_its_quotes_stripped = function()
    local lines = okf.lines('---\ntitle: "Quoted"\nother: \'single\'\nplain: bare\n---\n')
    assert(okf.field(lines, "title") == "Quoted")
    assert(okf.field(lines, "other") == "single")
    assert(okf.field(lines, "plain") == "bare")
  end,

  an_absent_field_reads_as_nil_and_an_empty_one_as_the_empty_string = function()
    local lines = okf.lines("---\ntype:\n---\n")
    assert(okf.field(lines, "type") == "", "an empty value is not the same as an absent key")
    assert(okf.field(lines, "missing") == nil)
  end,

  a_field_after_the_closing_fence_is_not_frontmatter = function()
    local lines = okf.lines("---\ntype: concept\n---\ntitle: in the body\n")
    assert(okf.field(lines, "title") == nil)
  end,

  the_okf_version_marker_identifies_a_bundle_index = function()
    local root = bundle({
      ["index.md"] = '---\nokf_version: "0.1"\n---\n# Index\n',
      ["other.md"] = "---\ntype: concept\n---\n",
    })
    assert(okf.has_marker(path.join(root, "index.md")) == true)
    assert(okf.has_marker(path.join(root, "other.md")) == false)
    assert(okf.has_marker(path.join(root, "absent.md")) == false)
  end,

  an_empty_marker_value_does_not_mark_a_bundle = function()
    local root = bundle({ ["index.md"] = "---\nokf_version:\n---\n" })
    assert(okf.has_marker(path.join(root, "index.md")) == false)
  end,

  the_walk_finds_markdown_at_every_depth_in_sorted_order = function()
    local root = bundle({
      ["z.md"] = "x",
      ["a.md"] = "x",
      ["sub/deep/m.md"] = "x",
      ["notes.txt"] = "x",
    })
    local found = okf.markdown_files(root)
    assert(table.concat(found, ",") == "a.md,sub/deep/m.md,z.md", table.concat(found, ","))
  end,

  a_hidden_directory_is_never_walked = function()
    local root = bundle({ ["good.md"] = "x", [".obsidian/cache.md"] = "x" })
    local found = okf.markdown_files(root)
    assert(table.concat(found, ",") == "good.md", table.concat(found, ","))
  end,

  absolute_links_are_extracted_deduplicated_and_sorted = function()
    local text = "See [a](/b.md), [again](/b.md) and [c](/sub/c.md).\n"
    local found = okf.absolute_links(text)
    assert(table.concat(found, ",") == "b.md,sub/c.md", table.concat(found, ","))
  end,

  a_relative_link_is_not_checked = function()
    -- Only the recommended absolute form is checked, which is what the shell original grepped for.
    assert(#okf.absolute_links("See [a](b.md) and [c](./d.md).\n") == 0)
  end,

  an_explicit_path_wins_over_discovery = function()
    local root = bundle({ ["index.md"] = '---\nokf_version: "0.1"\n---\n' })
    assert(okf.resolve_root(root, root) == okf.absolute_dir(root))
  end,

  an_explicit_path_that_is_not_a_directory_is_refused = function()
    local resolved, reason = okf.resolve_root("/nonexistent-bundle-path", "/tmp")
    assert(resolved == nil)
    assert(reason:find("not a directory", 1, true), reason)
  end,

  a_directory_with_no_bundle_reports_that_rather_than_guessing = function()
    local resolved, reason = okf.resolve_root(nil, bundle({ ["notes.md"] = "x" }))
    assert(resolved == nil)
    assert(reason:find("no OKF bundle found", 1, true), reason)
  end,

  the_conventional_knowledge_directory_is_found_without_a_scan = function()
    local root = bundle({ ["knowledge/index.md"] = '---\nokf_version: "0.1"\n---\n' })
    local found = okf.resolve_root(nil, root)
    assert(found == path.join(okf.absolute_dir(root), "knowledge"), tostring(found))
  end,

  two_marked_bundles_are_reported_as_ambiguous = function()
    local root = bundle({
      ["one/index.md"] = '---\nokf_version: "0.1"\n---\n',
      ["two/index.md"] = '---\nokf_version: "0.1"\n---\n',
    })
    local found, reason = okf.resolve_root(nil, root)
    assert(found == nil, "two candidates must not resolve to one")
    assert(reason:find("multiple bundle candidates", 1, true), reason)
  end,
}
