#!/usr/bin/env python3
"""airsstack rule-enforcement dispatcher — PreToolUse(Edit|Write) hook.

Reads the installed-plugins registry, keeps only airsstack-marketplace
plugins, loads each one's enforcement.json, and — for the file being edited —
surfaces the matching guideline skill via additionalContext. Fail-open: never
blocks, denies, or raises out of main().
"""

import fnmatch
import hashlib
import json
import os
import re
import subprocess
import sys
import time

MARKETPLACE_SUFFIX = "@airsstack"
MARKER_MAX_AGE = 24 * 3600  # seconds; stale dedup markers are pruned past this


def glob_to_regex(pattern):
    """Compile a path glob into an anchored regex.

    Both stdlib options are unusable on 3.9.6: `fnmatch`'s `*` crosses `/`,
    and `PurePath.match` treats `**` as non-recursive (`full_match` arrives
    in 3.13). Neither matches a root-level `Cargo.toml` against
    `**/Cargo.toml`, which is this repo's most important Rust file.

    `**/` deliberately matches ZERO or more leading segments.
    """
    i, n, out = 0, len(pattern), []
    while i < n:
        c = pattern[i]
        if pattern.startswith("**/", i):
            out.append("(?:[^/]+/)*")
            i += 3
        elif pattern.startswith("**", i):
            out.append(".*")
            i += 2
        elif c == "*":
            out.append("[^/]*")
            i += 1
        elif c == "?":
            out.append("[^/]")
            i += 1
        elif c == "[":
            j = i + 1
            if j < n and pattern[j] in "!^":
                j += 1
            if j < n and pattern[j] == "]":
                j += 1  # a leading ] is a literal member
            while j < n and pattern[j] != "]":
                j += 1
            if j >= n:
                out.append(re.escape("["))  # unclosed [ is a literal
                i += 1
            else:
                body = pattern[i + 1:j].replace("\\", "\\\\")
                if body.startswith("!"):
                    body = "^" + body[1:]
                out.append("[" + body + "]")
                i = j + 1
        else:
            out.append(re.escape(c))
            i += 1
    return re.compile("^" + "".join(out) + "$")


def _git(cwd, args):
    """Run git in `cwd`; return stripped stdout, or None on any failure."""
    try:
        out = subprocess.check_output(
            ["git"] + args, cwd=cwd, stderr=subprocess.DEVNULL
        )
    except (OSError, subprocess.CalledProcessError):
        return None
    text = out.decode("utf-8", "replace").strip()
    return text or None


def _sanitize(text):
    """Replace every character outside [A-Za-z0-9._-] with '-' (tr -c parity)."""
    return re.sub(r"[^A-Za-z0-9._-]", "-", text or "")


def project_key(cwd):
    """Stable per-repo key; every linked worktree collapses to one value.

    Mirrors the sh formula in airsstack-sdd/references/artifact-paths.md,
    which the snapshot store and the SDD roots already share. Keys, never
    path prefixes, are what gate 1 compares — a linked worktree may live
    anywhere on disk.
    """
    common = _git(cwd, ["rev-parse", "--git-common-dir"])
    try:
        if common:
            if not os.path.isabs(common):
                common = os.path.join(cwd, common)
            parent = os.path.realpath(os.path.dirname(common) or ".")
            abs_path = os.path.join(parent, os.path.basename(common))
            base = os.path.basename(os.path.dirname(abs_path))
        else:
            abs_path = os.path.realpath(cwd)
            base = os.path.basename(abs_path)
    except OSError:
        return None
    digest = hashlib.sha1(abs_path.encode("utf-8")).hexdigest()[:8]
    return _sanitize(base) + "-" + digest


def path_for_matching(file_path, cwd):
    """Path that `match` globs are tested against.

    Inside a repository: the path relative to the git toplevel, which is what
    `match` is documented to mean. Outside any repository: the basename,
    preserving the pre-repair behavior there rather than silently dropping
    coverage.
    """
    target = os.path.realpath(os.path.abspath(file_path))
    top = _git(cwd, ["rev-parse", "--show-toplevel"])
    if top:
        try:
            top = os.path.realpath(top)
        except OSError:
            top = None
    if top and (target == top or target.startswith(top + os.sep)):
        return os.path.relpath(target, top)
    return os.path.basename(target)


def matches_any(candidate, globs):
    """True when `candidate` matches at least one glob."""
    for pattern in globs or []:
        try:
            if glob_to_regex(str(pattern)).match(candidate):
                return True
        except re.error:
            continue  # a malformed glob disables itself, never the manifest
    return False


