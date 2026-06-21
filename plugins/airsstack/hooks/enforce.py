#!/usr/bin/env python3
"""airsstack rule-enforcement dispatcher — PreToolUse(Edit|Write) hook.

Reads the installed-plugins registry, keeps only airsstack-marketplace
plugins, loads each one's enforcement.json, and — for the file being edited —
surfaces the matching guideline skill via additionalContext. Fail-open: never
blocks, denies, or raises out of main().
"""

import fnmatch
import json
import os
import sys
import time

MARKETPLACE_SUFFIX = "@airsstack"
MARKER_MAX_AGE = 24 * 3600  # seconds; stale dedup markers are pruned past this


def _registry_path():
    return os.environ.get("AIRSSTACK_ENFORCE_REGISTRY") or os.path.join(
        os.path.expanduser("~"), ".claude", "plugins", "installed_plugins.json"
    )


def _sdd_root():
    home = os.environ.get("AIRSSTACK_HOME") or os.path.join(
        os.path.expanduser("~"), ".airsstack"
    )
    return os.path.join(home, "cc", "plugins", "sdd")


def _is_design_doc(file_path):
    fp = os.path.abspath(file_path)
    root = os.path.abspath(_sdd_root())
    if not (fp == root or fp.startswith(root + os.sep)):
        return False
    return "/specs/" in fp or "/plans/" in fp


def _read_registry():
    """Return unique installPaths of airsstack-marketplace plugins only."""
    try:
        with open(_registry_path(), "r", encoding="utf-8") as fh:
            data = json.load(fh)
        plugins = (data or {}).get("plugins") or {}
    except (OSError, ValueError):
        return []
    seen, paths = set(), []
    for key, records in plugins.items():
        if not key.endswith(MARKETPLACE_SUFFIX):
            continue  # scope guard: external plugins never touched
        if not isinstance(records, list):
            continue
        for rec in records:
            if isinstance(rec, dict) and rec.get("installPath"):
                p = rec["installPath"]
                if p not in seen:
                    seen.add(p)
                    paths.append(p)
    return paths


def _load_manifests(paths):
    manifests = []
    for p in paths:
        try:
            with open(os.path.join(p, "enforcement.json"), "r", encoding="utf-8") as fh:
                m = json.load(fh)
        except (OSError, ValueError):
            continue  # absent or malformed → skip this plugin, keep the rest
        if not isinstance(m, dict):
            continue
        stack, skill = m.get("stack"), m.get("skill")
        if not stack or not skill:
            continue
        manifests.append(
            {
                "stack": stack,
                "skill": skill,
                "detect": m.get("detect") or [],
                "match": m.get("match") or [],
                "phase": m.get("phase") or ["code", "design"],
            }
        )
    return manifests


def _basename_match(file_path, globs):
    """Match the file's basename against each glob's final segment.

    Manifest globs are `**/`-prefixed (e.g. `**/*.rs`, `**/Cargo.toml`), so the
    final segment carries the meaning; this matches both root and nested files.
    """
    base = os.path.basename(file_path)
    for g in globs:
        seg = str(g).rsplit("/", 1)[-1]
        if fnmatch.fnmatch(base, seg):
            return True
    return False


def _marker_active(cwd, markers):
    """True if any marker file exists at cwd or any ancestor directory."""
    if not markers:
        return False
    d = os.path.abspath(cwd or ".")
    while True:
        for m in markers:
            if os.path.isfile(os.path.join(d, m)):
                return True
        parent = os.path.dirname(d)
        if parent == d:
            return False
        d = parent


def _matches(file_path, cwd, manifests):
    """Return list of (stack, phase, skill) for this event.

    A file under the SDD specs/plans tree is a design-phase doc → trigger on
    detect markers. Any other file is code-phase → trigger on match globs.
    """
    hits = []
    design = _is_design_doc(file_path)
    for m in manifests:
        if design:
            if "design" in m["phase"] and _marker_active(cwd, m["detect"]):
                hits.append((m["stack"], "design", m["skill"]))
        else:
            if "code" in m["phase"] and _basename_match(file_path, m["match"]):
                hits.append((m["stack"], "code", m["skill"]))
    return hits


def _pointer(stack, skill):
    return (
        stack + " work is in play. The " + skill + " skill is MANDATORY for "
        "this work — load it now via Skill before proceeding, and apply its "
        "rules (Definition of Done + architecture)."
    )


def _marker_dir():
    return os.environ.get("TMPDIR") or "/tmp"


def _marker_path(session_id):
    safe = "".join(
        c if (c.isalnum() or c in "-_") else "-" for c in (session_id or "nosession")
    )
    return os.path.join(_marker_dir(), "airsstack-enforce-" + safe)


def _prune_markers():
    try:
        now = time.time()
        d = _marker_dir()
        for name in os.listdir(d):
            if not name.startswith("airsstack-enforce-"):
                continue
            p = os.path.join(d, name)
            try:
                if now - os.path.getmtime(p) > MARKER_MAX_AGE:
                    os.unlink(p)
            except OSError:
                pass
    except OSError:
        pass


def _already(session_id):
    try:
        with open(_marker_path(session_id), "r", encoding="utf-8") as fh:
            return set(line.strip() for line in fh if line.strip())
    except OSError:
        return set()


def _record(session_id, keys):
    try:
        with open(_marker_path(session_id), "a", encoding="utf-8") as fh:
            for k in keys:
                fh.write(k + "\n")
    except OSError:
        pass  # best-effort; degrade to a possible repeat, never crash


def main():
    try:
        data = json.loads(sys.stdin.read() or "{}")
        tool_input = data.get("tool_input") or {}
        file_path = tool_input.get("file_path")
        if not file_path:
            return
        cwd = data.get("cwd") or os.getcwd()
        session_id = data.get("session_id") or ""

        _prune_markers()

        manifests = _load_manifests(_read_registry())
        if not manifests:
            return

        hits = _matches(file_path, cwd, manifests)
        if not hits:
            return

        seen = _already(session_id)
        pointers, new_keys = [], []
        for stack, phase, skill in hits:
            key = stack + ":" + phase
            if key in seen or key in new_keys:
                continue
            new_keys.append(key)
            pointers.append(_pointer(stack, skill))

        if not pointers:
            return

        _record(session_id, new_keys)
        sys.stdout.write(
            json.dumps(
                {
                    "hookSpecificOutput": {
                        "hookEventName": "PreToolUse",
                        "additionalContext": "\n".join(pointers),
                        "permissionDecision": "defer",
                    }
                }
            )
        )
    except Exception:
        pass  # fail-open: never block an edit


if __name__ == "__main__":
    main()
