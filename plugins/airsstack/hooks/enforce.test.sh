#!/usr/bin/env sh
# Contract tests for the airsstack rule-enforcement dispatcher.
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
LAUNCHER="$SCRIPT_DIR/enforce.sh"

fail() { printf 'FAIL: %s\n' "$1" >&2; exit 1; }

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

# --- fixtures ----------------------------------------------------------
# airsstack rust guideline plugin dir + manifest
rustdir="$work/plugins/rust"; mkdir -p "$rustdir"
cat > "$rustdir/enforcement.json" <<'JSON'
{ "stack": "rust", "detect": ["Cargo.toml"], "match": ["**/*.rs", "**/Cargo.toml"],
  "skill": "airsstack-guideline-rust:rust-guidelines", "phase": ["code", "design"] }
JSON

# airsstack python guideline plugin dir + manifest (for polyglot design)
pydir="$work/plugins/py"; mkdir -p "$pydir"
cat > "$pydir/enforcement.json" <<'JSON'
{ "stack": "python", "detect": ["pyproject.toml"], "match": ["**/*.py"],
  "skill": "airsstack-guideline-python:python-guidelines", "phase": ["code", "design"] }
JSON

# external plugin dir + manifest — MUST be ignored by the scope guard
extdir="$work/plugins/ext"; mkdir -p "$extdir"
cat > "$extdir/enforcement.json" <<'JSON'
{ "stack": "evil", "match": ["**/*.rs"], "skill": "x:y", "phase": ["code"] }
JSON

# broken airsstack manifest — MUST be skipped without killing siblings
brokendir="$work/plugins/broken"; mkdir -p "$brokendir"
printf '{ not json ' > "$brokendir/enforcement.json"

# registry: three @airsstack entries (rust, python, broken) + one external
registry="$work/installed_plugins.json"
cat > "$registry" <<JSON
{ "plugins": {
  "airsstack-guideline-rust@airsstack": [ { "installPath": "$rustdir" } ],
  "airsstack-guideline-python@airsstack": [ { "installPath": "$pydir" } ],
  "airsstack-broken@airsstack": [ { "installPath": "$brokendir" } ],
  "superpowers@claude-plugins-official": [ { "installPath": "$extdir" } ]
} }
JSON

# fake repo cwd with Cargo.toml and pyproject.toml markers
repo="$work/repo"; mkdir -p "$repo/src"; : > "$repo/Cargo.toml"; : > "$repo/pyproject.toml"

# fake AIRSSTACK_HOME with an sdd specs dir
home="$work/home"; specdir="$home/cc/plugins/sdd/proj/specs"; mkdir -p "$specdir"

markers="$work/markers"; mkdir -p "$markers"

run() {  # session_id file_path cwd
  printf '{"session_id":"%s","cwd":"%s","tool_input":{"file_path":"%s"}}' "$1" "$3" "$2" \
  | AIRSSTACK_ENFORCE_REGISTRY="$registry" AIRSSTACK_HOME="$home" TMPDIR="$markers" \
    sh "$LAUNCHER"
}

# --- case 1: code-phase match → rust pointer ---------------------------
out=$(run s1 "$repo/src/lib.rs" "$repo")
printf '%s' "$out" | grep -q 'airsstack-guideline-rust:rust-guidelines' \
  || fail "case1: rust pointer not injected for .rs edit"
printf '%s' "$out" | grep -q '"permissionDecision"' \
  || fail "case1: permissionDecision missing"
printf '%s' "$out" | grep -q 'defer' \
  || fail "case1: permissionDecision is not defer"

# --- case 2: dedup → 2nd call same session is silent -------------------
out2=$(run s1 "$repo/src/other.rs" "$repo")
[ -z "$out2" ] || fail "case2: dedup failed, 2nd call emitted: $out2"

# --- case 3: scope guard → external 'evil' manifest never read ---------
printf '%s' "$out" | grep -q 'evil' && fail "case3: external manifest was read"

# --- case 8: malformed sibling skipped, rust still fires ---------------
# (case1 already proves rust fires while a broken @airsstack manifest is present)

# --- case 5: no match → silent ----------------------------------------
out5=$(run s5 "$repo/README.md" "$repo")
[ -z "$out5" ] || fail "case5: unrelated file emitted: $out5"

