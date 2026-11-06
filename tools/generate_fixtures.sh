#!/usr/bin/env bash
# Recreate tests/fixtures/h1 from a live QuestDB 10.1.0 server.
# Refuses to run when the reference binary or Java 25 is missing.
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
BIN_DIR="${QDB_REFERENCE_BIN:-$ROOT_DIR/reference/questdb-10.1.0-no-jre-bin}"
DATA_DIR="${QDB_REFERENCE_ROOT:-$ROOT_DIR/reference/runtime/fixture-regen}"
HTTP="${QDB_HTTP:-http://127.0.0.1:19011/exec}"
if [[ ! -x "$BIN_DIR/questdb.sh" ]]; then
  echo "error: cannot generate fixtures; missing $BIN_DIR/questdb.sh" >&2
  exit 1
fi
if [[ -z "${JAVA_HOME:-}" ]]; then
  if [[ -x /usr/lib/jvm/java-25-openjdk-amd64/bin/java ]]; then
    export JAVA_HOME=/usr/lib/jvm/java-25-openjdk-amd64
  else
    echo "error: JAVA_HOME is unset and Java 25 was not found" >&2
    exit 1
  fi
fi
rm -rf "$DATA_DIR"
mkdir -p "$DATA_DIR/conf"
cat > "$DATA_DIR/conf/server.conf" << 'EOF'
http.enabled=true
http.bind.to=127.0.0.1:19011
http.min.enabled=false
pg.enabled=false
line.tcp.enabled=false
line.udp.enabled=false
line.http.enabled=false
telemetry.enabled=false
cairo.wal.enabled.default=false
query.timeout.sec=60
EOF
"$BIN_DIR/questdb.sh" start -d "$DATA_DIR"
cleanup() {
  "$BIN_DIR/questdb.sh" stop -d "$DATA_DIR" >/dev/null 2>&1 || true
}
trap cleanup EXIT
for _ in $(seq 1 30); do
  if curl -sf -G "$HTTP" --data-urlencode "query=SELECT 1" >/dev/null; then
    break
  fi
  sleep 1
done
curl -sf -G "$HTTP" --data-urlencode "query=SELECT 1" >/dev/null
while IFS= read -r -d '' sql; do
  curl -sf -G "$HTTP" --data-urlencode "query=$sql" >/dev/null
done < <(python3 - "$ROOT_DIR/tests/fixtures/h1/load.sql" << 'PY'
import sys
from pathlib import Path
lines = []
for line in Path(sys.argv[1]).read_text().splitlines():
    if line.strip().startswith("--"):
        continue
    lines.append(line)
for statement in "\n".join(lines).split(";"):
    statement = " ".join(statement.split())
    if statement:
        sys.stdout.write(statement + "\0")
PY
)
curl -sf -G "$HTTP" --data-urlencode "query=SELECT count() FROM sensors" | grep -q '"dataset":[[4]]'
"$BIN_DIR/questdb.sh" stop -d "$DATA_DIR"
trap - EXIT
DEST="$ROOT_DIR/tests/fixtures/h1/db/sensors"
rm -rf "$DEST"
mkdir -p "$DEST"
cp -a "$DATA_DIR/db/sensors/." "$DEST/"
(
  cd "$ROOT_DIR"
  find tests/fixtures/h1/db -type f -print0 | sort -z | xargs -0 sha256sum > tests/fixtures/h1/SHA256SUMS
)
echo "fixture rewritten at $DEST"
echo "re-run cargo test; byte checksums can change if table id or preallocation differs"
