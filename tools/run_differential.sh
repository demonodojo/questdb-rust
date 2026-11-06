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
sort_tsv() {
  local src="$1"
  local dest="$2"
  {
    head -n 1 "$src"
    tail -n +2 "$src" | LC_ALL=C sort
  } > "$dest"
}

compare_captured() {
  local data="$1"
  shift
  local count=0
  while [[ $# -ge 2 ]]; do
    local name="$1"
    local sql="$2"
    shift 2
    local got expected
    got="$(mktemp)"
    expected="$(mktemp)"
    "$BIN" query --data-dir "$data" --sql "$sql" > "$got"
    # GROUP BY without ORDER BY has no defined row order in 10.1.0.
    if [[ "$sql" == *"GROUP BY"* && "$sql" != *"ORDER BY"* ]]; then
      sort_tsv "$data/expected/${name}.tsv" "$expected"
      sort_tsv "$got" "$got.sorted"
      mv "$got.sorted" "$got"
    else
      cp "$data/expected/${name}.tsv" "$expected"
    fi
    if ! diff -u "$expected" "$got"; then
      echo "error: rust output differs from captured reference for ${name}" >&2
      rm -f "$got" "$expected"
      exit 1
    fi
    rm -f "$got" "$expected"
    count=$((count + 1))
  done
  echo "captured-oracle: ${count} queries match ${data#"$ROOT_DIR/"}/expected"
}

compare_captured "$ROOT_DIR/tests/fixtures/h1" \
  select_star "SELECT * FROM sensors ORDER BY ts" \
  id_gt_1 "SELECT id, note, location, tag FROM sensors WHERE id > 1" \
  madrid "SELECT ts, location, tag FROM sensors WHERE location = 'madrid' ORDER BY ts" \
  ok_true "SELECT id, ok FROM sensors WHERE ok = true ORDER BY ts" \
  id_null "SELECT id FROM sensors WHERE id = NULL" \
  order_id "SELECT id FROM sensors ORDER BY id" \
  limit_order_id "SELECT id FROM sensors ORDER BY id LIMIT 2" \
  limit_ts "SELECT id FROM sensors ORDER BY ts LIMIT 1" \
  limit_plain "SELECT id FROM sensors LIMIT 2" \
  limit_filtered "SELECT id FROM sensors WHERE id != NULL ORDER BY id LIMIT 1" \
  limit_neg "SELECT id FROM sensors ORDER BY id LIMIT -1" \
  limit_zero "SELECT id FROM sensors ORDER BY id LIMIT 0" \
  limit_range "SELECT id FROM sensors ORDER BY id LIMIT 1, 3" \
  limit_plain "SELECT id FROM sensors ORDER BY id LIMIT -2, 1" \
  limit_order_id "SELECT id FROM sensors ORDER BY id LIMIT ,2" \
  desc_id "SELECT id FROM sensors ORDER BY id DESC" \
  desc_id_limit "SELECT id FROM sensors ORDER BY id DESC LIMIT 2" \
  desc_id_last "SELECT id FROM sensors ORDER BY id DESC LIMIT -1" \
  asc_reading "SELECT reading FROM sensors ORDER BY reading" \
  desc_reading "SELECT reading FROM sensors ORDER BY reading DESC" \
  desc_note "SELECT note FROM sensors ORDER BY note DESC" \
  desc_location "SELECT location FROM sensors ORDER BY location DESC" \
  desc_score "SELECT score FROM sensors ORDER BY score DESC" \
  order_location_id "SELECT id FROM sensors ORDER BY location, id" \
  order_location_id_desc "SELECT id FROM sensors ORDER BY location, id DESC" \
  order_location_desc_id "SELECT id FROM sensors ORDER BY location DESC, id" \
  order_reading_id "SELECT id FROM sensors ORDER BY reading, id" \
  order_reading_desc_id "SELECT id FROM sensors ORDER BY reading DESC, id" \
  order_location "SELECT id FROM sensors ORDER BY location" \
  order_location_id_limit "SELECT id FROM sensors ORDER BY location, id LIMIT 2" \
  order_ts_id_mixed "SELECT id FROM sensors ORDER BY ts, id DESC LIMIT -2, 1" \
  count_all "SELECT count() FROM sensors" \
  count_id "SELECT count(id) FROM sensors" \
  count_note "SELECT count(note) FROM sensors" \
  count_reading "SELECT count(reading) FROM sensors" \
  count_ch "SELECT count(ch) FROM sensors" \
  count_byte "SELECT count(b) FROM sensors" \
  count_one "SELECT count() FROM sensors WHERE id = NULL" \
  count_two "SELECT count() FROM sensors WHERE location = 'madrid'" \
  count_zero "SELECT count() FROM sensors WHERE id > 100" \
  count_limit_zero "SELECT count() FROM sensors LIMIT 0" \
  group_id "SELECT id, count() FROM sensors GROUP BY id" \
  group_id_order "SELECT id, count() FROM sensors GROUP BY id ORDER BY id" \
  group_id_desc "SELECT id, count() FROM sensors GROUP BY id ORDER BY id DESC" \
  group_id_limit "SELECT id, count() FROM sensors GROUP BY id ORDER BY id LIMIT 2" \
  group_id_tail "SELECT id, count() FROM sensors GROUP BY id ORDER BY id LIMIT -2, 1" \
  group_id_limit_zero "SELECT id, count() FROM sensors GROUP BY id ORDER BY id LIMIT 0" \
  group_id_note "SELECT id, count(note) FROM sensors GROUP BY id ORDER BY id" \
  group_id_ch "SELECT id, count(ch) FROM sensors GROUP BY id ORDER BY id" \
  group_id_byte "SELECT id, count(b) FROM sensors GROUP BY id ORDER BY id" \
  group_location "SELECT location, count() FROM sensors GROUP BY location ORDER BY location" \
  group_note "SELECT note, count() FROM sensors GROUP BY note ORDER BY note" \
  group_reading "SELECT reading, count() FROM sensors GROUP BY reading ORDER BY reading" \
  group_ch "SELECT ch, count() FROM sensors GROUP BY ch ORDER BY ch" \
  group_empty "SELECT id, count() FROM sensors WHERE id > 100 GROUP BY id" \
  group_madrid "SELECT location, count() FROM sensors WHERE location = 'madrid' GROUP BY location" \
  group_case "SELECT ID, COUNT() FROM sensors GROUP BY ID ORDER BY ID" \
  group_count_location "SELECT count() FROM sensors GROUP BY location ORDER BY location"

compare_captured "$ROOT_DIR/tests/fixtures/h2" \
  select_star "SELECT * FROM days ORDER BY ts" \
  madrid "SELECT id, note FROM days WHERE location = 'madrid' ORDER BY ts" \
  id_null "SELECT id FROM days WHERE id = NULL" \
  notes "SELECT note FROM days ORDER BY ts" \
  limit_ts "SELECT id FROM days ORDER BY ts LIMIT 2" \
  limit_madrid "SELECT id, note FROM days WHERE location = 'madrid' ORDER BY ts LIMIT 1" \
  limit_notes "SELECT note FROM days ORDER BY ts LIMIT 3" \
  limit_neg "SELECT id FROM days ORDER BY ts LIMIT -2" \
  limit_range "SELECT id FROM days ORDER BY ts LIMIT 1, 3" \
  limit_ts "SELECT id FROM days ORDER BY ts LIMIT -4, -2" \
  limit_madrid "SELECT id, note FROM days WHERE location = 'madrid' ORDER BY ts LIMIT 0, 1" \
  desc_ts "SELECT id FROM days ORDER BY ts DESC" \
  desc_madrid "SELECT id, note FROM days WHERE location = 'madrid' ORDER BY ts DESC" \
  desc_id "SELECT id FROM days ORDER BY id DESC" \
  desc_note "SELECT note FROM days ORDER BY note DESC" \
  order_location_ts_desc "SELECT id FROM days ORDER BY location, ts DESC" \
  order_location_desc_id "SELECT id FROM days ORDER BY location DESC, id" \
  order_madrid_note "SELECT id, note FROM days WHERE location = 'madrid' ORDER BY note DESC, id" \
  count_all "SELECT count() FROM days" \
  count_id "SELECT count(id) FROM days" \
  count_note "SELECT count(note) FROM days" \
  count_madrid "SELECT count(location) FROM days WHERE location = 'madrid'" \
  group_id "SELECT id, count() FROM days GROUP BY id ORDER BY id" \
  group_location "SELECT location, count() FROM days GROUP BY location ORDER BY location" \
  group_madrid "SELECT location, count(id) FROM days WHERE location = 'madrid' GROUP BY location" \
  group_null_id "SELECT id, count() FROM days WHERE id = NULL GROUP BY id"
HTTP="${QDB_HTTP:-http://127.0.0.1:19011/exec}"
if curl -sf -G "$HTTP" --data-urlencode "query=SELECT count() FROM sensors" -o /tmp/qdb-diff-count.json; then
  DIFF_H1="$ROOT_DIR/tests/fixtures/h1" DIFF_H2="$ROOT_DIR/tests/fixtures/h2" DIFF_BIN="$BIN" python3 - "$HTTP" << 'PY'
import json, os, subprocess, sys, urllib.parse, urllib.request
http = sys.argv[1]
queries = [
    (os.environ["DIFF_H1"], "SELECT * FROM sensors ORDER BY ts"),
    (os.environ["DIFF_H1"], "SELECT id, note, location, tag FROM sensors WHERE id > 1"),
    (os.environ["DIFF_H1"], "SELECT ts, location, tag FROM sensors WHERE location = 'madrid' ORDER BY ts"),
    (os.environ["DIFF_H1"], "SELECT id, ok FROM sensors WHERE ok = true ORDER BY ts"),
    (os.environ["DIFF_H1"], "SELECT id FROM sensors WHERE id = NULL"),
    (os.environ["DIFF_H1"], "SELECT id FROM sensors ORDER BY id"),
    (os.environ["DIFF_H1"], "SELECT id FROM sensors ORDER BY id LIMIT 2"),
    (os.environ["DIFF_H1"], "SELECT id FROM sensors ORDER BY ts LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT id FROM sensors LIMIT 2"),
    (os.environ["DIFF_H1"], "SELECT id FROM sensors WHERE id != NULL ORDER BY id LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT id FROM sensors ORDER BY id LIMIT -1"),
    (os.environ["DIFF_H1"], "SELECT id FROM sensors ORDER BY id LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT id FROM sensors ORDER BY id LIMIT 1, 3"),
    (os.environ["DIFF_H1"], "SELECT id FROM sensors ORDER BY id LIMIT -2, 1"),
    (os.environ["DIFF_H1"], "SELECT id FROM sensors ORDER BY id LIMIT ,2"),
    (os.environ["DIFF_H1"], "SELECT id FROM sensors ORDER BY id DESC"),
    (os.environ["DIFF_H1"], "SELECT id FROM sensors ORDER BY id DESC LIMIT 2"),
    (os.environ["DIFF_H1"], "SELECT id FROM sensors ORDER BY id DESC LIMIT -1"),
    (os.environ["DIFF_H1"], "SELECT reading FROM sensors ORDER BY reading"),
    (os.environ["DIFF_H1"], "SELECT reading FROM sensors ORDER BY reading DESC"),
    (os.environ["DIFF_H1"], "SELECT note FROM sensors ORDER BY note DESC"),
    (os.environ["DIFF_H1"], "SELECT location FROM sensors ORDER BY location DESC"),
    (os.environ["DIFF_H1"], "SELECT score FROM sensors ORDER BY score DESC"),
    (os.environ["DIFF_H1"], "SELECT id FROM sensors ORDER BY location, id"),
    (os.environ["DIFF_H1"], "SELECT id FROM sensors ORDER BY location, id DESC"),
    (os.environ["DIFF_H1"], "SELECT id FROM sensors ORDER BY location DESC, id"),
    (os.environ["DIFF_H1"], "SELECT id FROM sensors ORDER BY reading, id"),
    (os.environ["DIFF_H1"], "SELECT id FROM sensors ORDER BY reading DESC, id"),
    (os.environ["DIFF_H1"], "SELECT id FROM sensors ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT id FROM sensors ORDER BY location, id LIMIT 2"),
    (os.environ["DIFF_H1"], "SELECT id FROM sensors ORDER BY ts, id DESC LIMIT -2, 1"),
    (os.environ["DIFF_H1"], "SELECT count() FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT count(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT count(note) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT count(reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT count(ch) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT count(b) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT count() FROM sensors WHERE id = NULL"),
    (os.environ["DIFF_H1"], "SELECT count() FROM sensors WHERE location = 'madrid'"),
    (os.environ["DIFF_H1"], "SELECT count() FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT count() FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT id, count() FROM sensors GROUP BY id"),
    (os.environ["DIFF_H1"], "SELECT id, count() FROM sensors GROUP BY id ORDER BY id"),
    (os.environ["DIFF_H1"], "SELECT id, count() FROM sensors GROUP BY id ORDER BY id DESC"),
    (os.environ["DIFF_H1"], "SELECT id, count() FROM sensors GROUP BY id ORDER BY id LIMIT 2"),
    (os.environ["DIFF_H1"], "SELECT id, count() FROM sensors GROUP BY id ORDER BY id LIMIT -2, 1"),
    (os.environ["DIFF_H1"], "SELECT id, count() FROM sensors GROUP BY id ORDER BY id LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT id, count(note) FROM sensors GROUP BY id ORDER BY id"),
    (os.environ["DIFF_H1"], "SELECT id, count(ch) FROM sensors GROUP BY id ORDER BY id"),
    (os.environ["DIFF_H1"], "SELECT id, count(b) FROM sensors GROUP BY id ORDER BY id"),
    (os.environ["DIFF_H1"], "SELECT location, count() FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT note, count() FROM sensors GROUP BY note ORDER BY note"),
    (os.environ["DIFF_H1"], "SELECT reading, count() FROM sensors GROUP BY reading ORDER BY reading"),
    (os.environ["DIFF_H1"], "SELECT ch, count() FROM sensors GROUP BY ch ORDER BY ch"),
    (os.environ["DIFF_H1"], "SELECT id, count() FROM sensors WHERE id > 100 GROUP BY id"),
    (os.environ["DIFF_H1"], "SELECT location, count() FROM sensors WHERE location = 'madrid' GROUP BY location"),
    (os.environ["DIFF_H1"], "SELECT ID, COUNT() FROM sensors GROUP BY ID ORDER BY ID"),
    (os.environ["DIFF_H1"], "SELECT count() FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H2"], "SELECT * FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT id, note FROM days WHERE location = 'madrid' ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT id FROM days WHERE id = NULL"),
    (os.environ["DIFF_H2"], "SELECT note FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT id FROM days ORDER BY ts LIMIT 2"),
    (os.environ["DIFF_H2"], "SELECT id, note FROM days WHERE location = 'madrid' ORDER BY ts LIMIT 1"),
    (os.environ["DIFF_H2"], "SELECT note FROM days ORDER BY ts LIMIT 3"),
    (os.environ["DIFF_H2"], "SELECT id FROM days ORDER BY ts LIMIT -2"),
    (os.environ["DIFF_H2"], "SELECT id FROM days ORDER BY ts LIMIT 1, 3"),
    (os.environ["DIFF_H2"], "SELECT id FROM days ORDER BY ts LIMIT -4, -2"),
    (os.environ["DIFF_H2"], "SELECT id, note FROM days WHERE location = 'madrid' ORDER BY ts LIMIT 0, 1"),
    (os.environ["DIFF_H2"], "SELECT id FROM days ORDER BY ts DESC"),
    (os.environ["DIFF_H2"], "SELECT id, note FROM days WHERE location = 'madrid' ORDER BY ts DESC"),
    (os.environ["DIFF_H2"], "SELECT id FROM days ORDER BY id DESC"),
    (os.environ["DIFF_H2"], "SELECT note FROM days ORDER BY note DESC"),
    (os.environ["DIFF_H2"], "SELECT id FROM days ORDER BY location, ts DESC"),
    (os.environ["DIFF_H2"], "SELECT id FROM days ORDER BY location DESC, id"),
    (os.environ["DIFF_H2"], "SELECT id, note FROM days WHERE location = 'madrid' ORDER BY note DESC, id"),
    (os.environ["DIFF_H2"], "SELECT count() FROM days"),
    (os.environ["DIFF_H2"], "SELECT count(id) FROM days"),
    (os.environ["DIFF_H2"], "SELECT count(note) FROM days"),
    (os.environ["DIFF_H2"], "SELECT count(location) FROM days WHERE location = 'madrid'"),
    (os.environ["DIFF_H2"], "SELECT id, count() FROM days GROUP BY id ORDER BY id"),
    (os.environ["DIFF_H2"], "SELECT location, count() FROM days GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H2"], "SELECT location, count(id) FROM days WHERE location = 'madrid' GROUP BY location"),
    (os.environ["DIFF_H2"], "SELECT id, count() FROM days WHERE id = NULL GROUP BY id"),
]
bin_path = os.environ["DIFF_BIN"]
for root, sql in queries:
    url = http + "?" + urllib.parse.urlencode({"query": sql})
    with urllib.request.urlopen(url) as response:
        payload = json.load(response)
    rust = subprocess.check_output([bin_path, "query", "--data-dir", root, "--sql", sql], text=True)
    data_rows = rust.splitlines()[1:]
    if payload["count"] != len(data_rows):
        raise SystemExit(f"live count mismatch for {sql}: reference {payload['count']} rust {len(data_rows)}")
print(f"live-reference: {len(queries)} queries matched row counts against {http}")
PY
else
  echo "live-reference: NOT RUNNING at $HTTP"
  echo "compared only against captured output from QuestDB 10.1.0; this is not a live differential run"
fi
