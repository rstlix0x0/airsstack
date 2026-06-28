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
expose, e.g. a specific `--cwd` per pane or `--no-focus` during bulk creation):

```sh
# 1. Create the workspace without stealing focus.
cmux new-workspace --name dev --no-focus

# 2. Add splits. Each new-split targets the caller workspace by default.
cmux new-split right --no-focus
cmux new-split down --no-focus

# 3. Send startup commands to each pane in order.
#    Pane refs are assigned 1-based: pane:1 = first (initial), pane:2, pane:3, ...
cmux focus-pane --pane pane:1
cmux send "nvim ."
cmux send-key enter

cmux focus-pane --pane pane:2
cmux send "npm run dev"
cmux send-key enter

cmux focus-pane --pane pane:3
cmux send "npm test"
cmux send-key enter
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
cmux new-workspace --name diff --no-focus
cmux new-split right --no-focus
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
cmux new-workspace --name scratch --no-focus
cmux new-split right --no-focus

# Leave pane:1 at shell (no send needed).
cmux focus-pane --pane pane:2
cmux send "python -m http.server 8080"
cmux send-key enter
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
| Need `--cwd` per pane, or custom `--no-focus` logic | Drop to raw commands |
| Layout from a pre-built JSON spec (`--layout <json>`) | Use `cmux new-workspace --layout` directly |
| More than 4-5 panes with complex ordering | Raw commands give more control |