def registry_path():
    return os.environ.get("AIRSSTACK_ENFORCE_REGISTRY") or os.path.join(
        os.path.expanduser("~"), ".claude", "plugins", "installed_plugins.json"
    )


def sdd_root(home=None):
    base = home or os.environ.get("AIRSSTACK_HOME") or os.path.join(
        os.path.expanduser("~"), ".airsstack"
    )
    return os.path.join(base, "cc", "plugins", "sdd")


def is_design_doc(file_path, home=None):
    """True for an SDD spec/plan under the HOME-global root.

    The directory name must appear as a whole path SEGMENT. The pre-repair
    test was a substring check on '/specs/', so
    `<key>/a/specs/b/plans/c.md` matched by accident.
    """
    target = os.path.abspath(file_path)
    root = os.path.abspath(sdd_root(home))
    if not target.startswith(root + os.sep):
        return False
    rel = os.path.relpath(target, root)
    segments = rel.split(os.sep)[:-1]  # directories only, never the filename
    if len(segments) < 2:
        return False
    # Layout is <key>/<specs|plans>/... — the artifact dir is the key's child.
    return segments[1] in ("specs", "plans")


def read_registry(path=None):
    """Return {plugin_key: [record, ...]} for @airsstack plugins only.

    The suffix check is the scope guard: a plugin from any other marketplace
    is never read, never routed.
    """
    target = path or registry_path()
    try:
        with open(target, "r", encoding="utf-8") as fh:
            data = json.load(fh)
        plugins = (data or {}).get("plugins") or {}
    except (OSError, ValueError):
        return {}
    kept = {}
    for key, records in plugins.items():
        if not key.endswith(MARKETPLACE_SUFFIX):
            continue
        if isinstance(records, list):
            kept[key] = [r for r in records if isinstance(r, dict) and r.get("installPath")]
    return kept


def select_record(records, current_key, key_cache):
    """Pick the registry record that governs this project (gate 1, D2).

    1. A record whose `projectPath` resolves to the current project key.
    2. Otherwise the user-scope record.
    3. Otherwise nothing — the anti-leak property: a plugin installed only
       for repo A contributes nothing in repo B.

    `key_cache` maps projectPath -> project key; the caller owns it so the
    git subprocess runs at most once per distinct path. `local` is treated
    as project-bound; that reading is inferred, not documented, and a
    mis-read can only pick a wrong installPath, never widen enforcement.
    """
    fallback = None
    for record in records:
        project_path = record.get("projectPath")
        if project_path:
            if project_path not in key_cache:
                key_cache[project_path] = project_key(project_path)
            if current_key and key_cache[project_path] == current_key:
                return record
        elif fallback is None:
            fallback = record
    return fallback


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


def marker_active_in(directory, markers):
    """True when a `detect` marker sits in `directory` or any ancestor.

    Split out of `marker_active` because the two phases anchor differently:
    a code file anchors on itself, an SDD design doc has no in-repo location
    to anchor on (it lives under AIRSSTACK_HOME) so it anchors on `cwd`.
    """
    if not markers:
        return False
    d = os.path.abspath(directory or ".")
    while True:
        for marker in markers:
            if os.path.isfile(os.path.join(d, marker)):
                return True
        parent = os.path.dirname(d)
        if parent == d:
            return False
        d = parent


def marker_active(file_path, markers, cwd=None):
    """True when a `detect` marker sits at or above the FILE's directory.

    The pre-repair version searched upward from `cwd`, which is wrong for any
    file outside the session's working directory. `cwd` survives only as the
    fallback when the file has no usable directory component.
    """
    start = os.path.dirname(os.path.abspath(file_path)) or (cwd or ".")
    return marker_active_in(start, markers)


def _matches(file_path, cwd, manifests):
    """Return list of (stack, phase, skill) for this event.

    A file under the SDD specs/plans tree is a design-phase doc → trigger on
    detect markers. Any other file is code-phase → trigger on match globs.
    """
    hits = []
    design = is_design_doc(file_path)
    for m in manifests:
        if design:
            if "design" in m["phase"] and marker_active_in(cwd, m["detect"]):
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

        # T10 replaces this with the ordered `resolve` pipeline; until then the
        # dict from read_registry is flattened back to unique install paths.
        seen, paths = set(), []
        for records in read_registry().values():
            for record in records:
                if record["installPath"] not in seen:
                    seen.add(record["installPath"])
                    paths.append(record["installPath"])
        manifests = _load_manifests(paths)
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
