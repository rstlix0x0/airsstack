# Workspace Groups

Collapsible sidebar collections grounded on `cmux 0.64.17`. Groups are managed by the
`workspace-group` family of subcommands. `workspace-group` is a real nested cmux command
(`cmux workspace-group --help` exits 0) — it does not appear in the top-level `cmux --help`
Commands list but is fully supported. For the live subcommand reference, run
`cmux workspace-group --help`.

---

## Concept

A group is a collapsible collection of workspaces in the cmux sidebar. Each group is
**anchored** to one workspace: the group header in the sidebar IS the anchor workspace's
row. Closing the anchor dissolves the group but preserves the other member workspaces as
ungrouped entries — they are not closed.

Groups are addressed by a UUID or a `workspace_group:N` short ref printed by `list`.
All subcommands accept `--json`.

---

## Group subcommand table

| Subcommand | Syntax | Purpose |
|---|---|---|
| `list` | `list [--json]` | List all groups with their member workspaces. |
| `create` | `create [--name <name>] [--cwd <path>] [--from <id>,<id>...]` | Create a group from workspaces. Defaults `--from` to the active sidebar selection / caller workspace when omitted. |
| `add` | `add --group <group> --workspace <ws>` | Add a workspace to an existing group. |
| `remove` | `remove --workspace <ws>` | Remove a workspace from its current group (workspace is ungrouped, not closed). |
| `rename` | `rename <group> --name <new>` | Rename the group. |
| `collapse` | `collapse <group>` | Collapse the group row in the sidebar. |
| `expand` | `expand <group>` | Expand the group row. |
| `pin` | `pin <group>` | Pin the group (prevents accidental close / dissolve). |
| `unpin` | `unpin <group>` | Unpin the group. |
| `set-color` | `set-color <group> [--hex #RRGGBB]` | Set the sidebar accent color for the group header. |
| `set-icon` | `set-icon <group> [--symbol <sf-symbol>]` | Set the SF Symbol icon for the group header. |
| `set-anchor` | `set-anchor --group <group> --workspace <ws>` | Reassign the anchor workspace before closing the current anchor. |
| `new-workspace` | `new-workspace <group> [--placement afterCurrent\|top\|end]` | Create a new workspace inside the group. Placement defaults to `afterCurrent`. |
| `focus` | `focus <group>` | Focus the group's anchor workspace. |
| `move` | `move <group> (--to-index <n> \| --before <group> \| --after <group>)` | Reorder the group in the sidebar. |
| `ungroup` | `ungroup <group>` | Dissolve the group; member workspaces become ungrouped. Non-destructive. |
| `delete` | `delete <group>` | Destructive — delete the group AND close every workspace inside it. Prefer `ungroup` to preserve workspaces. |

---

## Creating workspaces in groups

**Option A — At creation time via `new-workspace --group`:**

```sh
# Add a new workspace to an existing group:
cmux new-workspace --name feature-x --group workspace_group:1 --group-placement afterCurrent

# Equivalent with --group-reference to inherit an existing member's placement:
cmux new-workspace --name feature-x --group workspace_group:1 --group-reference workspace:2
```

**Option B — Create the group first, then add members:**

```sh
# 1. Create a group from the caller workspace plus workspace:2:
cmux workspace-group create --name "Dev Session" --from workspace:1,workspace:2

# 2. Add another workspace later:
cmux workspace-group add --group workspace_group:1 --workspace workspace:3
```

---

## Anchor workspace concept

The group header displayed in the cmux sidebar IS the anchor workspace's row. Rules:

- Closing the anchor workspace dissolves the group; surviving members become ungrouped.
- Use `set-anchor` to reassign the anchor before closing the current one if you want
  the group to survive the close.
- Use `ungroup` to dissolve the group while keeping all workspaces open.
- `delete` is destructive — it closes every workspace in the group.
