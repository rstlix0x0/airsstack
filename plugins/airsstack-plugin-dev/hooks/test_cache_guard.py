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


class TestBackfill(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp()
        self.src = os.path.join(self.tmp, "src")
        self.cache = os.path.join(self.tmp, "cache")
        os.makedirs(os.path.join(self.src, "hooks"))
        os.makedirs(self.cache)
        self._write(self.src, "enforcement.json", '{"stack":"rust"}')
        self._write(self.src, "hooks/x.sh", "echo new\n")

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def _sync(self):
        """Sync with the containment guard LIVE, rooted at this test's tmpdir.

        The guard is never disabled for tests — it is pointed somewhere else —
        so every case below exercises it rather than bypassing it.
        """
        return cache_guard.sync_tree(self.src, self.cache, containment_root=self.tmp)

    @staticmethod
    def _write(root, rel, text):
        path = os.path.join(root, rel)
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "w") as fh:
            fh.write(text)

    def _read(self, root, rel):
        with open(os.path.join(root, rel)) as fh:
            return fh.read()

    def test_missing_file_is_copied(self):
        result = self._sync()
        self.assertIn("enforcement.json", result["copied"])
        self.assertEqual(self._read(self.cache, "enforcement.json"), '{"stack":"rust"}')

    def test_differing_file_is_updated(self):
        self._write(self.cache, "hooks/x.sh", "echo old\n")
        result = self._sync()
        self.assertIn(os.path.join("hooks", "x.sh"), result["copied"])
        self.assertEqual(self._read(self.cache, "hooks/x.sh"), "echo new\n")

    def test_identical_file_is_not_recopied(self):
        self._sync()
        result = self._sync()
        self.assertEqual(result["copied"], [])

    def test_cache_only_file_is_reported_and_kept(self):
        self._write(self.cache, "hooks/enforce.js", "// removed upstream\n")
        result = self._sync()
        self.assertIn(os.path.join("hooks", "enforce.js"), result["extras"])
        self.assertTrue(os.path.exists(os.path.join(self.cache, "hooks", "enforce.js")))

    def test_ignored_names_never_appear_as_extras(self):
        self._write(self.cache, ".in_use", "")
        self._write(self.cache, ".DS_Store", "")
        os.makedirs(os.path.join(self.cache, ".git"))
        self._write(self.cache, ".git/HEAD", "ref: refs/heads/main\n")
        result = self._sync()
        self.assertEqual(result["extras"], [])

    def test_source_ignored_names_are_not_copied(self):
        self._write(self.src, ".DS_Store", "")
        result = self._sync()
        self.assertNotIn(".DS_Store", result["copied"])

    def test_destination_outside_the_containment_root_is_refused(self):
        """The guard is the only thing bounding an unattended hook's writes."""
        elsewhere = os.path.join(self.tmp, "elsewhere")
        os.makedirs(elsewhere)
        result = cache_guard.sync_tree(
            self.src, elsewhere, containment_root=os.path.join(self.tmp, "cache")
        )
        self.assertEqual(result["copied"], [])
        self.assertFalse(os.path.exists(os.path.join(elsewhere, "enforcement.json")))

    def test_containment_root_defaults_to_the_install_cache(self):
        """Production callers must not have to remember to pass the root."""
        result = cache_guard.sync_tree(self.src, self.cache)
        self.assertEqual(result["copied"], [])
        self.assertFalse(os.path.exists(os.path.join(self.cache, "enforcement.json")))


class TestPluginDiscovery(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp()

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def test_lists_plugin_dirs_with_a_manifest(self):
        top = os.path.join(self.tmp, "repo")
        os.makedirs(os.path.join(top, "plugins", "alpha", ".claude-plugin"))
        os.makedirs(os.path.join(top, "plugins", "beta", ".claude-plugin"))
        os.makedirs(os.path.join(top, "plugins", "notaplugin"))
        for name in ("alpha", "beta"):
            path = os.path.join(top, "plugins", name, ".claude-plugin", "plugin.json")
            with open(path, "w") as fh:
                json.dump({"name": name, "version": "0.1.0"}, fh)
        self.assertEqual(cache_guard.source_plugins(top), ["alpha", "beta"])

    def test_no_plugins_dir_returns_empty(self):
        self.assertEqual(cache_guard.source_plugins(self.tmp), [])

    def test_cache_dirs_come_from_the_registry(self):
        registry = {
            "plugins": {
                "alpha@airsstack": [
                    {"installPath": "/cache/alpha/0.1.0"},
                    {"installPath": "/cache/alpha/0.1.0"},
                ],
                "alpha@somewhere-else": [{"installPath": "/other/alpha"}],
            }
        }
        self.assertEqual(
            cache_guard.cache_dirs(registry, "alpha"), ["/cache/alpha/0.1.0"]
        )

    def test_unregistered_plugin_has_no_cache_dirs(self):
        self.assertEqual(cache_guard.cache_dirs({"plugins": {}}, "alpha"), [])


if __name__ == "__main__":
    unittest.main()
