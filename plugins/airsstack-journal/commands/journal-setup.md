---
description: Provision the airsstack-journal vault and force a full rebuild of its derived recall index. Idempotent.
---

# journal-setup

Provision the journal vault and force a full rebuild of the derived `.index/`.
Run both steps with the project's RTK-aware shell:

1. Provision the vault directory tree (idempotent):

   ```sh
   HOME_ROOT="${AIRSSTACK_HOME:-$HOME/.airsstack}"
airsl run --policy confined \
  --allow-env AIRSSTACK_HOME --allow-env HOME --allow-write "$HOME_ROOT" \
  "${CLAUDE_PLUGIN_ROOT}/scripts/provision.lua"
   ```

2. Rebuild the derived index from the full corpus:

   ```sh
   HOME_ROOT="${AIRSSTACK_HOME:-$HOME/.airsstack}"
airsl run --policy confined \
  --allow-env AIRSSTACK_HOME --allow-env HOME \
  --allow-read "$HOME_ROOT" --allow-write "$HOME_ROOT" \
  "${CLAUDE_PLUGIN_ROOT}/scripts/build-index.lua"
   ```

If `python3` is unavailable, report that the index could not be rebuilt; the
vault is still provisioned and usable. Report the resolved vault path
(`${AIRSSTACK_HOME:-~/.airsstack}/journal/`) and which index files were written.