# --- case 6: missing registry → fail-open silent ----------------------
out6=$(printf '{"session_id":"s6","cwd":"%s","tool_input":{"file_path":"%s"}}' \
    "$repo" "$repo/src/lib.rs" \
  | AIRSSTACK_ENFORCE_REGISTRY="$work/nope.json" AIRSSTACK_HOME="$home" TMPDIR="$markers" \
    sh "$LAUNCHER")
[ -z "$out6" ] || fail "case6: missing registry not fail-open: $out6"

# --- case 4: design-phase → spec edit + repo marker → rust pointer -----
out4=$(run s4 "$specdir/2026-01-01-x.md" "$repo")
printf '%s' "$out4" | grep -q 'airsstack-guideline-rust:rust-guidelines' \
  || fail "case4: design-phase rust pointer not injected"

# --- case 7: polyglot design → both rust and python pointers ----------
printf '%s' "$out4" | grep -q 'airsstack-guideline-python:python-guidelines' \
  || fail "case7: design-phase python pointer not injected (polyglot)"


# --- case 11: hooks.json registers PreToolUse → enforce.sh -------------
hooks_json="$SCRIPT_DIR/hooks.json"
python3 -c "import json,sys; json.load(open('$hooks_json'))" \
  || fail "case11: hooks.json is not valid JSON"
grep -q '"PreToolUse"' "$hooks_json" || fail "case11: PreToolUse not registered"
grep -q 'enforce.sh' "$hooks_json" || fail "case11: enforce.sh not wired"

# --- case 12: docs describe the dispatcher; stale claim removed --------
air_readme="$SCRIPT_DIR/../README.md"
grep -qi 'enforcement.json' "$air_readme" || fail "case12: airsstack README missing enforcement.json"
rust_readme="$SCRIPT_DIR/../../airsstack-guideline-rust/README.md"
grep -qi 'No agents, no hooks' "$rust_readme" \
  && fail "case12: stale 'No agents, no hooks' claim still present"
printf 'docs ok\n' >/dev/null

# --- case 14: launcher exit matrix (D10) — every mode must exit 0 --------
# PreToolUse exit 2 blocks the tool call. Assert the launcher swallows every
# child failure mode instead of propagating it.
lwork="$work/launcher"; mkdir -p "$lwork"
cp "$LAUNCHER" "$lwork/enforce.sh"

probe() {  # -> prints the launcher's exit code
  printf '{"session_id":"L","cwd":"%s","tool_input":{"file_path":"%s"}}' "$repo" "$repo/src/lib.rs" \
  | AIRSSTACK_ENFORCE_REGISTRY="$registry" AIRSSTACK_HOME="$home" TMPDIR="$markers" \
    sh "$lwork/enforce.sh" >/dev/null 2>&1
  printf '%s' "$?"
}

rm -f "$lwork/enforce.py"
[ "$(probe)" = 0 ] || fail "case14: enforce.py missing did not exit 0"

printf 'import sys\n' > "$lwork/enforce.py"; chmod 000 "$lwork/enforce.py"
[ "$(probe)" = 0 ] || fail "case14: enforce.py unreadable did not exit 0"
chmod 644 "$lwork/enforce.py"

printf 'def (\n' > "$lwork/enforce.py"
[ "$(probe)" = 0 ] || fail "case14: enforce.py SyntaxError did not exit 0"

printf 'raise SystemExit(3)\n' > "$lwork/enforce.py"
[ "$(probe)" = 0 ] || fail "case14: enforce.py nonzero SystemExit did not exit 0"

cp "$SCRIPT_DIR/enforce.py" "$lwork/enforce.py"
[ "$(probe)" = 0 ] || fail "case14: healthy enforce.py did not exit 0"

# --- case 15: node runtime fully removed (D5) ---------------------------
# The runtime's filename is assembled from parts on purpose: this file is one
# of the files scanned below, so spelling the name literally here would make
# the check match its own source and fail forever.
js_stem='enforce'
[ -e "$SCRIPT_DIR/${js_stem}.js" ] && fail "case15: node runtime file still exists"
for f in "$SCRIPT_DIR/enforce.sh" "$SCRIPT_DIR/hooks.json" "$SCRIPT_DIR/enforce.test.sh" \
         "$SCRIPT_DIR/../README.md"; do
  grep -q "${js_stem}\\.js" "$f" && fail "case15: node runtime referenced in $f"
