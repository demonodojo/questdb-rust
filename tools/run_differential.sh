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
  tu_tag "SELECT to_uppercase(tag) FROM sensors"

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
  tl_location "SELECT to_lowercase(location) FROM days ORDER BY ts"
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
