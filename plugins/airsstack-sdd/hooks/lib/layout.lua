-- The SDD artifact tree: where each artifact type lives, and provisioning it.
--
-- Single source of truth (code side) for the layout. The prose mirror is
-- `references/artifact-paths.md`; the two MUST agree. Change one, change the other.
--
-- Two roots, split by artifact type:
--   - rfcs/ : worktree-local, transient input, under the git-ignored `.airsstack/` tree.
--   - specs/, plans/, plans/_archive/ : HOME-global, durable, shared across every worktree of one
--     repo, keyed by a stable per-repo project key.
--
-- Separate from the `ensure-layout.lua` driver so the key derivation and the provisioning can be
-- exercised against temporary repositories by `airsl test`.

local fs = airsstack.fs
local hash = airsstack.hash
local path = airsstack.path
local proc = airsstack.proc
local regex = airsstack.regex

local M = {}

-- Worktree-local root, relative to the consuming project root.
M.RFC_LOCAL_ROOT = ".airsstack/cc/plugins/sdd"

-- The line that keeps the worktree-local tree out of the repository.
M.IGNORE_LINE = ".airsstack/"

-- Every character outside [A-Za-z0-9._-] becomes '-', matching `tr -c 'A-Za-z0-9._-' '-'`.
function M.sanitize(text)
  return regex.replace_all("[^A-Za-z0-9._-]", text or "", "-")
end

-- Runs git in `dir`, returning trimmed stdout or nil.
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

-- `fs.canonicalize` where it succeeds, the input where it does not.
local function realpath(target)
  local ok, resolved = pcall(fs.canonicalize, target)
  return ok and resolved or target
end

-- The stable per-repo project key for the working directory `dir`.
--
-- Every linked worktree resolves to the main repo's git-common-dir, so all worktrees of one
-- repository collapse onto one store. Without git, the canonical working directory is hashed
-- instead. The human-readable component is sanitised; the hash is taken from the full unsanitised
-- path, so two directories whose names sanitise to the same token still get distinct keys.
function M.project_key(dir)
  local common = M.git(dir, "rev-parse", "--git-common-dir")
  local absolute, base
  if common then
    if not path.is_absolute(common) then
      common = path.join(dir, common)
    end
    -- `absolute` is the main repository's `.git`, so the repository's own name is its parent's.
    absolute = path.join(realpath(path.dirname(common)), path.basename(common))
    base = M.sanitize(path.basename(path.dirname(absolute)))
  else
    absolute = realpath(dir)
    base = M.sanitize(path.basename(absolute))
  end

  -- SHA-1 rather than SHA-256, and only for compatibility: this key names directories that
  -- already exist on every machine the suite runs on, and a wider digest would silently orphan
  -- every one of them.
  return base .. "-" .. hash.sha1(absolute):sub(1, 8), absolute
end

-- The HOME-global root for one project key.
function M.home_root(home, key)
  return path.join(home, "cc", "plugins", "sdd", key)
end

-- Creates `directory` when it is missing. Returns true when it was created.
local function ensure_dir(directory)
  if fs.exists(directory) and fs.is_dir(directory) then
    return false
  end
  fs.mkdir(directory)
  return true
end

-- Appends `IGNORE_LINE` to `<dir>/.gitignore` unless it is already a whole line there.
--
-- Whole-line rather than substring: a `.gitignore` already carrying `!.airsstack/keep` contains
-- the text without ignoring the tree, and appending nothing would leave it un-ignored.
function M.ensure_gitignore(dir)
  local file = path.join(dir, ".gitignore")
  if not fs.exists(file) then
    fs.write(file, M.IGNORE_LINE .. "\n")
    return "created .gitignore with " .. M.IGNORE_LINE
  end

  for _, line in ipairs(fs.read_lines(file)) do
    if line == M.IGNORE_LINE then
      return nil
    end
  end

  fs.append(file, M.IGNORE_LINE .. "\n")
  return "appended " .. M.IGNORE_LINE .. " to .gitignore"
end

-- Provisions both roots for the project at `dir`. Returns the lines describing what was created.
--
-- Idempotent: creates only what is missing, and never duplicates the `.gitignore` line.
function M.provision(dir, home)
  local key = M.project_key(dir)
  local home_root = M.home_root(home, key)
  local created = {}

  local local_rfcs = path.join(dir, M.RFC_LOCAL_ROOT, "rfcs")
  if ensure_dir(local_rfcs) then
    created[#created + 1] = "created " .. path.join(M.RFC_LOCAL_ROOT, "rfcs")
  end

  for _, name in ipairs({ "specs", "plans", "plans/_archive" }) do
    local directory = path.join(home_root, name)
    if ensure_dir(directory) then
      created[#created + 1] = "created " .. directory
    end
  end

  -- Only the worktree-local root needs ignoring; the HOME-global one is outside every repository
  -- and cannot leak into a commit.
  local ignored = M.ensure_gitignore(dir)
  if ignored then
    created[#created + 1] = ignored
  end

  return created, key, home_root
end

return M
