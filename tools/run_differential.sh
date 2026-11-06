#!/usr/bin/env bash
# Compare the Rust reader with captured QuestDB 10.1.0 results.
# If the reference HTTP server is already listening, also check row counts live.
# Missing reference server is reported; it is not treated as a passing live run.
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"
BIN="$(
  cargo build -p qdb-server --bin qdb --message-format=json \
    | python3 -c 'import json,sys
for line in sys.stdin:
    event = json.loads(line)
    target = event.get("target") or {}
    if event.get("reason") == "compiler-artifact" and target.get("name") == "qdb" and "bin" in target.get("kind", []):
        print(event.get("executable") or "")
'
)"
if [[ ! -x "$BIN" ]]; then
  echo "error: qdb binary was not produced" >&2
  exit 1
fi
DATA="$ROOT_DIR/tests/fixtures/h1"
declare -a NAMES=(select_star id_gt_1 madrid ok_true id_null order_id)
declare -a SQLS=(
  "SELECT * FROM sensors ORDER BY ts"
  "SELECT id, note, location, tag FROM sensors WHERE id > 1"
  "SELECT ts, location, tag FROM sensors WHERE location = 'madrid' ORDER BY ts"
  "SELECT id, ok FROM sensors WHERE ok = true ORDER BY ts"
  "SELECT id FROM sensors WHERE id = NULL"
  "SELECT id FROM sensors ORDER BY id"
)
for i in "${!NAMES[@]}"; do
  got="$(mktemp)"
  "$BIN" query --data-dir "$DATA" --sql "${SQLS[$i]}" > "$got"
  if ! diff -u "$DATA/expected/${NAMES[$i]}.tsv" "$got"; then
    echo "error: rust output differs from captured reference for ${NAMES[$i]}" >&2
    rm -f "$got"
    exit 1
  fi
  rm -f "$got"
done
echo "captured-oracle: 6 queries match tests/fixtures/h1/expected"
HTTP="${QDB_HTTP:-http://127.0.0.1:19011/exec}"
if curl -sf -G "$HTTP" --data-urlencode "query=SELECT count() FROM sensors" -o /tmp/qdb-diff-count.json; then
  DIFF_ROOT="$DATA" DIFF_BIN="$BIN" python3 - "$HTTP" << 'PY'
import json, os, subprocess, sys, urllib.parse, urllib.request
http = sys.argv[1]
queries = [
    "SELECT * FROM sensors ORDER BY ts",
    "SELECT id, note, location, tag FROM sensors WHERE id > 1",
    "SELECT ts, location, tag FROM sensors WHERE location = 'madrid' ORDER BY ts",
    "SELECT id, ok FROM sensors WHERE ok = true ORDER BY ts",
    "SELECT id FROM sensors WHERE id = NULL",
    "SELECT id FROM sensors ORDER BY id",
]
root = os.environ.get("DIFF_ROOT")
bin_path = os.environ["DIFF_BIN"]
for sql in queries:
    url = http + "?" + urllib.parse.urlencode({"query": sql})
    with urllib.request.urlopen(url) as response:
        payload = json.load(response)
    rust = subprocess.check_output([bin_path, "query", "--data-dir", root, "--sql", sql], text=True)
    data_rows = [line for line in rust.splitlines()[1:] if line != ""]
    if payload["count"] != len(data_rows):
        raise SystemExit(f"live count mismatch for {sql}: reference {payload['count']} rust {len(data_rows)}")
print(f"live-reference: {len(queries)} queries matched row counts against {http}")
PY
else
  echo "live-reference: NOT RUNNING at $HTTP"
  echo "compared only against captured output from QuestDB 10.1.0; this is not a live differential run"
fi
