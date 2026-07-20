#!/usr/bin/env python3
"""airsstack enforcement re-arm — SessionStart(compact) hook.

Compaction drops the injected additionalContext out of the window, but the
session_id survives it (measured: one sessionId across a transcript spanning a
compact event). Without this the dispatcher's one-shot-per-context sentinel
would keep the rule suppressed for the rest of the session. Unlinking this
session's sentinels lets the pointer re-enter context on the next Read/Edit.
"""

import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import enforce  # noqa: E402


def main():
    try:
        data = json.loads(sys.stdin.read() or "{}")
        enforce.clear_session(data.get("session_id") or "")
    except Exception:
        pass  # fail-open: a re-arm failure must never disturb the session


if __name__ == "__main__":
    main()
