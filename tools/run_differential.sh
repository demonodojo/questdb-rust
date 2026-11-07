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
  group_count_location "SELECT count() FROM sensors GROUP BY location ORDER BY location" \
  min_id "SELECT min(id) FROM sensors" \
  max_id "SELECT max(id) FROM sensors" \
  min_note "SELECT min(note) FROM sensors" \
  max_note "SELECT max(note) FROM sensors" \
  min_reading "SELECT min(reading) FROM sensors" \
  max_reading "SELECT max(reading) FROM sensors" \
  min_null "SELECT min(id) FROM sensors WHERE id > 100" \
  min_ok "SELECT min(ok) FROM sensors" \
  max_ok "SELECT max(ok) FROM sensors" \
  min_ch "SELECT min(ch) FROM sensors" \
  min_ch_null "SELECT min(ch) FROM sensors WHERE ch = ''" \
  min_b "SELECT min(b) FROM sensors" \
  min_code "SELECT min(code) FROM sensors" \
  min_flags "SELECT min(flags) FROM sensors" \
  max_flags "SELECT max(flags) FROM sensors" \
  min_ts "SELECT min(ts) FROM sensors" \
  max_ts "SELECT max(ts) FROM sensors" \
  min_day "SELECT min(day) FROM sensors" \
  max_day "SELECT max(day) FROM sensors" \
  min_tag "SELECT min(tag) FROM sensors" \
  max_tag "SELECT max(tag) FROM sensors" \
  min_location "SELECT min(location) FROM sensors" \
  max_location "SELECT max(location) FROM sensors" \
  min_score "SELECT min(score) FROM sensors" \
  max_score "SELECT max(score) FROM sensors" \
  min_case "SELECT MIN(ID) FROM sensors" \
  min_limit_zero "SELECT min(id) FROM sensors LIMIT 0" \
  group_min_id "SELECT location, min(id) FROM sensors GROUP BY location ORDER BY location" \
  group_min_by_value "SELECT location, min(id) FROM sensors GROUP BY location ORDER BY min(id)" \
  group_max_note "SELECT location, max(note) FROM sensors GROUP BY location ORDER BY location" \
  group_max_reading "SELECT id, max(reading) FROM sensors GROUP BY id ORDER BY id" \
  group_min_reading "SELECT location, min(reading) FROM sensors GROUP BY location ORDER BY location" \
  sum_id "SELECT sum(id) FROM sensors" \
  sum_reading "SELECT sum(reading) FROM sensors" \
  sum_score "SELECT sum(score) FROM sensors" \
  sum_b "SELECT sum(b) FROM sensors" \
  sum_code "SELECT sum(code) FROM sensors" \
  sum_flags "SELECT sum(flags) FROM sensors" \
  sum_b_zero "SELECT sum(b) FROM sensors WHERE b = 0" \
  sum_null "SELECT sum(id) FROM sensors WHERE id > 100" \
  sum_neg_zero "SELECT sum(reading) FROM sensors WHERE id = -1" \
  sum_zeros "SELECT sum(reading) FROM sensors WHERE reading = 0.0" \
  sum_ts "SELECT sum(ts) FROM sensors" \
  sum_day "SELECT sum(day) FROM sensors" \
  sum_location "SELECT sum(location) FROM sensors" \
  sum_case "SELECT SUM(ID) FROM sensors" \
  sum_limit_zero "SELECT sum(id) FROM sensors LIMIT 0" \
  group_sum_id "SELECT location, sum(id) FROM sensors GROUP BY location ORDER BY location" \
  group_sum_by_value "SELECT sum(id) FROM sensors GROUP BY location ORDER BY sum(id)" \
  group_sum_reading "SELECT id, sum(reading) FROM sensors GROUP BY id ORDER BY id" \
  group_sum_score "SELECT location, sum(score) FROM sensors GROUP BY location ORDER BY location" \
  avg_id "SELECT avg(id) FROM sensors" \
  avg_reading "SELECT avg(reading) FROM sensors" \
  avg_score "SELECT avg(score) FROM sensors" \
  avg_b "SELECT avg(b) FROM sensors" \
  avg_code "SELECT avg(code) FROM sensors" \
  avg_flags "SELECT avg(flags) FROM sensors" \
  avg_ok "SELECT avg(ok) FROM sensors" \
  avg_null "SELECT avg(id) FROM sensors WHERE id > 100" \
  avg_neg_zero "SELECT avg(reading) FROM sensors WHERE id = -1" \
  avg_zeros "SELECT avg(reading) FROM sensors WHERE reading = 0.0" \
  avg_b_zero "SELECT avg(b) FROM sensors WHERE b = 0" \
  avg_ts "SELECT avg(ts) FROM sensors" \
  avg_day "SELECT avg(day) FROM sensors" \
  avg_location "SELECT avg(location) FROM sensors" \
  avg_case "SELECT AVG(ID) FROM sensors" \
  avg_limit_zero "SELECT avg(id) FROM sensors LIMIT 0" \
  group_avg_id "SELECT location, avg(id) FROM sensors GROUP BY location ORDER BY location" \
  group_avg_reading "SELECT id, avg(reading) FROM sensors GROUP BY id ORDER BY id" \
  group_avg_by_value "SELECT location, avg(id) FROM sensors GROUP BY location ORDER BY avg(id)" \
  first_id "SELECT first(id) FROM sensors" \
  last_id "SELECT last(id) FROM sensors" \
  first_reading "SELECT first(reading) FROM sensors" \
  last_reading "SELECT last(reading) FROM sensors" \
  first_ok "SELECT first(ok) FROM sensors" \
  last_ok "SELECT last(ok) FROM sensors" \
  first_note "SELECT first(note) FROM sensors" \
  last_note "SELECT last(note) FROM sensors" \
  first_ch "SELECT first(ch) FROM sensors" \
  last_tag "SELECT last(tag) FROM sensors" \
  first_ts "SELECT first(ts) FROM sensors" \
  last_day "SELECT last(day) FROM sensors" \
  first_location "SELECT first(location) FROM sensors" \
  last_location "SELECT last(location) FROM sensors" \
  first_null "SELECT first(id) FROM sensors WHERE id > 100" \
  first_reading_null "SELECT first(reading) FROM sensors WHERE id = 2" \
  last_empty_note "SELECT last(note) FROM sensors WHERE note = ''" \
  first_empty_ch "SELECT first(ch) FROM sensors WHERE ch = ''" \
  first_case "SELECT FIRST(ID) FROM sensors" \
  first_limit_zero "SELECT first(id) FROM sensors LIMIT 0" \
  last_score "SELECT last(score) FROM sensors" \
  group_first_id "SELECT location, first(id) FROM sensors GROUP BY location ORDER BY location" \
  group_last_id "SELECT location, last(id) FROM sensors GROUP BY location ORDER BY location" \
  group_first_reading "SELECT id, first(reading) FROM sensors GROUP BY id ORDER BY id" \
  group_first_by_value "SELECT location, first(id) FROM sensors GROUP BY location ORDER BY first(id)" \
  cd_id "SELECT count_distinct(id) FROM sensors" \
  cd_reading "SELECT count_distinct(reading) FROM sensors" \
  cd_score "SELECT count_distinct(score) FROM sensors" \
  cd_ok "SELECT count_distinct(ok) FROM sensors" \
  cd_ch "SELECT count_distinct(ch) FROM sensors" \
  cd_note "SELECT count_distinct(note) FROM sensors" \
  cd_location "SELECT count_distinct(location) FROM sensors" \
  cd_b "SELECT count_distinct(b) FROM sensors" \
  cd_flags "SELECT count_distinct(flags) FROM sensors" \
  cd_day "SELECT count_distinct(day) FROM sensors" \
  cd_ts "SELECT count_distinct(ts) FROM sensors" \
  cd_null "SELECT count_distinct(id) FROM sensors WHERE id > 100" \
  cd_null_id "SELECT count_distinct(id) FROM sensors WHERE id = NULL" \
  cd_empty_note "SELECT count_distinct(note) FROM sensors WHERE note = ''" \
  cd_empty_ch "SELECT count_distinct(ch) FROM sensors WHERE ch = ''" \
  cd_zeros "SELECT count_distinct(reading) FROM sensors WHERE reading = 0.0" \
  cd_case "SELECT COUNT_DISTINCT(ID) FROM sensors" \
  cd_limit_zero "SELECT count_distinct(id) FROM sensors LIMIT 0" \
  group_cd_id "SELECT location, count_distinct(id) FROM sensors GROUP BY location ORDER BY location" \
  group_cd_note "SELECT location, count_distinct(note) FROM sensors GROUP BY location ORDER BY location" \
  group_cd_ch "SELECT location, count_distinct(ch) FROM sensors GROUP BY location ORDER BY location" \
  group_cd_flags "SELECT location, count_distinct(flags) FROM sensors GROUP BY location ORDER BY location" \
  group_cd_by_id "SELECT id, count_distinct(note) FROM sensors GROUP BY id ORDER BY id" \
  group_cd_by_value "SELECT location, count_distinct(note) FROM sensors GROUP BY location ORDER BY count_distinct(note)" \
  concat_note_location "SELECT concat(note, location) FROM sensors" \
  concat_bang "SELECT concat(note, '!') FROM sensors" \
  concat_prefix "SELECT concat('pre-', location) FROM sensors" \
  concat_null_lit "SELECT concat(note, null) FROM sensors" \
  concat_ch "SELECT concat(ch, note) FROM sensors" \
  concat_id "SELECT concat(id, note) FROM sensors" \
  concat_reading "SELECT concat(reading, '') FROM sensors" \
  concat_day "SELECT concat(day, '') FROM sensors" \
  concat_ts "SELECT concat(ts, '') FROM sensors" \
  concat_ok "SELECT concat(ok, '') FROM sensors" \
  concat_flags "SELECT concat(flags, '') FROM sensors" \
  concat_case "SELECT CONCAT(NOTE, LOCATION) FROM sensors" \
  concat_space "SELECT concat( note , location ) FROM sensors" \
  concat_quote "SELECT concat('it''s', '!') FROM sensors LIMIT 1" \
  concat_one "SELECT concat(note) FROM sensors" \
  concat_three "SELECT concat(note, location, tag) FROM sensors" \
  concat_order "SELECT concat(note, location) FROM sensors ORDER BY concat(note, location)" \
  concat_order_note "SELECT concat(note, location) FROM sensors ORDER BY note" \
  concat_with_id "SELECT id, concat(note, location) FROM sensors ORDER BY concat(note, location)" \
  concat_limit_zero "SELECT concat(note, location) FROM sensors LIMIT 0" \
  concat_where_empty "SELECT concat(note, location) FROM sensors WHERE note = ''" \
  length_note "SELECT length(note) FROM sensors" \
  length_location "SELECT length(location) FROM sensors" \
  length_tag "SELECT length(tag) FROM sensors" \
  length_ch "SELECT length(ch) FROM sensors" \
  length_empty "SELECT length('') FROM sensors LIMIT 1" \
  length_null "SELECT length(null) FROM sensors LIMIT 1" \
  length_beta "SELECT length('βeta') FROM sensors LIMIT 1" \
  length_tokyo "SELECT length('東京') FROM sensors LIMIT 1" \
  length_case "SELECT LENGTH(NOTE) FROM sensors" \
  length_where_empty "SELECT length(note) FROM sensors WHERE note = ''" \
  length_order "SELECT length(note) FROM sensors ORDER BY length(note)" \
  length_with_id "SELECT id, length(note) FROM sensors ORDER BY length(note)" \
  length_limit_zero "SELECT length(note) FROM sensors LIMIT 0" \
  lower_note "SELECT lower(note) FROM sensors" \
  lower_location "SELECT lower(location) FROM sensors" \
  lower_tag "SELECT lower(tag) FROM sensors" \
  lower_ch "SELECT lower(ch) FROM sensors" \
  lower_beta "SELECT lower('BETA') FROM sensors LIMIT 1" \
  lower_hello "SELECT lower('Héllo') FROM sensors LIMIT 1" \
  lower_tokyo "SELECT lower('東京') FROM sensors LIMIT 1" \
  lower_empty "SELECT lower('') FROM sensors LIMIT 1" \
  lower_null "SELECT lower(null) FROM sensors LIMIT 1" \
  lower_case "SELECT LOWER(NOTE) FROM sensors" \
  lower_order "SELECT lower(note) FROM sensors ORDER BY lower(note)" \
  lower_with_id "SELECT id, lower(note) FROM sensors ORDER BY lower(note)" \
  lower_where_empty "SELECT lower(note) FROM sensors WHERE note = ''" \
  lower_limit_zero "SELECT lower(note) FROM sensors LIMIT 0" \
  upper_note "SELECT upper(note) FROM sensors" \
  upper_location "SELECT upper(location) FROM sensors" \
  upper_tag "SELECT upper(tag) FROM sensors" \
  upper_ch "SELECT upper(ch) FROM sensors" \
  upper_beta "SELECT upper('βeta') FROM sensors LIMIT 1" \
  upper_empty "SELECT upper('') FROM sensors LIMIT 1" \
  upper_null "SELECT upper(null) FROM sensors LIMIT 1" \
  upper_case "SELECT UPPER(NOTE) FROM sensors" \
  upper_order "SELECT upper(note) FROM sensors ORDER BY upper(note)" \
  upper_with_id "SELECT id, upper(note) FROM sensors ORDER BY upper(note)" \
  upper_where_empty "SELECT upper(note) FROM sensors WHERE note = ''" \
  upper_limit_zero "SELECT upper(note) FROM sensors LIMIT 0" \
  left_note "SELECT left(note, 2) FROM sensors" \
  right_note "SELECT right(note, 2) FROM sensors" \
  left_location "SELECT left(location, 3) FROM sensors" \
  right_tag "SELECT right(tag, 4) FROM sensors" \
  left_ch "SELECT left(ch, 1) FROM sensors" \
  left_zero "SELECT left(note, 0) FROM sensors" \
  left_long "SELECT left(note, 10) FROM sensors" \
  left_neg "SELECT left(note, -1) FROM sensors" \
  right_neg "SELECT right(note, -2) FROM sensors" \
  left_beta "SELECT left('βeta', 1) FROM sensors LIMIT 1" \
  right_tokyo "SELECT right('東京', 1) FROM sensors LIMIT 1" \
  left_null "SELECT left(null, 2) FROM sensors LIMIT 1" \
  left_empty "SELECT left('', 2) FROM sensors LIMIT 1" \
  left_case "SELECT LEFT(NOTE, 1) FROM sensors" \
  left_order "SELECT left(note, 2) FROM sensors ORDER BY left(note, 2)" \
  left_with_id "SELECT id, left(note, 1) FROM sensors ORDER BY left(note, 1)" \
  right_where_empty "SELECT right(note, 2) FROM sensors WHERE note = ''" \
  left_limit_zero "SELECT left(note, 2) FROM sensors LIMIT 0" \
  left_symbol "SELECT left(note, location) FROM sensors" \
  left_byte "SELECT left(note, b) FROM sensors" \
  sub_note "SELECT substring(note, 1, 2) FROM sensors" \
  sub_zero_start "SELECT substring(note, 0, 2) FROM sensors" \
  sub_long "SELECT substring(note, 1, 10) FROM sensors" \
  sub_neg "SELECT substring(note, -1, 4) FROM sensors" \
  sub_before "SELECT substring(note, -2, 2) FROM sensors" \
  sub_zero_len "SELECT substring(note, 0, 0) FROM sensors" \
  sub_past "SELECT substring(note, 6, 2) FROM sensors" \
  sub_beta "SELECT substring('βeta', 1, 2) FROM sensors LIMIT 1" \
  sub_null "SELECT substring(null, 1, 2) FROM sensors LIMIT 1" \
  sub_empty "SELECT substring('', 1, 2) FROM sensors LIMIT 1" \
  sub_case "SELECT SUBSTRING(NOTE, 1, 1) FROM sensors" \
  sub_ch "SELECT substring(ch, 1, 1) FROM sensors" \
  sub_tag "SELECT substring(tag, 2, 3) FROM sensors" \
  sub_location "SELECT substring(location, 1, 3) FROM sensors" \
  sub_order "SELECT substring(note, 2, 4) FROM sensors ORDER BY substring(note, 2, 4)" \
  sub_with_id "SELECT id, substring(note, 1, 1) FROM sensors ORDER BY substring(note, 1, 1)" \
  sub_where_empty "SELECT substring(note, 1, 2) FROM sensors WHERE note = ''" \
  sub_limit_zero "SELECT substring(note, 1, 2) FROM sensors LIMIT 0" \
  sub_symbol "SELECT substring(note, location, 2) FROM sensors" \
  sub_flags "SELECT substring(note, flags, 2) FROM sensors" \
  sub_null_start "SELECT substring(note, null, 2) FROM sensors" \
  sub_code "SELECT substring(note, code, 1) FROM sensors" \
  trim_note "SELECT trim(note) FROM sensors" \
  trim_location "SELECT trim(location) FROM sensors" \
  trim_tag "SELECT trim(tag) FROM sensors" \
  trim_ch "SELECT trim(ch) FROM sensors" \
  trim_spaces "SELECT trim('  BETA  ') FROM sensors LIMIT 1" \
  trim_inner "SELECT trim(' a b ') FROM sensors LIMIT 1" \
  trim_beta "SELECT trim('βeta') FROM sensors LIMIT 1" \
  trim_empty "SELECT trim('') FROM sensors LIMIT 1" \
  trim_null "SELECT trim(null) FROM sensors LIMIT 1" \
  trim_blank "SELECT trim('  ') FROM sensors LIMIT 1" \
  trim_tab $'SELECT trim(\'\t ab \t\') FROM sensors LIMIT 1' \
  trim_case "SELECT TRIM(NOTE) FROM sensors" \
  trim_order "SELECT trim(note) FROM sensors ORDER BY trim(note)" \
  trim_with_id "SELECT id, trim(note) FROM sensors ORDER BY trim(note)" \
  trim_where_empty "SELECT trim(note) FROM sensors WHERE note = ''" \
  trim_limit_zero "SELECT trim(note) FROM sensors LIMIT 0" \
  ltrim_spaces "SELECT ltrim('  a b  ') FROM sensors LIMIT 1" \
  rtrim_spaces "SELECT rtrim('  a b  ') FROM sensors LIMIT 1" \
  ltrim_blank "SELECT ltrim('  ') FROM sensors LIMIT 1" \
  rtrim_null "SELECT rtrim(null) FROM sensors LIMIT 1" \
  rtrim_tab $'SELECT rtrim(\'\t ab \') FROM sensors LIMIT 1' \
  ltrim_note "SELECT ltrim(note) FROM sensors" \
  rtrim_note "SELECT rtrim(note) FROM sensors" \
  ltrim_location "SELECT ltrim(location) FROM sensors" \
  rtrim_tag "SELECT rtrim(tag) FROM sensors" \
  ltrim_ch "SELECT ltrim(ch) FROM sensors" \
  ltrim_case "SELECT LTRIM(NOTE) FROM sensors" \
  ltrim_order "SELECT ltrim(note) FROM sensors ORDER BY ltrim(note)" \
  rtrim_with_id "SELECT id, rtrim(note) FROM sensors ORDER BY rtrim(note)" \
  ltrim_where_empty "SELECT ltrim(note) FROM sensors WHERE note = ''" \
  rtrim_limit_zero "SELECT rtrim(note) FROM sensors LIMIT 0" \
  rep_note "SELECT replace(note, 'a', 'X') FROM sensors" \
  rep_drop "SELECT replace(note, 'ta', '') FROM sensors" \
  rep_beta "SELECT replace('βeta', 'β', 'B') FROM sensors LIMIT 1" \
  rep_null "SELECT replace(null, 'a', 'b') FROM sensors LIMIT 1" \
  rep_null_old "SELECT replace('ab', null, 'c') FROM sensors LIMIT 1" \
  rep_null_new "SELECT replace('ab', 'a', null) FROM sensors LIMIT 1" \
  rep_empty "SELECT replace('', 'a', 'b') FROM sensors LIMIT 1" \
  rep_grow "SELECT replace('aaa', 'a', 'bb') FROM sensors LIMIT 1" \
  rep_overlap "SELECT replace('aaa', 'aa', 'b') FROM sensors LIMIT 1" \
  rep_empty_old "SELECT replace('abc', '', 'x') FROM sensors LIMIT 1" \
  rep_case "SELECT REPLACE(NOTE, 'a', 'A') FROM sensors" \
  rep_order "SELECT replace(note, 'a', 'X') FROM sensors ORDER BY replace(note, 'a', 'X')" \
  rep_with_id "SELECT id, replace(note, 'a', 'X') FROM sensors ORDER BY replace(note, 'a', 'X')" \
  rep_where_empty "SELECT replace(note, 'a', 'X') FROM sensors WHERE note = ''" \
  rep_limit_zero "SELECT replace(note, 'a', 'X') FROM sensors LIMIT 0" \
  rep_symbol "SELECT replace(note, 'a', location) FROM sensors" \
  rep_const_symbol "SELECT replace('ab', location, 'x') FROM sensors" \
  rep_empty_value "SELECT replace('', location, 'x') FROM sensors" \
  rep_tag "SELECT replace(tag, '0', 'x') FROM sensors" \
  sw_note "SELECT starts_with(note, 'a') FROM sensors" \
  sw_beta "SELECT starts_with(note, 'β') FROM sensors" \
  sw_empty "SELECT starts_with(note, '') FROM sensors" \
  sw_null_prefix "SELECT starts_with(note, null) FROM sensors" \
  sw_null "SELECT starts_with(null, 'a') FROM sensors LIMIT 1" \
  sw_empty_text "SELECT starts_with('', 'a') FROM sensors LIMIT 1" \
  sw_prefix "SELECT starts_with('alpha', 'alp') FROM sensors LIMIT 1" \
  sw_long "SELECT starts_with('alpha', 'alphaextra') FROM sensors LIMIT 1" \
  sw_case_fold "SELECT starts_with('Alpha', 'a') FROM sensors LIMIT 1" \
  sw_case "SELECT STARTS_WITH(NOTE, 'a') FROM sensors" \
  sw_order "SELECT starts_with(note, 'a') FROM sensors ORDER BY starts_with(note, 'a')" \
  sw_with_id "SELECT id, starts_with(note, 'a') FROM sensors ORDER BY starts_with(note, 'a')" \
  sw_where "SELECT starts_with(note, '') FROM sensors WHERE note = ''" \
  sw_limit_zero "SELECT starts_with(note, 'a') FROM sensors LIMIT 0" \
  sw_ch "SELECT starts_with(ch, 'A') FROM sensors" \
  sw_tag "SELECT starts_with(tag, '0') FROM sensors" \
  sw_location "SELECT starts_with(location, '東') FROM sensors" \
  sp_note "SELECT strpos(note, 'a') FROM sensors" \
  sp_ph "SELECT strpos(note, 'ph') FROM sensors" \
  sp_miss "SELECT strpos(note, 'z') FROM sensors" \
  sp_empty "SELECT strpos(note, '') FROM sensors" \
  sp_null_needle "SELECT strpos(note, null) FROM sensors" \
  sp_null "SELECT strpos(null, 'a') FROM sensors LIMIT 1" \
  sp_empty_text "SELECT strpos('', 'a') FROM sensors LIMIT 1" \
  sp_beta "SELECT strpos('βeta', 'β') FROM sensors LIMIT 1" \
  sp_overlap "SELECT strpos('aaa', 'aa') FROM sensors LIMIT 1" \
  sp_case "SELECT STRPOS(NOTE, 'a') FROM sensors" \
  sp_order "SELECT strpos(note, 'a') FROM sensors ORDER BY strpos(note, 'a')" \
  sp_with_id "SELECT id, strpos(note, 'a') FROM sensors ORDER BY strpos(note, 'a')" \
  sp_where "SELECT strpos(note, 'a') FROM sensors WHERE note = ''" \
  sp_limit_zero "SELECT strpos(note, 'a') FROM sensors LIMIT 0" \
  sp_symbol "SELECT strpos(note, location) FROM sensors" \
  sp_tag "SELECT strpos(tag, '0') FROM sensors" \
  sp_ch "SELECT strpos(ch, 'A') FROM sensors" \
  sp_position "SELECT position(note, 'a') FROM sensors" \
  part_note "SELECT split_part(note, 'a', 1) FROM sensors" \
  part_second "SELECT split_part(note, 'a', 2) FROM sensors" \
  part_third "SELECT split_part(note, 'a', 3) FROM sensors" \
  part_dash "SELECT split_part('a-b-c', '-', 2) FROM sensors LIMIT 1" \
  part_last "SELECT split_part('a-b-c', '-', -1) FROM sensors LIMIT 1" \
  part_past "SELECT split_part('a-b-c', '-', 4) FROM sensors LIMIT 1" \
  part_double "SELECT split_part('a--b', '-', 2) FROM sensors LIMIT 1" \
  part_empty_delim "SELECT split_part(note, '', 1) FROM sensors" \
  part_empty_neg "SELECT split_part('abc', '', -1) FROM sensors LIMIT 1" \
  part_null "SELECT split_part(null, 'a', 1) FROM sensors LIMIT 1" \
  part_null_delim "SELECT split_part('ab', null, 1) FROM sensors LIMIT 1" \
  part_null_index "SELECT split_part(note, 'a', null) FROM sensors" \
  part_case "SELECT SPLIT_PART(NOTE, 'a', 1) FROM sensors" \
  part_order "SELECT split_part(note, 'a', 1) FROM sensors ORDER BY split_part(note, 'a', 1)" \
  part_with_id "SELECT id, split_part(note, 'a', 1) FROM sensors ORDER BY split_part(note, 'a', 1)" \
  part_where "SELECT split_part(note, 'a', 1) FROM sensors WHERE note = ''" \
  part_limit_zero "SELECT split_part(note, 'a', 1) FROM sensors LIMIT 0" \
  tl_note "SELECT to_lowercase(note) FROM sensors" \
  tu_note "SELECT to_uppercase(note) FROM sensors" \
  tu_beta "SELECT to_uppercase('β') FROM sensors LIMIT 1" \
  tl_case "SELECT TO_LOWERCASE(NOTE) FROM sensors" \
  tl_order "SELECT to_lowercase(note) FROM sensors ORDER BY to_lowercase(note)" \
  tu_order "SELECT to_uppercase(note) FROM sensors ORDER BY to_uppercase(note)" \
  tl_with_id "SELECT id, to_lowercase(note) FROM sensors ORDER BY to_lowercase(note)" \
  tl_null "SELECT to_lowercase(null) FROM sensors LIMIT 1" \
  tl_where "SELECT to_lowercase(note) FROM sensors WHERE note = ''" \
  tl_limit_zero "SELECT to_lowercase(note) FROM sensors LIMIT 0" \
  tu_ch "SELECT to_uppercase(ch) FROM sensors" \
  tu_tag "SELECT to_uppercase(tag) FROM sensors" \
  lp_note "SELECT lpad(note, 6, 'x') FROM sensors" \
  rp_note "SELECT rpad(note, 6, 'x') FROM sensors" \
  lp_fill "SELECT lpad('ab', 5, 'xy') FROM sensors LIMIT 1" \
  lp_cut "SELECT lpad('abc', 2, 'x') FROM sensors LIMIT 1" \
  rp_cut "SELECT rpad('abc', 2, 'x') FROM sensors LIMIT 1" \
  lp_zero "SELECT lpad('ab', 0, 'x') FROM sensors LIMIT 1" \
  lp_neg "SELECT lpad('ab', -1, 'x') FROM sensors LIMIT 1" \
  lp_null "SELECT lpad(null, 3, 'x') FROM sensors LIMIT 1" \
  lp_null_len "SELECT lpad('ab', null, 'x') FROM sensors LIMIT 1" \
  lp_null_fill "SELECT lpad('ab', 3, null) FROM sensors LIMIT 1" \
  lp_empty_fill "SELECT lpad('ab', 3, '') FROM sensors LIMIT 1" \
  lp_spaces "SELECT lpad(note, 3) FROM sensors" \
  rp_spaces "SELECT rpad(note, 3) FROM sensors" \
  lp_flags "SELECT lpad(note, flags, 'x') FROM sensors" \
  lp_byte "SELECT lpad(note, b) FROM sensors" \
  lp_ch "SELECT lpad(ch, 3, 'x') FROM sensors" \
  lp_symbol "SELECT lpad(note, 6, location) FROM sensors" \
  lp_case "SELECT LPAD(NOTE, 6, 'x') FROM sensors" \
  rp_order "SELECT rpad(note, 6, 'x') FROM sensors ORDER BY rpad(note, 6, 'x')" \
  lp_limit_zero "SELECT lpad(note, 6, 'x') FROM sensors LIMIT 0" \
  qi_note "SELECT quote_ident(note) FROM sensors" \
  qi_quote "SELECT quote_ident('a\"b') FROM sensors LIMIT 1" \
  qi_space "SELECT quote_ident('a b') FROM sensors LIMIT 1" \
  qi_dash "SELECT quote_ident('a-b') FROM sensors LIMIT 1" \
  qi_case_text "SELECT quote_ident('Alpha') FROM sensors LIMIT 1" \
  qi_dollar "SELECT quote_ident('a\$b') FROM sensors LIMIT 1" \
  qi_digit "SELECT quote_ident('1a') FROM sensors LIMIT 1" \
  qi_null "SELECT quote_ident(null) FROM sensors LIMIT 1" \
  qi_empty "SELECT quote_ident('') FROM sensors LIMIT 1" \
  qi_ch "SELECT quote_ident(ch) FROM sensors" \
  qi_tag "SELECT quote_ident(tag) FROM sensors" \
  qi_case "SELECT QUOTE_IDENT(NOTE) FROM sensors" \
  qi_order "SELECT quote_ident(note) FROM sensors ORDER BY quote_ident(note)" \
  qi_with_id "SELECT id, quote_ident(note) FROM sensors ORDER BY quote_ident(note)" \
  qi_where "SELECT quote_ident(note) FROM sensors WHERE note = ''" \
  qi_limit_zero "SELECT quote_ident(note) FROM sensors LIMIT 0" \
  lb_note "SELECT length_bytes(note) FROM sensors" \
  lb_tag "SELECT length_bytes(tag) FROM sensors" \
  lb_beta "SELECT length_bytes('β') FROM sensors LIMIT 1" \
  lb_tokyo "SELECT length_bytes('東京') FROM sensors LIMIT 1" \
  lb_null "SELECT length_bytes(null) FROM sensors LIMIT 1" \
  lb_empty "SELECT length_bytes('') FROM sensors LIMIT 1" \
  lb_ch "SELECT length_bytes(ch) FROM sensors" \
  lb_case "SELECT LENGTH_BYTES(NOTE) FROM sensors" \
  lb_order "SELECT length_bytes(note) FROM sensors ORDER BY length_bytes(note)" \
  lb_with_id "SELECT id, length_bytes(note) FROM sensors ORDER BY length_bytes(note)" \
  lb_where "SELECT length_bytes(note) FROM sensors WHERE note = ''" \
  lb_limit_zero "SELECT length_bytes(note) FROM sensors LIMIT 0" \
  sz_id "SELECT size_pretty(id) FROM sensors" \
  sz_zero "SELECT size_pretty(0) FROM sensors LIMIT 1" \
  sz_kib "SELECT size_pretty(1024) FROM sensors LIMIT 1" \
  sz_round "SELECT size_pretty(1500) FROM sensors LIMIT 1" \
  sz_mib "SELECT size_pretty(1048576) FROM sensors LIMIT 1" \
  sz_byte "SELECT size_pretty(1023) FROM sensors LIMIT 1" \
  sz_neg "SELECT size_pretty(-1024) FROM sensors LIMIT 1" \
  sz_max "SELECT size_pretty(9223372036854775807) FROM sensors LIMIT 1" \
  sz_null "SELECT size_pretty(null) FROM sensors LIMIT 1" \
  sz_flags "SELECT size_pretty(flags) FROM sensors" \
  sz_byte_col "SELECT size_pretty(b) FROM sensors" \
  sz_code "SELECT size_pretty(code) FROM sensors" \
  sz_case "SELECT SIZE_PRETTY(ID) FROM sensors" \
  sz_order "SELECT size_pretty(id) FROM sensors ORDER BY size_pretty(id)" \
  sz_with_id "SELECT id, size_pretty(id) FROM sensors ORDER BY size_pretty(id)" \
  sz_where "SELECT size_pretty(id) FROM sensors WHERE id = 1" \
  sz_limit_zero "SELECT size_pretty(id) FROM sensors LIMIT 0" \
  abs_id "SELECT abs(id) FROM sensors" \
  abs_reading "SELECT abs(reading) FROM sensors" \
  abs_score "SELECT abs(score) FROM sensors" \
  abs_byte "SELECT abs(b) FROM sensors" \
  abs_code "SELECT abs(code) FROM sensors" \
  abs_flags "SELECT abs(flags) FROM sensors" \
  abs_neg "SELECT abs(-1.5) FROM sensors LIMIT 1" \
  abs_int "SELECT abs(-1) FROM sensors LIMIT 1" \
  abs_null "SELECT abs(null) FROM sensors LIMIT 1" \
  abs_case "SELECT ABS(ID) FROM sensors" \
  abs_order "SELECT abs(id) FROM sensors ORDER BY abs(id)" \
  abs_with_id "SELECT id, abs(id) FROM sensors ORDER BY abs(id)" \
  abs_reading_order "SELECT abs(reading) FROM sensors ORDER BY abs(reading)" \
  abs_where "SELECT abs(id) FROM sensors WHERE id = -1" \
  abs_limit_zero "SELECT abs(id) FROM sensors LIMIT 0" \
  rd_reading "SELECT round(reading) FROM sensors" \
  rd_score "SELECT round(score) FROM sensors" \
  rd_id "SELECT round(id) FROM sensors" \
  rd_flags "SELECT round(flags) FROM sensors" \
  rd_byte "SELECT round(b) FROM sensors" \
  rd_half "SELECT round(1.5) FROM sensors LIMIT 1" \
  rd_half2 "SELECT round(2.5) FROM sensors LIMIT 1" \
  rd_neg "SELECT round(-1.5) FROM sensors LIMIT 1" \
  rd_scale "SELECT round(reading, 1) FROM sensors" \
  rd_quarter "SELECT round(1.25, 1) FROM sensors LIMIT 1" \
  rd_neg_scale "SELECT round(-1.25, 1) FROM sensors LIMIT 1" \
  rd_tens "SELECT round(15, -1) FROM sensors LIMIT 1" \
  rd_null "SELECT round(null) FROM sensors LIMIT 1" \
  rd_null_scale "SELECT round(reading, null) FROM sensors" \
  rd_far "SELECT round(1.5, 20) FROM sensors LIMIT 1" \
  rd_case "SELECT ROUND(READING) FROM sensors" \
  rd_order "SELECT round(id) FROM sensors ORDER BY round(id)" \
  rd_with_id "SELECT id, round(id) FROM sensors ORDER BY round(id)" \
  rd_where "SELECT round(id) FROM sensors WHERE id = -1" \
  rd_limit_zero "SELECT round(id) FROM sensors LIMIT 0" \
  fl_reading "SELECT floor(reading) FROM sensors" \
  fl_score "SELECT floor(score) FROM sensors" \
  fl_id "SELECT floor(id) FROM sensors" \
  fl_flags "SELECT floor(flags) FROM sensors" \
  fl_byte "SELECT floor(b) FROM sensors" \
  fl_code "SELECT floor(code) FROM sensors" \
  fl_half "SELECT floor(1.5) FROM sensors LIMIT 1" \
  fl_neg "SELECT floor(-1.5) FROM sensors LIMIT 1" \
  fl_one "SELECT floor(1) FROM sensors LIMIT 1" \
  fl_long "SELECT floor(3000000000) FROM sensors LIMIT 1" \
  fl_null "SELECT floor(null) FROM sensors LIMIT 1" \
  fl_case "SELECT FLOOR(READING) FROM sensors" \
  fl_order "SELECT floor(id) FROM sensors ORDER BY floor(id)" \
  fl_score_order "SELECT floor(score) FROM sensors ORDER BY floor(score)" \
  fl_reading_order "SELECT floor(reading) FROM sensors ORDER BY floor(reading)" \
  fl_with_id "SELECT id, floor(id) FROM sensors ORDER BY floor(id)" \
  fl_where "SELECT floor(id) FROM sensors WHERE id = -1" \
  fl_limit_zero "SELECT floor(id) FROM sensors LIMIT 0" \
  ce_reading "SELECT ceil(reading) FROM sensors" \
  ce_score "SELECT ceil(score) FROM sensors" \
  ce_id "SELECT ceil(id) FROM sensors" \
  ce_flags "SELECT ceil(flags) FROM sensors" \
  ce_byte "SELECT ceil(b) FROM sensors" \
  ce_code "SELECT ceil(code) FROM sensors" \
  ce_half "SELECT ceil(1.5) FROM sensors LIMIT 1" \
  ce_neg "SELECT ceil(-1.5) FROM sensors LIMIT 1" \
  ce_one "SELECT ceil(1) FROM sensors LIMIT 1" \
  ce_long "SELECT ceil(3000000000) FROM sensors LIMIT 1" \
  ce_null "SELECT ceil(null) FROM sensors LIMIT 1" \
  ce_case "SELECT CEIL(READING) FROM sensors" \
  ce_alias "SELECT ceiling(reading) FROM sensors" \
  ce_order "SELECT ceil(id) FROM sensors ORDER BY ceil(id)" \
  ce_score_order "SELECT ceil(score) FROM sensors ORDER BY ceil(score)" \
  ce_reading_order "SELECT ceil(reading) FROM sensors ORDER BY ceil(reading)" \
  ce_with_id "SELECT id, ceil(id) FROM sensors ORDER BY ceil(id)" \
  ce_where "SELECT ceil(id) FROM sensors WHERE id = -1" \
  ce_limit_zero "SELECT ceil(id) FROM sensors LIMIT 0" \
  sg_reading "SELECT sign(reading) FROM sensors" \
  sg_score "SELECT sign(score) FROM sensors" \
  sg_id "SELECT sign(id) FROM sensors" \
  sg_flags "SELECT sign(flags) FROM sensors" \
  sg_byte "SELECT sign(b) FROM sensors" \
  sg_code "SELECT sign(code) FROM sensors" \
  sg_neg "SELECT sign(-1.5) FROM sensors LIMIT 1" \
  sg_negzero "SELECT sign(-0.0) FROM sensors LIMIT 1" \
  sg_zero "SELECT sign(0) FROM sensors LIMIT 1" \
  sg_one "SELECT sign(1) FROM sensors LIMIT 1" \
  sg_long "SELECT sign(3000000000) FROM sensors LIMIT 1" \
  sg_null "SELECT sign(null) FROM sensors LIMIT 1" \
  sg_case "SELECT SIGN(READING) FROM sensors" \
  sg_order "SELECT sign(id) FROM sensors ORDER BY sign(id)" \
  sg_score_order "SELECT sign(score) FROM sensors ORDER BY sign(score)" \
  sg_reading_order "SELECT sign(reading) FROM sensors ORDER BY sign(reading)" \
  sg_with_id "SELECT id, sign(id) FROM sensors ORDER BY sign(id)" \
  sg_where "SELECT sign(id) FROM sensors WHERE id = -1" \
  sg_limit_zero "SELECT sign(id) FROM sensors LIMIT 0" \
  sq_reading "SELECT sqrt(reading) FROM sensors" \
  sq_score "SELECT sqrt(score) FROM sensors" \
  sq_id "SELECT sqrt(id) FROM sensors" \
  sq_flags "SELECT sqrt(flags) FROM sensors" \
  sq_byte "SELECT sqrt(b) FROM sensors" \
  sq_code "SELECT sqrt(code) FROM sensors" \
  sq_four "SELECT sqrt(4) FROM sensors LIMIT 1" \
  sq_neg "SELECT sqrt(-1) FROM sensors LIMIT 1" \
  sq_null "SELECT sqrt(null) FROM sensors LIMIT 1" \
  sq_case "SELECT SQRT(READING) FROM sensors" \
  sq_order "SELECT sqrt(id) FROM sensors ORDER BY sqrt(id)" \
  sq_reading_order "SELECT sqrt(reading) FROM sensors ORDER BY sqrt(reading)" \
  sq_with_id "SELECT id, sqrt(id) FROM sensors ORDER BY sqrt(id)" \
  sq_where "SELECT sqrt(id) FROM sensors WHERE id = 1" \
  sq_limit_zero "SELECT sqrt(id) FROM sensors LIMIT 0" \
  pw_reading "SELECT power(reading, 2) FROM sensors" \
  pw_score "SELECT power(score, 2) FROM sensors" \
  pw_id "SELECT power(id, 2) FROM sensors" \
  pw_flags "SELECT power(flags, 2) FROM sensors" \
  pw_byte "SELECT power(b, 2) FROM sensors" \
  pw_code "SELECT power(code, 2) FROM sensors" \
  pw_neg "SELECT power(-2, 3) FROM sensors LIMIT 1" \
  pw_root "SELECT power(4, 0.5) FROM sensors LIMIT 1" \
  pw_imag "SELECT power(-2, 0.5) FROM sensors LIMIT 1" \
  pw_zero "SELECT power(0, 0) FROM sensors LIMIT 1" \
  pw_inv "SELECT power(0, -1) FROM sensors LIMIT 1" \
  pw_negzero "SELECT power(-0.0, 1) FROM sensors LIMIT 1" \
  pw_null_base "SELECT power(null, 0) FROM sensors LIMIT 1" \
  pw_null_exp "SELECT power(reading, null) FROM sensors" \
  pw_cols "SELECT power(reading, id) FROM sensors" \
  pw_flags_exp "SELECT power(reading, flags) FROM sensors" \
  pw_case "SELECT POWER(READING, 2) FROM sensors" \
  pw_order "SELECT power(id, 2) FROM sensors ORDER BY power(id, 2)" \
  pw_with_id "SELECT id, power(id, 2) FROM sensors ORDER BY power(id, 2)" \
  pw_where "SELECT power(id, 2) FROM sensors WHERE id = -1" \
  pw_limit_zero "SELECT power(id, 2) FROM sensors LIMIT 0" \
  ln_reading "SELECT ln(reading) FROM sensors" \
  ln_score "SELECT ln(score) FROM sensors" \
  ln_id "SELECT ln(id) FROM sensors" \
  ln_flags "SELECT ln(flags) FROM sensors" \
  ln_byte "SELECT ln(b) FROM sensors" \
  ln_code "SELECT ln(code) FROM sensors" \
  ln_one "SELECT ln(1) FROM sensors LIMIT 1" \
  ln_zero "SELECT ln(0) FROM sensors LIMIT 1" \
  ln_neg "SELECT ln(-1) FROM sensors LIMIT 1" \
  ln_null "SELECT ln(null) FROM sensors LIMIT 1" \
  ln_case "SELECT LN(READING) FROM sensors" \
  ln_order "SELECT ln(id) FROM sensors ORDER BY ln(id)" \
  ln_reading_order "SELECT ln(reading) FROM sensors ORDER BY ln(reading)" \
  ln_with_id "SELECT id, ln(id) FROM sensors ORDER BY ln(id)" \
  ln_where "SELECT ln(id) FROM sensors WHERE id = 1" \
  ln_limit_zero "SELECT ln(id) FROM sensors LIMIT 0" \
  lg_reading "SELECT log(reading) FROM sensors" \
  lg_score "SELECT log(score) FROM sensors" \
  lg_id "SELECT log(id) FROM sensors" \
  lg_flags "SELECT log(flags) FROM sensors" \
  lg_byte "SELECT log(b) FROM sensors" \
  lg_code "SELECT log(code) FROM sensors" \
  lg_ten "SELECT log(10) FROM sensors LIMIT 1" \
  lg_zero "SELECT log(0) FROM sensors LIMIT 1" \
  lg_neg "SELECT log(-1) FROM sensors LIMIT 1" \
  lg_null "SELECT log(null) FROM sensors LIMIT 1" \
  lg_case "SELECT LOG(READING) FROM sensors" \
  lg_order "SELECT log(id) FROM sensors ORDER BY log(id)" \
  lg_reading_order "SELECT log(reading) FROM sensors ORDER BY log(reading)" \
  lg_with_id "SELECT id, log(id) FROM sensors ORDER BY log(id)" \
  lg_where "SELECT log(id) FROM sensors WHERE id = 1" \
  lg_limit_zero "SELECT log(id) FROM sensors LIMIT 0" \
  ex_reading "SELECT exp(reading) FROM sensors" \
  ex_score "SELECT exp(score) FROM sensors" \
  ex_id "SELECT exp(id) FROM sensors" \
  ex_flags "SELECT exp(flags) FROM sensors" \
  ex_byte "SELECT exp(b) FROM sensors" \
  ex_one "SELECT exp(1) FROM sensors LIMIT 1" \
  ex_zero "SELECT exp(0) FROM sensors LIMIT 1" \
  ex_neg "SELECT exp(-1) FROM sensors LIMIT 1" \
  ex_negzero "SELECT exp(-0.0) FROM sensors LIMIT 1" \
  ex_big "SELECT exp(1000) FROM sensors LIMIT 1" \
  ex_null "SELECT exp(null) FROM sensors LIMIT 1" \
  ex_case "SELECT EXP(READING) FROM sensors" \
  ex_order "SELECT exp(id) FROM sensors ORDER BY exp(id)" \
  ex_reading_order "SELECT exp(reading) FROM sensors ORDER BY exp(reading)" \
  ex_with_id "SELECT id, exp(id) FROM sensors ORDER BY exp(id)" \
  ex_where "SELECT exp(id) FROM sensors WHERE id = -1" \
  ex_limit_zero "SELECT exp(id) FROM sensors LIMIT 0" \
  pi_one "SELECT pi() FROM sensors LIMIT 1" \
  pi_all "SELECT pi() FROM sensors" \
  pi_case "SELECT PI() FROM sensors" \
  pi_with_id "SELECT id, pi() FROM sensors ORDER BY id" \
  pi_where "SELECT pi() FROM sensors WHERE id = 1" \
  pi_order "SELECT pi() FROM sensors ORDER BY pi()" \
  pi_limit_zero "SELECT pi() FROM sensors LIMIT 0" \
  gt_id_flags "SELECT greatest(id, flags) FROM sensors" \
  gt_reading_score "SELECT greatest(reading, score) FROM sensors" \
  gt_code_byte "SELECT greatest(code, b) FROM sensors" \
  gt_id "SELECT greatest(id) FROM sensors" \
  gt_three "SELECT greatest(id, flags, reading) FROM sensors" \
  gt_null_id "SELECT greatest(null, id) FROM sensors" \
  gt_nulls "SELECT greatest(null, null) FROM sensors LIMIT 1" \
  gt_negzero "SELECT greatest(-0.0, 0.0) FROM sensors LIMIT 1" \
  gt_poszero "SELECT greatest(0.0, -0.0) FROM sensors LIMIT 1" \
  gt_neg "SELECT greatest(-1.5, -0.0) FROM sensors LIMIT 1" \
  gt_code_one "SELECT greatest(code, 1) FROM sensors" \
  gt_double_lit "SELECT greatest(1.5, 2) FROM sensors LIMIT 1" \
  gt_long_lit "SELECT greatest(3000000000, 1) FROM sensors LIMIT 1" \
  gt_score_one "SELECT greatest(score, 1) FROM sensors" \
  gt_mix "SELECT greatest(b, code, flags) FROM sensors" \
  gt_case "SELECT GREATEST(ID, FLAGS) FROM sensors" \
  gt_order "SELECT greatest(id, flags) FROM sensors ORDER BY greatest(id, flags)" \
  gt_reading_order "SELECT greatest(reading) FROM sensors ORDER BY greatest(reading)" \
  gt_score_order "SELECT greatest(score) FROM sensors ORDER BY greatest(score)" \
  gt_flags_order "SELECT greatest(flags) FROM sensors ORDER BY greatest(flags)" \
  gt_where "SELECT greatest(id, flags) FROM sensors WHERE id = -1" \
  gt_limit_zero "SELECT greatest(id, flags) FROM sensors LIMIT 0" \
  ls_id_flags "SELECT least(id, flags) FROM sensors" \
  ls_reading_score "SELECT least(reading, score) FROM sensors" \
  ls_code_byte "SELECT least(code, b) FROM sensors" \
  ls_id "SELECT least(id) FROM sensors" \
  ls_three "SELECT least(id, flags, reading) FROM sensors" \
  ls_null_id "SELECT least(null, id) FROM sensors" \
  ls_nulls "SELECT least(null, null) FROM sensors LIMIT 1" \
  ls_negzero "SELECT least(-0.0, 0.0) FROM sensors LIMIT 1" \
  ls_poszero "SELECT least(0.0, -0.0) FROM sensors LIMIT 1" \
  ls_neg "SELECT least(-1.5, -0.0) FROM sensors LIMIT 1" \
  ls_code_one "SELECT least(code, 1) FROM sensors" \
  ls_double_lit "SELECT least(1.5, 2) FROM sensors LIMIT 1" \
  ls_long_lit "SELECT least(3000000000, 1) FROM sensors LIMIT 1" \
  ls_score_one "SELECT least(score, 1) FROM sensors" \
  ls_mix "SELECT least(b, code, flags) FROM sensors" \
  ls_flags_zero "SELECT least(flags, 0) FROM sensors" \
  ls_id_ten "SELECT least(id, 10) FROM sensors" \
  ls_case "SELECT LEAST(ID, FLAGS) FROM sensors" \
  ls_order "SELECT least(id, flags) FROM sensors ORDER BY least(id, flags)" \
  ls_reading_order "SELECT least(reading) FROM sensors ORDER BY least(reading)" \
  ls_score_order "SELECT least(score) FROM sensors ORDER BY least(score)" \
  ls_flags_order "SELECT least(flags) FROM sensors ORDER BY least(flags)" \
  ls_where "SELECT least(id, flags) FROM sensors WHERE id = -1" \
  ls_limit_zero "SELECT least(id, flags) FROM sensors LIMIT 0" \
  rdn_reading "SELECT round_down(reading, 0) FROM sensors" \
  rdn_reading_scale "SELECT round_down(reading, 1) FROM sensors" \
  rdn_score "SELECT round_down(score, 0) FROM sensors" \
  rdn_score_scale "SELECT round_down(score, 1) FROM sensors" \
  rdn_half "SELECT round_down(1.5, 0) FROM sensors LIMIT 1" \
  rdn_neg_half "SELECT round_down(-1.5, 0) FROM sensors LIMIT 1" \
  rdn_neg "SELECT round_down(-1.29, 1) FROM sensors LIMIT 1" \
  rdn_pos "SELECT round_down(1.29, 1) FROM sensors LIMIT 1" \
  rdn_tens "SELECT round_down(15, -1) FROM sensors LIMIT 1" \
  rdn_neg_tens "SELECT round_down(-15, -1) FROM sensors LIMIT 1" \
  rdn_negzero "SELECT round_down(-0.0, 1) FROM sensors LIMIT 1" \
  rdn_scale16 "SELECT round_down(1.29, 16) FROM sensors LIMIT 1" \
  rdn_null_scale "SELECT round_down(reading, null) FROM sensors" \
  rdn_scale_out "SELECT round_down(reading, 20) FROM sensors" \
  rdn_null "SELECT round_down(null, 1) FROM sensors LIMIT 1" \
  rdn_id "SELECT round_down(id, 0) FROM sensors" \
  rdn_code "SELECT round_down(code, 0) FROM sensors" \
  rdn_case "SELECT ROUND_DOWN(READING, 0) FROM sensors" \
  rdn_flags "SELECT round_down(reading, flags) FROM sensors" \
  rdn_order "SELECT round_down(reading, 1) FROM sensors ORDER BY round_down(reading, 1)" \
  rdn_where "SELECT round_down(id, 0) FROM sensors WHERE id = -1" \
  rdn_limit_zero "SELECT round_down(id, 0) FROM sensors LIMIT 0" \
  rup_reading "SELECT round_up(reading, 0) FROM sensors" \
  rup_reading_scale "SELECT round_up(reading, 1) FROM sensors" \
  rup_score "SELECT round_up(score, 0) FROM sensors" \
  rup_score_scale "SELECT round_up(score, 1) FROM sensors" \
  rup_half "SELECT round_up(1.5, 0) FROM sensors LIMIT 1" \
  rup_neg_half "SELECT round_up(-1.5, 0) FROM sensors LIMIT 1" \
  rup_neg "SELECT round_up(-1.21, 1) FROM sensors LIMIT 1" \
  rup_pos "SELECT round_up(1.21, 1) FROM sensors LIMIT 1" \
  rup_tens "SELECT round_up(15, -1) FROM sensors LIMIT 1" \
  rup_neg_tens "SELECT round_up(-15, -1) FROM sensors LIMIT 1" \
  rup_negzero "SELECT round_up(-0.0, 1) FROM sensors LIMIT 1" \
  rup_scale16 "SELECT round_up(1.21, 16) FROM sensors LIMIT 1" \
  rup_null_scale "SELECT round_up(reading, null) FROM sensors" \
  rup_scale_out "SELECT round_up(reading, 20) FROM sensors" \
  rup_null "SELECT round_up(null, 1) FROM sensors LIMIT 1" \
  rup_id "SELECT round_up(id, 0) FROM sensors" \
  rup_code "SELECT round_up(code, 0) FROM sensors" \
  rup_case "SELECT ROUND_UP(READING, 0) FROM sensors" \
  rup_flags "SELECT round_up(reading, flags) FROM sensors" \
  rup_order "SELECT round_up(reading, 1) FROM sensors ORDER BY round_up(reading, 1)" \
  rup_where "SELECT round_up(id, 0) FROM sensors WHERE id = -1" \
  rup_limit_zero "SELECT round_up(id, 0) FROM sensors LIMIT 0" \
  rhe_reading "SELECT round_half_even(reading, 0) FROM sensors" \
  rhe_reading_scale "SELECT round_half_even(reading, 1) FROM sensors" \
  rhe_score "SELECT round_half_even(score, 0) FROM sensors" \
  rhe_score_scale "SELECT round_half_even(score, 1) FROM sensors" \
  rhe_half "SELECT round_half_even(1.5, 0) FROM sensors LIMIT 1" \
  rhe_even "SELECT round_half_even(2.5, 0) FROM sensors LIMIT 1" \
  rhe_neg_half "SELECT round_half_even(-1.5, 0) FROM sensors LIMIT 1" \
  rhe_neg_even "SELECT round_half_even(-2.5, 0) FROM sensors LIMIT 1" \
  rhe_zero "SELECT round_half_even(0.0, 0) FROM sensors LIMIT 1" \
  rhe_negzero "SELECT round_half_even(-0.0, 1) FROM sensors LIMIT 1" \
  rhe_down "SELECT round_half_even(1.25, 1) FROM sensors LIMIT 1" \
  rhe_up "SELECT round_half_even(1.35, 1) FROM sensors LIMIT 1" \
  rhe_tens "SELECT round_half_even(15, -1) FROM sensors LIMIT 1" \
  rhe_neg_tens "SELECT round_half_even(-15, -1) FROM sensors LIMIT 1" \
  rhe_scale16 "SELECT round_half_even(1.25, 16) FROM sensors LIMIT 1" \
  rhe_null_scale "SELECT round_half_even(reading, null) FROM sensors" \
  rhe_scale_out "SELECT round_half_even(reading, 20) FROM sensors" \
  rhe_null "SELECT round_half_even(null, 1) FROM sensors LIMIT 1" \
  rhe_id "SELECT round_half_even(id, 0) FROM sensors" \
  rhe_code "SELECT round_half_even(code, 0) FROM sensors" \
  rhe_case "SELECT ROUND_HALF_EVEN(READING, 0) FROM sensors" \
  rhe_flags "SELECT round_half_even(reading, flags) FROM sensors" \
  rhe_order "SELECT round_half_even(reading, 0) FROM sensors ORDER BY round_half_even(reading, 0)" \
  rhe_where "SELECT round_half_even(id, 0) FROM sensors WHERE id = -1" \
  rhe_limit_zero "SELECT round_half_even(id, 0) FROM sensors LIMIT 0" \
  ca_id_flags "SELECT coalesce(id, flags) FROM sensors" \
  ca_flags_id "SELECT coalesce(flags, id) FROM sensors" \
  ca_reading_score "SELECT coalesce(reading, score) FROM sensors" \
  ca_score_reading "SELECT coalesce(score, reading) FROM sensors" \
  ca_note "SELECT coalesce(note, 'x') FROM sensors" \
  ca_location "SELECT coalesce(location, note) FROM sensors" \
  ca_reading_zero "SELECT coalesce(reading, 0.0) FROM sensors" \
  ca_negzero "SELECT coalesce(-0.0, 1.0) FROM sensors LIMIT 1" \
  ca_note_null "SELECT coalesce(note, null, 'z') FROM sensors" \
  ca_case "SELECT COALESCE(NOTE, 'x') FROM sensors" \
  ca_null_id "SELECT coalesce(null, id, 0) FROM sensors" \
  ca_code "SELECT coalesce(code, 1) FROM sensors" \
  ca_byte "SELECT coalesce(b, 1) FROM sensors" \
  ca_score "SELECT coalesce(score, 1) FROM sensors" \
  ca_reading_int "SELECT coalesce(reading, 0) FROM sensors" \
  ca_code_byte "SELECT coalesce(code, b) FROM sensors" \
  ca_byte_code "SELECT coalesce(b, code) FROM sensors" \
  ca_char "SELECT coalesce(ch, 'Z') FROM sensors" \
  ca_null_code "SELECT coalesce(null, code) FROM sensors LIMIT 1" \
  ca_null_char "SELECT coalesce(null, ch) FROM sensors LIMIT 1" \
  ca_order "SELECT coalesce(id, flags) FROM sensors ORDER BY coalesce(id, flags)" \
  ca_reading_order "SELECT coalesce(reading, 0.0) FROM sensors ORDER BY coalesce(reading, 0.0)" \
  ca_text_order "SELECT coalesce(note, tag) FROM sensors ORDER BY coalesce(note, tag)" \
  ca_where "SELECT coalesce(note, 'x') FROM sensors WHERE id = 1" \
  ca_limit_zero "SELECT coalesce(id, flags) FROM sensors LIMIT 0" \
  ni_id "SELECT nullif(id, 1) FROM sensors" \
  ni_id_flags "SELECT nullif(id, flags) FROM sensors" \
  ni_flags "SELECT nullif(flags, 10) FROM sensors" \
  ni_reading "SELECT nullif(reading, 1.5) FROM sensors" \
  ni_reading_score "SELECT nullif(reading, score) FROM sensors" \
  ni_score "SELECT nullif(score, 1.25) FROM sensors" \
  ni_note "SELECT nullif(note, 'alpha') FROM sensors" \
  ni_empty "SELECT nullif(note, '') FROM sensors" \
  ni_location "SELECT nullif(location, 'madrid') FROM sensors" \
  ni_tag "SELECT nullif(tag, 'v1') FROM sensors" \
  ni_code "SELECT nullif(code, 7) FROM sensors" \
  ni_byte "SELECT nullif(b, 3) FROM sensors" \
  ni_char "SELECT nullif(ch, 'A') FROM sensors" \
  ni_nulls "SELECT nullif(null, null) FROM sensors LIMIT 1" \
  ni_negzero "SELECT nullif(-0.0, 0.0) FROM sensors LIMIT 1" \
  ni_reading_zero "SELECT nullif(reading, 0) FROM sensors" \
  ni_id_double "SELECT nullif(id, 1.5) FROM sensors" \
  ni_flags_id "SELECT nullif(flags, id) FROM sensors" \
  ni_one_id "SELECT nullif(1, id) FROM sensors" \
  ni_beta "SELECT nullif(ch, 'β') FROM sensors" \
  ni_note_null "SELECT nullif(note, null) FROM sensors" \
  ni_empty_lit "SELECT nullif('', '') FROM sensors LIMIT 1" \
  ni_code_byte "SELECT nullif(code, b) FROM sensors" \
  ni_score_int "SELECT nullif(score, 1) FROM sensors" \
  ni_char_text "SELECT nullif(ch, 'ab') FROM sensors" \
  ni_lit_char "SELECT nullif('A', ch) FROM sensors" \
  ni_case "SELECT NULLIF(NOTE, 'alpha') FROM sensors" \
  ni_order "SELECT nullif(id, flags) FROM sensors ORDER BY nullif(id, flags)" \
  ni_reading_order "SELECT nullif(reading, 1.5) FROM sensors ORDER BY nullif(reading, 1.5)" \
  ni_text_order "SELECT nullif(note, 'alpha') FROM sensors ORDER BY nullif(note, 'alpha')" \
  ni_where "SELECT nullif(note, 'alpha') FROM sensors WHERE id = 1" \
  ni_limit_zero "SELECT nullif(id, 1) FROM sensors LIMIT 0" \
  cs_id_int "SELECT cast(id AS INT) FROM sensors" \
  cs_id_double "SELECT cast(id AS DOUBLE) FROM sensors" \
  cs_id_short "SELECT cast(id AS SHORT) FROM sensors" \
  cs_id_byte "SELECT cast(id AS BYTE) FROM sensors" \
  cs_id_bool "SELECT cast(id AS BOOLEAN) FROM sensors" \
  cs_id_string "SELECT cast(id AS STRING) FROM sensors" \
  cs_id_timestamp "SELECT cast(id AS TIMESTAMP) FROM sensors" \
  cs_id_date "SELECT cast(id AS DATE) FROM sensors" \
  cs_id_char "SELECT cast(id AS CHAR) FROM sensors" \
  cs_flags_long "SELECT cast(flags AS LONG) FROM sensors" \
  cs_flags_bool "SELECT cast(flags AS BOOLEAN) FROM sensors" \
  cs_reading_long "SELECT cast(reading AS LONG) FROM sensors" \
  cs_reading_int "SELECT cast(reading AS INT) FROM sensors" \
  cs_reading_float "SELECT cast(reading AS FLOAT) FROM sensors" \
  cs_reading_string "SELECT cast(reading AS STRING) FROM sensors" \
  cs_score_double "SELECT cast(score AS DOUBLE) FROM sensors" \
  cs_score_string "SELECT cast(score AS STRING) FROM sensors" \
  cs_code_int "SELECT cast(code AS INT) FROM sensors" \
  cs_byte_int "SELECT cast(b AS INT) FROM sensors" \
  cs_ok_int "SELECT cast(ok AS INT) FROM sensors" \
  cs_ok_string "SELECT cast(ok AS STRING) FROM sensors" \
  cs_char_string "SELECT cast(ch AS STRING) FROM sensors" \
  cs_note_varchar "SELECT cast(note AS VARCHAR) FROM sensors" \
  cs_note_symbol "SELECT cast(note AS SYMBOL) FROM sensors" \
  cs_note_long "SELECT cast(note AS LONG) FROM sensors" \
  cs_location_string "SELECT cast(location AS STRING) FROM sensors" \
  cs_ts_string "SELECT cast(ts AS STRING) FROM sensors" \
  cs_ts_long "SELECT cast(ts AS LONG) FROM sensors" \
  cs_ts_date "SELECT cast(ts AS DATE) FROM sensors" \
  cs_case "SELECT CAST(ID AS INT) FROM sensors" \
  cs_lower "SELECT cast(id as int) FROM sensors" \
  cs_order "SELECT cast(id AS INT) FROM sensors ORDER BY cast(id AS INT)" \
  cs_where "SELECT cast(id AS INT) FROM sensors WHERE id = 1" \
  cs_limit_zero "SELECT cast(id AS INT) FROM sensors LIMIT 0" \
  da_day "SELECT dateadd('d', 1, ts) FROM sensors" \
  da_hour "SELECT dateadd('h', -1, ts) FROM sensors" \
  da_month "SELECT dateadd('M', 1, ts) FROM sensors" \
  da_month_back "SELECT dateadd('M', -1, ts) FROM sensors" \
  da_year "SELECT dateadd('y', 1, ts) FROM sensors" \
  da_minute "SELECT dateadd('m', 1, ts) FROM sensors" \
  da_week "SELECT dateadd('w', 1, ts) FROM sensors" \
  da_second "SELECT dateadd('s', 1, ts) FROM sensors" \
  da_nano "SELECT dateadd('n', 1500, ts) FROM sensors" \
  da_hour_upper "SELECT dateadd('H', 1, ts) FROM sensors" \
  da_milli "SELECT dateadd('T', 1, ts) FROM sensors" \
  da_micro "SELECT dateadd('u', 1, ts) FROM sensors" \
  da_flags "SELECT dateadd('d', flags, ts) FROM sensors" \
  da_case "SELECT DATEADD('d', 1, ts) FROM sensors" \
  da_order "SELECT dateadd('d', 1, ts) FROM sensors ORDER BY dateadd('d', 1, ts)" \
  da_flags_order "SELECT dateadd('d', flags, ts) FROM sensors ORDER BY dateadd('d', flags, ts)" \
  da_where "SELECT dateadd('d', 1, ts) FROM sensors WHERE id = 1" \
  da_limit_zero "SELECT dateadd('d', 1, ts) FROM sensors LIMIT 0" \
  df_day "SELECT datediff('d', ts, ts) FROM sensors" \
  df_second "SELECT datediff('s', ts, '2020-01-02T03:04:06.000000Z') FROM sensors" \
  df_hour "SELECT datediff('h', ts, '2020-01-02T06:04:05.000000Z') FROM sensors" \
  df_month_short "SELECT datediff('M', ts, '2020-02-02T03:04:04.000000Z') FROM sensors" \
  df_month "SELECT datediff('M', ts, '2020-02-02T03:04:05.000000Z') FROM sensors" \
  df_micro "SELECT datediff('u', ts, ts) FROM sensors" \
  df_id "SELECT datediff('d', ts, id) FROM sensors" \
  df_bad "SELECT datediff('x', ts, ts) FROM sensors" \
  df_case "SELECT DATEDIFF('s', ts, ts) FROM sensors" \
  df_order "SELECT datediff('s', ts, '2020-01-02T03:04:06.000000Z') FROM sensors ORDER BY datediff('s', ts, '2020-01-02T03:04:06.000000Z')" \
  df_where "SELECT datediff('s', ts, '2020-01-02T03:04:06.000000Z') FROM sensors WHERE id = 1" \
  df_limit_zero "SELECT datediff('d', ts, ts) FROM sensors LIMIT 0" \
  yr_ts "SELECT year(ts) FROM sensors" \
  yr_id "SELECT year(id) FROM sensors" \
  yr_lit "SELECT year('2020-01-02T03:04:05.000000Z') FROM sensors LIMIT 1" \
  yr_prev "SELECT year('2019-12-31T23:00:00.000000Z') FROM sensors LIMIT 1" \
  yr_case "SELECT YEAR(TS) FROM sensors" \
  yr_order "SELECT year(id) FROM sensors ORDER BY year(id)" \
  yr_where "SELECT year(ts) FROM sensors WHERE id = 1" \
  yr_limit_zero "SELECT year(ts) FROM sensors LIMIT 0" \
  mo_ts "SELECT month(ts) FROM sensors" \
  mo_id "SELECT month(id) FROM sensors" \
  mo_lit "SELECT month('2020-01-02T03:04:05.000000Z') FROM sensors LIMIT 1" \
  mo_prev "SELECT month('2019-12-31T23:00:00.000000Z') FROM sensors LIMIT 1" \
  mo_case "SELECT MONTH(TS) FROM sensors" \
  mo_order "SELECT month(id) FROM sensors ORDER BY month(id)" \
  mo_where "SELECT month(ts) FROM sensors WHERE id = 1" \
  mo_limit_zero "SELECT month(ts) FROM sensors LIMIT 0" \
  dy_ts "SELECT day(ts) FROM sensors" \
  dy_id "SELECT day(id) FROM sensors" \
  dy_lit "SELECT day('2020-01-02T03:04:05.000000Z') FROM sensors LIMIT 1" \
  dy_prev "SELECT day('2019-12-31T23:00:00.000000Z') FROM sensors LIMIT 1" \
  dy_case "SELECT DAY(TS) FROM sensors" \
  dy_order "SELECT day(id) FROM sensors ORDER BY day(id)" \
  dy_where "SELECT day(ts) FROM sensors WHERE id = 1" \
  dy_limit_zero "SELECT day(ts) FROM sensors LIMIT 0" \
  hr_ts "SELECT hour(ts) FROM sensors" \
  hr_id "SELECT hour(id) FROM sensors" \
  hr_lit "SELECT hour('2020-01-02T03:04:05.000000Z') FROM sensors LIMIT 1" \
  hr_prev "SELECT hour('2019-12-31T23:00:00.000000Z') FROM sensors LIMIT 1" \
  hr_case "SELECT HOUR(TS) FROM sensors" \
  hr_order "SELECT hour(id) FROM sensors ORDER BY hour(id)" \
  hr_where "SELECT hour(ts) FROM sensors WHERE id = 1" \
  hr_limit_zero "SELECT hour(ts) FROM sensors LIMIT 0" \
  mi_ts "SELECT minute(ts) FROM sensors" \
  mi_id "SELECT minute(id) FROM sensors" \
  mi_lit "SELECT minute('2020-01-02T03:04:05.000000Z') FROM sensors LIMIT 1" \
  mi_prev "SELECT minute('2019-12-31T23:59:00.000000Z') FROM sensors LIMIT 1" \
  mi_case "SELECT MINUTE(TS) FROM sensors" \
  mi_order "SELECT minute(id) FROM sensors ORDER BY minute(id)" \
  mi_where "SELECT minute(ts) FROM sensors WHERE id = 1" \
  mi_limit_zero "SELECT minute(ts) FROM sensors LIMIT 0" \
  se_ts "SELECT second(ts) FROM sensors" \
  se_id "SELECT second(id) FROM sensors" \
  se_lit "SELECT second('2020-01-02T03:04:05.000000Z') FROM sensors LIMIT 1" \
  se_prev "SELECT second('2019-12-31T23:59:59.000000Z') FROM sensors LIMIT 1" \
  se_case "SELECT SECOND(TS) FROM sensors" \
  se_order "SELECT second(id) FROM sensors ORDER BY second(id)" \
  se_where "SELECT second(ts) FROM sensors WHERE id = 1" \
  se_limit_zero "SELECT second(ts) FROM sensors LIMIT 0" \
  ms_ts "SELECT millis(ts) FROM sensors" \
  ms_id "SELECT millis(id) FROM sensors" \
  ms_lit "SELECT millis('2020-01-02T03:04:05.123456Z') FROM sensors LIMIT 1" \
  ms_trunc "SELECT millis('2020-01-02T03:04:05.123999Z') FROM sensors LIMIT 1" \
  ms_case "SELECT MILLIS(TS) FROM sensors" \
  ms_order "SELECT millis(id) FROM sensors ORDER BY millis(id)" \
  ms_where "SELECT millis(ts) FROM sensors WHERE id = 1" \
  ms_limit_zero "SELECT millis(ts) FROM sensors LIMIT 0" \
  mc_ts "SELECT micros(ts) FROM sensors" \
  mc_id "SELECT micros(id) FROM sensors" \
  mc_lit "SELECT micros('2020-01-02T03:04:05.000123Z') FROM sensors LIMIT 1" \
  mc_within "SELECT micros('2020-01-02T03:04:05.001123Z') FROM sensors LIMIT 1" \
  mc_case "SELECT MICROS(TS) FROM sensors" \
  mc_order "SELECT micros(id) FROM sensors ORDER BY micros(id)" \
  mc_where "SELECT micros(ts) FROM sensors WHERE id = 1" \
  mc_limit_zero "SELECT micros(ts) FROM sensors LIMIT 0" \
  dw_ts "SELECT day_of_week(ts) FROM sensors" \
  dw_id "SELECT day_of_week(id) FROM sensors" \
  dw_lit "SELECT day_of_week('2020-01-02T03:04:05.000000Z') FROM sensors LIMIT 1" \
  dw_sun "SELECT day_of_week('2020-01-05T00:00:00.000000Z') FROM sensors LIMIT 1" \
  dw_mon "SELECT day_of_week('2020-01-06T00:00:00.000000Z') FROM sensors LIMIT 1" \
  dw_case "SELECT DAY_OF_WEEK(TS) FROM sensors" \
  dw_order "SELECT day_of_week(id) FROM sensors ORDER BY day_of_week(id)" \
  dw_where "SELECT day_of_week(ts) FROM sensors WHERE id = 1" \
  dw_limit_zero "SELECT day_of_week(ts) FROM sensors LIMIT 0" \
  wy_ts "SELECT week_of_year(ts) FROM sensors" \
  wy_id "SELECT week_of_year(id) FROM sensors" \
  wy_lit "SELECT week_of_year('2020-01-02T03:04:05.000000Z') FROM sensors LIMIT 1" \
  wy_prev "SELECT week_of_year('2019-12-29T00:00:00.000000Z') FROM sensors LIMIT 1" \
  wy_w53 "SELECT week_of_year('2020-12-31T00:00:00.000000Z') FROM sensors LIMIT 1" \
  wy_ny "SELECT week_of_year('2021-01-01T00:00:00.000000Z') FROM sensors LIMIT 1" \
  wy_case "SELECT WEEK_OF_YEAR(TS) FROM sensors" \
  wy_order "SELECT week_of_year(id) FROM sensors ORDER BY week_of_year(id)" \
  wy_where "SELECT week_of_year(ts) FROM sensors WHERE id = 1" \
  wy_limit_zero "SELECT week_of_year(ts) FROM sensors LIMIT 0" \
  dt_day "SELECT date_trunc('day', ts) FROM sensors" \
  dt_year "SELECT date_trunc('year', id) FROM sensors" \
  dt_minute "SELECT date_trunc('minute', '2020-01-02T03:04:05.123456Z') FROM sensors LIMIT 1" \
  dt_week "SELECT date_trunc('week', id) FROM sensors" \
  dt_mill "SELECT date_trunc('millennium', ts) FROM sensors LIMIT 1" \
  dt_cent "SELECT date_trunc('century', '2008-01-01T04:15:11.123000Z') FROM sensors LIMIT 1" \
  dt_dec "SELECT date_trunc('decade', '2008-01-01T00:00:00.000000Z') FROM sensors LIMIT 1" \
  dt_q "SELECT date_trunc('quarter', '2020-05-15T12:30:00.000000Z') FROM sensors LIMIT 1" \
  dt_ms "SELECT date_trunc('millisecond', '2020-01-02T03:04:05.123456Z') FROM sensors LIMIT 1" \
  dt_case "SELECT DATE_TRUNC('day', TS) FROM sensors LIMIT 1" \
  dt_order "SELECT date_trunc('hour', id) FROM sensors ORDER BY date_trunc('hour', id)" \
  dt_where "SELECT date_trunc('day', ts) FROM sensors WHERE id = 1" \
  dt_zero "SELECT date_trunc('day', ts) FROM sensors LIMIT 0" \
  tf_day "SELECT timestamp_floor('d', ts) FROM sensors" \
  tf_year "SELECT timestamp_floor('y', id) FROM sensors" \
  tf_2h "SELECT timestamp_floor('2h', id) FROM sensors" \
  tf_3d "SELECT timestamp_floor('3d', '2020-01-05T12:00:00.000000Z') FROM sensors LIMIT 1" \
  tf_3days "SELECT timestamp_floor('3d', id) FROM sensors" \
  tf_2m "SELECT timestamp_floor('2M', '2020-05-15T12:30:00.000000Z') FROM sensors LIMIT 1" \
  tf_2y "SELECT timestamp_floor('2y', '2021-06-01T00:00:00.000000Z') FROM sensors LIMIT 1" \
  tf_2w "SELECT timestamp_floor('2w', ts) FROM sensors LIMIT 1" \
  tf_ms "SELECT timestamp_floor('5T', '2020-01-02T03:04:05.123456Z') FROM sensors LIMIT 1" \
  tf_us "SELECT timestamp_floor('10U', '2020-01-02T03:04:05.123456Z') FROM sensors LIMIT 1" \
  tf_case "SELECT TIMESTAMP_FLOOR('d', TS) FROM sensors LIMIT 1" \
  tf_order "SELECT timestamp_floor('h', id) FROM sensors ORDER BY timestamp_floor('h', id)" \
  tf_where "SELECT timestamp_floor('d', ts) FROM sensors WHERE id = 1" \
  tf_zero "SELECT timestamp_floor('d', ts) FROM sensors LIMIT 0" \
  tc_day "SELECT timestamp_ceil('d', ts) FROM sensors" \
  tc_year "SELECT timestamp_ceil('y', id) FROM sensors" \
  tc_hour_id "SELECT timestamp_ceil('h', id) FROM sensors" \
  tc_week "SELECT timestamp_ceil('w', '2020-01-06T00:00:00.000000Z') FROM sensors LIMIT 1" \
  tc_month "SELECT timestamp_ceil('M', '2020-05-15T12:30:00.000000Z') FROM sensors LIMIT 1" \
  tc_ms "SELECT timestamp_ceil('T', '2020-01-02T03:04:05.123456Z') FROM sensors LIMIT 1" \
  tc_us "SELECT timestamp_ceil('U', '2020-01-02T03:04:05.123456Z') FROM sensors LIMIT 1" \
  tc_case "SELECT TIMESTAMP_CEIL('d', TS) FROM sensors LIMIT 1" \
  tc_order "SELECT timestamp_ceil('h', id) FROM sensors ORDER BY timestamp_ceil('h', id)" \
  tc_where "SELECT timestamp_ceil('d', ts) FROM sensors WHERE id = 1" \
  tc_zero "SELECT timestamp_ceil('d', ts) FROM sensors LIMIT 0" \
  ly_ts "SELECT is_leap_year(ts) FROM sensors" \
  ly_id "SELECT is_leap_year(id) FROM sensors" \
  ly_2020 "SELECT is_leap_year('2020-01-02T00:00:00.000000Z') FROM sensors LIMIT 1" \
  ly_2019 "SELECT is_leap_year('2019-12-31T00:00:00.000000Z') FROM sensors LIMIT 1" \
  ly_1900 "SELECT is_leap_year('1900-01-01T00:00:00.000000Z') FROM sensors LIMIT 1" \
  ly_2000 "SELECT is_leap_year('2000-01-01T00:00:00.000000Z') FROM sensors LIMIT 1" \
  ly_null "SELECT is_leap_year(null) FROM sensors LIMIT 1" \
  ly_case "SELECT IS_LEAP_YEAR(TS) FROM sensors LIMIT 1" \
  ly_where "SELECT is_leap_year(ts) FROM sensors WHERE id = 1" \
  ly_zero "SELECT is_leap_year(ts) FROM sensors LIMIT 0" \
  dm_ts "SELECT days_in_month(ts) FROM sensors" \
  dm_id "SELECT days_in_month(id) FROM sensors" \
  dm_feb "SELECT days_in_month('2020-02-01T00:00:00.000000Z') FROM sensors LIMIT 1" \
  dm_feb19 "SELECT days_in_month('2019-02-01T00:00:00.000000Z') FROM sensors LIMIT 1" \
  dm_1900 "SELECT days_in_month('1900-02-01T00:00:00.000000Z') FROM sensors LIMIT 1" \
  dm_2000 "SELECT days_in_month('2000-02-01T00:00:00.000000Z') FROM sensors LIMIT 1" \
  dm_apr "SELECT days_in_month('2020-04-15T00:00:00.000000Z') FROM sensors LIMIT 1" \
  dm_null "SELECT days_in_month(null) FROM sensors LIMIT 1" \
  dm_case "SELECT DAYS_IN_MONTH(TS) FROM sensors LIMIT 1" \
  dm_order "SELECT days_in_month(id) FROM sensors ORDER BY days_in_month(id)" \
  dm_where "SELECT days_in_month(ts) FROM sensors WHERE id = 1" \
  dm_zero "SELECT days_in_month(ts) FROM sensors LIMIT 0" \
  ts_day "SELECT to_str(ts, 'yyyy-MM-dd') FROM sensors" \
  ts_id "SELECT to_str(id, 'yyyy-MM-dd') FROM sensors" \
  ts_frac "SELECT to_str('2020-01-02T03:04:05.123456Z', 'SSS UUU') FROM sensors LIMIT 1" \
  ts_ampm "SELECT to_str('2020-01-02T15:07:09.000000Z', 'h:mm a') FROM sensors LIMIT 1" \
  ts_k "SELECT to_str('2020-01-02T00:00:00.000000Z', 'k') FROM sensors LIMIT 1" \
  ts_names "SELECT to_str(ts, 'EEE MMM') FROM sensors LIMIT 1" \
  ts_week "SELECT to_str('2019-12-31T00:00:00.000000Z', 'w ww YYYY') FROM sensors LIMIT 1" \
  ts_quote "SELECT to_str(ts, 'yyyy''Q''MM') FROM sensors LIMIT 1" \
  ts_doy "SELECT to_str('2020-03-01T00:00:00.000000Z', 'D') FROM sensors LIMIT 1" \
  ts_dow "SELECT to_str('2020-01-05T00:00:00.000000Z', 'u') FROM sensors LIMIT 1" \
  ts_era "SELECT to_str('0001-01-01T00:00:00.000000Z', 'G yyyy') FROM sensors LIMIT 1" \
  ts_null "SELECT to_str(null, 'yyyy') FROM sensors LIMIT 1" \
  ts_case "SELECT TO_STR(TS, 'yyyy') FROM sensors LIMIT 1" \
  ts_order "SELECT to_str(id, 'yyyy-MM-dd') FROM sensors ORDER BY to_str(id, 'yyyy-MM-dd')" \
  ts_where "SELECT to_str(ts, 'yyyy-MM-dd') FROM sensors WHERE id = 1" \
  ts_zero "SELECT to_str(ts, 'yyyy-MM-dd') FROM sensors LIMIT 0" \
  em_ts "SELECT is_end_of_month(ts) FROM sensors" \
  em_id "SELECT is_end_of_month(id) FROM sensors" \
  em_jan "SELECT is_end_of_month('2020-01-31T00:00:00.000000Z') FROM sensors LIMIT 1" \
  em_leap "SELECT is_end_of_month('2020-02-29T12:00:00.000000Z') FROM sensors LIMIT 1" \
  em_feb28 "SELECT is_end_of_month('2020-02-28T00:00:00.000000Z') FROM sensors LIMIT 1" \
  em_feb19 "SELECT is_end_of_month('2019-02-28T00:00:00.000000Z') FROM sensors LIMIT 1" \
  em_apr "SELECT is_end_of_month('2020-04-30T00:00:00.000000Z') FROM sensors LIMIT 1" \
  em_mid "SELECT is_end_of_month('2020-04-15T00:00:00.000000Z') FROM sensors LIMIT 1" \
  em_null "SELECT is_end_of_month(null) FROM sensors LIMIT 1" \
  em_case "SELECT IS_END_OF_MONTH(TS) FROM sensors LIMIT 1" \
  em_order "SELECT is_end_of_month(id) FROM sensors ORDER BY is_end_of_month(id)" \
  em_where "SELECT is_end_of_month(ts) FROM sensors WHERE id = 1" \
  em_zero "SELECT is_end_of_month(ts) FROM sensors LIMIT 0"

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
  group_null_id "SELECT id, count() FROM days WHERE id = NULL GROUP BY id" \
  min_id "SELECT min(id) FROM days" \
  min_note "SELECT min(note) FROM days" \
  max_note "SELECT max(note) FROM days" \
  min_null "SELECT min(id) FROM days WHERE id = NULL" \
  group_min_id "SELECT location, min(id) FROM days GROUP BY location ORDER BY location" \
  group_max_note "SELECT location, max(note) FROM days GROUP BY location ORDER BY location" \
  sum_id "SELECT sum(id) FROM days" \
  sum_ts "SELECT sum(ts) FROM days" \
  sum_location "SELECT sum(location) FROM days" \
  sum_null "SELECT sum(id) FROM days WHERE id = NULL" \
  group_sum_id "SELECT location, sum(id) FROM days GROUP BY location ORDER BY location" \
  avg_id "SELECT avg(id) FROM days" \
  avg_null "SELECT avg(id) FROM days WHERE id = NULL" \
  avg_ts "SELECT avg(ts) FROM days" \
  group_avg_id "SELECT location, avg(id) FROM days GROUP BY location ORDER BY location" \
  group_avg_by_value "SELECT location, avg(id) FROM days GROUP BY location ORDER BY avg(id)" \
  first_id "SELECT first(id) FROM days" \
  last_id "SELECT last(id) FROM days" \
  first_note "SELECT first(note) FROM days" \
  last_note "SELECT last(note) FROM days" \
  first_ts "SELECT first(ts) FROM days" \
  last_ts "SELECT last(ts) FROM days" \
  first_null "SELECT first(id) FROM days WHERE id = NULL" \
  group_first_id "SELECT location, first(id) FROM days GROUP BY location ORDER BY location" \
  group_last_id "SELECT location, last(id) FROM days GROUP BY location ORDER BY location" \
  group_last_note "SELECT location, last(note) FROM days GROUP BY location ORDER BY last(note)" \
  cd_id "SELECT count_distinct(id) FROM days" \
  cd_note "SELECT count_distinct(note) FROM days" \
  cd_location "SELECT count_distinct(location) FROM days" \
  cd_null "SELECT count_distinct(id) FROM days WHERE id = NULL" \
  group_cd_id "SELECT location, count_distinct(id) FROM days GROUP BY location ORDER BY location" \
  group_cd_by_value "SELECT location, count_distinct(note) FROM days GROUP BY location ORDER BY count_distinct(note)" \
  concat_note_location "SELECT concat(note, location) FROM days" \
  concat_order_ts "SELECT concat(location, note) FROM days ORDER BY ts" \
  length_note "SELECT length(note) FROM days" \
  length_location "SELECT length(location) FROM days ORDER BY ts" \
  lower_note "SELECT lower(note) FROM days" \
  lower_location "SELECT lower(location) FROM days ORDER BY ts" \
  upper_note "SELECT upper(note) FROM days" \
  upper_location "SELECT upper(location) FROM days ORDER BY ts" \
  left_note "SELECT left(note, 2) FROM days" \
  right_note "SELECT right(note, 2) FROM days" \
  left_location "SELECT left(location, 3) FROM days ORDER BY ts" \
  sub_note "SELECT substring(note, 2, 4) FROM days" \
  sub_neg "SELECT substring(note, -1, 4) FROM days" \
  sub_location "SELECT substring(location, 2, 3) FROM days ORDER BY ts" \
  trim_note "SELECT trim(note) FROM days" \
  trim_location "SELECT trim(location) FROM days ORDER BY ts" \
  ltrim_note "SELECT ltrim(note) FROM days" \
  rtrim_location "SELECT rtrim(location) FROM days ORDER BY ts" \
  rep_note "SELECT replace(note, 'a', 'X') FROM days" \
  rep_location "SELECT replace(location, 'a', 'A') FROM days ORDER BY ts" \
  sw_note "SELECT starts_with(note, 'al') FROM days" \
  sw_location "SELECT starts_with(location, 'mad') FROM days ORDER BY ts" \
  sp_note "SELECT strpos(note, 'a') FROM days" \
  sp_location "SELECT strpos(location, 'id') FROM days ORDER BY ts" \
  part_note "SELECT split_part(note, 'a', 1) FROM days" \
  part_location "SELECT split_part(location, 'a', 2) FROM days ORDER BY ts" \
  tl_note "SELECT to_lowercase(note) FROM days" \
  tu_note "SELECT to_uppercase(note) FROM days ORDER BY ts" \
  tl_location "SELECT to_lowercase(location) FROM days ORDER BY ts" \
  rp_location "SELECT rpad(location, 8, '.') FROM days ORDER BY ts" \
  qi_location "SELECT quote_ident(location) FROM days ORDER BY ts" \
  qi_note "SELECT quote_ident(note) FROM days" \
  lb_note "SELECT length_bytes(note) FROM days" \
  sz_id "SELECT size_pretty(id) FROM days ORDER BY ts" \
  abs_id "SELECT abs(id) FROM days ORDER BY ts" \
  rd_id "SELECT round(id) FROM days ORDER BY ts" \
  fl_id "SELECT floor(id) FROM days ORDER BY ts" \
  ce_id "SELECT ceil(id) FROM days ORDER BY ts" \
  sg_id "SELECT sign(id) FROM days ORDER BY ts" \
  sq_id "SELECT sqrt(id) FROM days ORDER BY ts" \
  pw_id "SELECT power(id, 2) FROM days ORDER BY ts" \
  ln_id "SELECT ln(id) FROM days ORDER BY ts" \
  lg_id "SELECT log(id) FROM days ORDER BY ts" \
  ex_id "SELECT exp(id) FROM days ORDER BY ts" \
  pi_id "SELECT pi() FROM days ORDER BY ts" \
  gt_id "SELECT greatest(id, 2) FROM days ORDER BY ts" \
  ls_id "SELECT least(id, 2) FROM days ORDER BY ts" \
  rdn_id "SELECT round_down(id, 0) FROM days ORDER BY ts" \
  rup_id "SELECT round_up(id, 0) FROM days ORDER BY ts" \
  rhe_id "SELECT round_half_even(id, 0) FROM days ORDER BY ts" \
  ca_note "SELECT coalesce(note, 'x') FROM days ORDER BY ts" \
  ca_id "SELECT coalesce(id, 0) FROM days ORDER BY ts" \
  ni_note "SELECT nullif(note, 'x') FROM days ORDER BY ts" \
  ni_id "SELECT nullif(id, 2) FROM days ORDER BY ts" \
  cs_id_int "SELECT cast(id AS INT) FROM days ORDER BY ts" \
  cs_note "SELECT cast(note AS STRING) FROM days ORDER BY ts" \
  da_day "SELECT dateadd('d', 1, ts) FROM days ORDER BY ts" \
  df_day "SELECT datediff('d', ts, '2020-01-05T00:00:00.000000Z') FROM days ORDER BY ts" \
  yr_ts "SELECT year(ts) FROM days ORDER BY ts" \
  mo_ts "SELECT month(ts) FROM days ORDER BY ts" \
  dy_ts "SELECT day(ts) FROM days ORDER BY ts" \
  hr_ts "SELECT hour(ts) FROM days ORDER BY ts" \
  mi_ts "SELECT minute(ts) FROM days ORDER BY ts" \
  se_ts "SELECT second(ts) FROM days ORDER BY ts" \
  ms_ts "SELECT millis(ts) FROM days ORDER BY ts" \
  mc_ts "SELECT micros(ts) FROM days ORDER BY ts" \
  dw_ts "SELECT day_of_week(ts) FROM days ORDER BY ts" \
  wy_ts "SELECT week_of_year(ts) FROM days ORDER BY ts" \
  dt_hour "SELECT date_trunc('hour', ts) FROM days ORDER BY ts" \
  tf_hour "SELECT timestamp_floor('h', ts) FROM days ORDER BY ts" \
  tc_hour "SELECT timestamp_ceil('h', ts) FROM days ORDER BY ts" \
  ly_days "SELECT is_leap_year(ts) FROM days ORDER BY ts" \
  dm_days "SELECT days_in_month(ts) FROM days ORDER BY ts" \
  ts_days "SELECT to_str(ts, 'yyyy-MM-dd') FROM days ORDER BY ts" \
  em_days "SELECT is_end_of_month(ts) FROM days ORDER BY ts"
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
    (os.environ["DIFF_H1"], "SELECT min(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT max(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT min(note) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT max(note) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT min(reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT max(reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT min(id) FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT min(ok) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT max(ok) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT min(ch) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT min(ch) FROM sensors WHERE ch = ''"),
    (os.environ["DIFF_H1"], "SELECT min(b) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT min(code) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT min(flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT max(flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT min(ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT max(ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT min(day) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT max(day) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT min(tag) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT max(tag) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT min(location) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT max(location) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT min(score) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT max(score) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT MIN(ID) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT min(id) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT location, min(id) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT location, min(id) FROM sensors GROUP BY location ORDER BY min(id)"),
    (os.environ["DIFF_H1"], "SELECT location, max(note) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT id, max(reading) FROM sensors GROUP BY id ORDER BY id"),
    (os.environ["DIFF_H1"], "SELECT location, min(reading) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT sum(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT sum(reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT sum(score) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT sum(b) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT sum(code) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT sum(flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT sum(b) FROM sensors WHERE b = 0"),
    (os.environ["DIFF_H1"], "SELECT sum(id) FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT sum(reading) FROM sensors WHERE id = -1"),
    (os.environ["DIFF_H1"], "SELECT sum(reading) FROM sensors WHERE reading = 0.0"),
    (os.environ["DIFF_H1"], "SELECT sum(ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT sum(day) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT sum(location) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT SUM(ID) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT sum(id) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT location, sum(id) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT sum(id) FROM sensors GROUP BY location ORDER BY sum(id)"),
    (os.environ["DIFF_H1"], "SELECT id, sum(reading) FROM sensors GROUP BY id ORDER BY id"),
    (os.environ["DIFF_H1"], "SELECT location, sum(score) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT avg(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT avg(reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT avg(score) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT avg(b) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT avg(code) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT avg(flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT avg(ok) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT avg(id) FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT avg(reading) FROM sensors WHERE id = -1"),
    (os.environ["DIFF_H1"], "SELECT avg(reading) FROM sensors WHERE reading = 0.0"),
    (os.environ["DIFF_H1"], "SELECT avg(b) FROM sensors WHERE b = 0"),
    (os.environ["DIFF_H1"], "SELECT avg(ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT avg(day) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT avg(location) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT AVG(ID) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT avg(id) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT location, avg(id) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT id, avg(reading) FROM sensors GROUP BY id ORDER BY id"),
    (os.environ["DIFF_H1"], "SELECT location, avg(id) FROM sensors GROUP BY location ORDER BY avg(id)"),
    (os.environ["DIFF_H1"], "SELECT first(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT last(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT first(reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT last(reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT first(ok) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT last(ok) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT first(note) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT last(note) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT first(ch) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT last(tag) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT first(ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT last(day) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT first(location) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT last(location) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT first(id) FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT first(reading) FROM sensors WHERE id = 2"),
    (os.environ["DIFF_H1"], "SELECT last(note) FROM sensors WHERE note = ''"),
    (os.environ["DIFF_H1"], "SELECT first(ch) FROM sensors WHERE ch = ''"),
    (os.environ["DIFF_H1"], "SELECT FIRST(ID) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT first(id) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT last(score) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT location, first(id) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT location, last(id) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT id, first(reading) FROM sensors GROUP BY id ORDER BY id"),
    (os.environ["DIFF_H1"], "SELECT location, first(id) FROM sensors GROUP BY location ORDER BY first(id)"),
    (os.environ["DIFF_H1"], "SELECT count_distinct(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT count_distinct(reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT count_distinct(score) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT count_distinct(ok) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT count_distinct(ch) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT count_distinct(note) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT count_distinct(location) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT count_distinct(b) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT count_distinct(flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT count_distinct(day) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT count_distinct(ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT count_distinct(id) FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT count_distinct(id) FROM sensors WHERE id = NULL"),
    (os.environ["DIFF_H1"], "SELECT count_distinct(note) FROM sensors WHERE note = ''"),
    (os.environ["DIFF_H1"], "SELECT count_distinct(ch) FROM sensors WHERE ch = ''"),
    (os.environ["DIFF_H1"], "SELECT count_distinct(reading) FROM sensors WHERE reading = 0.0"),
    (os.environ["DIFF_H1"], "SELECT COUNT_DISTINCT(ID) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT count_distinct(id) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT location, count_distinct(id) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT location, count_distinct(note) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT location, count_distinct(ch) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT location, count_distinct(flags) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT id, count_distinct(note) FROM sensors GROUP BY id ORDER BY id"),
    (os.environ["DIFF_H1"], "SELECT location, count_distinct(note) FROM sensors GROUP BY location ORDER BY count_distinct(note)"),
    (os.environ["DIFF_H1"], "SELECT concat(note, location) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT concat(note, '!') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT concat('pre-', location) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT concat(note, null) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT concat(ch, note) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT concat(id, note) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT concat(reading, '') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT concat(day, '') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT concat(ts, '') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT concat(ok, '') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT concat(flags, '') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT CONCAT(NOTE, LOCATION) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT concat( note , location ) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT concat('it''s', '!') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT concat(note) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT concat(note, location, tag) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT concat(note, location) FROM sensors ORDER BY concat(note, location)"),
    (os.environ["DIFF_H1"], "SELECT concat(note, location) FROM sensors ORDER BY note"),
    (os.environ["DIFF_H1"], "SELECT id, concat(note, location) FROM sensors ORDER BY concat(note, location)"),
    (os.environ["DIFF_H1"], "SELECT concat(note, location) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT concat(note, location) FROM sensors WHERE note = ''"),
    (os.environ["DIFF_H1"], "SELECT length(note) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT length(location) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT length(tag) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT length(ch) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT length('') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT length(null) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT length('βeta') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT length('東京') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT LENGTH(NOTE) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT length(note) FROM sensors WHERE note = ''"),
    (os.environ["DIFF_H1"], "SELECT length(note) FROM sensors ORDER BY length(note)"),
    (os.environ["DIFF_H1"], "SELECT id, length(note) FROM sensors ORDER BY length(note)"),
    (os.environ["DIFF_H1"], "SELECT length(note) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT lower(note) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT lower(location) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT lower(tag) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT lower(ch) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT lower('BETA') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT lower('Héllo') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT lower('東京') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT lower('') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT lower(null) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT LOWER(NOTE) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT lower(note) FROM sensors ORDER BY lower(note)"),
    (os.environ["DIFF_H1"], "SELECT id, lower(note) FROM sensors ORDER BY lower(note)"),
    (os.environ["DIFF_H1"], "SELECT lower(note) FROM sensors WHERE note = ''"),
    (os.environ["DIFF_H1"], "SELECT lower(note) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT upper(note) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT upper(location) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT upper(tag) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT upper(ch) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT upper('βeta') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT upper('') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT upper(null) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT UPPER(NOTE) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT upper(note) FROM sensors ORDER BY upper(note)"),
    (os.environ["DIFF_H1"], "SELECT id, upper(note) FROM sensors ORDER BY upper(note)"),
    (os.environ["DIFF_H1"], "SELECT upper(note) FROM sensors WHERE note = ''"),
    (os.environ["DIFF_H1"], "SELECT upper(note) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT left(note, 2) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT right(note, 2) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT left(location, 3) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT right(tag, 4) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT left(ch, 1) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT left(note, 0) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT left(note, 10) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT left(note, -1) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT right(note, -2) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT left('βeta', 1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT right('東京', 1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT left(null, 2) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT left('', 2) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT LEFT(NOTE, 1) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT left(note, 2) FROM sensors ORDER BY left(note, 2)"),
    (os.environ["DIFF_H1"], "SELECT id, left(note, 1) FROM sensors ORDER BY left(note, 1)"),
    (os.environ["DIFF_H1"], "SELECT right(note, 2) FROM sensors WHERE note = ''"),
    (os.environ["DIFF_H1"], "SELECT left(note, 2) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT left(note, location) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT left(note, b) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT substring(note, 1, 2) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT substring(note, 0, 2) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT substring(note, 1, 10) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT substring(note, -1, 4) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT substring(note, -2, 2) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT substring(note, 0, 0) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT substring(note, 6, 2) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT substring('βeta', 1, 2) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT substring(null, 1, 2) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT substring('', 1, 2) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT SUBSTRING(NOTE, 1, 1) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT substring(ch, 1, 1) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT substring(tag, 2, 3) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT substring(location, 1, 3) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT substring(note, 2, 4) FROM sensors ORDER BY substring(note, 2, 4)"),
    (os.environ["DIFF_H1"], "SELECT id, substring(note, 1, 1) FROM sensors ORDER BY substring(note, 1, 1)"),
    (os.environ["DIFF_H1"], "SELECT substring(note, 1, 2) FROM sensors WHERE note = ''"),
    (os.environ["DIFF_H1"], "SELECT substring(note, 1, 2) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT substring(note, location, 2) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT substring(note, flags, 2) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT substring(note, null, 2) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT substring(note, code, 1) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT trim(note) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT trim(location) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT trim(tag) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT trim(ch) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT trim('  BETA  ') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT trim(' a b ') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT trim('βeta') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT trim('') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT trim(null) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT trim('  ') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT trim('\t ab \t') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT TRIM(NOTE) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT trim(note) FROM sensors ORDER BY trim(note)"),
    (os.environ["DIFF_H1"], "SELECT id, trim(note) FROM sensors ORDER BY trim(note)"),
    (os.environ["DIFF_H1"], "SELECT trim(note) FROM sensors WHERE note = ''"),
    (os.environ["DIFF_H1"], "SELECT trim(note) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT ltrim('  a b  ') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT rtrim('  a b  ') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT ltrim('  ') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT rtrim(null) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT rtrim('\t ab ') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT ltrim(note) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT rtrim(note) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT ltrim(location) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT rtrim(tag) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT ltrim(ch) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT LTRIM(NOTE) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT ltrim(note) FROM sensors ORDER BY ltrim(note)"),
    (os.environ["DIFF_H1"], "SELECT id, rtrim(note) FROM sensors ORDER BY rtrim(note)"),
    (os.environ["DIFF_H1"], "SELECT ltrim(note) FROM sensors WHERE note = ''"),
    (os.environ["DIFF_H1"], "SELECT rtrim(note) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT replace(note, 'a', 'X') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT replace(note, 'ta', '') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT replace('βeta', 'β', 'B') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT replace(null, 'a', 'b') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT replace('ab', null, 'c') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT replace('ab', 'a', null) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT replace('', 'a', 'b') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT replace('aaa', 'a', 'bb') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT replace('aaa', 'aa', 'b') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT replace('abc', '', 'x') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT REPLACE(NOTE, 'a', 'A') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT replace(note, 'a', 'X') FROM sensors ORDER BY replace(note, 'a', 'X')"),
    (os.environ["DIFF_H1"], "SELECT id, replace(note, 'a', 'X') FROM sensors ORDER BY replace(note, 'a', 'X')"),
    (os.environ["DIFF_H1"], "SELECT replace(note, 'a', 'X') FROM sensors WHERE note = ''"),
    (os.environ["DIFF_H1"], "SELECT replace(note, 'a', 'X') FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT replace(note, 'a', location) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT replace('ab', location, 'x') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT replace('', location, 'x') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT replace(tag, '0', 'x') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT starts_with(note, 'a') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT starts_with(note, 'β') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT starts_with(note, '') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT starts_with(note, null) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT starts_with(null, 'a') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT starts_with('', 'a') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT starts_with('alpha', 'alp') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT starts_with('alpha', 'alphaextra') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT starts_with('Alpha', 'a') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT STARTS_WITH(NOTE, 'a') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT starts_with(note, 'a') FROM sensors ORDER BY starts_with(note, 'a')"),
    (os.environ["DIFF_H1"], "SELECT id, starts_with(note, 'a') FROM sensors ORDER BY starts_with(note, 'a')"),
    (os.environ["DIFF_H1"], "SELECT starts_with(note, '') FROM sensors WHERE note = ''"),
    (os.environ["DIFF_H1"], "SELECT starts_with(note, 'a') FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT starts_with(ch, 'A') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT starts_with(tag, '0') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT starts_with(location, '東') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT strpos(note, 'a') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT strpos(note, 'ph') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT strpos(note, 'z') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT strpos(note, '') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT strpos(note, null) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT strpos(null, 'a') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT strpos('', 'a') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT strpos('βeta', 'β') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT strpos('aaa', 'aa') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT STRPOS(NOTE, 'a') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT strpos(note, 'a') FROM sensors ORDER BY strpos(note, 'a')"),
    (os.environ["DIFF_H1"], "SELECT id, strpos(note, 'a') FROM sensors ORDER BY strpos(note, 'a')"),
    (os.environ["DIFF_H1"], "SELECT strpos(note, 'a') FROM sensors WHERE note = ''"),
    (os.environ["DIFF_H1"], "SELECT strpos(note, 'a') FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT strpos(note, location) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT strpos(tag, '0') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT strpos(ch, 'A') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT position(note, 'a') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT split_part(note, 'a', 1) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT split_part(note, 'a', 2) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT split_part(note, 'a', 3) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT split_part('a-b-c', '-', 2) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT split_part('a-b-c', '-', -1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT split_part('a-b-c', '-', 4) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT split_part('a--b', '-', 2) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT split_part(note, '', 1) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT split_part('abc', '', -1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT split_part(null, 'a', 1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT split_part('ab', null, 1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT split_part(note, 'a', null) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT SPLIT_PART(NOTE, 'a', 1) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT split_part(note, 'a', 1) FROM sensors ORDER BY split_part(note, 'a', 1)"),
    (os.environ["DIFF_H1"], "SELECT id, split_part(note, 'a', 1) FROM sensors ORDER BY split_part(note, 'a', 1)"),
    (os.environ["DIFF_H1"], "SELECT split_part(note, 'a', 1) FROM sensors WHERE note = ''"),
    (os.environ["DIFF_H1"], "SELECT split_part(note, 'a', 1) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT to_lowercase(note) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT to_uppercase(note) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT to_uppercase('β') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT TO_LOWERCASE(NOTE) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT to_lowercase(note) FROM sensors ORDER BY to_lowercase(note)"),
    (os.environ["DIFF_H1"], "SELECT to_uppercase(note) FROM sensors ORDER BY to_uppercase(note)"),
    (os.environ["DIFF_H1"], "SELECT id, to_lowercase(note) FROM sensors ORDER BY to_lowercase(note)"),
    (os.environ["DIFF_H1"], "SELECT to_lowercase(null) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT to_lowercase(note) FROM sensors WHERE note = ''"),
    (os.environ["DIFF_H1"], "SELECT to_lowercase(note) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT to_uppercase(ch) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT to_uppercase(tag) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT lpad(note, 6, 'x') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT rpad(note, 6, 'x') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT lpad('ab', 5, 'xy') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT lpad('abc', 2, 'x') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT rpad('abc', 2, 'x') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT lpad('ab', 0, 'x') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT lpad('ab', -1, 'x') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT lpad(null, 3, 'x') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT lpad('ab', null, 'x') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT lpad('ab', 3, null) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT lpad('ab', 3, '') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT lpad(note, 3) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT rpad(note, 3) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT lpad(note, flags, 'x') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT lpad(note, b) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT lpad(ch, 3, 'x') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT lpad(note, 6, location) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT LPAD(NOTE, 6, 'x') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT rpad(note, 6, 'x') FROM sensors ORDER BY rpad(note, 6, 'x')"),
    (os.environ["DIFF_H1"], "SELECT lpad(note, 6, 'x') FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT quote_ident(note) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT quote_ident('a\"b') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT quote_ident('a b') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT quote_ident('a-b') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT quote_ident('Alpha') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT quote_ident('a$b') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT quote_ident('1a') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT quote_ident(null) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT quote_ident('') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT quote_ident(ch) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT quote_ident(tag) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT QUOTE_IDENT(NOTE) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT quote_ident(note) FROM sensors ORDER BY quote_ident(note)"),
    (os.environ["DIFF_H1"], "SELECT id, quote_ident(note) FROM sensors ORDER BY quote_ident(note)"),
    (os.environ["DIFF_H1"], "SELECT quote_ident(note) FROM sensors WHERE note = ''"),
    (os.environ["DIFF_H1"], "SELECT quote_ident(note) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT length_bytes(note) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT length_bytes(tag) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT length_bytes('β') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT length_bytes('東京') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT length_bytes(null) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT length_bytes('') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT length_bytes(ch) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT LENGTH_BYTES(NOTE) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT length_bytes(note) FROM sensors ORDER BY length_bytes(note)"),
    (os.environ["DIFF_H1"], "SELECT id, length_bytes(note) FROM sensors ORDER BY length_bytes(note)"),
    (os.environ["DIFF_H1"], "SELECT length_bytes(note) FROM sensors WHERE note = ''"),
    (os.environ["DIFF_H1"], "SELECT length_bytes(note) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT size_pretty(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT size_pretty(0) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT size_pretty(1024) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT size_pretty(1500) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT size_pretty(1048576) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT size_pretty(1023) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT size_pretty(-1024) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT size_pretty(9223372036854775807) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT size_pretty(null) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT size_pretty(flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT size_pretty(b) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT size_pretty(code) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT SIZE_PRETTY(ID) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT size_pretty(id) FROM sensors ORDER BY size_pretty(id)"),
    (os.environ["DIFF_H1"], "SELECT id, size_pretty(id) FROM sensors ORDER BY size_pretty(id)"),
    (os.environ["DIFF_H1"], "SELECT size_pretty(id) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT size_pretty(id) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT abs(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT abs(reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT abs(score) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT abs(b) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT abs(code) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT abs(flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT abs(-1.5) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT abs(-1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT abs(null) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT ABS(ID) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT abs(id) FROM sensors ORDER BY abs(id)"),
    (os.environ["DIFF_H1"], "SELECT id, abs(id) FROM sensors ORDER BY abs(id)"),
    (os.environ["DIFF_H1"], "SELECT abs(reading) FROM sensors ORDER BY abs(reading)"),
    (os.environ["DIFF_H1"], "SELECT abs(id) FROM sensors WHERE id = -1"),
    (os.environ["DIFF_H1"], "SELECT abs(id) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT round(reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT round(score) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT round(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT round(flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT round(b) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT round(1.5) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT round(2.5) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT round(-1.5) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT round(reading, 1) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT round(1.25, 1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT round(-1.25, 1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT round(15, -1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT round(null) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT round(reading, null) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT round(1.5, 20) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT ROUND(READING) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT round(id) FROM sensors ORDER BY round(id)"),
    (os.environ["DIFF_H1"], "SELECT id, round(id) FROM sensors ORDER BY round(id)"),
    (os.environ["DIFF_H1"], "SELECT round(id) FROM sensors WHERE id = -1"),
    (os.environ["DIFF_H1"], "SELECT round(id) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT floor(reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT floor(score) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT floor(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT floor(flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT floor(b) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT floor(code) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT floor(1.5) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT floor(-1.5) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT floor(1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT floor(3000000000) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT floor(null) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT FLOOR(READING) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT floor(id) FROM sensors ORDER BY floor(id)"),
    (os.environ["DIFF_H1"], "SELECT floor(score) FROM sensors ORDER BY floor(score)"),
    (os.environ["DIFF_H1"], "SELECT floor(reading) FROM sensors ORDER BY floor(reading)"),
    (os.environ["DIFF_H1"], "SELECT id, floor(id) FROM sensors ORDER BY floor(id)"),
    (os.environ["DIFF_H1"], "SELECT floor(id) FROM sensors WHERE id = -1"),
    (os.environ["DIFF_H1"], "SELECT floor(id) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT ceil(reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT ceil(score) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT ceil(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT ceil(flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT ceil(b) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT ceil(code) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT ceil(1.5) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT ceil(-1.5) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT ceil(1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT ceil(3000000000) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT ceil(null) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT CEIL(READING) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT ceiling(reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT ceil(id) FROM sensors ORDER BY ceil(id)"),
    (os.environ["DIFF_H1"], "SELECT ceil(score) FROM sensors ORDER BY ceil(score)"),
    (os.environ["DIFF_H1"], "SELECT ceil(reading) FROM sensors ORDER BY ceil(reading)"),
    (os.environ["DIFF_H1"], "SELECT id, ceil(id) FROM sensors ORDER BY ceil(id)"),
    (os.environ["DIFF_H1"], "SELECT ceil(id) FROM sensors WHERE id = -1"),
    (os.environ["DIFF_H1"], "SELECT ceil(id) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT sign(reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT sign(score) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT sign(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT sign(flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT sign(b) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT sign(code) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT sign(-1.5) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT sign(-0.0) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT sign(0) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT sign(1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT sign(3000000000) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT sign(null) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT SIGN(READING) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT sign(id) FROM sensors ORDER BY sign(id)"),
    (os.environ["DIFF_H1"], "SELECT sign(score) FROM sensors ORDER BY sign(score)"),
    (os.environ["DIFF_H1"], "SELECT sign(reading) FROM sensors ORDER BY sign(reading)"),
    (os.environ["DIFF_H1"], "SELECT id, sign(id) FROM sensors ORDER BY sign(id)"),
    (os.environ["DIFF_H1"], "SELECT sign(id) FROM sensors WHERE id = -1"),
    (os.environ["DIFF_H1"], "SELECT sign(id) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT sqrt(reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT sqrt(score) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT sqrt(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT sqrt(flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT sqrt(b) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT sqrt(code) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT sqrt(4) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT sqrt(-1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT sqrt(null) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT SQRT(READING) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT sqrt(id) FROM sensors ORDER BY sqrt(id)"),
    (os.environ["DIFF_H1"], "SELECT sqrt(reading) FROM sensors ORDER BY sqrt(reading)"),
    (os.environ["DIFF_H1"], "SELECT id, sqrt(id) FROM sensors ORDER BY sqrt(id)"),
    (os.environ["DIFF_H1"], "SELECT sqrt(id) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT sqrt(id) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT power(reading, 2) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT power(score, 2) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT power(id, 2) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT power(flags, 2) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT power(b, 2) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT power(code, 2) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT power(-2, 3) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT power(4, 0.5) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT power(-2, 0.5) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT power(0, 0) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT power(0, -1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT power(-0.0, 1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT power(null, 0) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT power(reading, null) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT power(reading, id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT power(reading, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT POWER(READING, 2) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT power(id, 2) FROM sensors ORDER BY power(id, 2)"),
    (os.environ["DIFF_H1"], "SELECT id, power(id, 2) FROM sensors ORDER BY power(id, 2)"),
    (os.environ["DIFF_H1"], "SELECT power(id, 2) FROM sensors WHERE id = -1"),
    (os.environ["DIFF_H1"], "SELECT power(id, 2) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT ln(reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT ln(score) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT ln(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT ln(flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT ln(b) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT ln(code) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT ln(1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT ln(0) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT ln(-1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT ln(null) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT LN(READING) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT ln(id) FROM sensors ORDER BY ln(id)"),
    (os.environ["DIFF_H1"], "SELECT ln(reading) FROM sensors ORDER BY ln(reading)"),
    (os.environ["DIFF_H1"], "SELECT id, ln(id) FROM sensors ORDER BY ln(id)"),
    (os.environ["DIFF_H1"], "SELECT ln(id) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT ln(id) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT log(reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT log(score) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT log(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT log(flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT log(b) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT log(code) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT log(10) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT log(0) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT log(-1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT log(null) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT LOG(READING) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT log(id) FROM sensors ORDER BY log(id)"),
    (os.environ["DIFF_H1"], "SELECT log(reading) FROM sensors ORDER BY log(reading)"),
    (os.environ["DIFF_H1"], "SELECT id, log(id) FROM sensors ORDER BY log(id)"),
    (os.environ["DIFF_H1"], "SELECT log(id) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT log(id) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT exp(reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT exp(score) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT exp(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT exp(flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT exp(b) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT exp(1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT exp(0) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT exp(-1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT exp(-0.0) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT exp(1000) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT exp(null) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT EXP(READING) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT exp(id) FROM sensors ORDER BY exp(id)"),
    (os.environ["DIFF_H1"], "SELECT exp(reading) FROM sensors ORDER BY exp(reading)"),
    (os.environ["DIFF_H1"], "SELECT id, exp(id) FROM sensors ORDER BY exp(id)"),
    (os.environ["DIFF_H1"], "SELECT exp(id) FROM sensors WHERE id = -1"),
    (os.environ["DIFF_H1"], "SELECT exp(id) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT pi() FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT pi() FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT PI() FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT id, pi() FROM sensors ORDER BY id"),
    (os.environ["DIFF_H1"], "SELECT pi() FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT pi() FROM sensors ORDER BY pi()"),
    (os.environ["DIFF_H1"], "SELECT pi() FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT greatest(id, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT greatest(reading, score) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT greatest(code, b) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT greatest(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT greatest(id, flags, reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT greatest(null, id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT greatest(null, null) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT greatest(-0.0, 0.0) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT greatest(0.0, -0.0) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT greatest(-1.5, -0.0) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT greatest(code, 1) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT greatest(1.5, 2) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT greatest(3000000000, 1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT greatest(score, 1) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT greatest(b, code, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT GREATEST(ID, FLAGS) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT greatest(id, flags) FROM sensors ORDER BY greatest(id, flags)"),
    (os.environ["DIFF_H1"], "SELECT greatest(reading) FROM sensors ORDER BY greatest(reading)"),
    (os.environ["DIFF_H1"], "SELECT greatest(score) FROM sensors ORDER BY greatest(score)"),
    (os.environ["DIFF_H1"], "SELECT greatest(flags) FROM sensors ORDER BY greatest(flags)"),
    (os.environ["DIFF_H1"], "SELECT greatest(id, flags) FROM sensors WHERE id = -1"),
    (os.environ["DIFF_H1"], "SELECT greatest(id, flags) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT least(id, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT least(reading, score) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT least(code, b) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT least(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT least(id, flags, reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT least(null, id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT least(null, null) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT least(-0.0, 0.0) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT least(0.0, -0.0) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT least(-1.5, -0.0) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT least(code, 1) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT least(1.5, 2) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT least(3000000000, 1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT least(score, 1) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT least(b, code, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT least(flags, 0) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT least(id, 10) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT LEAST(ID, FLAGS) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT least(id, flags) FROM sensors ORDER BY least(id, flags)"),
    (os.environ["DIFF_H1"], "SELECT least(reading) FROM sensors ORDER BY least(reading)"),
    (os.environ["DIFF_H1"], "SELECT least(score) FROM sensors ORDER BY least(score)"),
    (os.environ["DIFF_H1"], "SELECT least(flags) FROM sensors ORDER BY least(flags)"),
    (os.environ["DIFF_H1"], "SELECT least(id, flags) FROM sensors WHERE id = -1"),
    (os.environ["DIFF_H1"], "SELECT least(id, flags) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT round_down(reading, 0) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT round_down(reading, 1) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT round_down(score, 0) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT round_down(score, 1) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT round_down(1.5, 0) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT round_down(-1.5, 0) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT round_down(-1.29, 1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT round_down(1.29, 1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT round_down(15, -1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT round_down(-15, -1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT round_down(-0.0, 1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT round_down(1.29, 16) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT round_down(reading, null) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT round_down(reading, 20) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT round_down(null, 1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT round_down(id, 0) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT round_down(code, 0) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT ROUND_DOWN(READING, 0) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT round_down(reading, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT round_down(reading, 1) FROM sensors ORDER BY round_down(reading, 1)"),
    (os.environ["DIFF_H1"], "SELECT round_down(id, 0) FROM sensors WHERE id = -1"),
    (os.environ["DIFF_H1"], "SELECT round_down(id, 0) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT round_up(reading, 0) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT round_up(reading, 1) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT round_up(score, 0) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT round_up(score, 1) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT round_up(1.5, 0) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT round_up(-1.5, 0) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT round_up(-1.21, 1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT round_up(1.21, 1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT round_up(15, -1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT round_up(-15, -1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT round_up(-0.0, 1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT round_up(1.21, 16) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT round_up(reading, null) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT round_up(reading, 20) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT round_up(null, 1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT round_up(id, 0) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT round_up(code, 0) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT ROUND_UP(READING, 0) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT round_up(reading, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT round_up(reading, 1) FROM sensors ORDER BY round_up(reading, 1)"),
    (os.environ["DIFF_H1"], "SELECT round_up(id, 0) FROM sensors WHERE id = -1"),
    (os.environ["DIFF_H1"], "SELECT round_up(id, 0) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT round_half_even(reading, 0) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT round_half_even(reading, 1) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT round_half_even(score, 0) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT round_half_even(score, 1) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT round_half_even(1.5, 0) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT round_half_even(2.5, 0) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT round_half_even(-1.5, 0) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT round_half_even(-2.5, 0) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT round_half_even(0.0, 0) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT round_half_even(-0.0, 1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT round_half_even(1.25, 1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT round_half_even(1.35, 1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT round_half_even(15, -1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT round_half_even(-15, -1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT round_half_even(1.25, 16) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT round_half_even(reading, null) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT round_half_even(reading, 20) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT round_half_even(null, 1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT round_half_even(id, 0) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT round_half_even(code, 0) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT ROUND_HALF_EVEN(READING, 0) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT round_half_even(reading, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT round_half_even(reading, 0) FROM sensors ORDER BY round_half_even(reading, 0)"),
    (os.environ["DIFF_H1"], "SELECT round_half_even(id, 0) FROM sensors WHERE id = -1"),
    (os.environ["DIFF_H1"], "SELECT round_half_even(id, 0) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT coalesce(id, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT coalesce(flags, id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT coalesce(reading, score) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT coalesce(score, reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT coalesce(note, 'x') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT coalesce(location, note) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT coalesce(reading, 0.0) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT coalesce(-0.0, 1.0) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT coalesce(note, null, 'z') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT COALESCE(NOTE, 'x') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT coalesce(null, id, 0) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT coalesce(code, 1) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT coalesce(b, 1) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT coalesce(score, 1) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT coalesce(reading, 0) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT coalesce(code, b) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT coalesce(b, code) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT coalesce(ch, 'Z') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT coalesce(null, code) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT coalesce(null, ch) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT coalesce(id, flags) FROM sensors ORDER BY coalesce(id, flags)"),
    (os.environ["DIFF_H1"], "SELECT coalesce(reading, 0.0) FROM sensors ORDER BY coalesce(reading, 0.0)"),
    (os.environ["DIFF_H1"], "SELECT coalesce(note, tag) FROM sensors ORDER BY coalesce(note, tag)"),
    (os.environ["DIFF_H1"], "SELECT coalesce(note, 'x') FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT coalesce(id, flags) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT nullif(id, 1) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT nullif(id, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT nullif(flags, 10) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT nullif(reading, 1.5) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT nullif(reading, score) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT nullif(score, 1.25) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT nullif(note, 'alpha') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT nullif(note, '') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT nullif(location, 'madrid') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT nullif(tag, 'v1') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT nullif(code, 7) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT nullif(b, 3) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT nullif(ch, 'A') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT nullif(null, null) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT nullif(-0.0, 0.0) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT nullif(reading, 0) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT nullif(id, 1.5) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT nullif(flags, id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT nullif(1, id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT nullif(ch, 'β') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT nullif(note, null) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT nullif('', '') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT nullif(code, b) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT nullif(score, 1) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT nullif(ch, 'ab') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT nullif('A', ch) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT NULLIF(NOTE, 'alpha') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT nullif(id, flags) FROM sensors ORDER BY nullif(id, flags)"),
    (os.environ["DIFF_H1"], "SELECT nullif(reading, 1.5) FROM sensors ORDER BY nullif(reading, 1.5)"),
    (os.environ["DIFF_H1"], "SELECT nullif(note, 'alpha') FROM sensors ORDER BY nullif(note, 'alpha')"),
    (os.environ["DIFF_H1"], "SELECT nullif(note, 'alpha') FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT nullif(id, 1) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT cast(id AS INT) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT cast(id AS DOUBLE) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT cast(id AS SHORT) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT cast(id AS BYTE) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT cast(id AS BOOLEAN) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT cast(id AS STRING) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT cast(id AS TIMESTAMP) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT cast(id AS DATE) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT cast(id AS CHAR) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT cast(flags AS LONG) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT cast(flags AS BOOLEAN) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT cast(reading AS LONG) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT cast(reading AS INT) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT cast(reading AS FLOAT) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT cast(reading AS STRING) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT cast(score AS DOUBLE) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT cast(score AS STRING) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT cast(code AS INT) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT cast(b AS INT) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT cast(ok AS INT) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT cast(ok AS STRING) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT cast(ch AS STRING) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT cast(note AS VARCHAR) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT cast(note AS SYMBOL) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT cast(note AS LONG) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT cast(location AS STRING) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT cast(ts AS STRING) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT cast(ts AS LONG) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT cast(ts AS DATE) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT CAST(ID AS INT) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT cast(id as int) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT cast(id AS INT) FROM sensors ORDER BY cast(id AS INT)"),
    (os.environ["DIFF_H1"], "SELECT cast(id AS INT) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT cast(id AS INT) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT dateadd('d', 1, ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT dateadd('h', -1, ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT dateadd('M', 1, ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT dateadd('M', -1, ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT dateadd('y', 1, ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT dateadd('m', 1, ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT dateadd('w', 1, ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT dateadd('s', 1, ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT dateadd('n', 1500, ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT dateadd('H', 1, ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT dateadd('T', 1, ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT dateadd('u', 1, ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT dateadd('d', flags, ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT DATEADD('d', 1, ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT dateadd('d', 1, ts) FROM sensors ORDER BY dateadd('d', 1, ts)"),
    (os.environ["DIFF_H1"], "SELECT dateadd('d', flags, ts) FROM sensors ORDER BY dateadd('d', flags, ts)"),
    (os.environ["DIFF_H1"], "SELECT dateadd('d', 1, ts) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT dateadd('d', 1, ts) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT datediff('d', ts, ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT datediff('s', ts, '2020-01-02T03:04:06.000000Z') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT datediff('h', ts, '2020-01-02T06:04:05.000000Z') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT datediff('M', ts, '2020-02-02T03:04:04.000000Z') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT datediff('M', ts, '2020-02-02T03:04:05.000000Z') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT datediff('u', ts, ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT datediff('d', ts, id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT datediff('x', ts, ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT DATEDIFF('s', ts, ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT datediff('s', ts, '2020-01-02T03:04:06.000000Z') FROM sensors ORDER BY datediff('s', ts, '2020-01-02T03:04:06.000000Z')"),
    (os.environ["DIFF_H1"], "SELECT datediff('s', ts, '2020-01-02T03:04:06.000000Z') FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT datediff('d', ts, ts) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT year(ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT year(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT year('2020-01-02T03:04:05.000000Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT year('2019-12-31T23:00:00.000000Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT YEAR(TS) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT year(id) FROM sensors ORDER BY year(id)"),
    (os.environ["DIFF_H1"], "SELECT year(ts) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT year(ts) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT month(ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT month(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT month('2020-01-02T03:04:05.000000Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT month('2019-12-31T23:00:00.000000Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT MONTH(TS) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT month(id) FROM sensors ORDER BY month(id)"),
    (os.environ["DIFF_H1"], "SELECT month(ts) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT month(ts) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT day(ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT day(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT day('2020-01-02T03:04:05.000000Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT day('2019-12-31T23:00:00.000000Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT DAY(TS) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT day(id) FROM sensors ORDER BY day(id)"),
    (os.environ["DIFF_H1"], "SELECT day(ts) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT day(ts) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT hour(ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT hour(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT hour('2020-01-02T03:04:05.000000Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT hour('2019-12-31T23:00:00.000000Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT HOUR(TS) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT hour(id) FROM sensors ORDER BY hour(id)"),
    (os.environ["DIFF_H1"], "SELECT hour(ts) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT hour(ts) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT minute(ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT minute(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT minute('2020-01-02T03:04:05.000000Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT minute('2019-12-31T23:59:00.000000Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT MINUTE(TS) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT minute(id) FROM sensors ORDER BY minute(id)"),
    (os.environ["DIFF_H1"], "SELECT minute(ts) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT minute(ts) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT second(ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT second(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT second('2020-01-02T03:04:05.000000Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT second('2019-12-31T23:59:59.000000Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT SECOND(TS) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT second(id) FROM sensors ORDER BY second(id)"),
    (os.environ["DIFF_H1"], "SELECT second(ts) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT second(ts) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT millis(ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT millis(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT millis('2020-01-02T03:04:05.123456Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT millis('2020-01-02T03:04:05.123999Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT MILLIS(TS) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT millis(id) FROM sensors ORDER BY millis(id)"),
    (os.environ["DIFF_H1"], "SELECT millis(ts) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT millis(ts) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT micros(ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT micros(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT micros('2020-01-02T03:04:05.000123Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT micros('2020-01-02T03:04:05.001123Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT MICROS(TS) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT micros(id) FROM sensors ORDER BY micros(id)"),
    (os.environ["DIFF_H1"], "SELECT micros(ts) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT micros(ts) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT day_of_week(ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT day_of_week(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT day_of_week('2020-01-02T03:04:05.000000Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT day_of_week('2020-01-05T00:00:00.000000Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT day_of_week('2020-01-06T00:00:00.000000Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT DAY_OF_WEEK(TS) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT day_of_week(id) FROM sensors ORDER BY day_of_week(id)"),
    (os.environ["DIFF_H1"], "SELECT day_of_week(ts) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT day_of_week(ts) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT week_of_year(ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT week_of_year(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT week_of_year('2020-01-02T03:04:05.000000Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT week_of_year('2019-12-29T00:00:00.000000Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT week_of_year('2020-12-31T00:00:00.000000Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT week_of_year('2021-01-01T00:00:00.000000Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT WEEK_OF_YEAR(TS) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT week_of_year(id) FROM sensors ORDER BY week_of_year(id)"),
    (os.environ["DIFF_H1"], "SELECT week_of_year(ts) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT week_of_year(ts) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT date_trunc('day', ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT date_trunc('year', id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT date_trunc('minute', '2020-01-02T03:04:05.123456Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT date_trunc('week', id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT date_trunc('millennium', ts) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT date_trunc('century', '2008-01-01T04:15:11.123000Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT date_trunc('decade', '2008-01-01T00:00:00.000000Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT date_trunc('quarter', '2020-05-15T12:30:00.000000Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT date_trunc('millisecond', '2020-01-02T03:04:05.123456Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT DATE_TRUNC('day', TS) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT date_trunc('hour', id) FROM sensors ORDER BY date_trunc('hour', id)"),
    (os.environ["DIFF_H1"], "SELECT date_trunc('day', ts) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT date_trunc('day', ts) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT timestamp_floor('d', ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT timestamp_floor('y', id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT timestamp_floor('2h', id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT timestamp_floor('3d', '2020-01-05T12:00:00.000000Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT timestamp_floor('3d', id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT timestamp_floor('2M', '2020-05-15T12:30:00.000000Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT timestamp_floor('2y', '2021-06-01T00:00:00.000000Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT timestamp_floor('2w', ts) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT timestamp_floor('5T', '2020-01-02T03:04:05.123456Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT timestamp_floor('10U', '2020-01-02T03:04:05.123456Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT TIMESTAMP_FLOOR('d', TS) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT timestamp_floor('h', id) FROM sensors ORDER BY timestamp_floor('h', id)"),
    (os.environ["DIFF_H1"], "SELECT timestamp_floor('d', ts) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT timestamp_floor('d', ts) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT timestamp_ceil('d', ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT timestamp_ceil('y', id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT timestamp_ceil('h', id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT timestamp_ceil('w', '2020-01-06T00:00:00.000000Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT timestamp_ceil('M', '2020-05-15T12:30:00.000000Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT timestamp_ceil('T', '2020-01-02T03:04:05.123456Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT timestamp_ceil('U', '2020-01-02T03:04:05.123456Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT TIMESTAMP_CEIL('d', TS) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT timestamp_ceil('h', id) FROM sensors ORDER BY timestamp_ceil('h', id)"),
    (os.environ["DIFF_H1"], "SELECT timestamp_ceil('d', ts) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT timestamp_ceil('d', ts) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT is_leap_year(ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT is_leap_year(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT is_leap_year('2020-01-02T00:00:00.000000Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT is_leap_year('2019-12-31T00:00:00.000000Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT is_leap_year('1900-01-01T00:00:00.000000Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT is_leap_year('2000-01-01T00:00:00.000000Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT is_leap_year(null) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT IS_LEAP_YEAR(TS) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT is_leap_year(ts) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT is_leap_year(ts) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT days_in_month(ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT days_in_month(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT days_in_month('2020-02-01T00:00:00.000000Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT days_in_month('2019-02-01T00:00:00.000000Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT days_in_month('1900-02-01T00:00:00.000000Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT days_in_month('2000-02-01T00:00:00.000000Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT days_in_month('2020-04-15T00:00:00.000000Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT days_in_month(null) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT DAYS_IN_MONTH(TS) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT days_in_month(id) FROM sensors ORDER BY days_in_month(id)"),
    (os.environ["DIFF_H1"], "SELECT days_in_month(ts) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT days_in_month(ts) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT to_str(ts, 'yyyy-MM-dd') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT to_str(id, 'yyyy-MM-dd') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT to_str('2020-01-02T03:04:05.123456Z', 'SSS UUU') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT to_str('2020-01-02T15:07:09.000000Z', 'h:mm a') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT to_str('2020-01-02T00:00:00.000000Z', 'k') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT to_str(ts, 'EEE MMM') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT to_str('2019-12-31T00:00:00.000000Z', 'w ww YYYY') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT to_str(ts, 'yyyy''Q''MM') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT to_str('2020-03-01T00:00:00.000000Z', 'D') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT to_str('2020-01-05T00:00:00.000000Z', 'u') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT to_str('0001-01-01T00:00:00.000000Z', 'G yyyy') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT to_str(null, 'yyyy') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT TO_STR(TS, 'yyyy') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT to_str(id, 'yyyy-MM-dd') FROM sensors ORDER BY to_str(id, 'yyyy-MM-dd')"),
    (os.environ["DIFF_H1"], "SELECT to_str(ts, 'yyyy-MM-dd') FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT to_str(ts, 'yyyy-MM-dd') FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT is_end_of_month(ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT is_end_of_month(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT is_end_of_month('2020-01-31T00:00:00.000000Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT is_end_of_month('2020-02-29T12:00:00.000000Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT is_end_of_month('2020-02-28T00:00:00.000000Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT is_end_of_month('2019-02-28T00:00:00.000000Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT is_end_of_month('2020-04-30T00:00:00.000000Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT is_end_of_month('2020-04-15T00:00:00.000000Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT is_end_of_month(null) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT IS_END_OF_MONTH(TS) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT is_end_of_month(id) FROM sensors ORDER BY is_end_of_month(id)"),
    (os.environ["DIFF_H1"], "SELECT is_end_of_month(ts) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT is_end_of_month(ts) FROM sensors LIMIT 0"),
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
    (os.environ["DIFF_H2"], "SELECT min(id) FROM days"),
    (os.environ["DIFF_H2"], "SELECT min(note) FROM days"),
    (os.environ["DIFF_H2"], "SELECT max(note) FROM days"),
    (os.environ["DIFF_H2"], "SELECT min(id) FROM days WHERE id = NULL"),
    (os.environ["DIFF_H2"], "SELECT location, min(id) FROM days GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H2"], "SELECT location, max(note) FROM days GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H2"], "SELECT sum(id) FROM days"),
    (os.environ["DIFF_H2"], "SELECT sum(ts) FROM days"),
    (os.environ["DIFF_H2"], "SELECT sum(location) FROM days"),
    (os.environ["DIFF_H2"], "SELECT sum(id) FROM days WHERE id = NULL"),
    (os.environ["DIFF_H2"], "SELECT location, sum(id) FROM days GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H2"], "SELECT avg(id) FROM days"),
    (os.environ["DIFF_H2"], "SELECT avg(id) FROM days WHERE id = NULL"),
    (os.environ["DIFF_H2"], "SELECT avg(ts) FROM days"),
    (os.environ["DIFF_H2"], "SELECT location, avg(id) FROM days GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H2"], "SELECT location, avg(id) FROM days GROUP BY location ORDER BY avg(id)"),
    (os.environ["DIFF_H2"], "SELECT first(id) FROM days"),
    (os.environ["DIFF_H2"], "SELECT last(id) FROM days"),
    (os.environ["DIFF_H2"], "SELECT first(note) FROM days"),
    (os.environ["DIFF_H2"], "SELECT last(note) FROM days"),
    (os.environ["DIFF_H2"], "SELECT first(ts) FROM days"),
    (os.environ["DIFF_H2"], "SELECT last(ts) FROM days"),
    (os.environ["DIFF_H2"], "SELECT first(id) FROM days WHERE id = NULL"),
    (os.environ["DIFF_H2"], "SELECT location, first(id) FROM days GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H2"], "SELECT location, last(id) FROM days GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H2"], "SELECT location, last(note) FROM days GROUP BY location ORDER BY last(note)"),
    (os.environ["DIFF_H2"], "SELECT count_distinct(id) FROM days"),
    (os.environ["DIFF_H2"], "SELECT count_distinct(note) FROM days"),
    (os.environ["DIFF_H2"], "SELECT count_distinct(location) FROM days"),
    (os.environ["DIFF_H2"], "SELECT count_distinct(id) FROM days WHERE id = NULL"),
    (os.environ["DIFF_H2"], "SELECT location, count_distinct(id) FROM days GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H2"], "SELECT location, count_distinct(note) FROM days GROUP BY location ORDER BY count_distinct(note)"),
    (os.environ["DIFF_H2"], "SELECT concat(note, location) FROM days"),
    (os.environ["DIFF_H2"], "SELECT concat(location, note) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT length(note) FROM days"),
    (os.environ["DIFF_H2"], "SELECT length(location) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT lower(note) FROM days"),
    (os.environ["DIFF_H2"], "SELECT lower(location) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT upper(note) FROM days"),
    (os.environ["DIFF_H2"], "SELECT upper(location) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT left(note, 2) FROM days"),
    (os.environ["DIFF_H2"], "SELECT right(note, 2) FROM days"),
    (os.environ["DIFF_H2"], "SELECT left(location, 3) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT substring(note, 2, 4) FROM days"),
    (os.environ["DIFF_H2"], "SELECT substring(note, -1, 4) FROM days"),
    (os.environ["DIFF_H2"], "SELECT substring(location, 2, 3) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT trim(note) FROM days"),
    (os.environ["DIFF_H2"], "SELECT trim(location) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT ltrim(note) FROM days"),
    (os.environ["DIFF_H2"], "SELECT rtrim(location) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT replace(note, 'a', 'X') FROM days"),
    (os.environ["DIFF_H2"], "SELECT replace(location, 'a', 'A') FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT starts_with(note, 'al') FROM days"),
    (os.environ["DIFF_H2"], "SELECT starts_with(location, 'mad') FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT strpos(note, 'a') FROM days"),
    (os.environ["DIFF_H2"], "SELECT strpos(location, 'id') FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT split_part(note, 'a', 1) FROM days"),
    (os.environ["DIFF_H2"], "SELECT split_part(location, 'a', 2) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT to_lowercase(note) FROM days"),
    (os.environ["DIFF_H2"], "SELECT to_uppercase(note) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT to_lowercase(location) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT rpad(location, 8, '.') FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT quote_ident(location) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT quote_ident(note) FROM days"),
    (os.environ["DIFF_H2"], "SELECT length_bytes(note) FROM days"),
    (os.environ["DIFF_H2"], "SELECT size_pretty(id) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT abs(id) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT round(id) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT floor(id) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT ceil(id) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT sign(id) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT sqrt(id) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT power(id, 2) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT ln(id) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT log(id) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT exp(id) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT pi() FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT greatest(id, 2) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT least(id, 2) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT round_down(id, 0) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT round_up(id, 0) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT round_half_even(id, 0) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT coalesce(note, 'x') FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT coalesce(id, 0) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT nullif(note, 'x') FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT nullif(id, 2) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT cast(id AS INT) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT cast(note AS STRING) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT dateadd('d', 1, ts) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT datediff('d', ts, '2020-01-05T00:00:00.000000Z') FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT year(ts) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT month(ts) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT day(ts) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT hour(ts) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT minute(ts) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT second(ts) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT millis(ts) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT micros(ts) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT day_of_week(ts) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT week_of_year(ts) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT date_trunc('hour', ts) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT timestamp_floor('h', ts) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT timestamp_ceil('h', ts) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT is_leap_year(ts) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT days_in_month(ts) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT to_str(ts, 'yyyy-MM-dd') FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT is_end_of_month(ts) FROM days ORDER BY ts"),
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
