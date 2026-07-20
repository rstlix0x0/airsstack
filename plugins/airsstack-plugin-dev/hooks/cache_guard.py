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
