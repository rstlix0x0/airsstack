import os
import sys
import unittest
import io
import json
import shutil
import tempfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import cache_sync


class ExtractPluginRel(unittest.TestCase):
    def test_nested_skill_path(self):
        p = os.path.join(os.sep, "repo", "plugins", "airsstack",
                         "skills", "concise", "SKILL.md")
        self.assertEqual(
            cache_sync.extract_plugin_rel(p),
            ("airsstack", os.path.join("skills", "concise", "SKILL.md")),
        )

    def test_top_level_plugin_file(self):
        p = os.path.join(os.sep, "repo", "plugins", "airsstack-sdd", "README.md")
        self.assertEqual(
            cache_sync.extract_plugin_rel(p),
            ("airsstack-sdd", "README.md"),
        )

    def test_no_plugins_segment(self):
        p = os.path.join(os.sep, "repo", "src", "main.rs")
        self.assertIsNone(cache_sync.extract_plugin_rel(p))

    def test_plugins_dir_with_no_file(self):
        p = os.path.join(os.sep, "repo", "plugins", "airsstack")
        self.assertIsNone(cache_sync.extract_plugin_rel(p))


class ResolveInstallPaths(unittest.TestCase):
    def test_dedupes_distinct_paths(self):
        data = {"plugins": {"airsstack@airsstack": [
            {"installPath": "/c/airsstack/airsstack/0.1.0"},
            {"installPath": "/c/airsstack/airsstack/0.1.0"},
        ]}}
        self.assertEqual(
            cache_sync.resolve_install_paths(data, "airsstack"),
            ["/c/airsstack/airsstack/0.1.0"],
        )

    def test_missing_plugin_returns_empty(self):
        self.assertEqual(
            cache_sync.resolve_install_paths({"plugins": {}}, "ghost"), []
        )

    def test_non_airsstack_marketplace_not_selected(self):
        data = {"plugins": {"airsstack@elsewhere": [{"installPath": "/c/x"}]}}
        self.assertEqual(
            cache_sync.resolve_install_paths(data, "airsstack"), []
        )

    def test_entry_without_install_path_skipped(self):
        data = {"plugins": {"airsstack@airsstack": [
            {"scope": "user"},
            {"installPath": "/c/airsstack/airsstack/0.1.0"},
        ]}}
        self.assertEqual(
            cache_sync.resolve_install_paths(data, "airsstack"),
            ["/c/airsstack/airsstack/0.1.0"],
        )


class IsWithin(unittest.TestCase):
    def test_nested_true(self):
        self.assertTrue(cache_sync.is_within("/a/b/c", "/a/b"))

    def test_same_path_true(self):
        self.assertTrue(cache_sync.is_within("/a/b", "/a/b"))

    def test_sibling_prefix_false(self):
        self.assertFalse(cache_sync.is_within("/a/bc", "/a/b"))

    def test_outside_false(self):
        self.assertFalse(cache_sync.is_within("/etc/passwd", "/a/b"))


class EndToEndSync(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp()
        self.cache_root = os.path.join(self.tmp, "cache")
        self.install_path = os.path.join(
            self.cache_root, "airsstack", "airsstack", "0.1.0")
        os.makedirs(self.install_path)
        self.src = os.path.join(
            self.tmp, "repo", "plugins", "airsstack",
            "skills", "concise", "SKILL.md")
        os.makedirs(os.path.dirname(self.src))
        with open(self.src, "w") as fh:
            fh.write("FRESH BODY\n")
        self.installed = os.path.join(self.tmp, "installed_plugins.json")
        self._write_installed(self.install_path)
        self._orig = (cache_sync.CACHE_ROOT, cache_sync.INSTALLED_PLUGINS)
        cache_sync.CACHE_ROOT = self.cache_root
        cache_sync.INSTALLED_PLUGINS = self.installed

    def tearDown(self):
        cache_sync.CACHE_ROOT, cache_sync.INSTALLED_PLUGINS = self._orig
        shutil.rmtree(self.tmp, ignore_errors=True)

    def _write_installed(self, install_path):
        with open(self.installed, "w") as fh:
            json.dump({"plugins": {"airsstack@airsstack": [
                {"installPath": install_path}]}}, fh)

    def _run_main(self, file_path):
        payload = json.dumps(
            {"tool_name": "Edit", "tool_input": {"file_path": file_path}})
        orig_stdin = sys.stdin
        sys.stdin = io.StringIO(payload)
        try:
            return cache_sync.main()
        finally:
            sys.stdin = orig_stdin

    def test_main_syncs_edited_file_into_cache(self):
        rc = self._run_main(self.src)
        self.assertEqual(rc, 0)
        dest = os.path.join(
            self.install_path, "skills", "concise", "SKILL.md")
        self.assertTrue(os.path.isfile(dest))
        with open(dest) as fh:
            self.assertEqual(fh.read(), "FRESH BODY\n")

    def test_main_overwrites_existing_cache_file(self):
        dest = os.path.join(
            self.install_path, "skills", "concise", "SKILL.md")
        os.makedirs(os.path.dirname(dest))
        with open(dest, "w") as fh:
            fh.write("STALE\n")
        self._run_main(self.src)
        with open(dest) as fh:
            self.assertEqual(fh.read(), "FRESH BODY\n")

    def test_main_skips_dest_outside_cache_root(self):
        bad = os.path.join(self.tmp, "evil", "0.1.0")
        os.makedirs(bad)
        self._write_installed(bad)
        rc = self._run_main(self.src)
        self.assertEqual(rc, 0)
        self.assertFalse(os.path.exists(
            os.path.join(bad, "skills", "concise", "SKILL.md")))

    def test_main_noops_on_non_plugin_path(self):
        other = os.path.join(self.tmp, "repo", "src", "main.rs")
        os.makedirs(os.path.dirname(other))
        with open(other, "w") as fh:
            fh.write("x\n")
        self.assertEqual(self._run_main(other), 0)

    def test_main_noops_on_empty_payload(self):
        orig_stdin = sys.stdin
        sys.stdin = io.StringIO("")
        try:
            self.assertEqual(cache_sync.main(), 0)
        finally:
            sys.stdin = orig_stdin


if __name__ == "__main__":
    unittest.main()
