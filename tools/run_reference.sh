#!/usr/bin/env bash
# Start the QuestDB 10.1.0 reference binary in an isolated root.
# Does not build from source. Exits non-zero when Java or the binary is missing.
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
BIN_DIR="${QDB_REFERENCE_BIN:-$ROOT_DIR/reference/questdb-10.1.0-no-jre-bin}"
DATA_DIR="${QDB_REFERENCE_ROOT:-$ROOT_DIR/reference/runtime/questdb-root}"
if [[ ! -x "$BIN_DIR/questdb.sh" ]]; then
  echo "error: reference binary not found at $BIN_DIR" >&2
  echo "download: https://github.com/questdb/questdb/releases/download/10.1.0/questdb-10.1.0-no-jre-bin.tar.gz" >&2
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
mkdir -p "$DATA_DIR/conf"
if [[ ! -f "$DATA_DIR/conf/server.conf" ]]; then
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
fi
cd "$BIN_DIR"
exec ./questdb.sh start -d "$DATA_DIR"
