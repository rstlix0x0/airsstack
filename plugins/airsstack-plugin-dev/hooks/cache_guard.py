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

import json
import os
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
