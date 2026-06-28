#!/bin/sh
# Tests for cmux-preflight — stubs `cmux` on PATH; checks status + exit codes.
set -u
fail=0
SCRIPT_DIR=$(CDPATH='' cd "$(dirname "$0")" && pwd)
SCRIPT="$SCRIPT_DIR/cmux-preflight"
check() { if [ "$1" -eq 0 ]; then printf 'PASS: %s\n' "$2"; else printf 'FAIL: %s\n' "$2"; fail=1; fi; }

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
mkdir -p "$TMP/bin"

# Stub cmux: ping ok, version prints a line.
cat > "$TMP/bin/cmux" <<'EOF'
#!/bin/sh
case "$1" in
  ping) exit 0 ;;
  version) echo "cmux 0.64.17 (97)"; exit 0 ;;
  *) exit 0 ;;
esac
EOF
chmod +x "$TMP/bin/cmux"

# Mint a real unix socket file so [ -S ] is true.
sock="$TMP/cmux.sock"
python3 -c 'import socket,sys; s=socket.socket(socket.AF_UNIX); s.bind(sys.argv[1])' "$sock"

# 1. Healthy: binary + socket + ping ok → exit 0, status ok.
out=$(PATH="$TMP/bin:$PATH" CMUX_SOCKET_PATH="$sock" CMUX_WORKSPACE_ID=ws1 CMUX_SURFACE_ID=sf1 sh "$SCRIPT" --json)
rc=$?
{ [ "$rc" -eq 0 ] && printf '%s' "$out" | grep -q '"status":"ok"'; }
check $? "healthy → exit 0 + status ok (rc=$rc, out=$out)"

# 2. No binary on PATH → exit nonzero.
( PATH="/nonexistent" CMUX_SOCKET_PATH="$sock" sh "$SCRIPT" >/dev/null 2>&1 ) && r=1 || r=0
check "$r" "missing binary → nonzero exit"

# 3. Binary present, socket absent → exit nonzero + status no-socket.
out=$(PATH="$TMP/bin:$PATH" CMUX_SOCKET_PATH="$TMP/absent.sock" sh "$SCRIPT" --json 2>&1)
rc=$?
{ [ "$rc" -ne 0 ] && printf '%s' "$out" | grep -q '"status":"no-socket"'; }
check $? "missing socket → nonzero + no-socket (rc=$rc, out=$out)"

# 4. ping fails → exit nonzero + status unreachable.
cat > "$TMP/bin/cmux" <<'EOF'
#!/bin/sh
case "$1" in
  ping) exit 1 ;;
  version) echo "cmux 0.64.17 (97)"; exit 0 ;;
  *) exit 0 ;;
esac
EOF
chmod +x "$TMP/bin/cmux"
out=$(PATH="$TMP/bin:$PATH" CMUX_SOCKET_PATH="$sock" sh "$SCRIPT" --json 2>&1)
rc=$?
{ [ "$rc" -ne 0 ] && printf '%s' "$out" | grep -q '"status":"unreachable"'; }
check $? "ping fail → nonzero + unreachable (rc=$rc, out=$out)"

if [ "$fail" -eq 0 ]; then printf 'ALL PASS\n'; exit 0; else printf 'FAILURES\n'; exit 1; fi
