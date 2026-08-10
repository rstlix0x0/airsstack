-- The airsstack rule-enforcement pipeline.
--
-- Reads the installed-plugins registry, keeps only airsstack-marketplace plugins, loads each one's
-- enforcement.json, and — for the file being edited — decides which guideline skill to surface.
--
-- Split from the hook driver because this is the part with rules in it, and because the doctor
-- (`--explain`) has to drive exactly the same resolution the hook drives. A doctor that
-- reimplemented resolution would eventually disagree with the hook, and would then be lying about
-- the one thing it exists to make trustworthy.
--
-- Fail-open is the driver's job, not this module's: everything here returns values, and nothing
-- decides what the process does about them.

local globs = require("lib.globs")
local fs = airsstack.fs
local hash = airsstack.hash
local json = airsstack.json
local path = airsstack.path
local proc = airsstack.proc
local regex = airsstack.regex

local M = {}

M.MARKETPLACE_SUFFIX = "@airsstack"
M.SENTINEL_PREFIX = "airsstack-enforce-"

-- Seconds; a sentinel older than this is pruned rather than kept suppressing its rule forever.
M.SENTINEL_MAX_AGE = 24 * 3600

-- Names the parity comparison never reports. Claude Code writes `.in_use` into every cache dir,
-- and without this the extras report is never empty and you learn to skim past it.
M.PARITY_IGNORED = { [".in_use"] = true, [".DS_Store"] = true, [".git"] = true }

-- Every character outside [A-Za-z0-9._-] becomes '-', matching `tr -c 'A-Za-z0-9._-' '-'`.
function M.sanitize(text)
  return regex.replace_all("[^A-Za-z0-9._-]", text or "", "-")
end

