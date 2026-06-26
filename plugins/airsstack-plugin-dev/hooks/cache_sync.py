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
