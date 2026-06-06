#!/usr/bin/env python3
"""airsstack concise — UserPromptSubmit hook.

Detects concise activation, level switch, and deactivation in the user prompt
(slash command + natural language), persists the active level to a
brand-namespaced flag file, and re-injects the active level's directive every
turn so terse mode survives the whole session instead of drifting back to
verbose. Must never throw or block the prompt — every path fails silently.
"""

import json
import os
import re
import sys

LEVELS = ("lite", "full", "ultra")
DEFAULT_LEVEL = "full"

STATE_ROOT = os.environ.get("AIRSSTACK_HOME") or os.path.join(
    os.path.expanduser("~"), ".airsstack"
)
FLAG_PATH = os.path.join(STATE_ROOT, "cc", "concise.json")


def write_level(level):
    try:
        os.makedirs(os.path.dirname(FLAG_PATH), exist_ok=True)
        # Never write through a symlink planted at the flag path.
        try:
            if os.path.islink(FLAG_PATH):
                os.unlink(FLAG_PATH)
        except OSError:
            pass  # missing is fine
        with open(FLAG_PATH, "w", encoding="utf-8") as fh:
            fh.write(json.dumps({"level": level}) + "\n")
        try:
            os.chmod(FLAG_PATH, 0o600)
        except OSError:
            pass
    except OSError:
        pass  # silent


def clear_level():
    try:
        os.unlink(FLAG_PATH)
    except OSError:
        pass  # already off


def read_level():
    try:
        if os.path.islink(FLAG_PATH):
            return None
        if os.stat(FLAG_PATH).st_size > 1024:
            return None
        with open(FLAG_PATH, "r", encoding="utf-8") as fh:
            level = (json.load(fh) or {}).get("level")
        return level if level in LEVELS else None
    except (OSError, ValueError):
        return None


def directive(level):
    common = (
        "Keep ALL technical substance, code blocks, shell commands, and error "
        "text verbatim. Technical terms exact. Write normally (clarity over "
        "brevity) for security warnings, irreversible-action confirmations, and "
        "ordered multi-step instructions."
    )
    by_level = {
        "lite": (
            "AIRSSTACK CONCISE: LITE. Drop filler (just/really/basically/"
            "actually/simply), hedging, and pleasantries. Keep articles and "
            "complete sentences."
        ),
        "full": (
            "AIRSSTACK CONCISE: FULL. Drop articles where unambiguous, filler, "
            "hedging, pleasantries. Fragments OK. Prefer short synonyms."
        ),
        "ultra": (
            "AIRSSTACK CONCISE: ULTRA. Telegraphic. Maximal compression — "
            "fragments, bullets, minimal connective words."
        ),
    }
    return by_level[level] + " " + common


def main():
    try:
        data = json.loads(sys.stdin.read() or "{}")
        lower = (data.get("prompt") or "").strip().lower()

        handled = False

        # Deactivation first, so "stop concise" never re-activates below.
        if (
            re.search(r"\bnormal mode\b", lower)
            or re.search(r"\bverbose mode\b", lower)
            or re.search(r"\b(stop|disable|deactivate|turn off|exit)\b[^.]*\bconcise\b", lower)
            or re.search(r"\bconcise\b[^.]*\b(off|stop|disable|deactivate|turn off)\b", lower)
        ):
            clear_level()
            handled = True

        # Slash command: /concise or /airsstack:concise [level|off]
        if not handled:
            m = re.match(r"/(?:airsstack:)?concise(?:\s+(\S+))?", lower)
            if m:
                arg = m.group(1)
                if not arg:
                    write_level(DEFAULT_LEVEL)
                elif arg in ("off", "stop", "disable"):
                    clear_level()
                elif arg in LEVELS:
                    write_level(arg)
                # unknown arg → flag untouched (no silent overwrite)
                handled = True

        # Natural-language activation: "concise mode", "be terse", "ultra concise"...
        if (
            not handled
            and re.search(r"\b(concise|terse)\b", lower)
            and re.search(r"\b(mode|be|use|go|make it|turn on|enable|activate|talk)\b", lower)
        ):
            lvl = next(
                (l for l in LEVELS if re.search(r"\b" + l + r"\b", lower)),
                DEFAULT_LEVEL,
            )
            write_level(lvl)

        # Persistence: re-inject the active level's directive every turn.
        active = read_level()
        if active:
            sys.stdout.write(
                json.dumps(
                    {
                        "hookSpecificOutput": {
                            "hookEventName": "UserPromptSubmit",
                            "additionalContext": directive(active),
                        }
                    }
                )
            )
    except Exception:
        pass  # silent — the hook must never block the prompt


if __name__ == "__main__":
    main()
