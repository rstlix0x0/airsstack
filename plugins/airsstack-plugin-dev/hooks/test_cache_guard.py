#!/usr/bin/env python3
"""Unit tests for the airsstack-plugin-dev cache delivery guard.

Run: python3 -m unittest discover -s plugins/airsstack-plugin-dev/hooks -p 'test_*.py'
"""

import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import cache_guard  # noqa: E402


def git(cwd, *args):
    return subprocess.check_output(
        ["git"] + list(args), cwd=cwd, stderr=subprocess.DEVNULL
    ).decode("utf-8").strip()


def make_repo(root, marketplace_name="airsstack"):
    """A git repo with one committed plugin and a marketplace manifest."""
    os.makedirs(os.path.join(root, ".claude-plugin"))
    if marketplace_name is not None:
        with open(os.path.join(root, ".claude-plugin", "marketplace.json"), "w") as fh:
            json.dump({"name": marketplace_name, "plugins": []}, fh)
    pdir = os.path.join(root, "plugins", "demo", ".claude-plugin")
    os.makedirs(pdir)
    with open(os.path.join(pdir, "plugin.json"), "w") as fh:
        json.dump({"name": "demo", "version": "0.1.0"}, fh)
    git(root, "init", "-q")
    git(root, "config", "user.email", "t@example.com")
    git(root, "config", "user.name", "t")
    git(root, "add", "-A")
    git(root, "commit", "-qm", "init")
    return root


class TestActivation(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp()

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def test_main_worktree_activates(self):
        repo = make_repo(os.path.join(self.tmp, "repo"))
        self.assertTrue(cache_guard.is_main_worktree(repo))

    def test_linked_worktree_does_not_activate(self):
        repo = make_repo(os.path.join(self.tmp, "repo"))
        linked = os.path.join(self.tmp, "linked")
        git(repo, "worktree", "add", "-q", "-b", "wt", linked)
        self.assertTrue(cache_guard.is_main_worktree(repo))
        self.assertFalse(cache_guard.is_main_worktree(linked))

    def test_subdirectories_agree_with_their_worktree(self):
        """`--git-common-dir` is cwd-relative in the main tree, absolute in a
        linked one; a session's cwd is not always the repo root."""
        repo = make_repo(os.path.join(self.tmp, "repo"))
        linked = os.path.join(self.tmp, "linked")
        git(repo, "worktree", "add", "-q", "-b", "wt", linked)
        main_sub = os.path.join(repo, "plugins", "demo", ".claude-plugin")
        linked_sub = os.path.join(linked, "plugins", "demo", ".claude-plugin")
        self.assertTrue(cache_guard.is_main_worktree(main_sub))
        self.assertFalse(cache_guard.is_main_worktree(linked_sub))

    def test_non_git_does_not_activate(self):
        plain = os.path.join(self.tmp, "plain")
        os.makedirs(plain)
        self.assertFalse(cache_guard.is_main_worktree(plain))

    def test_marketplace_must_be_airsstack(self):
        repo = make_repo(os.path.join(self.tmp, "ok"))
        self.assertTrue(cache_guard.is_airsstack_marketplace(repo))
        other = make_repo(os.path.join(self.tmp, "other"), marketplace_name="somebody-else")
        self.assertFalse(cache_guard.is_airsstack_marketplace(other))
        none = make_repo(os.path.join(self.tmp, "none"), marketplace_name=None)
        self.assertFalse(cache_guard.is_airsstack_marketplace(none))


if __name__ == "__main__":
    unittest.main()
