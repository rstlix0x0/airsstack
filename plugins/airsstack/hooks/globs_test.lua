-- Tests for lib/globs — the manifest glob semantics.
--
--   airsl test --allow-read /tmp --allow-write /tmp --allow-exec git plugins/airsstack/hooks

local globs = require("lib.globs")

local function matches(pattern, candidate)
  return globs.matches_any(candidate, { pattern })
end

return {
  a_star_does_not_cross_a_path_separator = function()
    -- `airsstack.glob` agrees with this now; it did not when this module was written, and the
    -- host module was fixed rather than worked around. Pinned here anyway: this compiler is what
    -- the manifests are actually matched against.
    assert(matches("*.rs", "main.rs") == true)
    assert(matches("*.rs", "src/main.rs") == false)
    assert(matches("src/*.rs", "src/main.rs") == true)
    assert(matches("src/*.rs", "src/a/b.rs") == false)
  end,

  a_leading_double_star_matches_zero_segments = function()
    -- `**/Cargo.toml` must match a root-level Cargo.toml, this repository's most important Rust
    -- file. A `**/` that meant "one or more segments" would silently exempt it.
    assert(matches("**/Cargo.toml", "Cargo.toml") == true)
    assert(matches("**/Cargo.toml", "crates/a/Cargo.toml") == true)
    assert(matches("**/*.rs", "main.rs") == true)
    assert(matches("**/*.rs", "a/b/c/x.rs") == true)
  end,

  a_bare_double_star_matches_across_separators = function()
    assert(matches("**", "a/b/c") == true)
    assert(matches("docs/**/*.md", "docs/b.md") == true)
    assert(matches("docs/**/*.md", "docs/a/b.md") == true)
  end,

  a_question_mark_is_exactly_one_non_separator_character = function()
    assert(matches("?.rs", "a.rs") == true)
    assert(matches("?.rs", "ab.rs") == false)
    assert(matches("?.rs", "a/b.rs") == false)
  end,

  a_character_class_selects_and_negates = function()
    assert(matches("[ab].rs", "a.rs") == true)
    assert(matches("[ab].rs", "c.rs") == false)
    assert(matches("[!ab].rs", "c.rs") == true)
    assert(matches("[!ab].rs", "a.rs") == false)
  end,

  a_leading_bracket_inside_a_class_is_a_literal_member = function()
    assert(matches("[]ab].rs", "].rs") == true)
  end,

  an_unclosed_bracket_is_a_literal = function()
    assert(matches("[unclosed", "[unclosed") == true)
  end,

  regex_metacharacters_in_a_pattern_are_literal = function()
    assert(matches("a.b", "a.b") == true)
    assert(matches("a.b", "axb") == false, "a dot must not be a wildcard")
    assert(matches("a+b", "a+b") == true)
    assert(matches("(x).rs", "(x).rs") == true)
  end,

  the_pattern_is_anchored_at_both_ends = function()
    assert(matches("*.rs", "main.rss") == false)
    assert(matches("*.rs", "xmain.rs") == true, "a leading wildcard still matches a prefix")
    assert(matches("main.rs", "src/main.rs") == false)
  end,

  a_candidate_matches_when_any_pattern_hits = function()
    assert(globs.matches_any("Cargo.toml", { "**/*.rs", "**/Cargo.toml" }) == true)
    assert(globs.matches_any("README.md", { "**/*.rs", "**/Cargo.toml" }) == false)
  end,

  an_empty_pattern_list_matches_nothing = function()
    assert(globs.matches_any("a.rs", {}) == false)
    assert(globs.matches_any("a.rs", nil) == false)
  end,

  a_malformed_glob_disables_itself_rather_than_the_manifest = function()
    -- One unusable pattern must not take the rules beside it down too.
    assert(globs.matches_any("a.rs", { "[[:bogus:]", "**/*.rs" }) == true)
  end,
}
