-- Detecting concise-mode intent in a user prompt, and persisting the active level.
--
-- Split from the hook driver so the classifier can be exercised against strings. That matters more
-- here than elsewhere: the rules are a pile of overlapping natural-language patterns, and the only
-- way to keep "stop being concise" from *enabling* concise mode is to pin the precedence in tests.

local fs = airsstack.fs
local json = airsstack.json
local path = airsstack.path
local regex = airsstack.regex

local M = {}

M.LEVELS = { "lite", "full", "ultra" }
M.DEFAULT_LEVEL = "full"

-- The largest a flag file may be before it is treated as junk rather than as state.
M.MAX_FLAG_BYTES = 1024

-- Deactivation is tested first, so "stop concise" never falls through to the activation rules.
local DEACTIVATE = {
  regex.compile([[\bnormal mode\b]]),
  regex.compile([[\bverbose mode\b]]),
  regex.compile([[\b(stop|disable|deactivate|turn off|exit)\b[^.]*\bconcise\b]]),
  regex.compile([[\bconcise\b[^.]*\b(off|stop|disable|deactivate|turn off)\b]]),
}

-- `/concise` or `/airsstack:concise [level|off]`, anchored: a slash command is the whole prompt's
-- opening, not something found in the middle of a sentence.
local COMMAND = regex.compile([[^/(?:airsstack:)?concise(?:\s+(\S+))?]])

local SUBJECT = regex.compile([[\b(concise|terse)\b]])
local VERB = regex.compile([[\b(mode|be|use|go|make it|turn on|enable|activate|talk)\b]])

local OFF_WORDS = { off = true, stop = true, disable = true }

-- Whether `level` is one this plugin knows.
function M.known(level)
  for _, candidate in ipairs(M.LEVELS) do
    if candidate == level then
      return true
    end
  end
  return false
end

-- What a prompt asks for: "off", a level name, or nil for "it said nothing about concise mode".
--
-- The third answer is distinct from the first on purpose. An unrecognised argument to the slash
-- command leaves the flag untouched rather than overwriting it, because guessing which level the
-- author meant is worse than doing nothing.
function M.classify(prompt)
  local lower = prompt:match("^%s*(.-)%s*$"):lower()

  for _, pattern in ipairs(DEACTIVATE) do
    if pattern.is_match(lower) then
      return "off"
    end
  end

  local command = COMMAND.captures(lower)
  if command then
    local argument = command[1]
    if not argument or argument == "" then
      return M.DEFAULT_LEVEL
    end
    if OFF_WORDS[argument] then
      return "off"
    end
    if M.known(argument) then
      return argument
    end
    return nil -- an unknown argument is not an instruction
  end

  if SUBJECT.is_match(lower) and VERB.is_match(lower) then
    for _, level in ipairs(M.LEVELS) do
      if regex.is_match([[\b]] .. level .. [[\b]], lower) then
        return level
      end
    end
    return M.DEFAULT_LEVEL
  end

  return nil
end

-- The flag file holding the active level.
function M.flag_path(home)
  return path.join(home, "cc", "concise.json")
end

-- The active level, or nil.
--
-- A symlink at the flag path is refused rather than followed, and so is an oversized file: both
-- mean something other than this hook is writing there, and neither is state this hook should
-- trust or overwrite through.
function M.read_level(file)
  local ok, stamp = pcall(fs.stat, file)
  if not ok or stamp.kind == "symlink" or stamp.size > M.MAX_FLAG_BYTES then
    return nil
  end
  local read, text = pcall(fs.read, file)
  if not read then
    return nil
  end
  local decoded, value = pcall(json.decode, text)
  if not decoded or type(value) ~= "table" then
    return nil
  end
  return M.known(value.level) and value.level or nil
end

-- Persists `level`, replacing a symlink planted at the flag path rather than writing through it.
function M.write_level(file, level)
  pcall(fs.mkdir, path.dirname(file))
  local ok, stamp = pcall(fs.stat, file)
  if ok and stamp.kind == "symlink" then
    pcall(fs.remove, file)
  end
  fs.write(file, json.encode({ level = level }) .. "\n")
end

-- Clears the flag. An absent flag is already off, so failing to remove one is not an error.
function M.clear_level(file)
  pcall(fs.remove, file)
end

-- The directive re-injected on every turn while a level is active.
function M.directive(level)
  local common = "Keep ALL technical substance, code blocks, shell commands, and error "
    .. "text verbatim. Technical terms exact. Write normally (clarity over "
    .. "brevity) for security warnings, irreversible-action confirmations, and "
    .. "ordered multi-step instructions."

  local by_level = {
    lite = "AIRSSTACK CONCISE: LITE. Drop filler (just/really/basically/"
      .. "actually/simply), hedging, and pleasantries. Keep articles and "
      .. "complete sentences.",
    full = "AIRSSTACK CONCISE: FULL. Drop articles where unambiguous, filler, "
      .. "hedging, pleasantries. Fragments OK. Prefer short synonyms.",
    ultra = "AIRSSTACK CONCISE: ULTRA. Telegraphic. Maximal compression — "
      .. "fragments, bullets, minimal connective words.",
  }

  return by_level[level] .. " " .. common
end

return M