done
printf 'no node references\n' >/dev/null

# --- case 16: gate 1 — record bound to another project contributes nothing
otherrepo="$work/other"; mkdir -p "$otherrepo"; : > "$otherrepo/Cargo.toml"
registry16="$work/installed_plugins_16.json"
cat > "$registry16" <<JSON
{ "plugins": {
  "airsstack-guideline-rust@airsstack": [
    { "scope": "project", "projectPath": "$otherrepo", "installPath": "$rustdir" }
  ]
} }
JSON
out16=$(printf '{"session_id":"s16","cwd":"%s","tool_input":{"file_path":"%s"}}' \
    "$repo" "$repo/src/lib.rs" \
  | AIRSSTACK_ENFORCE_REGISTRY="$registry16" AIRSSTACK_HOME="$home" TMPDIR="$markers" \
    sh "$LAUNCHER")
[ -z "$out16" ] || fail "case16: project-bound record leaked into another repo: $out16"

# --- case 17: gate 2 — .rs file with no Cargo.toml above it is silent ----
bare="$work/bare"; mkdir -p "$bare/src"
out17=$(run s17 "$bare/src/lib.rs" "$bare")
[ -z "$out17" ] || fail "case17: .rs outside a Cargo project emitted: $out17"

# --- case 18: gate 3 — root Cargo.toml matches **/Cargo.toml ------------
out18=$(run s18 "$repo/Cargo.toml" "$repo")
printf '%s' "$out18" | grep -q 'airsstack-guideline-rust:rust-guidelines' \
  || fail "case18: root Cargo.toml did not match **/Cargo.toml"

# --- case 19: subagent gets its own shot --------------------------------
out19a=$(printf '{"session_id":"s19","cwd":"%s","tool_input":{"file_path":"%s"}}' \
    "$repo" "$repo/src/lib.rs" \
  | AIRSSTACK_ENFORCE_REGISTRY="$registry" AIRSSTACK_HOME="$home" TMPDIR="$markers" \
    sh "$LAUNCHER")
printf '%s' "$out19a" | grep -q 'rust-guidelines' || fail "case19: main thread got no pointer"
out19b=$(printf '{"session_id":"s19","agent_id":"explorer-1","cwd":"%s","tool_input":{"file_path":"%s"}}' \
    "$repo" "$repo/src/other.rs" \
  | AIRSSTACK_ENFORCE_REGISTRY="$registry" AIRSSTACK_HOME="$home" TMPDIR="$markers" \
    sh "$LAUNCHER")
printf '%s' "$out19b" | grep -q 'rust-guidelines' \
  || fail "case19: subagent shot was consumed by the main thread"

# --- case 20: hooks.json trigger surface --------------------------------
grep -q '"Read|Edit|Write"' "$hooks_json" || fail "case20: PreToolUse matcher is not Read|Edit|Write"
grep -q '"compact"' "$hooks_json" || fail "case20: SessionStart compact matcher missing"
grep -q 'rearm.sh' "$hooks_json" || fail "case20: rearm.sh not wired"

# --- case 21: rearm clears this session's sentinels and re-arms ----------
out21a=$(run s21 "$repo/src/lib.rs" "$repo")
printf '%s' "$out21a" | grep -q 'rust-guidelines' || fail "case21: first pointer missing"
out21b=$(run s21 "$repo/src/two.rs" "$repo")
[ -z "$out21b" ] || fail "case21: dedup failed before rearm"

# rearm.sh must see the same TMPDIR the dispatcher wrote its sentinels into.
if ! printf '{"session_id":"s21"}' \
   | TMPDIR="$markers" sh "$SCRIPT_DIR/rearm.sh" >/dev/null 2>&1; then
  fail "case21: rearm.sh exited non-zero"
fi

out21c=$(run s21 "$repo/src/three.rs" "$repo")
printf '%s' "$out21c" | grep -q 'rust-guidelines' || fail "case21: pointer did not re-arm after compact"

# --- case 21b: rearm is scoped to one session ---------------------------
out21d=$(run s21b "$repo/src/lib.rs" "$repo")
printf '%s' "$out21d" | grep -q 'rust-guidelines' || fail "case21b: first pointer missing"
if ! printf '{"session_id":"s21"}' \
   | TMPDIR="$markers" sh "$SCRIPT_DIR/rearm.sh" >/dev/null 2>&1; then
  fail "case21b: rearm.sh exited non-zero"
