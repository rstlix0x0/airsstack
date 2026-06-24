#!/bin/sh
# Tests for journal-backup.sh — snapshot the vault, prune to KEEP, restore.
set -u
fail=0
SCRIPT_DIR=$(CDPATH= cd "$(dirname "$0")" && pwd)
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
export AIRSSTACK_HOME="$TMP"
root="$TMP/journal"
mkdir -p "$root/notes" "$root/daily" "$root/.index"
printf 'n\n' > "$root/notes/alpha.md"
printf 'd\n' > "$root/daily/2026-06-24.md"
printf 'idx\n' > "$root/.index/index.json"

check() { if [ "$1" -eq 0 ]; then printf 'PASS: %s\n' "$2"; else printf 'FAIL: %s\n' "$2"; fail=1; fi; }

# 1. Backup creates a tarball containing notes/ + daily/, excluding .index/.
ARCH=$(sh "$SCRIPT_DIR/journal-backup.sh")
check $? "backup exits 0"
[ -f "$ARCH" ]; check $? "tarball created"
tar -tzf "$ARCH" | grep -q 'notes/alpha.md'; check $? "notes/ in archive"
tar -tzf "$ARCH" | grep -q 'daily/2026-06-24.md'; check $? "daily/ in archive"
if tar -tzf "$ARCH" | grep -q '\.index'; then nidx=1; else nidx=0; fi
[ "$nidx" -eq 0 ]; check $? ".index/ excluded"

# 2. Restore reproduces a deleted note.
rm -f "$root/notes/alpha.md"
tar -xzf "$ARCH" -C "$root"
[ -f "$root/notes/alpha.md" ]; check $? "restore reproduces deleted note"

# 3. Retention caps at KEEP (oldest pruned).
export AIRSSTACK_JOURNAL_BACKUP_KEEP=2
i=0
while [ "$i" -lt 4 ]; do
  sh "$SCRIPT_DIR/journal-backup.sh" >/dev/null 2>&1
  sleep 1
  i=$((i + 1))
done
n=$(ls -1 "$root/.backups"/*.tar.gz 2>/dev/null | wc -l | tr -d ' ')
[ "$n" -le 2 ]; check $? "retention caps backups at KEEP=2 (got $n)"

# 4. Absent vault -> exit 0, no tarball.
TMP2=$(mktemp -d); export AIRSSTACK_HOME="$TMP2"
sh "$SCRIPT_DIR/journal-backup.sh" >/dev/null 2>&1; check $? "absent vault exits 0"
rm -rf "$TMP2"

if [ "$fail" -eq 0 ]; then printf 'ALL PASS\n'; exit 0; else printf 'FAILURES\n'; exit 1; fi
