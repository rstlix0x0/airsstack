#!/bin/sh
# Snapshot the airsstack-journal vault before a /journal-review run so any
# curator edit is reversible. Tars daily/ sessions/ notes/ mocs/ into
# .backups/<timestamp>.tar.gz and prunes to the newest KEEP backups. Excludes
# .index/ (derived) and .backups/ (recursive). Empty/absent vault is a no-op
# (exit 0). Any tar/IO failure -> stderr + nonzero so the review aborts before
# any write. Prints the archive path on success.
set -u

root="${AIRSSTACK_HOME:-$HOME/.airsstack}/journal"
keep="${AIRSSTACK_JOURNAL_BACKUP_KEEP:-10}"

# Nothing to protect yet.
[ -d "$root" ] || exit 0

# Collect the content dirs that actually exist.
set --
for d in daily sessions notes mocs; do
  [ -d "$root/$d" ] && set -- "$@" "$d"
done
# Empty vault (no content dirs) -> no-op.
[ "$#" -eq 0 ] && exit 0

backups="$root/.backups"
mkdir -p "$backups" || { printf 'journal-backup: cannot create %s\n' "$backups" >&2; exit 1; }

stamp=$(date +%Y-%m-%d-%H%M%S)
archive="$backups/$stamp.tar.gz"

# -C roots the archive at the vault so member paths are daily/... not absolute.
if ! tar -czf "$archive" -C "$root" "$@" 2>/dev/null; then
  printf 'journal-backup: tar failed\n' >&2
  rm -f "$archive"
  exit 1
fi

# Retention: keep the newest $keep tarballs, prune older.
count=0
for f in $(ls -1t "$backups"/*.tar.gz 2>/dev/null); do
  count=$((count + 1))
  [ "$count" -gt "$keep" ] && rm -f "$f"
done

printf '%s\n' "$archive"
exit 0
