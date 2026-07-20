#!/usr/bin/env python3
"""airsstack-plugin-dev — SessionStart cache delivery guard.

The PostToolUse cache-sync hook only mirrors files it observes being edited,
so it never backfills a commit, a branch switch, or an edit that predates its
own installation. That blind spot is how `enforcement.json` reached the repo
but no install cache, leaving the enforcement framework silently dead.

This guard closes it: on session start, in the MAIN worktree of the plugin
source repo only, it copies every source file that is missing or differing in
the cache, reports cache-only extras without deleting them, and reports
version drift. Fail-open throughout; it never blocks a session.
"""

import filecmp
import json
import os
import shutil
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import cache_sync  # noqa: E402  — registry + containment units, not duplicated

MARKETPLACE = "airsstack"
# Claude Code writes .in_use into every cache dir; without this the extras
# report is never empty and you learn to skim past it.
IGNORED_NAMES = frozenset([".in_use", ".DS_Store", ".git"])


def _git(cwd, args):
    """Run git in `cwd`; return stripped stdout, or None on any failure."""
    try:
        out = subprocess.check_output(
            ["git"] + args, cwd=cwd, stderr=subprocess.DEVNULL
        )
    except (OSError, subprocess.CalledProcessError):
        return None
    return out.decode("utf-8", "replace").strip() or None


def is_main_worktree(cwd):
    """True only in the repo's main working tree.

    `git rev-parse --show-toplevel` succeeds from every linked worktree, so
    it cannot be the gate on its own: four checkouts at three commits share
    one version-keyed cache, and with add-and-update-only semantics the cache
    would converge on a union of branches. Comparing the common git dir
    against the toplevel's own .git answers 'which branch is authoritative'.

    `--git-common-dir` comes back cwd-relative in the main tree (`.git`,
    `../../.git` from a subdirectory) and absolute in a linked one, so the
    relative form is joined onto `cwd` before comparison.
    """
    top = _git(cwd, ["rev-parse", "--show-toplevel"])
    common = _git(cwd, ["rev-parse", "--git-common-dir"])
    if not top or not common:
        return False
    if not os.path.isabs(common):
        common = os.path.join(cwd, common)
    try:
        return os.path.realpath(common) == os.path.realpath(os.path.join(top, ".git"))
    except OSError:
        return False


def is_airsstack_marketplace(top):
    """True when `top` hosts the airsstack marketplace manifest."""
    manifest = os.path.join(top, ".claude-plugin", "marketplace.json")
    try:
        with open(manifest, "r", encoding="utf-8") as fh:
            data = json.load(fh)
    except (OSError, ValueError):
        return False
    return isinstance(data, dict) and data.get("name") == MARKETPLACE


def _relative_files(root):
    """Every file under `root` as a root-relative path, ignore-list applied."""
    found = []
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames[:] = [d for d in dirnames if d not in IGNORED_NAMES]
        for name in filenames:
            if name in IGNORED_NAMES:
                continue
            full = os.path.join(dirpath, name)
            found.append(os.path.relpath(full, root))
    return sorted(found)


def sync_tree(src_dir, cache_dir, containment_root=None):
    """Add-and-update-only mirror of `src_dir` into `cache_dir` (D7).

    Returns {"copied": [rel, ...], "extras": [rel, ...]}. Extras are cache-only
    files; they are REPORTED, never deleted. An unreferenced leftover is inert,
    whereas an unattended hook with delete authority over Claude Code's data
    directory is a larger risk than the cruft it would clean.

    `containment_root` bounds every write and defaults to the install cache
    root — the same rule the PostToolUse hook applies. It is a parameter rather
    than an environment switch so that tests re-point the guard instead of
    disabling it: the guard is the only bound on an unattended hook's writes,
    so it must stay live in the code paths the tests actually exercise.
    """
    if containment_root is None:
        containment_root = cache_sync.CACHE_ROOT

    copied, source_files = [], _relative_files(src_dir)
    for rel in source_files:
        src = os.path.join(src_dir, rel)
        dest = os.path.join(cache_dir, rel)
        if not cache_sync.is_within(dest, containment_root):
            continue  # containment guard, same rule as the PostToolUse hook
        try:
            if os.path.exists(dest) and filecmp.cmp(src, dest, shallow=False):
                continue
            os.makedirs(os.path.dirname(dest), exist_ok=True)
            shutil.copy2(src, dest)
            copied.append(rel)
        except OSError:
            continue  # one unwritable file must not abort the whole backfill

    known = set(source_files)
    extras = [rel for rel in _relative_files(cache_dir) if rel not in known]
    return {"copied": copied, "extras": extras}


def source_plugins(top):
    """Names of every plugin directory in the source repo, sorted."""
    root = os.path.join(top, "plugins")
    try:
        names = os.listdir(root)
    except OSError:
        return []
    found = []
    for name in sorted(names):
        manifest = os.path.join(root, name, ".claude-plugin", "plugin.json")
        if os.path.isfile(manifest):
            found.append(name)
    return found


def cache_dirs(registry, plugin):
    """Distinct install paths for `<plugin>@airsstack`, first-seen order.

    Delegates to the PostToolUse hook's own resolver so the two never drift
    apart on which marketplace or which records count.
    """
    return cache_sync.resolve_install_paths(registry, plugin)


def _version_at(top, rev, rel_path):
    """The `version` field of rel_path at `rev`, or None if unreadable."""
    raw = _git(top, ["show", "%s:%s" % (rev, rel_path)])
    if not raw:
        return None
    try:
        return (json.loads(raw) or {}).get("version")
    except ValueError:
        return None


