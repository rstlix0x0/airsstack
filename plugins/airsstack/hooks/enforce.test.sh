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

printf 'PASS\n'
