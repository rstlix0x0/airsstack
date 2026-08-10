-- cmux-settings — safe-edit helper for ~/.config/cmux/cmux.json.
--
-- Backs the config up to a timestamped .bak before any edit, runs the edit, then runs
-- `cmux config validate`; on an invalid result it restores the backup and exits non-zero, so a
-- broken config is never left in place. The config path honours CMUX_SETTINGS_FILE.
--
-- Usage:
--   cmux-settings.lua backup
--   cmux-settings.lua validate
--   cmux-settings.lua backup-then <cmd> [args...]
--
-- `backup-then` runs an arbitrary program, so the caller names it in the grant:
--
--   airsl run --policy confined \
--     --allow-env HOME --allow-env CMUX_SETTINGS_FILE --allow-env CMUX_QUIET \
--     --allow-read "$HOME/.config/cmux" --allow-write "$HOME/.config/cmux" \
--     --allow-exec cmux --allow-exec <edit-command> \
--     scripts/cmux-settings.lua backup-then <cmd> [args...]

local cmux = require("lib.cmux")
local fs = airsstack.fs
local path = airsstack.path

local config = airsstack.env.get("CMUX_SETTINGS_FILE")
if not config or config == "" then
  config = path.join(airsstack.env.get("HOME") or "", ".config", "cmux", "cmux.json")
end

local function backup()
  local ok, found = pcall(fs.is_file, config)
  if not ok or not found then
    cmux.die("cmux-settings: config not found: " .. config)
  end
  local target = config .. "." .. os.date("%Y%m%d-%H%M%S") .. ".bak"
  if not pcall(fs.copy, config, target) then
    cmux.die("cmux-settings: backup failed")
  end
  return target
end

local function valid()
  return cmux.run({ "config", "validate" }, false).status == 0
end

local function restore(from)
  pcall(fs.copy, from, config)
end

local command = arg[1]

if command == "backup" then
  backup()
elseif command == "validate" then
  if not valid() then
    cmux.die("cmux-settings: config invalid")
  end
elseif command == "backup-then" then
  if not arg[2] then
    cmux.die("cmux-settings: backup-then needs a command")
  end

  local argv = {}
  for index = 2, #arg do
    argv[#argv + 1] = arg[index]
  end

  local saved = backup()

  local ok, result = pcall(airsstack.proc.run, argv)
  if not ok or result.status ~= 0 then
    if ok and result.stderr ~= "" then
      airsstack.stdio.error(result.stderr)
    end
    restore(saved)
    cmux.die("cmux-settings: edit command failed; restoring backup")
  end
  if result.stdout ~= "" then
    airsstack.stdio.write(result.stdout)
  end

  if not valid() then
    restore(saved)
    cmux.die("cmux-settings: config invalid after edit; restoring backup")
  end
else
  cmux.die("cmux-settings: unknown command: " .. tostring(command)
    .. " (use backup|validate|backup-then)")
end