-- Runs git in `dir`; returns trimmed stdout, or nil on any failure.
--
-- `-C` rather than a working directory on the child: `proc.run` takes an argv array and nothing
-- else, so the directory travels as an argument.
function M.git(dir, ...)
  local argv = { "git", "-C", dir }
  for _, value in ipairs({ ... }) do
    argv[#argv + 1] = value
  end
  local ok, result = pcall(proc.run, argv)
  if not ok or result.status ~= 0 then
    return nil
  end
  local text = result.stdout:gsub("%s+$", "")
  return text ~= "" and text or nil
end

-- `fs.canonicalize` where the policy and the filesystem allow it, nil otherwise.
function M.realpath(target)
  local ok, resolved = pcall(fs.canonicalize, target)
  return ok and resolved or nil
end

-- Whether `target` exists, answering false where the question itself is refused.
function M.exists(target)
  local ok, found = pcall(fs.exists, target)
  return ok and found
end

-- Whether `target` is a file.
function M.is_file(target)
  local ok, found = pcall(fs.is_file, target)
  return ok and found
end

-- The stable per-repo key. Every linked worktree of one repository collapses to one value.
--
-- Keys, never path prefixes, are what gate 1 compares: a linked worktree may live anywhere on
-- disk, so comparing where it sits would fragment one project into several.
function M.project_key(cwd)
  local common = M.git(cwd, "rev-parse", "--git-common-dir")
  local absolute, base
  if common then
    if not path.is_absolute(common) then
      common = path.join(cwd, common)
    end
    local parent = M.realpath(path.dirname(common)) or path.dirname(common)
    absolute = path.join(parent, path.basename(common))
    base = path.basename(path.dirname(absolute))
  else
    absolute = M.realpath(cwd)
    if not absolute then
      return nil
    end
    base = path.basename(absolute)
  end
  return M.sanitize(base) .. "-" .. hash.sha1(absolute):sub(1, 8)
end

-- The path `match` globs are tested against.
--
-- Inside a repository: the path relative to the git toplevel, which is what `match` is documented
-- to mean. Outside any repository: the basename, which preserves coverage there rather than
-- silently dropping it.
function M.path_for_matching(file_path, cwd)
  local absolute = path.absolute(file_path)
  local target = M.realpath(absolute) or absolute

  local top = M.git(cwd, "rev-parse", "--show-toplevel")
  if top then
    top = M.realpath(top) or top
    if target == top then
      return "."
    end
    if target:sub(1, #top + 1) == top .. "/" then
      return target:sub(#top + 2)
    end
  end
  return path.basename(target)
end

-- Where the installed-plugins registry lives.
function M.registry_path(env)
  local override = env.get("AIRSSTACK_ENFORCE_REGISTRY")
  if override and override ~= "" then
    return override
  end
  return path.join(env.get("HOME") or "", ".claude", "plugins", "installed_plugins.json")
end

-- The HOME-global SDD artifact root.
function M.sdd_root(home)
  return path.join(home, "cc", "plugins", "sdd")
end

-- Whether `file_path` is an SDD spec or plan under the HOME-global root.
--
-- The directory name must appear as a whole path SEGMENT. A substring check on '/specs/' matched
-- `<key>/a/specs/b/plans/c.md` by accident, which is a phase misclassification rather than a
-- cosmetic slip: it swaps the Definition of Done for the architecture rules.
function M.is_design_doc(file_path, home)
  local target = path.absolute(file_path)
  local root = path.absolute(M.sdd_root(home))
  if target:sub(1, #root + 1) ~= root .. "/" then
    return false
  end

  local relative = target:sub(#root + 2)
  local segments = {}
  for segment in relative:gmatch("[^/]+") do
    segments[#segments + 1] = segment
  end
  -- Directories only, never the filename; the layout is <key>/<specs|plans>/...
  if #segments - 1 < 2 then
    return false
  end
  return segments[2] == "specs" or segments[2] == "plans"
end

-- `{plugin_key = {record, ...}}` for @airsstack plugins only.
--
-- The suffix check is the scope guard: a plugin from any other marketplace is never read and never
-- routed.
function M.read_registry(file)
  local ok, text = pcall(fs.read, file)
  if not ok then
    return {}
  end
  local decoded, data = pcall(json.decode, text)
  if not decoded or type(data) ~= "table" or type(data.plugins) ~= "table" then
    return {}
  end

  local kept = {}
  for key, records in pairs(data.plugins) do
    local suffix = key:sub(-#M.MARKETPLACE_SUFFIX)
    if suffix == M.MARKETPLACE_SUFFIX and type(records) == "table" then
      local usable = {}
      for _, record in ipairs(records) do
        if type(record) == "table" and type(record.installPath) == "string" then
          usable[#usable + 1] = record
        end
      end
      kept[key] = usable
    end
  end
  return kept
end

-- The registry record that governs this project (gate 1).
--
--   1. A record whose `projectPath` resolves to the current project key.
--   2. Otherwise the user-scope record.
--   3. Otherwise nothing — the anti-leak property: a plugin installed only for repo A contributes
--      nothing in repo B.
--
-- `cache` maps projectPath to project key; the caller owns it so the git subprocess runs at most
-- once per distinct path.
function M.select_record(records, current_key, cache)
  local fallback
  for _, record in ipairs(records) do
    local project_path = record.projectPath
    if type(project_path) == "string" and project_path ~= "" then
      if cache[project_path] == nil then
        cache[project_path] = M.project_key(project_path) or false
      end
      if current_key and cache[project_path] == current_key then
        return record
      end
    elseif not fallback then
      fallback = record
    end
  end
  return fallback
end

-- One plugin's enforcement.json, validated, or nil.
function M.load_manifest(install_path)
  local ok, text = pcall(fs.read, path.join(install_path, "enforcement.json"))
  if not ok then
    return nil -- absent or unreadable: skip this plugin, keep the rest
  end
  local decoded, data = pcall(json.decode, text)
  if not decoded or type(data) ~= "table" then
    return nil
  end
  if type(data.stack) ~= "string" or data.stack == ""
    or type(data.skill) ~= "string" or data.skill == "" then
    return nil
  end

  local function list(value)
    return type(value) == "table" and value or {}
  end

  local phase = list(data.phase)
  if #phase == 0 then
    phase = { "code", "design" }
  end

  return {
    stack = data.stack,
    skill = data.skill,
    detect = list(data.detect),
    match = list(data.match),
    phase = phase,
  }
end

-- Whether `manifest` declares `phase`.
function M.declares_phase(manifest, phase)
  for _, declared in ipairs(manifest.phase) do
    if declared == phase then
      return true
    end
  end
  return false
end

-- Whether a `detect` marker sits in `directory` or any ancestor.
--
-- Split from `marker_active` because the two phases anchor differently: a code file anchors on
-- itself, while an SDD design doc lives under AIRSSTACK_HOME and has no in-repo location to
-- anchor on, so it anchors on the working directory.
function M.marker_active_in(directory, markers)
  if #markers == 0 then
    return false
  end
  local current = path.absolute(directory ~= "" and directory or ".")
  while true do
    for _, marker in ipairs(markers) do
      if M.is_file(path.join(current, tostring(marker))) then
        return true
      end
    end
    local parent = path.dirname(current)
    if parent == current then
      return false
    end
    current = parent
  end
end

-- Whether a `detect` marker sits at or above the FILE's own directory.
--
-- The file's directory, not the session's: searching upward from `cwd` is wrong for any file
-- outside it, and `cwd` survives only as the fallback when the path has no directory component.
function M.marker_active(file_path, markers, cwd)
  local directory = path.dirname(path.absolute(file_path))
  if directory == "" then
    directory = cwd or "."
  end
  return M.marker_active_in(directory, markers)
end

-- The injected text for one stack and phase.
--
-- The two phases ask for different things. Code phase runs a build, so the Definition of Done
-- applies. A design doc is a spec or plan with nothing to build, so naming the Definition of Done
-- there is a demand the reader cannot satisfy; only the architecture rules can shape a design.
function M.pointer(stack, skill, phase)
  local tail = phase == "design"
    and "apply its architecture rules to this design."
    or "apply its rules (Definition of Done + architecture)."
  return stack .. " work is in play. The " .. skill .. " skill is MANDATORY for "
    .. "this work \u{2014} load it now via Skill before proceeding, and " .. tail
end

-- Where one-shot sentinels live.
function M.sentinel_dir(env)
  local override = env.get("TMPDIR")
  if override and override ~= "" then
    return override
  end
  return "/tmp"
end

-- One sentinel per (session, agent context, stack, phase).
--
-- `agent` is the subagent id when the hook fires inside one, else 'main'. Subagents inherit the
-- parent's session_id, so without that component an explorer reading one .rs file would consume
-- the main thread's only pointer — and the main thread is exactly the context this exists to
-- inform.
function M.sentinel_path(directory, session_id, agent, stack, phase)
  local parts = {
    M.sanitize(session_id ~= "" and session_id or "nosession"),
    M.sanitize(agent ~= "" and agent or "main"),
    M.sanitize(stack),
    M.sanitize(phase),
  }
  return path.join(directory, M.SENTINEL_PREFIX .. table.concat(parts, "-"))
end

-- Read-only probe for the cheap gate; never creates anything.
function M.sentinel_claimed(file)
  return M.exists(file)
end

-- Atomically claims a sentinel. True means this invocation must emit.
--
-- `create_exclusive` is O_CREAT|O_EXCL, atomic by construction, so no locking is needed. The
-- previous read-then-append design was an unguarded read-modify-write: under measurement 3 of 4
-- concurrent hooks all fired. A failure that is not "already there" returns true — a repeated
-- pointer is better than a silently suppressed one.
function M.claim(file)
  local ok, created = pcall(fs.create_exclusive, file)
  if not ok then
    return true
  end
  return created
end

-- Unlinks sentinels older than `SENTINEL_MAX_AGE`.
function M.prune_sentinels(directory, now)
  local ok, names = pcall(fs.list, directory)
  if not ok then
    return
  end
  for _, name in ipairs(names) do
    if name:sub(1, #M.SENTINEL_PREFIX) == M.SENTINEL_PREFIX then
      local file = path.join(directory, name)
      local read, stamp = pcall(fs.stat, file)
      if read and now - stamp.modified > M.SENTINEL_MAX_AGE then
        pcall(fs.remove, file)
      end
    end
  end
end

-- Unlinks every sentinel for one session; returns the count removed.
function M.clear_session(directory, session_id)
  local prefix = M.SENTINEL_PREFIX
    .. M.sanitize(session_id ~= "" and session_id or "nosession") .. "-"
  local removed = 0
  local ok, names = pcall(fs.list, directory)
  if not ok then
    return 0
  end
  for _, name in ipairs(names) do
    if name:sub(1, #prefix) == prefix and pcall(fs.remove, path.join(directory, name)) then
      removed = removed + 1
    end
  end
  return removed
end

-- The stack:phase keys already claimed for one session and agent context.
function M.held_sentinels(directory, session_id, agent)
  local prefix = M.SENTINEL_PREFIX .. M.sanitize(session_id ~= "" and session_id or "nosession")
    .. "-" .. M.sanitize(agent ~= "" and agent or "main") .. "-"
  local held = {}
  local ok, names = pcall(fs.list, directory)
  if not ok then
    return held
  end
  for _, name in ipairs(names) do
    if name:sub(1, #prefix) == prefix then
      held[#held + 1] = (name:sub(#prefix + 1):gsub("%-", ":", 1))
    end
  end
  table.sort(held)
  return held
end

-- The keys of a table, sorted.
local function sorted_keys(map)
  local keys = {}
  for key in pairs(map) do
    keys[#keys + 1] = key
  end
  table.sort(keys)
  return keys
end

-- The ordered resolution pipeline. Returns the pointers to emit.
--
-- `context` carries `file_path`, `cwd`, `session_id`, `agent`, `registry`, `home` and
-- `sentinel_dir`. `trace`, when a table is passed, collects one line per stage so the doctor can
-- explain any of the silent-exit paths without reimplementing this.
function M.resolve(context, trace)
  local function note(line)
    if trace then
      trace[#trace + 1] = line
    end
  end

  local plugins = M.read_registry(context.registry)
  local keys = sorted_keys(plugins)
  note("registry: " .. #keys .. " @airsstack plugin(s)")
  if #keys == 0 then
    note("STOP: no @airsstack plugins in the registry")
    return {}
  end

  -- Manifests are plugin content, identical across a plugin's install paths, so any readable
  -- record answers "which stack:phase might fire".
  local candidates = {}
  for _, key in ipairs(keys) do
    local manifest
    for _, record in ipairs(plugins[key]) do
      manifest = M.load_manifest(record.installPath)
      if manifest then
        break
      end
    end
    if manifest then
      candidates[#candidates + 1] = { key = key, records = plugins[key], manifest = manifest }
    else
      note(key .. ": no usable enforcement.json")
    end
  end
  note("manifests: " .. #candidates .. " loaded")
  if #candidates == 0 then
    note("STOP: zero manifests loaded (delivery failure — run the parity check)")
    return {}
  end

  local phase = M.is_design_doc(context.file_path, context.home) and "design" or "code"
  note("phase: " .. phase)

  -- CHEAP GATE: if every key this event could produce is already claimed, stop before paying for
  -- any git subprocess.
  local wanted = {}
  for _, candidate in ipairs(candidates) do
    if M.declares_phase(candidate.manifest, phase) then
      wanted[#wanted + 1] = candidate
    end
  end
  if #wanted == 0 then
    note("STOP: no manifest declares phase " .. phase)
    return {}
  end

  local unclaimed = {}
  for _, candidate in ipairs(wanted) do
    local sentinel = M.sentinel_path(context.sentinel_dir, context.session_id, context.agent,
      candidate.manifest.stack, phase)
    if not M.sentinel_claimed(sentinel) then
      unclaimed[#unclaimed + 1] = candidate
    end
  end
  if #unclaimed == 0 then
    note("STOP: every candidate stack:phase already claimed this context")
    return {}
  end

  local current_key = M.project_key(context.cwd)
  note("project key: " .. tostring(current_key))
  local cache = {}
  local candidate_path = phase == "code"
    and M.path_for_matching(context.file_path, context.cwd)
    or nil
  if candidate_path then
    note("match path: " .. candidate_path)
  end

  local pointers = {}
  for _, candidate in ipairs(unclaimed) do
    local record = M.select_record(candidate.records, current_key, cache)
    if not record then
      note(candidate.key .. ": GATE 1 no record bound to this project")
    else
      note(candidate.key .. ": using " .. record.installPath)
      local bound = M.load_manifest(record.installPath) or candidate.manifest

      local active
      if phase == "code" then
        active = M.marker_active(context.file_path, bound.detect, context.cwd)
      else
        active = M.marker_active_in(context.cwd, bound.detect)
      end

      if not active then
        note(candidate.key .. ": GATE 2 no detect marker")
      elseif phase == "code" and not globs.matches_any(candidate_path, bound.match) then
        note(candidate.key .. ": GATE 3 no match glob hit")
      elseif not M.claim(M.sentinel_path(context.sentinel_dir, context.session_id, context.agent,
        bound.stack, phase)) then
        note(candidate.key .. ": sentinel claimed concurrently")
      else
        note(candidate.key .. ": EMIT " .. bound.skill)
        pointers[#pointers + 1] = M.pointer(bound.stack, bound.skill, phase)
      end
    end
  end
  return pointers
end

-- Root-relative file paths under `root`, ignore-list applied, sorted.
--
-- Returns nil plus a reason when the tree cannot be listed. That distinction is the whole point:
-- an unreadable source tree must not read back as an empty one, which is how a delivery failure
-- reports "repo and cache agree".
function M.tree_files(root)
  local ok, entries = pcall(fs.walk, root)
  if not ok then
    return nil, "source tree unreadable, parity unknown"
  end

  local found = {}
  for _, rel in ipairs(entries) do
    local skip = false
    for component in rel:gmatch("[^/]+") do
      skip = skip or M.PARITY_IGNORED[component] == true
    end
    if not skip and M.is_file(path.join(root, rel)) then
      found[#found + 1] = rel
    end
  end
  table.sort(found)
  return found
end

-- Lines describing source files missing from or differing in the install cache.
--
-- The doctor ships inside the plugin and therefore runs FROM the cache. Faced with the delivery
-- bug it was built for, a pipeline trace alone would report "zero manifests loaded" and be unable
-- to say why. This is the part that can say why — but only when invoked inside the plugin source
-- repository.
function M.parity_report(top, plugins)
  local source_root = path.join(top, "plugins")
  if not M.exists(source_root) then
    return {}
  end

  local report = {}
  for _, key in ipairs(sorted_keys(plugins)) do
    local name = key
    if key:sub(-#M.MARKETPLACE_SUFFIX) == M.MARKETPLACE_SUFFIX then
      name = key:sub(1, #key - #M.MARKETPLACE_SUFFIX)
    end
    local src_dir = path.join(source_root, name)

    if M.exists(src_dir) then
      local files, reason = M.tree_files(src_dir)
      if not files then
        report[#report + 1] = name .. ": " .. reason
      elseif M.is_file(path.join(src_dir, ".claude-plugin", "plugin.json")) then
        -- Distinct installPath values only: several registry records can point at the SAME cache
        -- directory, and comparing once per record would multiply every line.
        local seen, cache_dirs = {}, {}
        for _, record in ipairs(plugins[key]) do
          if record.installPath and not seen[record.installPath] then
            seen[record.installPath] = true
            cache_dirs[#cache_dirs + 1] = record.installPath
          end
        end
        table.sort(cache_dirs)

        for _, cache_dir in ipairs(cache_dirs) do
          -- No existence guard on `cache_dir`: a registry-listed plugin whose cache directory is
          -- absent is the most complete delivery failure there is, and every path below simply
          -- reports MISSING for it. Guarding it out made a wholly-missing cache read as agreement.
          for _, rel in ipairs(files) do
            local dest = path.join(cache_dir, rel)
            if not M.exists(dest) then
              report[#report + 1] = name .. ": " .. rel .. " MISSING from cache"
            else
              local ok, same = pcall(fs.same_content, path.join(src_dir, rel), dest)
              if not ok or not same then
                report[#report + 1] = name .. ": " .. rel .. " DIFFERS from source"
              end
            end
          end
        end
      end
    end
  end
  return report
end

return M
