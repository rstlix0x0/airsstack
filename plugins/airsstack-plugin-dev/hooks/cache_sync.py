#!/usr/bin/env python3
"""airsstack-plugin-dev — PostToolUse cache-sync hook.

Mirrors an edited plugins/<plugin>/<rel> file into that plugin's install cache
(~/.claude/plugins/cache/<marketplace>/<plugin>/<version>/<rel>) so plugin
development at a fixed version needs no manual cp and no reinstall.

Only plugins installed from the `airsstack` marketplace are touched. Every
failure mode no-ops to exit 0; the hook never blocks the triggering tool.
"""

import json
import os
import shutil
import sys

MARKETPLACE = "airsstack"
CACHE_ROOT = os.path.join(
    os.path.expanduser("~"), ".claude", "plugins", "cache"
)
INSTALLED_PLUGINS = os.path.join(
    os.path.expanduser("~"), ".claude", "plugins", "installed_plugins.json"
)
DEBUG = bool(os.environ.get("AIRSSTACK_PLUGIN_DEV_DEBUG"))


def _debug(msg):
    if DEBUG:
        sys.stderr.write("[cache-sync] " + msg + "\n")


def extract_plugin_rel(path):
    """Return (plugin, rel) for a path under plugins/<plugin>/, else None.

    `rel` is the path remainder after plugins/<plugin>/. Returns None when the
    path has no plugins/<plugin>/ segment or names a plugin dir with no file
    remainder.
    """
    parts = path.split(os.sep)
    for i, part in enumerate(parts):
        if part == "plugins" and i + 2 < len(parts):
            plugin = parts[i + 1]
            rel_parts = parts[i + 2:]
            if not plugin or not rel_parts:
                return None
            return plugin, os.sep.join(rel_parts)
    return None


def resolve_install_paths(installed_data, plugin):
    """Distinct installPath values for `<plugin>@airsstack`, first-seen order.

    The `@airsstack` suffix is the marketplace gate: plugins from any other
    marketplace are never selected.
    """
    key = plugin + "@" + MARKETPLACE
    entries = (installed_data.get("plugins") or {}).get(key) or []
    seen = []
    for entry in entries:
        ip = entry.get("installPath")
        if ip and ip not in seen:
            seen.append(ip)
    return seen


def is_within(child, parent):
    """True if `child` is `parent` or nested under it (normalized paths)."""
    parent_n = os.path.normpath(parent)
    child_n = os.path.normpath(child)
    return child_n == parent_n or child_n.startswith(parent_n + os.sep)


def sync_one(src, rel, install_path):
    """Copy src -> install_path/rel when dest is within CACHE_ROOT.

    Returns the destination path on success, or None when the destination
    falls outside the cache root (containment guard).
    """
    dest = os.path.normpath(os.path.join(install_path, rel))
    if not is_within(dest, CACHE_ROOT):
        _debug("skip (outside cache root): " + dest)
        return None
    os.makedirs(os.path.dirname(dest), exist_ok=True)
    shutil.copy2(src, dest)
    _debug("synced: " + src + " -> " + dest)
    return dest


def _read_payload():
    try:
        raw = sys.stdin.read()
    except Exception:
        return None
    if not raw.strip():
        return None
    try:
        return json.loads(raw)
    except (ValueError, TypeError):
        return None


def _load_installed():
    try:
        with open(INSTALLED_PLUGINS, encoding="utf-8") as fh:
            return json.load(fh)
    except (OSError, ValueError):
        return None


def main():
    payload = _read_payload()
    if not payload:
        return 0
    tool_input = payload.get("tool_input") or {}
    src = tool_input.get("file_path")
    if not src:
        return 0
    src = os.path.abspath(src)
    if not os.path.isfile(src):
        return 0
    found = extract_plugin_rel(src)
    if not found:
        return 0
    plugin, rel = found
    installed = _load_installed()
    if not installed:
        return 0
    for install_path in resolve_install_paths(installed, plugin):
        try:
            sync_one(src, rel, install_path)
        except OSError as exc:
            _debug("sync failed: " + str(exc))
    return 0


if __name__ == "__main__":
    sys.exit(main())
