#!/usr/bin/env python3
"""Unit tests for the airsstack enforcement dispatcher's pure units.

Run: python3 -m unittest discover -s plugins/airsstack/hooks -p 'test_*.py'
"""

import os
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import enforce  # noqa: E402


class TestGlobToRegex(unittest.TestCase):
    # The spec's verified 11-case baseline (§11), plus character classes.
    CASES = [
        ("crates/**/*.rs", "crates/clauders/src/lib.rs", True),
        ("crates/**/*.rs", "crates/lib.rs", True),
        ("crates/**/*.rs", "src/x.rs", False),
        ("**/*.rs", "lib.rs", True),
        ("**/*.rs", "crates/clauders/src/lib.rs", True),
        ("**/*.rs", "src/x.rs", True),
        ("src/gen/*.rs", "src/gen/a.rs", True),
        ("src/gen/*.rs", "src/gen/nested/a.rs", False),
        ("**/Cargo.toml", "Cargo.toml", True),
        ("**/Cargo.toml", "crates/clauders/Cargo.toml", True),
        ("**/*.rs", "src/main.py", False),
        ("**/v[0-9].rs", "src/v3.rs", True),
        ("**/v[0-9].rs", "src/vx.rs", False),
        ("**/v[!0-9].rs", "src/vx.rs", True),
        ("**/v[!0-9].rs", "src/v3.rs", False),
        ("**/a?.rs", "src/ab.rs", True),
        ("**/a?.rs", "src/abc.rs", False),
    ]

    def test_baseline(self):
        for pattern, path, expected in self.CASES:
            got = bool(enforce.glob_to_regex(pattern).match(path))
            self.assertEqual(got, expected, "%r vs %r" % (pattern, path))

    def test_unclosed_bracket_is_literal(self):
        self.assertTrue(enforce.glob_to_regex("a[b.rs").match("a[b.rs"))

    def test_dot_is_not_a_wildcard(self):
        self.assertFalse(enforce.glob_to_regex("**/*.rs").match("src/xxrs"))


class TestProjectKey(unittest.TestCase):
    def test_matches_the_shell_formula(self):
        """Python key must equal the sh formula in references/artifact-paths.md."""
        import subprocess
        script = (
            'if common_dir=$(git rev-parse --git-common-dir 2>/dev/null); then\n'
            '  abs=$(cd "$(dirname "$common_dir")" 2>/dev/null && pwd -P)/$(basename "$common_dir")\n'
            '  base=$(basename "$(dirname "$abs")")\n'
            'else\n'
            '  abs=$(pwd -P); base=$(basename "$abs")\n'
            'fi\n'
            "base=$(printf '%s' \"$base\" | LC_ALL=C tr -c 'A-Za-z0-9._-' '-')\n"
            "hash8=$(printf '%s' \"$abs\" | shasum | cut -c1-8)\n"
            'printf "%s-%s" "$base" "$hash8"\n'
        )
        here = os.path.dirname(os.path.abspath(__file__))
        expected = subprocess.check_output(
            ["sh", "-c", script], cwd=here
        ).decode("utf-8").strip()
        self.assertEqual(enforce.project_key(here), expected)

    def test_no_git_falls_back_to_cwd_hash(self):
        import tempfile
        with tempfile.TemporaryDirectory() as tmp:
            key = enforce.project_key(tmp)
            self.assertIsNotNone(key)
            self.assertRegex(key, r"^[A-Za-z0-9._-]+-[0-9a-f]{8}$")


class TestPathForMatching(unittest.TestCase):
    def test_inside_repo_returns_repo_relative(self):
        here = os.path.dirname(os.path.abspath(__file__))
        target = os.path.join(here, "enforce.py")
        self.assertEqual(
            enforce.path_for_matching(target, here),
            "plugins/airsstack/hooks/enforce.py",
        )

    def test_outside_repo_falls_back_to_basename(self):
        import tempfile
        with tempfile.TemporaryDirectory() as tmp:
            target = os.path.join(tmp, "sub", "lib.rs")
            self.assertEqual(enforce.path_for_matching(target, tmp), "lib.rs")

    def test_matches_any_glob(self):
        self.assertTrue(
            enforce.matches_any("crates/clauders/src/lib.rs", ["**/Cargo.toml", "**/*.rs"])
        )
        self.assertFalse(enforce.matches_any("README.md", ["**/*.rs"]))


class TestDesignDoc(unittest.TestCase):
    def setUp(self):
        self.root = "/tmp/aihome"
        self.sdd = os.path.join(self.root, "cc", "plugins", "sdd")

    def check(self, rel):
        return enforce.is_design_doc(os.path.join(self.sdd, rel), self.root)

    def test_specs_segment_matches(self):
        self.assertTrue(self.check("proj/specs/2026-01-01-x.md"))

    def test_plans_segment_matches(self):
        self.assertTrue(self.check("proj/plans/2026-01-01-x.md"))

    def test_substring_only_rejects(self):
        self.assertFalse(self.check("proj/myspecs/x.md"))
        self.assertFalse(self.check("proj/specsheet/x.md"))

    def test_nested_accidental_substring_rejects(self):
        # The pre-repair '/specs/' in fp test matched this by accident.
        self.assertFalse(self.check("proj/a/specs/b/plans/c.md"))

    def test_outside_sdd_root_rejects(self):
        self.assertFalse(enforce.is_design_doc("/elsewhere/specs/x.md", self.root))


class TestMarkerActive(unittest.TestCase):
    def test_found_at_ancestor_of_the_file(self):
        import tempfile
        with tempfile.TemporaryDirectory() as tmp:
            repo = os.path.join(tmp, "repo", "src", "deep")
            os.makedirs(repo)
            open(os.path.join(tmp, "repo", "Cargo.toml"), "w").close()
            target = os.path.join(repo, "lib.rs")
            # cwd is deliberately elsewhere: the search must start at the
            # FILE's directory, not at the session's working directory.
            self.assertTrue(enforce.marker_active(target, ["Cargo.toml"], cwd=tmp))

    def test_absent_above_the_file(self):
        import tempfile
        with tempfile.TemporaryDirectory() as tmp:
            marked = os.path.join(tmp, "marked")
            plain = os.path.join(tmp, "plain")
            os.makedirs(marked)
            os.makedirs(plain)
            open(os.path.join(marked, "Cargo.toml"), "w").close()
            target = os.path.join(plain, "lib.rs")
            self.assertFalse(enforce.marker_active(target, ["Cargo.toml"], cwd=marked))

    def test_empty_marker_list_is_inactive(self):
        self.assertFalse(enforce.marker_active("/tmp/x/lib.rs", [], cwd="/tmp"))

    def test_design_docs_anchor_on_cwd_not_the_file(self):
        """SDD specs/plans live under AIRSSTACK_HOME, outside every repo.

        Anchoring them on their own directory can never find a marker, which
        would silently kill design-phase enforcement (e2e case 4).
        """
        import tempfile
        with tempfile.TemporaryDirectory() as tmp:
            repo = os.path.join(tmp, "repo")
            sdd = os.path.join(tmp, "home", "cc", "plugins", "sdd", "proj", "specs")
            os.makedirs(repo)
            os.makedirs(sdd)
            open(os.path.join(repo, "Cargo.toml"), "w").close()
            spec = os.path.join(sdd, "2026-01-01-x.md")
            self.assertFalse(enforce.marker_active(spec, ["Cargo.toml"], cwd=repo))
            self.assertTrue(enforce.marker_active_in(repo, ["Cargo.toml"]))


if __name__ == "__main__":
    unittest.main()
