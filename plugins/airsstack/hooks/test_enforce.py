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


if __name__ == "__main__":
    unittest.main()