def last_bump_commit(top, plugin):
    """Newest commit where plugin.json's `version` VALUE changed.

    Not 'the last commit touching plugin.json'. That naive rule reported 5 of
    7 plugins stale on its first run — content newer than the last bump is the
    normal state of a plugin under development — and had three proven false
    negatives: a manifest edited without changing the version, a squash merge
    collapsing bump and content into one commit (this repo's entire history),
    and uncommitted work invisible to `git log`.
    """
    rel = os.path.join("plugins", plugin, ".claude-plugin", "plugin.json")
    log = _git(top, ["log", "--format=%H", "--", rel])
    if not log:
        return None
    for commit in log.split("\n"):
        current = _version_at(top, commit, rel)
        parent = _version_at(top, commit + "^", rel)
        if current != parent:
            return commit
    return None


def version_drift(top, plugin):
    """'ok', 'stale', or 'unknown' for one plugin's committed content."""
    bump = last_bump_commit(top, plugin)
    if not bump:
        return "unknown"
    newer = _git(top, ["rev-list", bump + "..HEAD", "--", os.path.join("plugins", plugin)])
    return "stale" if newer else "ok"


def has_uncommitted(top, plugin):
    """True when the working tree has changes under plugins/<plugin>/."""
    status = _git(top, ["status", "--porcelain", "--", os.path.join("plugins", plugin)])
    return bool(status)


RESTART_NOTE = (
    "NOTE: enforcement.json is read at hook-fire time and takes effect now, but "
    "hooks.json and commands/ are read at plugin-load time — restart the session "
    "to pick up anything backfilled just now."
)
PUSH_NOTE = (
    "NOTE: user-scope installs pull the marketplace from GitHub, so a version bump "
    "reaches them only after the commit is pushed to main. Neither the backfill nor "
    "this check substitutes for that."
)


def _listing(names, limit=5):
    """`a, b, c` with a `(+N more)` tail once the list runs past `limit`."""
    shown = ", ".join(names[:limit])
    if len(names) <= limit:
        return shown
    return "%s (+%d more)" % (shown, len(names) - limit)


def format_report(active, results):
    """Report lines for SessionStart stdout; empty list when nothing to say.

    Backfills, extras and uncommitted edits are per-plugin: each names one thing
    to look at. Staleness is not — it is a PUBLICATION reminder, and "content
    committed after the last version bump" is the normal state of a plugin under
    active development (5 of 7 in this repo, 6 of 7 on main). Locally the
    backfill has already corrected the content; the only consumer of the stale
    signal is another machine pulling the marketplace from GitHub, and that
    message needs saying once. So the count is aggregated into a single line
    rather than one line per plugin, which would print a wall on every start and
    train you to skim past the whole report.
    """
    body, copied_any, stale = [], False, 0
    for item in results:
        plugin = item["plugin"]
        if item["copied"]:
            copied_any = True
            body.append("  %s: backfilled %d file(s): %s"
                        % (plugin, len(item["copied"]), _listing(item["copied"])))
        if item["extras"]:
            body.append("  %s: %d cache-only file(s), not deleted: %s"
                        % (plugin, len(item["extras"]), _listing(item["extras"])))
        if item["drift"] == "stale":
            stale += 1
        if item["uncommitted"]:
            body.append("  %s: uncommitted edits in the working tree" % plugin)

    if stale:
        body.append("  %d of %d %s stale — content committed after the last version bump"
                    % (stale, len(results), "plugin" if len(results) == 1 else "plugins"))

    if not body:
        return []

    header = ("airsstack cache guard:" if active else
              "airsstack cache guard (linked worktree — reporting only, nothing written):")
    lines = [header] + body
    if copied_any:
        lines.append(RESTART_NOTE)
    if stale:
        lines.append(PUSH_NOTE)
    return lines


def run(top, registry, write, containment_root=None):
    """Per-plugin backfill and drift results. `write=False` reports only.

    `containment_root` is threaded down to `sync_tree` rather than left to its
    default so that tests re-point the boundary instead of switching it off.
    """
    results = []
    for plugin in source_plugins(top):
        targets = cache_dirs(registry, plugin)
        if not targets:
            continue  # not installed from this marketplace: nothing to mirror
        src_dir = os.path.join(top, "plugins", plugin)
        copied, extras = [], []
        for cache_dir in targets:
            if write:
                outcome = sync_tree(src_dir, cache_dir, containment_root)
                copied.extend(outcome["copied"])
                extras.extend(outcome["extras"])
            else:
                known = set(_relative_files(src_dir))
                extras.extend(
                    rel for rel in _relative_files(cache_dir) if rel not in known
                )
        results.append({
            "plugin": plugin,
            "copied": sorted(set(copied)),
            "extras": sorted(set(extras)),
            "drift": version_drift(top, plugin),
            "uncommitted": has_uncommitted(top, plugin),
        })
    return results


def main():
    try:
        try:
            payload = json.loads(sys.stdin.read() or "{}")
        except ValueError:
            payload = {}
        if not isinstance(payload, dict):
            payload = {}
        cwd = payload.get("cwd") or os.getcwd()

        top = _git(cwd, ["rev-parse", "--show-toplevel"])
        if not top or not is_airsstack_marketplace(top):
            return 0  # not the plugin source repo: nothing to guard

        active = is_main_worktree(cwd)
        registry = cache_sync._load_installed() or {}
        lines = format_report(active, run(top, registry, write=active))
        if lines:
            sys.stdout.write("\n".join(lines) + "\n")
    except Exception:
        pass  # fail-open: a guard failure must never disturb the session
    return 0


if __name__ == "__main__":
    sys.exit(main())
