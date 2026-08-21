---
name: demo
description: Fixture skill whose fenced command the scripted case runs verbatim.
---

# Demo

1. Run the gate. It reads its payload from stdin, the same way the runtime
   invokes it:

   ```sh
   sh "${CLAUDE_PLUGIN_ROOT}/hooks/gate.sh"
   ```
