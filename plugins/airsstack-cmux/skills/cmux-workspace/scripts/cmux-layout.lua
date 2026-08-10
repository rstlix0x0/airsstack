-- cmux-layout — build a cmux workspace layout: create a named workspace, add splits, and
-- optionally run a shell command in each pane.
--
-- Pure geometry plus an optional per-pane command. It does NOT spawn agent sessions or coordinate
-- agents (that is the deferred super-agent's job); --provider is rejected.
--
-- Usage: cmux-layout.lua --name <title> [--split left|right|up|down]... [--cmd <shell>]...
-- Commands map to panes in order: pane 1 = initial surface, then each split.
--
-- Targeting is EXPLICIT: every cmux call is pinned to a ref captured from the command that created
-- it. The script never relies on the caller's $CMUX_SURFACE_ID or focus, so splits and commands
-- always land in the NEW workspace — never leaked into the caller's interactive surface.
--
--   airsl run --policy confined --allow-env CMUX_QUIET --allow-exec cmux \
--     scripts/cmux-layout.lua --name <title> [--split ...] [--cmd ...]

local cmux = require("lib.cmux")
local layout = require("lib.layout")

local args = {}
for index = 1, #arg do
  args[index] = arg[index]
end

local plan, reason = layout.parse(args)
if not plan then
  cmux.die("cmux-layout: " .. reason)
end

-- Create the workspace WITHOUT stealing focus. The output carries `workspace:N` and no surface
-- ref, so the initial surface is resolved separately below.
local created = cmux.run({ "new-workspace", "--name", plan.name, "--focus", "false" }, true)
if created.status ~= 0 then
  cmux.die("cmux-layout: new-workspace failed")
end
local workspace = cmux.first_ref(created.stdout, "workspace")
if not workspace then
  cmux.die("cmux-layout: could not parse new workspace ref")
end

local listed = cmux.run({ "list-pane-surfaces", "--workspace", workspace }, true)
local initial = cmux.first_ref(listed.stdout, "surface")
if not initial then
  cmux.die("cmux-layout: could not resolve initial surface")
end

-- Line N is pane N, in creation order: pane 1 is the initial surface, then each split's surface.
local surfaces = { initial }

-- Splits are addressed by --workspace rather than --surface: cmux refuses to split a surface
-- belonging to a backgrounded workspace, but a workspace-scoped split succeeds and returns the new
-- ref. Geometry fans off pane 1, which stays the active pane under --focus false.
for _, direction in ipairs(plan.splits) do
  local split = cmux.run({ "new-split", direction, "--workspace", workspace, "--focus", "false" }, true)
  if split.status ~= 0 then
    cmux.die("cmux-layout: new-split " .. direction .. " failed")
  end
  local surface = cmux.first_ref(split.stdout, "surface")
  if not surface then
    cmux.die("cmux-layout: could not parse split surface")
  end
  surfaces[#surfaces + 1] = surface
end

-- The i-th command runs in the i-th pane. An empty command leaves that pane at a plain shell
-- prompt but still consumes its slot, so later commands stay aligned with their panes.
for index, command in ipairs(plan.cmds) do
  local surface = surfaces[index]
  if command ~= "" and surface then
    if cmux.run({ "send", "--surface", surface, command }, true).status ~= 0 then
      cmux.die("cmux-layout: send to " .. surface .. " failed")
    end
    cmux.run({ "send-key", "--surface", surface, "enter" }, true)
  end
end

airsstack.stdio.write(workspace .. "\n")
