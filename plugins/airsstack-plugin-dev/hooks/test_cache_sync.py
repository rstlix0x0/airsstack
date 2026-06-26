import os
import sys
import unittest

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


if __name__ == "__main__":
    unittest.main()
