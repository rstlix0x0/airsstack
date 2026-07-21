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


class TestVersionDrift(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp()
        self.repo = make_repo(os.path.join(self.tmp, "repo"))

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def _manifest(self):
        return os.path.join(self.repo, "plugins", "demo", ".claude-plugin", "plugin.json")

    def _set_version(self, value):
        with open(self._manifest()) as fh:
            data = json.load(fh)
        data["version"] = value
        with open(self._manifest(), "w") as fh:
            json.dump(data, fh)

    def _touch_manifest_without_bumping(self):
        with open(self._manifest()) as fh:
            data = json.load(fh)
        data["description"] = "touched"
        with open(self._manifest(), "w") as fh:
            json.dump(data, fh)

    def _commit(self, message):
        git(self.repo, "add", "-A")
        git(self.repo, "commit", "-qm", message)

    def test_bump_with_no_later_content_is_ok(self):
        self._set_version("0.2.0")
        self._commit("bump")
        self.assertEqual(cache_guard.version_drift(self.repo, "demo"), "ok")

    def test_content_committed_after_the_bump_is_stale(self):
        self._set_version("0.2.0")
        self._commit("bump")
        with open(os.path.join(self.repo, "plugins", "demo", "new.txt"), "w") as fh:
            fh.write("x\n")
        self._commit("content after bump")
        self.assertEqual(cache_guard.version_drift(self.repo, "demo"), "stale")

    def test_manifest_touched_without_a_version_change_is_stale(self):
        """The naive 'last commit touching plugin.json' rule reports a false ok here."""
        self._set_version("0.2.0")
        self._commit("bump")
        self._touch_manifest_without_bumping()
        self._commit("edit manifest, same version")
        self.assertEqual(cache_guard.version_drift(self.repo, "demo"), "stale")

    def test_squash_merged_bump_plus_content_is_stale(self):
        """This repo's history is all squash merges; both land in one commit."""
        self._set_version("0.2.0")
        with open(os.path.join(self.repo, "plugins", "demo", "new.txt"), "w") as fh:
            fh.write("x\n")
        self._commit("squash: bump + content (#42)")
        with open(os.path.join(self.repo, "plugins", "demo", "later.txt"), "w") as fh:
            fh.write("y\n")
        self._commit("more content")
        self.assertEqual(cache_guard.version_drift(self.repo, "demo"), "stale")

    def test_content_committed_elsewhere_after_the_bump_is_ok(self):
        """Drift is per-plugin: a sibling plugin's commit must not age this one.

        With seven plugins in one repo this is the common case, so an unscoped
        rev-list would report nearly everything stale.
        """
        self._set_version("0.2.0")
        self._commit("bump demo")
        other = os.path.join(self.repo, "plugins", "other", ".claude-plugin")
        os.makedirs(other)
        with open(os.path.join(other, "plugin.json"), "w") as fh:
            json.dump({"name": "other", "version": "0.1.0"}, fh)
        self._commit("unrelated plugin")
        self.assertEqual(cache_guard.version_drift(self.repo, "demo"), "ok")

    def test_uncommitted_elsewhere_is_not_attributed_to_this_plugin(self):
        with open(os.path.join(self.repo, "unrelated.txt"), "w") as fh:
            fh.write("z\n")
        self.assertFalse(cache_guard.has_uncommitted(self.repo, "demo"))

    def test_first_version_at_the_root_commit_is_a_bump(self):
        """`<root>^` does not resolve, so the parent version reads as None."""
        self.assertEqual(cache_guard.version_drift(self.repo, "demo"), "ok")

    def test_unknown_when_the_plugin_has_no_manifest_history(self):
        self.assertEqual(cache_guard.version_drift(self.repo, "ghost"), "unknown")

    def test_uncommitted_edits_are_reported(self):
        with open(os.path.join(self.repo, "plugins", "demo", "dirty.txt"), "w") as fh:
            fh.write("z\n")
        self.assertTrue(cache_guard.has_uncommitted(self.repo, "demo"))

    def test_clean_tree_has_no_uncommitted_edits(self):
        self.assertFalse(cache_guard.has_uncommitted(self.repo, "demo"))


def result(plugin, copied=(), extras=(), drift="ok", uncommitted=False):
    return {"plugin": plugin, "copied": list(copied), "extras": list(extras),
            "drift": drift, "uncommitted": uncommitted}


class TestReport(unittest.TestCase):
    def test_silent_when_everything_is_clean(self):
        self.assertEqual(cache_guard.format_report(True, [result("demo")]), [])

    def test_backfill_is_reported_with_the_restart_caveat(self):
        joined = "\n".join(
            cache_guard.format_report(True, [result("demo", copied=["enforcement.json"])])
        )
        self.assertIn("enforcement.json", joined)
        self.assertIn("restart", joined.lower())

    def test_extras_are_reported_as_not_deleted(self):
        joined = "\n".join(
            cache_guard.format_report(True, [result("demo", extras=["hooks/enforce.js"])])
        )
        self.assertIn("hooks/enforce.js", joined)
        self.assertIn("not deleted", joined.lower())

    def test_stale_version_carries_the_push_caveat(self):
        joined = "\n".join(cache_guard.format_report(True, [result("demo", drift="stale")]))
        self.assertIn("stale", joined)
        self.assertIn("push", joined.lower())

    def test_uncommitted_is_reported_distinctly_from_stale(self):
        joined = "\n".join(
            cache_guard.format_report(True, [result("demo", uncommitted=True)])
        )
        self.assertIn("uncommitted", joined.lower())
        self.assertIn("demo", joined)
        self.assertNotIn("stale", joined)

    def test_inactive_reports_drift_but_says_it_did_not_write(self):
        joined = "\n".join(
            cache_guard.format_report(False, [result("demo", drift="stale")])
        ).lower()
        self.assertIn("stale", joined)
        self.assertIn("linked worktree", joined)

    def test_stale_is_one_aggregate_line_regardless_of_how_many_are_stale(self):
        """Stale is a PUBLICATION reminder, and 'content newer than the last
        bump' is the normal state of a plugin under development — 5 of 7 here,
        6 of 7 on main. Per-plugin lines would be a wall on every session start,
        so the count is said once."""
        results = [result("p%d" % i, drift="stale") for i in range(6)]
        lines = cache_guard.format_report(True, results)
        stale_lines = [ln for ln in lines if "stale" in ln]
        self.assertEqual(len(stale_lines), 1)

    def test_the_aggregate_counts_stale_against_the_total(self):
        results = [result("a", drift="stale"), result("b", drift="stale"),
                   result("c"), result("d", drift="unknown")]
        lines = cache_guard.format_report(True, results)
        stale_lines = [ln for ln in lines if "stale" in ln]
        self.assertIn("2 of 4", stale_lines[0])


class TestRun(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp()
        self.repo = make_repo(os.path.join(self.tmp, "repo"))
        with open(os.path.join(self.repo, "plugins", "demo", "enforcement.json"), "w") as fh:
            fh.write('{"stack":"rust"}')
        git(self.repo, "add", "-A")
        git(self.repo, "commit", "-qm", "add manifest")
        self.cache = os.path.join(self.tmp, "cache", "demo", "0.1.0")
        os.makedirs(self.cache)
        self.registry = {"plugins": {"demo@airsstack": [{"installPath": self.cache}]}}

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def _run(self, write, registry=None):
        """Containment guard LIVE, re-pointed at this test's tmpdir — never off."""
        return cache_guard.run(
            self.repo, self.registry if registry is None else registry,
            write=write, containment_root=self.tmp,
        )

    def _demo(self, results):
        return [r for r in results if r["plugin"] == "demo"][0]

    def test_backfills_the_missing_manifest(self):
        results = self._run(write=True)
        self.assertTrue(os.path.exists(os.path.join(self.cache, "enforcement.json")))
        self.assertIn("enforcement.json", self._demo(results)["copied"])

    def test_write_false_reports_without_copying(self):
        results = self._run(write=False)
        self.assertFalse(os.path.exists(os.path.join(self.cache, "enforcement.json")))
        self.assertEqual(self._demo(results)["copied"], [])

    def test_write_false_still_reports_cache_only_extras(self):
        """A linked worktree writes nothing but must still diagnose the cache."""
        with open(os.path.join(self.cache, "leftover.js"), "w") as fh:
            fh.write("// removed upstream\n")
        results = self._run(write=False)
        self.assertIn("leftover.js", self._demo(results)["extras"])
        self.assertTrue(os.path.exists(os.path.join(self.cache, "leftover.js")))

    def test_unregistered_plugin_is_skipped_without_error(self):
        self.assertEqual(self._run(write=True, registry={"plugins": {}}), [])

    def test_containment_root_reaches_sync_tree(self):
        """run() must thread the boundary down, not let sync_tree fall back to
        the real cache root: without this the backfill above would silently
        copy nothing and the write path would be untested."""
        results = cache_guard.run(self.repo, self.registry, write=True)
        self.assertEqual(self._demo(results)["copied"], [])
        self.assertFalse(os.path.exists(os.path.join(self.cache, "enforcement.json")))

    def test_results_carry_drift_and_uncommitted(self):
        with open(os.path.join(self.repo, "plugins", "demo", "dirty.txt"), "w") as fh:
            fh.write("z\n")
        demo = self._demo(self._run(write=False))
        self.assertEqual(demo["drift"], "stale")
        self.assertTrue(demo["uncommitted"])


class TestMainEntryPoint(unittest.TestCase):
    """main() is the production path; the plan left it uncovered.

    Run as a real subprocess under a throwaway HOME. cache_sync derives both
    CACHE_ROOT and the registry path from `~`, so a fake HOME relocates the
    whole install cache into the tmpdir — which exercises main() end to end,
    containment guard included and pointing at its true default, without a
    single test-only switch and without any risk to the real cache.
    """

    SCRIPT = os.path.join(
        os.path.dirname(os.path.abspath(__file__)), "cache_guard.py"
    )

    def setUp(self):
        self.tmp = tempfile.mkdtemp()
        self.home = os.path.join(self.tmp, "home")
        self.cache = os.path.join(
            self.home, ".claude", "plugins", "cache", "airsstack", "demo", "0.1.0"
        )
        os.makedirs(self.cache)
        registry = os.path.join(self.home, ".claude", "plugins", "installed_plugins.json")
        with open(registry, "w") as fh:
            json.dump({"plugins": {"demo@airsstack": [{"installPath": self.cache}]}}, fh)
        self.repo = self._repo("repo", "airsstack")
        self.env = dict(os.environ, HOME=self.home)

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def _repo(self, name, marketplace):
        root = make_repo(os.path.join(self.tmp, name), marketplace_name=marketplace)
        with open(os.path.join(root, "plugins", "demo", "enforcement.json"), "w") as fh:
            fh.write('{"stack":"rust"}')
        git(root, "add", "-A")
        git(root, "commit", "-qm", "add manifest")
        return root

    def _main(self, stdin_text, cwd=None):
        return subprocess.run(
            [sys.executable, self.SCRIPT], input=stdin_text, env=self.env,
            cwd=cwd or self.tmp, capture_output=True, text=True,
        )

    def _mirrored(self):
        return os.path.exists(os.path.join(self.cache, "enforcement.json"))

    def test_main_worktree_backfills_and_reports(self):
        done = self._main(json.dumps({"cwd": self.repo}))
        self.assertEqual(done.returncode, 0)
        self.assertTrue(self._mirrored())
        self.assertIn("enforcement.json", done.stdout)
        self.assertIn("restart", done.stdout.lower())

    def test_linked_worktree_reports_without_writing(self):
        linked = os.path.join(self.tmp, "linked")
        git(self.repo, "worktree", "add", "-q", "-b", "wt", linked)
        done = self._main(json.dumps({"cwd": linked}))
        self.assertEqual(done.returncode, 0)
        self.assertFalse(self._mirrored())
        self.assertIn("linked worktree", done.stdout.lower())

    def test_non_airsstack_marketplace_is_refused(self):
        """Same plugin name, same registry entry — only the marketplace differs."""
        other = self._repo("other", "somebody-else")
        done = self._main(json.dumps({"cwd": other}))
        self.assertEqual(done.returncode, 0)
        self.assertEqual(done.stdout, "")
        self.assertFalse(self._mirrored())

    def test_silent_outside_a_git_repo(self):
        done = self._main(json.dumps({"cwd": self.tmp}))
        self.assertEqual(done.returncode, 0)
        self.assertEqual(done.stdout, "")

    def test_garbage_stdin_exits_zero(self):
        self.assertEqual(self._main("}{ not json").returncode, 0)

    def test_empty_stdin_exits_zero(self):
        self.assertEqual(self._main("").returncode, 0)

    def test_an_unexpected_error_is_swallowed(self):
        """Fail-open is the whole contract: a guard crash must not reach the
        session. A non-string cwd raises inside subprocess, past every
        exception type the git helper names."""
        done = self._main(json.dumps({"cwd": 12345}))
        self.assertEqual(done.returncode, 0)
        self.assertEqual(done.stderr, "")


class TestWiring(unittest.TestCase):
    HOOKS_DIR = os.path.dirname(os.path.abspath(__file__))

    def _hooks_json(self):
        with open(os.path.join(self.HOOKS_DIR, "hooks.json")) as fh:
            return json.load(fh)

    def _launcher(self):
        with open(os.path.join(self.HOOKS_DIR, "cache-guard.sh")) as fh:
            return fh.read()

    def test_session_start_runs_the_guard(self):
        entries = self._hooks_json()["hooks"]["SessionStart"]
        self.assertEqual(entries[0]["matcher"], "startup|resume|clear")
        self.assertIn("cache-guard.sh", entries[0]["hooks"][0]["command"])

    def test_post_tool_use_mirrors_writes_only(self):
        """`Edit|Write`, deliberately NOT the dispatcher's `Read|Edit|Write`.

        That matcher exists so an enforcement rule lands before the design
        decision; a mirror hook has no such need. Reads are incidental — every
        sweep, every grep-then-read — so triggering on Read would let merely
        LOOKING at a plugin file from a linked worktree push that branch's
        bytes into the shared version-keyed cache, which is the cross-branch
        convergence the guard's main-worktree gate exists to prevent.
        """
        self.assertEqual(self._hooks_json()["hooks"]["PostToolUse"][0]["matcher"],
                         "Edit|Write")

    def test_the_launcher_never_execs(self):
        """`exec` would hand python's exit status straight back to Claude Code;
        the dispatcher's launcher shipped that bug and turned every hook fire
        into a potential block."""
        self.assertNotIn("exec ", self._launcher())

    def test_launcher_exits_zero_when_the_module_is_broken(self):
        work = tempfile.mkdtemp()
        try:
            shutil.copy2(os.path.join(self.HOOKS_DIR, "cache-guard.sh"), work)
            with open(os.path.join(work, "cache_guard.py"), "w") as fh:
                fh.write("def (\n")  # SyntaxError
            code = subprocess.call(
                ["sh", os.path.join(work, "cache-guard.sh")],
                stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
            self.assertEqual(code, 0)
        finally:
            shutil.rmtree(work, ignore_errors=True)

    def test_launcher_exits_zero_when_the_module_is_missing(self):
        work = tempfile.mkdtemp()
        try:
            shutil.copy2(os.path.join(self.HOOKS_DIR, "cache-guard.sh"), work)
            code = subprocess.call(
                ["sh", os.path.join(work, "cache-guard.sh")],
                stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
            self.assertEqual(code, 0)
        finally:
            shutil.rmtree(work, ignore_errors=True)


class TestDocsAndVersions(unittest.TestCase):
    REPO = os.path.normpath(
        os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "..")
    )

    def _version(self, plugin):
        path = os.path.join(self.REPO, "plugins", plugin, ".claude-plugin", "plugin.json")
        with open(path) as fh:
            return json.load(fh)["version"]

    def _readme(self):
        path = os.path.join(self.REPO, "plugins", "airsstack-plugin-dev", "README.md")
        with open(path) as fh:
            return fh.read().lower()

    def test_plugin_dev_version_bumped(self):
        """0.1.0 is what every install cache already holds; a plugin whose
        version never moves can only ever be delivered by the backfill."""
        self.assertNotEqual(self._version("airsstack-plugin-dev"), "0.1.0")

    def test_guideline_rust_version_bumped(self):
        """The manifest reaches other projects only via a clean cache dir."""
        self.assertNotEqual(self._version("airsstack-guideline-rust"), "0.1.0")

    def test_readme_documents_the_guard(self):
        text = self._readme()
        self.assertIn("cache-guard", text)
        self.assertIn("main worktree", text)
        self.assertIn("never delete", text)

    def test_readme_does_not_advertise_a_retired_matcher(self):
        """The README named the PostToolUse matcher literally, so the T7 change
        silently falsified it — docs that name a matcher must move with it."""
        self.assertNotIn("multiedit", self._readme())


if __name__ == "__main__":
    unittest.main()
