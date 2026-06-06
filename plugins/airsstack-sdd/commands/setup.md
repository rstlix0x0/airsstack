---
description: Provision the airsstack-sdd artifact tree (.airsstack/cc/plugins/sdd/{rfcs,specs,plans}) and ensure .airsstack/ is git-ignored. Idempotent.
---

## Layout provisioning output

!`sh "${CLAUDE_PLUGIN_ROOT}/hooks/ensure-layout.sh"`

## Task

Report to the user what the layout script did above — which directories were created,
whether `.gitignore` was updated, or that everything was already present. Do not run
any other commands; the provisioning has already happened.