fi
out21e=$(run s21b "$repo/src/four.rs" "$repo")
[ -z "$out21e" ] || fail "case21b: rearm of s21 cleared another session's sentinel: $out21e"

# --- case 22: README documents the repaired dispatcher ------------------
grep -q 'Read|Edit|Write' "$air_readme" || fail "case22: README does not document the Read trigger"
grep -qi 'repo-relative' "$air_readme" || fail "case22: README still describes basename matching"
grep -qi 'project binding' "$air_readme" || fail "case22: README does not document the project-binding gate"
grep -qi 'compact' "$air_readme" || fail "case22: README does not document the compaction re-arm"

# --- case 23: plugin version bumped past 0.1.0 --------------------------
plugin_json="$SCRIPT_DIR/../.claude-plugin/plugin.json"
python3 -c "
import json,sys
v=json.load(open('$plugin_json'))['version']
sys.exit(0 if v != '0.1.0' else 1)
" || fail "case23: airsstack plugin.json still at 0.1.0 (cache will not refresh)"

# --- case 24: the doctor command exists and drives enforce.py -----------
# The invocation LINE, not file-wide substring greps: `--explain` and
# `ARGUMENTS` each also appear in this doc's prose (`--explain` x2,
# `ARGUMENTS` x3), so a file-wide `grep -q` only proves the words exist
# somewhere in a markdown file — measured: deleting the entire fenced
# command block, or dropping any one of `--explain` / `$ARGUMENTS` /
# `${CLAUDE_PLUGIN_ROOT}` from the command line, still left a file-wide
# grep GREEN. This pattern anchors on the actual invocation, so all four
# of those mutations turn it RED.
doctor="$SCRIPT_DIR/../commands/enforce-doctor.md"
[ -f "$doctor" ] || fail "case24: commands/enforce-doctor.md missing"
grep -Eq '\$\{CLAUDE_PLUGIN_ROOT\}/hooks/enforce\.py"[[:space:]]+--explain[[:space:]]+"\$ARGUMENTS"' \
  "$doctor" \
  || fail "case24: doctor does not invoke \${CLAUDE_PLUGIN_ROOT}/hooks/enforce.py --explain \$ARGUMENTS"

# --- case 25: --explain runs end to end and names a stage ---------------
out25=$(AIRSSTACK_ENFORCE_REGISTRY="$registry" AIRSSTACK_HOME="$home" TMPDIR="$markers" \
  python3 "$SCRIPT_DIR/enforce.py" --explain "$repo/src/doctor.rs" 2>&1)
printf '%s' "$out25" | grep -q 'outcome:' || fail "case25: --explain printed no outcome"
printf '%s' "$out25" | grep -q 'python:' || fail "case25: --explain printed no runtime"

# --- case 26: --explain never exits non-zero -----------------------------
# Structural form deliberately, not `cmd; [ "$?" = 0 ] || fail ...`: under
# `set -eu` a bare failing command aborts the script at that line, so a
# `$?`-based check below it can never run and its `fail` message can never
# print — an assertion that cannot report its own failure is not a test.
if ! TMPDIR="$markers" python3 "$SCRIPT_DIR/enforce.py" --explain >/dev/null 2>&1; then
  fail "case26: --explain with no path exited non-zero"
fi

# --- case 27: README documents the doctor -------------------------------
# A bare `grep -q 'enforce-doctor'` is file-wide, same shape as case 24: a
# heading mention alone satisfies it even with the explanatory prose
# removed. Require the heading AND a substantive sentence from that section.
grep -q 'enforce-doctor' "$air_readme" || fail "case27: README does not document the doctor"
grep -q 'names the stage that' "$air_readme" \
  || fail "case27: README mentions enforce-doctor but the explanation of what it does is missing"

# --- case 28: plugin version bumped to 0.1.2 for the commands/ addition -
if ! python3 -c "
import json,sys
v=json.load(open('$SCRIPT_DIR/../.claude-plugin/plugin.json'))['version']
sys.exit(0 if v == '0.1.2' else 1)
"; then
  fail "case28: airsstack plugin.json not bumped to 0.1.2"
fi

printf 'PASS\n'
