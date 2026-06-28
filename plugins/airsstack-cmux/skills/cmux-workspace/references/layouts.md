# Layout Recipes

Worked examples using `cmux-layout` and the equivalent raw-CLI sequences,
grounded on `cmux 0.64.17`.

---

## Recipe 1 — 3-pane dev layout (build / test / shell)

```sh
# Using the helper (geometry + per-pane command in one call):
${CLAUDE_PLUGIN_ROOT}/skills/cmux-workspace/scripts/cmux-layout \
  --name dev \
  --split right \
  --split down \
  --cmd "nvim ." \
  --cmd "npm run dev" \
  --cmd "npm test"
```

Equivalent raw-CLI sequence (drop to this when you need precise flags the helper doesn't
expose, e.g. a specific `--cwd` per pane):

```sh
# 1. Create the workspace without stealing focus.
#    NOTE: new-workspace takes `--focus false` — there is NO `--no-focus` flag on
#    this command. It prints `OK workspace:N` (no surface ref).
ws=$(cmux new-workspace --name dev --focus false | grep -o 'workspace:[0-9]*' | head -1)

# 2. Resolve the workspace's initial surface (pane 0); new-workspace does not return it.
s0=$(cmux list-pane-surfaces --workspace "$ws" | grep -o 'surface:[0-9]*' | head -1)

# 3. Add splits SCOPED TO THE WORKSPACE. Splitting by `--surface` is rejected for a
#    backgrounded workspace ("not_found: Surface not found"); use `--workspace`.
#    Each split prints `OK surface:N workspace:<ws>` — capture the new surface ref.
s1=$(cmux new-split right --workspace "$ws" --focus false | grep -o 'surface:[0-9]*' | head -1)
s2=$(cmux new-split down  --workspace "$ws" --focus false | grep -o 'surface:[0-9]*' | head -1)

# 4. Send startup commands, each PINNED to its surface ref. Never rely on focus or
#    the caller's $CMUX_SURFACE_ID default — that targets the CALLER's surface and
#    leaks the command into your own shell (or, inside Claude Code, the agent prompt).
cmux send --surface "$s0" "nvim ."     ; cmux send-key --surface "$s0" enter
cmux send --surface "$s1" "npm run dev"; cmux send-key --surface "$s1" enter
cmux send --surface "$s2" "npm test"   ; cmux send-key --surface "$s2" enter
```

---

## Recipe 2 — Side-by-side diff viewer

```sh
# Using the helper: two panes, no startup commands.
${CLAUDE_PLUGIN_ROOT}/skills/cmux-workspace/scripts/cmux-layout \
  --name diff \
  --split right
```

Raw-CLI equivalent:

```sh
ws=$(cmux new-workspace --name diff --focus false | grep -o 'workspace:[0-9]*' | head -1)
cmux new-split right --workspace "$ws" --focus false
```

---

## Recipe 3 — Ephemeral scratch workspace with a long-running server

```sh
# Helper: single right split, server on the right, shell on the left.
${CLAUDE_PLUGIN_ROOT}/skills/cmux-workspace/scripts/cmux-layout \
  --name scratch \
  --split right \
  --cmd "" \
  --cmd "python -m http.server 8080"
```

The empty `--cmd ""` for pane 1 leaves the left pane at a plain shell prompt.

Raw-CLI equivalent:

```sh
ws=$(cmux new-workspace --name scratch --focus false | grep -o 'workspace:[0-9]*' | head -1)
s0=$(cmux list-pane-surfaces --workspace "$ws" | grep -o 'surface:[0-9]*' | head -1)
s1=$(cmux new-split right --workspace "$ws" --focus false | grep -o 'surface:[0-9]*' | head -1)

# Leave the left pane ($s0) at a shell; run the server in the right pane ($s1).
cmux send --surface "$s1" "python -m http.server 8080"
cmux send-key --surface "$s1" enter
```

---

## Selecting and switching workspaces

After layout creation, switch to a workspace with:

```sh
cmux select-workspace --workspace workspace:1
```

Or inspect the current workspace:

```sh
cmux current-workspace
```

---

## When to use raw commands vs the helper

| Situation | Recommendation |
|---|---|
| Simple geometry, optional per-pane command | Use `cmux-layout` |
| Need `--cwd` per pane, or custom per-surface targeting | Drop to raw commands |
| Layout from a pre-built JSON spec (`--layout <json>`) | Use `cmux new-workspace --layout` directly |
| More than 4-5 panes with complex ordering | Raw commands give more control |
