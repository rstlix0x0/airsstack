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

# --- case 10: node fallback parity (skipped if node absent) ------------
if command -v node >/dev/null 2>&1; then
  runjs() {  # session_id file_path cwd
    printf '{"session_id":"%s","cwd":"%s","tool_input":{"file_path":"%s"}}' "$1" "$3" "$2" \
    | AIRSSTACK_ENFORCE_REGISTRY="$registry" AIRSSTACK_HOME="$home" TMPDIR="$markers" \
      node "$SCRIPT_DIR/enforce.js"
  }
  outjs=$(runjs js1 "$repo/src/lib.rs" "$repo")
  printf '%s' "$outjs" | grep -q 'airsstack-guideline-rust:rust-guidelines' \
    || fail "case10: node enforce.js did not inject rust pointer"
  printf '%s' "$outjs" | grep -q 'defer' \
    || fail "case10: node enforce.js missing permissionDecision defer"
  outjs2=$(runjs js1 "$repo/src/other.rs" "$repo")
  [ -z "$outjs2" ] || fail "case10: node enforce.js dedup failed: $outjs2"
else
  printf 'SKIP case10: node not installed\n' >&2
fi

# --- case 13: glob char-class parity js==py ----------------------------
# Exercises [seq] and [!seq] character-class metacharacters.  Python fnmatch
# handles these natively; enforce.js globToRegExp must now match.
# Skipped when node is not installed (mirrors case 10 guard).
if command -v node >/dev/null 2>&1; then
  # fixture: airsstack plugin whose manifest match uses a char-class glob
  ccdir="$work/plugins/charclass"; mkdir -p "$ccdir"
  cat > "$ccdir/enforcement.json" <<'JSON'
{ "stack": "charclass-test", "detect": [], "match": ["**/v[0-9].rs"],
  "skill": "charclass-skill:check", "phase": ["code"] }
JSON
  registry13="$work/installed_plugins_13.json"
  cat > "$registry13" <<JSON
{ "plugins": {
  "airsstack-charclass@airsstack": [ { "installPath": "$ccdir" } ]
} }
JSON

  runjs13() {  # session_id file_path cwd
    printf '{"session_id":"%s","cwd":"%s","tool_input":{"file_path":"%s"}}' "$1" "$3" "$2" \
    | AIRSSTACK_ENFORCE_REGISTRY="$registry13" AIRSSTACK_HOME="$home" TMPDIR="$markers" \
      node "$SCRIPT_DIR/enforce.js"
  }
  runpy13() {  # session_id file_path cwd
    printf '{"session_id":"%s","cwd":"%s","tool_input":{"file_path":"%s"}}' "$1" "$3" "$2" \
    | AIRSSTACK_ENFORCE_REGISTRY="$registry13" AIRSSTACK_HOME="$home" TMPDIR="$markers" \
      python3 "$SCRIPT_DIR/enforce.py"
  }

  # matching file: v3.rs  → both runtimes must inject
  out13_js_match=$(runjs13 s13a "$repo/src/v3.rs" "$repo")
  out13_py_match=$(runpy13 s13b "$repo/src/v3.rs" "$repo")
  printf '%s' "$out13_js_match" | grep -q 'charclass-skill:check' \
    || fail "case13: enforce.js did not inject for v3.rs (char-class match)"
  printf '%s' "$out13_py_match" | grep -q 'charclass-skill:check' \
    || fail "case13: enforce.py did not inject for v3.rs (char-class match)"

  # non-matching file: vx.rs → both runtimes must be silent
  out13_js_nomatch=$(runjs13 s13c "$repo/src/vx.rs" "$repo")
  out13_py_nomatch=$(runpy13 s13d "$repo/src/vx.rs" "$repo")
  [ -z "$out13_js_nomatch" ] \
    || fail "case13: enforce.js injected for vx.rs (should not match [0-9]): $out13_js_nomatch"
  [ -z "$out13_py_nomatch" ] \
    || fail "case13: enforce.py injected for vx.rs (should not match [0-9]): $out13_py_nomatch"

  # negated class: vx.rs must match v[!0-9].rs  (x is not a digit)
  ccdir2="$work/plugins/charclass2"; mkdir -p "$ccdir2"
  cat > "$ccdir2/enforcement.json" <<'JSON'
{ "stack": "charclass-neg", "detect": [], "match": ["**/v[!0-9].rs"],
  "skill": "charclass-neg-skill:check", "phase": ["code"] }
JSON
  registry13b="$work/installed_plugins_13b.json"
  cat > "$registry13b" <<JSON
{ "plugins": {
  "airsstack-charclass-neg@airsstack": [ { "installPath": "$ccdir2" } ]
} }
JSON

  runjs13b() {
    printf '{"session_id":"%s","cwd":"%s","tool_input":{"file_path":"%s"}}' "$1" "$3" "$2" \
    | AIRSSTACK_ENFORCE_REGISTRY="$registry13b" AIRSSTACK_HOME="$home" TMPDIR="$markers" \
      node "$SCRIPT_DIR/enforce.js"
  }
  runpy13b() {
    printf '{"session_id":"%s","cwd":"%s","tool_input":{"file_path":"%s"}}' "$1" "$3" "$2" \
    | AIRSSTACK_ENFORCE_REGISTRY="$registry13b" AIRSSTACK_HOME="$home" TMPDIR="$markers" \
      python3 "$SCRIPT_DIR/enforce.py"
  }

  # vx.rs matches v[!0-9].rs → both inject
  out13b_js_match=$(runjs13b s13e "$repo/src/vx.rs" "$repo")
  out13b_py_match=$(runpy13b s13f "$repo/src/vx.rs" "$repo")
  printf '%s' "$out13b_js_match" | grep -q 'charclass-neg-skill:check' \
    || fail "case13: enforce.js did not inject for vx.rs (negated char-class [!0-9])"
  printf '%s' "$out13b_py_match" | grep -q 'charclass-neg-skill:check' \
    || fail "case13: enforce.py did not inject for vx.rs (negated char-class [!0-9])"

  # v3.rs does NOT match v[!0-9].rs → both silent
  out13b_js_nomatch=$(runjs13b s13g "$repo/src/v3.rs" "$repo")
  out13b_py_nomatch=$(runpy13b s13h "$repo/src/v3.rs" "$repo")
  [ -z "$out13b_js_nomatch" ] \
    || fail "case13: enforce.js injected for v3.rs against [!0-9] (should not match): $out13b_js_nomatch"
  [ -z "$out13b_py_nomatch" ] \
    || fail "case13: enforce.py injected for v3.rs against [!0-9] (should not match): $out13b_py_nomatch"
else
  printf 'SKIP case13: node not installed\n' >&2
fi

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

printf 'PASS\n'
