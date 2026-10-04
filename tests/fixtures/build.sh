#!/bin/sh
# Builds tests/fixtures/v1.db: a database made by the server itself, then
# filled from seed.sql. Run from the package directory, with a debug build:
#
#   cargo build && sh tests/fixtures/build.sh
#
# The file it writes is committed. Rebuild it only when the seed changes, and
# never to make a failing golden test pass.

set -e

HERE=$(cd "$(dirname "$0")" && pwd)
BIN="${FUGANTT_BIN:-$(cargo metadata --format-version 1 --no-deps | sed 's/.*"target_directory":"\([^"]*\)".*/\1/')/debug/fugantt}"
WORK=$(mktemp -d)
DB="$WORK/fugantt.db"
PORT=18611

# The server makes the schema, the shared account and the holidays for the
# years around the pinned day.
# `exec`, so the process to stop is the server and not a shell around it.
( cd "$WORK" && \
  FUGANTT_DB="$DB" FUGANTT_OPEN=0 FUGANTT_TODAY=2026-09-15 PORT=$PORT \
  FUGANTT_NO_AUTH=yes-everyone-on-this-network-can-edit \
  exec "$BIN" >"$WORK/log" 2>&1 ) &
SERVER=$!
trap 'kill $SERVER 2>/dev/null || true' EXIT

for _ in $(seq 1 100); do
  curl -fsS "http://127.0.0.1:$PORT/" >/dev/null 2>&1 && break
  sleep 0.1
done
curl -fsS "http://127.0.0.1:$PORT/" >/dev/null

kill $SERVER
wait $SERVER 2>/dev/null || true
trap - EXIT

sqlite3 "$DB" <"$HERE/seed.sql"
# One file, nothing left in the write-ahead log.
sqlite3 "$DB" "PRAGMA wal_checkpoint(TRUNCATE); PRAGMA journal_mode=DELETE; VACUUM;"

cp "$DB" "$HERE/v1.db"
rm -rf "$WORK"
echo "wrote $HERE/v1.db"
