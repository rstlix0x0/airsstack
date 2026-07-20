#!/usr/bin/env python3
"""airsstack rule-enforcement dispatcher — PreToolUse(Read|Edit|Write) hook.

Reads the installed-plugins registry, keeps only airsstack-marketplace
plugins, loads each one's enforcement.json, and — for the file being edited —
surfaces the matching guideline skill via additionalContext. Fail-open: never
blocks, denies, or raises out of main().
"""

import hashlib
import json
import os
import re
import subprocess
import sys
import time

MARKETPLACE_SUFFIX = "@airsstack"
SENTINEL_PREFIX = "airsstack-enforce-"
SENTINEL_MAX_AGE = 24 * 3600  # seconds; stale sentinels are pruned past this


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


def load_manifest(install_path):
    """Read and validate one plugin's enforcement.json, or None."""
    try:
        with open(os.path.join(install_path, "enforcement.json"), "r", encoding="utf-8") as fh:
            data = json.load(fh)
    except (OSError, ValueError):
        return None  # absent or malformed: skip this plugin, keep the rest
    if not isinstance(data, dict):
        return None
    stack, skill = data.get("stack"), data.get("skill")
    if not stack or not skill:
        return None
    return {
        "stack": stack,
        "skill": skill,
        "detect": data.get("detect") or [],
        "match": data.get("match") or [],
        "phase": data.get("phase") or ["code", "design"],
    }


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


def pointer(stack, skill):
    return (
        stack + " work is in play. The " + skill + " skill is MANDATORY for "
        "this work \u2014 load it now via Skill before proceeding, and apply its "
        "rules (Definition of Done + architecture)."
    )


def sentinel_dir():
    return os.environ.get("TMPDIR") or "/tmp"


def sentinel_path(session_id, agent, stack, phase):
    """One sentinel per (session, agent context, stack, phase).

    `agent` is the subagent id when the hook fires inside one, else 'main'.
    Subagents inherit the parent's session_id, so without that component an
    explorer reading one .rs file would consume the main thread's only
    pointer — and the main thread is exactly the context this exists to
    inform.
    """
    parts = [
        _sanitize(session_id or "nosession"),
        _sanitize(agent or "main"),
        _sanitize(stack),
        _sanitize(phase),
    ]
    return os.path.join(sentinel_dir(), SENTINEL_PREFIX + "-".join(parts))


def sentinel_claimed(path):
    """Read-only probe for the cheap gate; never creates anything."""
    return os.path.exists(path)


def claim(path):
    """Atomically claim a sentinel. True means this invocation must emit.

    O_CREAT|O_EXCL is atomic by construction, so no locking is needed. The
    previous read-then-append design was an unguarded read-modify-write:
    under measurement 3 of 4 concurrent hooks all fired.
    """
    try:
        fd = os.open(path, os.O_CREAT | os.O_EXCL | os.O_WRONLY, 0o600)
    except FileExistsError:
        return False
    except OSError:
        return True  # cannot write the marker: prefer a repeat over silence
    os.close(fd)
    return True


def prune_sentinels():
    try:
        now = time.time()
        directory = sentinel_dir()
        for name in os.listdir(directory):
            if not name.startswith(SENTINEL_PREFIX):
                continue
            path = os.path.join(directory, name)
            try:
                if now - os.path.getmtime(path) > SENTINEL_MAX_AGE:
                    os.unlink(path)
            except OSError:
                pass
    except OSError:
        pass


def clear_session(session_id):
    """Unlink every sentinel for one session; returns the count removed."""
    prefix = SENTINEL_PREFIX + _sanitize(session_id or "nosession") + "-"
    removed = 0
    try:
        directory = sentinel_dir()
        for name in os.listdir(directory):
            if not name.startswith(prefix):
                continue
            try:
                os.unlink(os.path.join(directory, name))
                removed += 1
            except OSError:
                pass
    except OSError:
        pass
    return removed


def resolve(file_path, cwd, session_id, agent, registry=None, home=None, trace=None):
    """The ordered pipeline of spec §7.1. Returns the list of pointers to emit.

    `trace`, when a list is passed, collects one line per stage so the doctor
    can explain any of the six silent-exit paths without reimplementing this.
    """
    def note(line):
        if trace is not None:
            trace.append(line)

    plugins = read_registry(registry)
    note("registry: %d @airsstack plugin(s)" % len(plugins))
    if not plugins:
        note("STOP: no @airsstack plugins in the registry")
        return []

    # Manifests are plugin content, identical across a plugin's install
    # paths, so any readable record answers "which stack:phase might fire".
    candidates = []
    for key, records in sorted(plugins.items()):
        manifest = None
        for record in records:
            manifest = load_manifest(record["installPath"])
            if manifest:
                break
        if not manifest:
            note("%s: no usable enforcement.json" % key)
            continue
        candidates.append((key, records, manifest))
    note("manifests: %d loaded" % len(candidates))
    if not candidates:
        note("STOP: zero manifests loaded (delivery failure — run the parity check)")
        return []

    phase = "design" if is_design_doc(file_path, home) else "code"
    note("phase: %s" % phase)

    # CHEAP GATE (§7.1 step 6): if every key this event could produce is
    # already claimed, stop before paying for any git subprocess.
    wanted = [c for c in candidates if phase in c[2]["phase"]]
    if not wanted:
        note("STOP: no manifest declares phase %s" % phase)
        return []
    unclaimed = [
        c for c in wanted
        if not sentinel_claimed(sentinel_path(session_id, agent, c[2]["stack"], phase))
    ]
    if not unclaimed:
        note("STOP: every candidate stack:phase already claimed this context")
        return []

    current_key = project_key(cwd)
    note("project key: %s" % current_key)
    key_cache = {}
    candidate_path = path_for_matching(file_path, cwd) if phase == "code" else None
    if candidate_path:
        note("match path: %s" % candidate_path)

    pointers = []
    for key, records, manifest in unclaimed:
        record = select_record(records, current_key, key_cache)
        if record is None:
            note("%s: GATE 1 no record bound to this project" % key)
            continue
        bound = load_manifest(record["installPath"]) or manifest
        # A design doc lives under AIRSSTACK_HOME, outside every repo, so it
        # has no directory of its own to anchor the marker search on; `cwd`
        # is the only signal of which project it describes.
        if phase == "code":
            active = marker_active(file_path, bound["detect"], cwd)
        else:
            active = marker_active_in(cwd, bound["detect"])
        if not active:
            note("%s: GATE 2 no detect marker" % key)
            continue
        if phase == "code" and not matches_any(candidate_path, bound["match"]):
            note("%s: GATE 3 no match glob hit" % key)
            continue
        if not claim(sentinel_path(session_id, agent, bound["stack"], phase)):
            note("%s: sentinel claimed concurrently" % key)
            continue
        note("%s: EMIT %s" % (key, bound["skill"]))
        pointers.append(pointer(bound["stack"], bound["skill"]))
    return pointers


def main():
    try:
        data = json.loads(sys.stdin.read() or "{}")
        tool_input = data.get("tool_input") or {}
        file_path = tool_input.get("file_path")
        if not file_path:
            return
        cwd = data.get("cwd") or os.getcwd()
        session_id = data.get("session_id") or ""
        agent = data.get("agent_id") or "main"

        prune_sentinels()

        pointers = resolve(file_path, cwd, session_id, agent)
        if not pointers:
            return

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
