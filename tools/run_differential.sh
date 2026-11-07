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
  em_zero "SELECT is_end_of_month(ts) FROM sensors LIMIT 0" \
  ds_ts "SELECT day_of_week_sunday_first(ts) FROM sensors" \
  ds_id "SELECT day_of_week_sunday_first(id) FROM sensors" \
  ds_sun "SELECT day_of_week_sunday_first('2020-01-05T00:00:00.000000Z') FROM sensors LIMIT 1" \
  ds_mon "SELECT day_of_week_sunday_first('2020-01-06T00:00:00.000000Z') FROM sensors LIMIT 1" \
  ds_sat "SELECT day_of_week_sunday_first('2020-01-04T00:00:00.000000Z') FROM sensors LIMIT 1" \
  ds_null "SELECT day_of_week_sunday_first(null) FROM sensors LIMIT 1" \
  ds_case "SELECT DAY_OF_WEEK_SUNDAY_FIRST(TS) FROM sensors LIMIT 1" \
  ds_order "SELECT day_of_week_sunday_first(id) FROM sensors ORDER BY day_of_week_sunday_first(id)" \
  ds_where "SELECT day_of_week_sunday_first(ts) FROM sensors WHERE id = 1" \
  ds_zero "SELECT day_of_week_sunday_first(ts) FROM sensors LIMIT 0" \
  tt_one "SELECT to_timestamp('1') FROM sensors LIMIT 1" \
  tt_neg "SELECT to_timestamp('-1') FROM sensors LIMIT 1" \
  tt_zero_us "SELECT to_timestamp('0') FROM sensors LIMIT 1" \
  tt_sep "SELECT to_timestamp('1_000') FROM sensors LIMIT 1" \
  tt_suffix "SELECT to_timestamp('1L') FROM sensors LIMIT 1" \
  tt_empty "SELECT to_timestamp('') FROM sensors LIMIT 1" \
  tt_max "SELECT to_timestamp('9223372036854775807') FROM sensors LIMIT 1" \
  tt_min "SELECT to_timestamp('-9223372036854775808') FROM sensors LIMIT 1" \
  tt_note "SELECT to_timestamp(note) FROM sensors" \
  tt_ch "SELECT to_timestamp(ch) FROM sensors" \
  tt_loc "SELECT to_timestamp(location) FROM sensors" \
  tt_null "SELECT to_timestamp(null) FROM sensors LIMIT 1" \
  tt_case "SELECT TO_TIMESTAMP('1') FROM sensors LIMIT 1" \
  tt_order "SELECT to_timestamp(note) FROM sensors ORDER BY to_timestamp(note)" \
  tt_where "SELECT to_timestamp('1') FROM sensors WHERE id = 1" \
  tt_limit "SELECT to_timestamp('1') FROM sensors LIMIT 0" \
  tp_date "SELECT to_timestamp('2020-01-02', 'yyyy-MM-dd') FROM sensors LIMIT 1" \
  tp_loose "SELECT to_timestamp('2020-1-2', 'yyyy-M-d') FROM sensors LIMIT 1" \
  tp_yy "SELECT to_timestamp('20', 'yy') FROM sensors LIMIT 1" \
  tp_yy69 "SELECT to_timestamp('69', 'yy') FROM sensors LIMIT 1" \
  tp_time "SELECT to_timestamp('2020-01-02 15:07:09', 'yyyy-MM-dd HH:mm:ss') FROM sensors LIMIT 1" \
  tp_frac "SELECT to_timestamp('03:04:05.123456', 'HH:mm:ss.SSSUUU') FROM sensors LIMIT 1" \
  tp_mon "SELECT to_timestamp('Jan 2, 2020', 'MMM d, yyyy') FROM sensors LIMIT 1" \
  tp_long "SELECT to_timestamp('January 2, 2020', 'MMMM d, yyyy') FROM sensors LIMIT 1" \
  tp_pm "SELECT to_timestamp('3 PM', 'h a') FROM sensors LIMIT 1" \
  tp_noon "SELECT to_timestamp('12 PM', 'h a') FROM sensors LIMIT 1" \
  tp_mid "SELECT to_timestamp('12 AM', 'h a') FROM sensors LIMIT 1" \
  tp_note "SELECT to_timestamp(note, 'yyyy-MM-dd') FROM sensors" \
  tp_null "SELECT to_timestamp(null, 'yyyy-MM-dd') FROM sensors LIMIT 1" \
  tp_bad "SELECT to_timestamp('nope', 'yyyy-MM-dd') FROM sensors LIMIT 1" \
  tp_case "SELECT TO_TIMESTAMP('2020-01-02', 'yyyy-MM-dd') FROM sensors LIMIT 1" \
  tp_order "SELECT to_timestamp(note, 'yyyy-MM-dd') FROM sensors ORDER BY to_timestamp(note, 'yyyy-MM-dd')" \
  tp_where "SELECT to_timestamp('2020-01-02', 'yyyy-MM-dd') FROM sensors WHERE id = 1" \
  tp_zero "SELECT to_timestamp('2020-01-02', 'yyyy-MM-dd') FROM sensors LIMIT 0" \
  xt_year "SELECT extract(year FROM ts) FROM sensors" \
  xt_id "SELECT extract(year FROM id) FROM sensors" \
  xt_hour "SELECT extract(hour FROM ts) FROM sensors" \
  xt_dow "SELECT extract(dow FROM ts) FROM sensors" \
  xt_sun "SELECT extract(dow FROM '2020-01-05T00:00:00.000000Z') FROM sensors LIMIT 1" \
  xt_iso "SELECT extract(isodow FROM '2020-01-05T00:00:00.000000Z') FROM sensors LIMIT 1" \
  xt_doy "SELECT extract(doy FROM '2020-03-01T00:00:00.000000Z') FROM sensors LIMIT 1" \
  xt_week "SELECT extract(week FROM '2019-12-31T00:00:00.000000Z') FROM sensors LIMIT 1" \
  xt_iy "SELECT extract(isoyear FROM '2019-12-31T00:00:00.000000Z') FROM sensors LIMIT 1" \
  xt_cen "SELECT extract(century FROM '2000-01-01T00:00:00.000000Z') FROM sensors LIMIT 1" \
  xt_dec "SELECT extract(decade FROM '2008-06-01T00:00:00.000000Z') FROM sensors LIMIT 1" \
  xt_mil "SELECT extract(millennium FROM '2000-01-01T00:00:00.000000Z') FROM sensors LIMIT 1" \
  xt_q "SELECT extract(quarter FROM '2020-05-15T00:00:00.000000Z') FROM sensors LIMIT 1" \
  xt_ep "SELECT extract(epoch FROM id) FROM sensors" \
  xt_us "SELECT extract(microseconds FROM '2020-01-02T03:04:05.123456Z') FROM sensors LIMIT 1" \
  xt_ms "SELECT extract(milliseconds FROM '2020-01-02T03:04:05.123456Z') FROM sensors LIMIT 1" \
  xt_ns "SELECT extract(nanoseconds FROM '2020-01-02T03:04:05.123456Z') FROM sensors LIMIT 1" \
  xt_null "SELECT extract(year FROM null) FROM sensors LIMIT 1" \
  xt_str "SELECT extract('YEAR' FROM ts) FROM sensors LIMIT 1" \
  xt_case "SELECT EXTRACT(YEAR FROM TS) FROM sensors LIMIT 1" \
  xt_order "SELECT extract(year FROM id) FROM sensors ORDER BY extract(year FROM id)" \
  xt_where "SELECT extract(year FROM ts) FROM sensors WHERE id = 1" \
  xt_zero "SELECT extract(year FROM ts) FROM sensors LIMIT 0" \
  td_date "SELECT to_date('2020-01-02', 'yyyy-MM-dd') FROM sensors LIMIT 1" \
  td_loose "SELECT to_date('2020-1-2', 'yyyy-M-d') FROM sensors LIMIT 1" \
  td_yy "SELECT to_date('20', 'yy') FROM sensors LIMIT 1" \
  td_yy69 "SELECT to_date('69', 'yy') FROM sensors LIMIT 1" \
  td_time "SELECT to_date('2020-01-02 15:07:09', 'yyyy-MM-dd HH:mm:ss') FROM sensors LIMIT 1" \
  td_frac "SELECT to_date('03:04:05.123', 'HH:mm:ss.SSS') FROM sensors LIMIT 1" \
  td_badu "SELECT to_date('03:04:05.123456', 'HH:mm:ss.SSSUUU') FROM sensors LIMIT 1" \
  td_mon "SELECT to_date('Jan 2, 2020', 'MMM d, yyyy') FROM sensors LIMIT 1" \
  td_pm "SELECT to_date('3 PM', 'h a') FROM sensors LIMIT 1" \
  td_mid "SELECT to_date('12 AM', 'h a') FROM sensors LIMIT 1" \
  td_lit "SELECT to_date('U', 'U') FROM sensors LIMIT 1" \
  td_leap "SELECT to_date('2020-02-29', 'yyyy-MM-dd') FROM sensors LIMIT 1" \
  td_nleap "SELECT to_date('2019-02-29', 'yyyy-MM-dd') FROM sensors LIMIT 1" \
  td_note "SELECT to_date(note, 'yyyy-MM-dd') FROM sensors" \
  td_sym "SELECT to_date(location, 'yyyy-MM-dd') FROM sensors LIMIT 1" \
  td_null "SELECT to_date(null, 'yyyy-MM-dd') FROM sensors LIMIT 1" \
  td_bad "SELECT to_date('nope', 'yyyy-MM-dd') FROM sensors LIMIT 1" \
  td_case "SELECT TO_DATE('2020-01-02', 'yyyy-MM-dd') FROM sensors LIMIT 1" \
  td_order "SELECT to_date(note, 'yyyy-MM-dd') FROM sensors ORDER BY to_date(note, 'yyyy-MM-dd')" \
  td_where "SELECT to_date('2020-01-02', 'yyyy-MM-dd') FROM sensors WHERE id = 1" \
  td_zero "SELECT to_date('2020-01-02', 'yyyy-MM-dd') FROM sensors LIMIT 0" \
  m5_abc "SELECT md5('abc') FROM sensors LIMIT 1" \
  m5_empty "SELECT md5('') FROM sensors LIMIT 1" \
  m5_null "SELECT md5(null) FROM sensors LIMIT 1" \
  m5_note "SELECT md5(note) FROM sensors" \
  m5_sym "SELECT md5(location) FROM sensors LIMIT 1" \
  m5_beta "SELECT md5('β') FROM sensors LIMIT 1" \
  m5_tokyo "SELECT md5('東京') FROM sensors LIMIT 1" \
  m5_case "SELECT MD5('abc') FROM sensors LIMIT 1" \
  m5_order "SELECT md5(note) FROM sensors ORDER BY md5(note)" \
  m5_where "SELECT md5(note) FROM sensors WHERE id = 1" \
  m5_zero "SELECT md5(note) FROM sensors LIMIT 0" \
  s1_abc "SELECT sha1('abc') FROM sensors LIMIT 1" \
  s1_empty "SELECT sha1('') FROM sensors LIMIT 1" \
  s1_null "SELECT sha1(null) FROM sensors LIMIT 1" \
  s1_note "SELECT sha1(note) FROM sensors" \
  s1_sym "SELECT sha1(location) FROM sensors LIMIT 1" \
  s1_beta "SELECT sha1('β') FROM sensors LIMIT 1" \
  s1_tokyo "SELECT sha1('東京') FROM sensors LIMIT 1" \
  s1_case "SELECT SHA1('abc') FROM sensors LIMIT 1" \
  s1_order "SELECT sha1(note) FROM sensors ORDER BY sha1(note)" \
  s1_where "SELECT sha1(note) FROM sensors WHERE id = 1" \
  s1_zero "SELECT sha1(note) FROM sensors LIMIT 0" \
  s2_abc "SELECT sha256('abc') FROM sensors LIMIT 1" \
  s2_empty "SELECT sha256('') FROM sensors LIMIT 1" \
  s2_null "SELECT sha256(null) FROM sensors LIMIT 1" \
  s2_note "SELECT sha256(note) FROM sensors" \
  s2_sym "SELECT sha256(location) FROM sensors LIMIT 1" \
  s2_beta "SELECT sha256('β') FROM sensors LIMIT 1" \
  s2_tokyo "SELECT sha256('東京') FROM sensors LIMIT 1" \
  s2_case "SELECT SHA256('abc') FROM sensors LIMIT 1" \
  s2_order "SELECT sha256(note) FROM sensors ORDER BY sha256(note)" \
  s2_where "SELECT sha256(note) FROM sensors WHERE id = 1" \
  s2_zero "SELECT sha256(note) FROM sensors LIMIT 0" \
  sn_zero_lit "SELECT sin(0) FROM sensors LIMIT 1" \
  sn_one "SELECT sin(1) FROM sensors LIMIT 1" \
  sn_neg "SELECT sin(-1) FROM sensors LIMIT 1" \
  sn_neg0 "SELECT sin(-0.0) FROM sensors LIMIT 1" \
  sn_null "SELECT sin(null) FROM sensors LIMIT 1" \
  sn_id "SELECT sin(id) FROM sensors" \
  sn_read "SELECT sin(reading) FROM sensors" \
  sn_score "SELECT sin(score) FROM sensors LIMIT 1" \
  sn_flags "SELECT sin(flags) FROM sensors LIMIT 1" \
  sn_byte "SELECT sin(b) FROM sensors LIMIT 1" \
  sn_short "SELECT sin(code) FROM sensors LIMIT 1" \
  sn_ts "SELECT sin(ts) FROM sensors LIMIT 1" \
  sn_day "SELECT sin(day) FROM sensors LIMIT 1" \
  sn_text "SELECT sin('1.5') FROM sensors LIMIT 1" \
  sn_ws "SELECT sin(' 1') FROM sensors LIMIT 1" \
  sn_suffix "SELECT sin('1.5d') FROM sensors LIMIT 1" \
  sn_nan "SELECT sin('NaN') FROM sensors LIMIT 1" \
  sn_case "SELECT SIN(0) FROM sensors LIMIT 1" \
  sn_note "SELECT sin(note) FROM sensors WHERE id = 2" \
  sn_where "SELECT sin(id) FROM sensors WHERE id = 1" \
  sn_order "SELECT sin(id) FROM sensors ORDER BY sin(id)" \
  sn_zero "SELECT sin(id) FROM sensors LIMIT 0" \
  sn_ch0 "SELECT sin(ch) FROM sensors LIMIT 0" \
  co_zero_lit "SELECT cos(0) FROM sensors LIMIT 1" \
  co_one "SELECT cos(1) FROM sensors LIMIT 1" \
  co_neg "SELECT cos(-1) FROM sensors LIMIT 1" \
  co_neg0 "SELECT cos(-0.0) FROM sensors LIMIT 1" \
  co_null "SELECT cos(null) FROM sensors LIMIT 1" \
  co_id "SELECT cos(id) FROM sensors" \
  co_read "SELECT cos(reading) FROM sensors" \
  co_score "SELECT cos(score) FROM sensors LIMIT 1" \
  co_flags "SELECT cos(flags) FROM sensors LIMIT 1" \
  co_byte "SELECT cos(b) FROM sensors LIMIT 1" \
  co_short "SELECT cos(code) FROM sensors LIMIT 1" \
  co_ts "SELECT cos(ts) FROM sensors LIMIT 1" \
  co_day "SELECT cos(day) FROM sensors LIMIT 1" \
  co_text "SELECT cos('1.5') FROM sensors LIMIT 1" \
  co_ws "SELECT cos(' 1') FROM sensors LIMIT 1" \
  co_suffix "SELECT cos('1.5d') FROM sensors LIMIT 1" \
  co_nan "SELECT cos('NaN') FROM sensors LIMIT 1" \
  co_case "SELECT COS(0) FROM sensors LIMIT 1" \
  co_note "SELECT cos(note) FROM sensors WHERE id = 2" \
  co_where "SELECT cos(id) FROM sensors WHERE id = 1" \
  co_order "SELECT cos(id) FROM sensors ORDER BY cos(id)" \
  co_zero "SELECT cos(id) FROM sensors LIMIT 0" \
  co_ch0 "SELECT cos(ch) FROM sensors LIMIT 0" \
  tn_zero_lit "SELECT tan(0) FROM sensors LIMIT 1" \
  tn_one "SELECT tan(1) FROM sensors LIMIT 1" \
  tn_neg "SELECT tan(-1) FROM sensors LIMIT 1" \
  tn_neg0 "SELECT tan(-0.0) FROM sensors LIMIT 1" \
  tn_null "SELECT tan(null) FROM sensors LIMIT 1" \
  tn_id "SELECT tan(id) FROM sensors" \
  tn_read "SELECT tan(reading) FROM sensors" \
  tn_score "SELECT tan(score) FROM sensors LIMIT 1" \
  tn_flags "SELECT tan(flags) FROM sensors LIMIT 1" \
  tn_byte "SELECT tan(b) FROM sensors LIMIT 1" \
  tn_ts "SELECT tan(ts) FROM sensors LIMIT 1" \
  tn_day "SELECT tan(day) FROM sensors LIMIT 1" \
  tn_text "SELECT tan('1.5') FROM sensors LIMIT 1" \
  tn_ws "SELECT tan(' 1') FROM sensors LIMIT 1" \
  tn_suffix "SELECT tan('1.5d') FROM sensors LIMIT 1" \
  tn_nan "SELECT tan('NaN') FROM sensors LIMIT 1" \
  tn_case "SELECT TAN(0) FROM sensors LIMIT 1" \
  tn_note "SELECT tan(note) FROM sensors WHERE id = 2" \
  tn_where "SELECT tan(id) FROM sensors WHERE id = 1" \
  tn_order "SELECT tan(id) FROM sensors ORDER BY tan(id)" \
  tn_zero "SELECT tan(id) FROM sensors LIMIT 0" \
  tn_ch0 "SELECT tan(ch) FROM sensors LIMIT 0" \
  an_zero_lit "SELECT asin(0) FROM sensors LIMIT 1" \
  an_one "SELECT asin(1) FROM sensors LIMIT 1" \
  an_neg "SELECT asin(-1) FROM sensors LIMIT 1" \
  an_half "SELECT asin(0.5) FROM sensors LIMIT 1" \
  an_neg0 "SELECT asin(-0.0) FROM sensors LIMIT 1" \
  an_two "SELECT asin(2) FROM sensors LIMIT 1" \
  an_null "SELECT asin(null) FROM sensors LIMIT 1" \
  an_id "SELECT asin(id) FROM sensors" \
  an_read "SELECT asin(reading) FROM sensors" \
  an_byte "SELECT asin(b) FROM sensors" \
  an_ts "SELECT asin(ts) FROM sensors LIMIT 1" \
  an_text "SELECT asin('0.5') FROM sensors LIMIT 1" \
  an_suffix "SELECT asin('1.5d') FROM sensors LIMIT 1" \
  an_ws "SELECT asin(' 1') FROM sensors LIMIT 1" \
  an_nan "SELECT asin('NaN') FROM sensors LIMIT 1" \
  an_case "SELECT ASIN(0) FROM sensors LIMIT 1" \
  an_note "SELECT asin(note) FROM sensors WHERE id = 2" \
  an_where "SELECT asin(id) FROM sensors WHERE id = 1" \
  an_order "SELECT asin(id) FROM sensors ORDER BY asin(id)" \
  an_zero "SELECT asin(id) FROM sensors LIMIT 0" \
  an_ch0 "SELECT asin(ch) FROM sensors LIMIT 0" \
  ac_zero_lit "SELECT acos(0) FROM sensors LIMIT 1" \
  ac_one "SELECT acos(1) FROM sensors LIMIT 1" \
  ac_neg "SELECT acos(-1) FROM sensors LIMIT 1" \
  ac_half "SELECT acos(0.5) FROM sensors LIMIT 1" \
  ac_neg0 "SELECT acos(-0.0) FROM sensors LIMIT 1" \
  ac_two "SELECT acos(2) FROM sensors LIMIT 1" \
  ac_null "SELECT acos(null) FROM sensors LIMIT 1" \
  ac_id "SELECT acos(id) FROM sensors" \
  ac_read "SELECT acos(reading) FROM sensors" \
  ac_byte "SELECT acos(b) FROM sensors" \
  ac_ts "SELECT acos(ts) FROM sensors LIMIT 1" \
  ac_text "SELECT acos('0.5') FROM sensors LIMIT 1" \
  ac_suffix "SELECT acos('1.5d') FROM sensors LIMIT 1" \
  ac_ws "SELECT acos(' 1') FROM sensors LIMIT 1" \
  ac_nan "SELECT acos('NaN') FROM sensors LIMIT 1" \
  ac_case "SELECT ACOS(1) FROM sensors LIMIT 1" \
  ac_note "SELECT acos(note) FROM sensors WHERE id = 2" \
  ac_where "SELECT acos(id) FROM sensors WHERE id = 1" \
  ac_order "SELECT acos(id) FROM sensors ORDER BY acos(id)" \
  ac_zero "SELECT acos(id) FROM sensors LIMIT 0" \
  ac_ch0 "SELECT acos(ch) FROM sensors LIMIT 0" \
  at_zero_lit "SELECT atan(0) FROM sensors LIMIT 1" \
  at_one "SELECT atan(1) FROM sensors LIMIT 1" \
  at_neg "SELECT atan(-1) FROM sensors LIMIT 1" \
  at_half "SELECT atan(0.5) FROM sensors LIMIT 1" \
  at_neg0 "SELECT atan(-0.0) FROM sensors LIMIT 1" \
  at_two "SELECT atan(2) FROM sensors LIMIT 1" \
  at_null "SELECT atan(null) FROM sensors LIMIT 1" \
  at_id "SELECT atan(id) FROM sensors" \
  at_read "SELECT atan(reading) FROM sensors" \
  at_byte "SELECT atan(b) FROM sensors" \
  at_ts "SELECT atan(ts) FROM sensors LIMIT 1" \
  at_text "SELECT atan('0.5') FROM sensors LIMIT 1" \
  at_suffix "SELECT atan('1.5d') FROM sensors LIMIT 1" \
  at_ws "SELECT atan(' 1') FROM sensors LIMIT 1" \
  at_nan "SELECT atan('NaN') FROM sensors LIMIT 1" \
  at_case "SELECT ATAN(1) FROM sensors LIMIT 1" \
  at_note "SELECT atan(note) FROM sensors WHERE id = 2" \
  at_where "SELECT atan(id) FROM sensors WHERE id = 1" \
  at_order "SELECT atan(id) FROM sensors ORDER BY atan(id)" \
  at_zero "SELECT atan(id) FROM sensors LIMIT 0" \
  at_ch0 "SELECT atan(ch) FROM sensors LIMIT 0" \
  a2_zero "SELECT atan2(0, 1) FROM sensors LIMIT 1" \
  a2_pi "SELECT atan2(0, -1) FROM sensors LIMIT 1" \
  a2_neg0 "SELECT atan2(-0.0, 1) FROM sensors LIMIT 1" \
  a2_posx "SELECT atan2(0, -0.0) FROM sensors LIMIT 1" \
  a2_negpi "SELECT atan2(-0.0, -0.0) FROM sensors LIMIT 1" \
  a2_half "SELECT atan2(1, 0) FROM sensors LIMIT 1" \
  a2_nhalf "SELECT atan2(-1, 0) FROM sensors LIMIT 1" \
  a2_origin "SELECT atan2(0, 0) FROM sensors LIMIT 1" \
  a2_quad "SELECT atan2(0.5, 0.5) FROM sensors LIMIT 1" \
  a2_nully "SELECT atan2(null, 1) FROM sensors LIMIT 1" \
  a2_nullx "SELECT atan2(1, null) FROM sensors LIMIT 1" \
  a2_id "SELECT atan2(id, 1) FROM sensors" \
  a2_read "SELECT atan2(reading, id) FROM sensors" \
  a2_ts "SELECT atan2(ts, 1) FROM sensors LIMIT 1" \
  a2_text "SELECT atan2('0.5', '1') FROM sensors LIMIT 1" \
  a2_suffix "SELECT atan2('1.5d', 1) FROM sensors LIMIT 1" \
  a2_ws "SELECT atan2(' 1', 1) FROM sensors LIMIT 1" \
  a2_nan "SELECT atan2('NaN', 1) FROM sensors LIMIT 1" \
  a2_case "SELECT ATAN2(0, 1) FROM sensors LIMIT 1" \
  a2_note "SELECT atan2(note, 1) FROM sensors WHERE id = 2" \
  a2_where "SELECT atan2(id, 1) FROM sensors WHERE id = 1" \
  a2_order "SELECT atan2(id, 1) FROM sensors ORDER BY atan2(id, 1)" \
  a2_lim0 "SELECT atan2(id, 1) FROM sensors LIMIT 0" \
  a2_ch0 "SELECT atan2(ch, 1) FROM sensors LIMIT 0" \
  dg_zero "SELECT degrees(0) FROM sensors LIMIT 1" \
  dg_one "SELECT degrees(1) FROM sensors LIMIT 1" \
  dg_neg "SELECT degrees(-1) FROM sensors LIMIT 1" \
  dg_half "SELECT degrees(0.5) FROM sensors LIMIT 1" \
  dg_neg0 "SELECT degrees(-0.0) FROM sensors LIMIT 1" \
  dg_pi "SELECT degrees(3.141592653589793) FROM sensors LIMIT 1" \
  dg_right "SELECT degrees(1.5707963267948966) FROM sensors LIMIT 1" \
  dg_two "SELECT degrees(2) FROM sensors LIMIT 1" \
  dg_null "SELECT degrees(null) FROM sensors LIMIT 1" \
  dg_id "SELECT degrees(id) FROM sensors" \
  dg_read "SELECT degrees(reading) FROM sensors" \
  dg_score "SELECT degrees(score) FROM sensors" \
  dg_byte "SELECT degrees(b) FROM sensors" \
  dg_short "SELECT degrees(code) FROM sensors" \
  dg_flags "SELECT degrees(flags) FROM sensors" \
  dg_text "SELECT degrees('1') FROM sensors LIMIT 1" \
  dg_suffix "SELECT degrees('1.5d') FROM sensors LIMIT 1" \
  dg_ws "SELECT degrees(' 1') FROM sensors LIMIT 1" \
  dg_nan "SELECT degrees('NaN') FROM sensors LIMIT 1" \
  dg_case "SELECT DEGREES(1) FROM sensors LIMIT 1" \
  dg_note "SELECT degrees(note) FROM sensors WHERE id = 2" \
  dg_where "SELECT degrees(id) FROM sensors WHERE id = 1" \
  dg_order "SELECT degrees(id) FROM sensors ORDER BY degrees(id)" \
  dg_lim0 "SELECT degrees(id) FROM sensors LIMIT 0" \
  dg_ch0 "SELECT degrees(ch) FROM sensors LIMIT 0" \
  ra_zero "SELECT radians(0) FROM sensors LIMIT 1" \
  ra_one "SELECT radians(1) FROM sensors LIMIT 1" \
  ra_neg "SELECT radians(-1) FROM sensors LIMIT 1" \
  ra_half "SELECT radians(0.5) FROM sensors LIMIT 1" \
  ra_neg0 "SELECT radians(-0.0) FROM sensors LIMIT 1" \
  ra_flat "SELECT radians(180) FROM sensors LIMIT 1" \
  ra_right "SELECT radians(90) FROM sensors LIMIT 1" \
  ra_two "SELECT radians(2) FROM sensors LIMIT 1" \
  ra_null "SELECT radians(null) FROM sensors LIMIT 1" \
  ra_id "SELECT radians(id) FROM sensors" \
  ra_read "SELECT radians(reading) FROM sensors" \
  ra_score "SELECT radians(score) FROM sensors" \
  ra_byte "SELECT radians(b) FROM sensors" \
  ra_short "SELECT radians(code) FROM sensors" \
  ra_flags "SELECT radians(flags) FROM sensors" \
  ra_text "SELECT radians('1') FROM sensors LIMIT 1" \
  ra_suffix "SELECT radians('1.5d') FROM sensors LIMIT 1" \
  ra_ws "SELECT radians(' 1') FROM sensors LIMIT 1" \
  ra_nan "SELECT radians('NaN') FROM sensors LIMIT 1" \
  ra_case "SELECT RADIANS(1) FROM sensors LIMIT 1" \
  ra_note "SELECT radians(note) FROM sensors WHERE id = 2" \
  ra_where "SELECT radians(id) FROM sensors WHERE id = 1" \
  ra_order "SELECT radians(id) FROM sensors ORDER BY radians(id)" \
  ra_lim0 "SELECT radians(id) FROM sensors LIMIT 0" \
  ra_ch0 "SELECT radians(ch) FROM sensors LIMIT 0" \
  ct_zero "SELECT cot(0) FROM sensors LIMIT 1" \
  ct_one "SELECT cot(1) FROM sensors LIMIT 1" \
  ct_neg "SELECT cot(-1) FROM sensors LIMIT 1" \
  ct_neg0 "SELECT cot(-0.0) FROM sensors LIMIT 1" \
  ct_half "SELECT cot(0.5) FROM sensors LIMIT 1" \
  ct_two "SELECT cot(2) FROM sensors LIMIT 1" \
  ct_null "SELECT cot(null) FROM sensors LIMIT 1" \
  ct_id "SELECT cot(id) FROM sensors" \
  ct_read "SELECT cot(reading) FROM sensors" \
  ct_score "SELECT cot(score) FROM sensors" \
  ct_byte "SELECT cot(b) FROM sensors" \
  ct_short "SELECT cot(code) FROM sensors" \
  ct_flags "SELECT cot(flags) FROM sensors" \
  ct_ts "SELECT cot(ts) FROM sensors LIMIT 1" \
  ct_text "SELECT cot('1') FROM sensors LIMIT 1" \
  ct_suffix "SELECT cot('1.5d') FROM sensors LIMIT 1" \
  ct_ws "SELECT cot(' 1') FROM sensors LIMIT 1" \
  ct_nan "SELECT cot('NaN') FROM sensors LIMIT 1" \
  ct_case "SELECT COT(1) FROM sensors LIMIT 1" \
  ct_note "SELECT cot(note) FROM sensors WHERE id = 2" \
  ct_where "SELECT cot(id) FROM sensors WHERE id = 1" \
  ct_order "SELECT cot(id) FROM sensors ORDER BY cot(id)" \
  ct_lim0 "SELECT cot(id) FROM sensors LIMIT 0" \
  ct_ch0 "SELECT cot(ch) FROM sensors LIMIT 0" \
  ty_int "SELECT typeof(1) FROM sensors LIMIT 1" \
  ty_imax "SELECT typeof(2147483647) FROM sensors LIMIT 1" \
  ty_long "SELECT typeof(2147483648) FROM sensors LIMIT 1" \
  ty_min "SELECT typeof(-2147483648) FROM sensors LIMIT 1" \
  ty_imin "SELECT typeof(-2147483647) FROM sensors LIMIT 1" \
  ty_dbl "SELECT typeof(1.5) FROM sensors LIMIT 1" \
  ty_null "SELECT typeof(null) FROM sensors LIMIT 1" \
  ty_char "SELECT typeof('A') FROM sensors LIMIT 1" \
  ty_str "SELECT typeof('ab') FROM sensors LIMIT 1" \
  ty_empty "SELECT typeof('') FROM sensors LIMIT 1" \
  ty_id "SELECT typeof(id) FROM sensors" \
  ty_read "SELECT typeof(reading) FROM sensors" \
  ty_score "SELECT typeof(score) FROM sensors" \
  ty_byte "SELECT typeof(b) FROM sensors" \
  ty_short "SELECT typeof(code) FROM sensors" \
  ty_flags "SELECT typeof(flags) FROM sensors" \
  ty_ts "SELECT typeof(ts) FROM sensors" \
  ty_day "SELECT typeof(day) FROM sensors" \
  ty_ok "SELECT typeof(ok) FROM sensors" \
  ty_note "SELECT typeof(note) FROM sensors" \
  ty_loc "SELECT typeof(location) FROM sensors" \
  ty_tag "SELECT typeof(tag) FROM sensors" \
  ty_ch "SELECT typeof(ch) FROM sensors" \
  ty_case "SELECT TYPEOF(1) FROM sensors LIMIT 1" \
  ty_where "SELECT typeof(id) FROM sensors WHERE id = 1" \
  ty_order "SELECT typeof(id) FROM sensors ORDER BY typeof(id)" \
  ty_lim0 "SELECT typeof(id) FROM sensors LIMIT 0" \
  nt_ok "SELECT not(ok) FROM sensors" \
  nt_null "SELECT not(null) FROM sensors LIMIT 1" \
  nt_case "SELECT NOT(ok) FROM sensors LIMIT 1" \
  nt_where "SELECT not(ok) FROM sensors WHERE id = 1" \
  nt_order "SELECT not(ok) FROM sensors ORDER BY not(ok)" \
  nt_desc "SELECT not(ok) FROM sensors ORDER BY not(ok) DESC" \
  nt_lim0 "SELECT not(ok) FROM sensors LIMIT 0" \
  nt_last "SELECT not(1, ok) FROM sensors" \
  nt_ignore "SELECT not(missing, ok) FROM sensors" \
  nt_const "SELECT not(ok, null) FROM sensors" \
  sd_read "SELECT stddev(reading) FROM sensors" \
  sd_id "SELECT stddev(id) FROM sensors" \
  sd_score "SELECT stddev(score) FROM sensors" \
  sd_byte "SELECT stddev(b) FROM sensors" \
  sd_short "SELECT stddev(code) FROM sensors" \
  sd_flags "SELECT stddev(flags) FROM sensors" \
  sd_ts "SELECT stddev(ts) FROM sensors" \
  sd_one "SELECT stddev(reading) FROM sensors WHERE id = 1" \
  sd_none "SELECT stddev(reading) FROM sensors WHERE id > 100" \
  sd_neg "SELECT stddev(reading) FROM sensors WHERE id = -1" \
  sd_case "SELECT STDDEV(reading) FROM sensors" \
  sd_lim0 "SELECT stddev(reading) FROM sensors LIMIT 0" \
  sd_grp "SELECT stddev(reading) FROM sensors GROUP BY location ORDER BY location" \
  sd_grp_id "SELECT stddev(id) FROM sensors GROUP BY location ORDER BY location" \
  sd_where "SELECT stddev(reading) FROM sensors WHERE location = 'madrid'" \
  vr_read "SELECT variance(reading) FROM sensors" \
  vr_id "SELECT variance(id) FROM sensors" \
  vr_score "SELECT variance(score) FROM sensors" \
  vr_byte "SELECT variance(b) FROM sensors" \
  vr_flags "SELECT variance(flags) FROM sensors" \
  vr_one "SELECT variance(reading) FROM sensors WHERE id = 1" \
  vr_none "SELECT variance(reading) FROM sensors WHERE id > 100" \
  vr_neg "SELECT variance(reading) FROM sensors WHERE id = -1" \
  vr_case "SELECT VARIANCE(reading) FROM sensors" \
  vr_lim0 "SELECT variance(reading) FROM sensors LIMIT 0" \
  vr_grp "SELECT variance(reading) FROM sensors GROUP BY location ORDER BY location" \
  vr_grp_id "SELECT variance(id) FROM sensors GROUP BY location ORDER BY location" \
  vr_where "SELECT variance(reading) FROM sensors WHERE location = 'madrid'" \
  vp_read "SELECT var_pop(reading) FROM sensors" \
  vp_id "SELECT var_pop(id) FROM sensors" \
  vp_score "SELECT var_pop(score) FROM sensors" \
  vp_byte "SELECT var_pop(b) FROM sensors" \
  vp_flags "SELECT var_pop(flags) FROM sensors" \
  vp_one "SELECT var_pop(reading) FROM sensors WHERE id = 1" \
  vp_none "SELECT var_pop(reading) FROM sensors WHERE id > 100" \
  vp_neg "SELECT var_pop(reading) FROM sensors WHERE id = -1" \
  vp_case "SELECT VAR_POP(reading) FROM sensors" \
  vp_lim0 "SELECT var_pop(reading) FROM sensors LIMIT 0" \
  vp_grp "SELECT var_pop(reading) FROM sensors GROUP BY location ORDER BY location" \
  vp_grp_id "SELECT var_pop(id) FROM sensors GROUP BY location ORDER BY location" \
  vp_where "SELECT var_pop(reading) FROM sensors WHERE location = 'madrid'" \
  dp_read "SELECT stddev_pop(reading) FROM sensors" \
  dp_id "SELECT stddev_pop(id) FROM sensors" \
  dp_score "SELECT stddev_pop(score) FROM sensors" \
  dp_byte "SELECT stddev_pop(b) FROM sensors" \
  dp_code "SELECT stddev_pop(code) FROM sensors" \
  dp_flags "SELECT stddev_pop(flags) FROM sensors" \
  dp_ts "SELECT stddev_pop(ts) FROM sensors" \
  dp_one "SELECT stddev_pop(reading) FROM sensors WHERE id = 1" \
  dp_none "SELECT stddev_pop(reading) FROM sensors WHERE id > 100" \
  dp_neg "SELECT stddev_pop(reading) FROM sensors WHERE id = -1" \
  dp_case "SELECT STDDEV_POP(reading) FROM sensors" \
  dp_lim0 "SELECT stddev_pop(reading) FROM sensors LIMIT 0" \
  dp_grp "SELECT stddev_pop(reading) FROM sensors GROUP BY location ORDER BY location" \
  dp_grp_id "SELECT stddev_pop(id) FROM sensors GROUP BY location ORDER BY location" \
  dp_where "SELECT stddev_pop(reading) FROM sensors WHERE location = 'madrid'" \
  ks_read "SELECT ksum(reading) FROM sensors" \
  ks_id "SELECT ksum(id) FROM sensors" \
  ks_score "SELECT ksum(score) FROM sensors" \
  ks_byte "SELECT ksum(b) FROM sensors" \
  ks_code "SELECT ksum(code) FROM sensors" \
  ks_flags "SELECT ksum(flags) FROM sensors" \
  ks_one "SELECT ksum(reading) FROM sensors WHERE id = 1" \
  ks_none "SELECT ksum(reading) FROM sensors WHERE id > 100" \
  ks_neg "SELECT ksum(reading) FROM sensors WHERE id = -1" \
  ks_case "SELECT KSUM(reading) FROM sensors" \
  ks_lim0 "SELECT ksum(reading) FROM sensors LIMIT 0" \
  ks_grp "SELECT ksum(reading) FROM sensors GROUP BY location ORDER BY location" \
  ks_grp_id "SELECT ksum(id) FROM sensors GROUP BY location ORDER BY location" \
  ks_where "SELECT ksum(reading) FROM sensors WHERE location = 'madrid'" \
  ns_read "SELECT nsum(reading) FROM sensors" \
  ns_id "SELECT nsum(id) FROM sensors" \
  ns_score "SELECT nsum(score) FROM sensors" \
  ns_byte "SELECT nsum(b) FROM sensors" \
  ns_code "SELECT nsum(code) FROM sensors" \
  ns_flags "SELECT nsum(flags) FROM sensors" \
  ns_one "SELECT nsum(reading) FROM sensors WHERE id = 1" \
  ns_none "SELECT nsum(reading) FROM sensors WHERE id > 100" \
  ns_neg "SELECT nsum(reading) FROM sensors WHERE id = -1" \
  ns_case "SELECT NSUM(reading) FROM sensors" \
  ns_lim0 "SELECT nsum(reading) FROM sensors LIMIT 0" \
  ns_grp "SELECT nsum(reading) FROM sensors GROUP BY location ORDER BY location" \
  ns_grp_id "SELECT nsum(id) FROM sensors GROUP BY location ORDER BY location" \
  ns_where "SELECT nsum(reading) FROM sensors WHERE location = 'madrid'" \
  ap_id "SELECT approx_count_distinct(id) FROM sensors" \
  ap_flags "SELECT approx_count_distinct(flags) FROM sensors" \
  ap_code "SELECT approx_count_distinct(code) FROM sensors" \
  ap_byte "SELECT approx_count_distinct(b) FROM sensors" \
  ap_ts "SELECT approx_count_distinct(ts) FROM sensors" \
  ap_day "SELECT approx_count_distinct(day) FROM sensors" \
  ap_location "SELECT approx_count_distinct(location) FROM sensors" \
  ap_one "SELECT approx_count_distinct(id) FROM sensors WHERE id = 1" \
  ap_none "SELECT approx_count_distinct(id) FROM sensors WHERE id > 100" \
  ap_neg "SELECT approx_count_distinct(id) FROM sensors WHERE id = -1" \
  ap_null "SELECT approx_count_distinct(id) FROM sensors WHERE id = NULL" \
  ap_case "SELECT APPROX_COUNT_DISTINCT(id) FROM sensors" \
  ap_lim0 "SELECT approx_count_distinct(id) FROM sensors LIMIT 0" \
  ap_grp "SELECT approx_count_distinct(id) FROM sensors GROUP BY location ORDER BY location" \
  ap_where "SELECT approx_count_distinct(id) FROM sensors WHERE location = 'madrid'" \
  ap_p14 "SELECT approx_count_distinct(id, 14) FROM sensors" \
  ap_p4 "SELECT approx_count_distinct(id, 4) FROM sensors" \
  sa_note "SELECT string_agg(note, ',') FROM sensors" \
  sa_tag "SELECT string_agg(tag, ',') FROM sensors" \
  sa_location "SELECT string_agg(location, ',') FROM sensors" \
  sa_ch "SELECT string_agg(ch, ',') FROM sensors" \
  sa_one "SELECT string_agg(note, ',') FROM sensors WHERE id = 1" \
  sa_none "SELECT string_agg(note, ',') FROM sensors WHERE id > 100" \
  sa_neg "SELECT string_agg(note, ',') FROM sensors WHERE id = -1" \
  sa_empty "SELECT string_agg(note, ',') FROM sensors WHERE note = ''" \
  sa_case "SELECT STRING_AGG(note, ',') FROM sensors" \
  sa_lim0 "SELECT string_agg(note, ',') FROM sensors LIMIT 0" \
  sa_grp "SELECT string_agg(note, ',') FROM sensors GROUP BY location ORDER BY location" \
  sa_pipe "SELECT string_agg(note, '|') FROM sensors" \
  sa_beta "SELECT string_agg(note, 'β') FROM sensors" \
  sa_nul "SELECT string_agg(note, '') FROM sensors" \
  sa_nullsep "SELECT string_agg(note, null) FROM sensors" \
  cp_read "SELECT covar_pop(reading, score) FROM sensors" \
  cp_id "SELECT covar_pop(reading, id) FROM sensors" \
  cp_flags "SELECT covar_pop(id, flags) FROM sensors" \
  cp_byte "SELECT covar_pop(b, code) FROM sensors" \
  cp_score "SELECT covar_pop(score, flags) FROM sensors" \
  cp_ts "SELECT covar_pop(ts, id) FROM sensors" \
  cp_code "SELECT covar_pop(flags, code) FROM sensors" \
  cp_one "SELECT covar_pop(reading, score) FROM sensors WHERE id = 1" \
  cp_none "SELECT covar_pop(reading, score) FROM sensors WHERE id > 100" \
  cp_neg "SELECT covar_pop(reading, score) FROM sensors WHERE id = -1" \
  cp_null "SELECT covar_pop(reading, score) FROM sensors WHERE id = NULL" \
  cp_case "SELECT COVAR_POP(reading, score) FROM sensors" \
  cp_lim0 "SELECT covar_pop(reading, score) FROM sensors LIMIT 0" \
  cp_grp "SELECT covar_pop(reading, score) FROM sensors GROUP BY location ORDER BY location" \
  cp_grp_id "SELECT covar_pop(reading, id) FROM sensors GROUP BY location ORDER BY location" \
  cp_where "SELECT covar_pop(reading, score) FROM sensors WHERE location = 'madrid'" \
  cm_read "SELECT covar_samp(reading, score) FROM sensors" \
  cm_id "SELECT covar_samp(reading, id) FROM sensors" \
  cm_flags "SELECT covar_samp(id, flags) FROM sensors" \
  cm_byte "SELECT covar_samp(b, code) FROM sensors" \
  cm_score "SELECT covar_samp(score, flags) FROM sensors" \
  cm_ts "SELECT covar_samp(ts, id) FROM sensors" \
  cm_code "SELECT covar_samp(flags, code) FROM sensors" \
  cm_one "SELECT covar_samp(reading, score) FROM sensors WHERE id = 1" \
  cm_none "SELECT covar_samp(reading, score) FROM sensors WHERE id > 100" \
  cm_neg "SELECT covar_samp(reading, score) FROM sensors WHERE id = -1" \
  cm_null "SELECT covar_samp(reading, score) FROM sensors WHERE id = NULL" \
  cm_case "SELECT COVAR_SAMP(reading, score) FROM sensors" \
  cm_lim0 "SELECT covar_samp(reading, score) FROM sensors LIMIT 0" \
  cm_grp "SELECT covar_samp(reading, score) FROM sensors GROUP BY location ORDER BY location" \
  cm_grp_id "SELECT covar_samp(reading, id) FROM sensors GROUP BY location ORDER BY location" \
  cm_where "SELECT covar_samp(reading, score) FROM sensors WHERE location = 'madrid'" \
  cr_read "SELECT corr(reading, score) FROM sensors" \
  cr_id "SELECT corr(reading, id) FROM sensors" \
  cr_flags "SELECT corr(id, flags) FROM sensors" \
  cr_byte "SELECT corr(b, code) FROM sensors" \
  cr_score "SELECT corr(score, flags) FROM sensors" \
  cr_ts "SELECT corr(ts, id) FROM sensors" \
  cr_day "SELECT corr(day, id) FROM sensors" \
  cr_code "SELECT corr(flags, code) FROM sensors" \
  cr_one "SELECT corr(reading, score) FROM sensors WHERE id = 1" \
  cr_none "SELECT corr(reading, score) FROM sensors WHERE id > 100" \
  cr_neg "SELECT corr(reading, score) FROM sensors WHERE id = -1" \
  cr_null "SELECT corr(reading, score) FROM sensors WHERE id = NULL" \
  cr_case "SELECT CORR(reading, score) FROM sensors" \
  cr_lim0 "SELECT corr(reading, score) FROM sensors LIMIT 0" \
  cr_grp "SELECT corr(reading, score) FROM sensors GROUP BY location ORDER BY location" \
  cr_grp_id "SELECT corr(reading, id) FROM sensors GROUP BY location ORDER BY location" \
  cr_where "SELECT corr(reading, score) FROM sensors WHERE location = 'madrid'" \
  rs_read "SELECT regr_slope(reading, score) FROM sensors" \
  rs_id "SELECT regr_slope(reading, id) FROM sensors" \
  rs_flags "SELECT regr_slope(id, flags) FROM sensors" \
  rs_byte "SELECT regr_slope(b, code) FROM sensors" \
  rs_score "SELECT regr_slope(score, flags) FROM sensors" \
  rs_ts "SELECT regr_slope(ts, id) FROM sensors" \
  rs_one "SELECT regr_slope(reading, score) FROM sensors WHERE id = 1" \
  rs_none "SELECT regr_slope(reading, score) FROM sensors WHERE id > 100" \
  rs_neg "SELECT regr_slope(reading, score) FROM sensors WHERE id = -1" \
  rs_null "SELECT regr_slope(reading, score) FROM sensors WHERE id = NULL" \
  rs_case "SELECT REGR_SLOPE(reading, score) FROM sensors" \
  rs_lim0 "SELECT regr_slope(reading, score) FROM sensors LIMIT 0" \
  rs_grp "SELECT regr_slope(reading, score) FROM sensors GROUP BY location ORDER BY location" \
  rs_grp_id "SELECT regr_slope(reading, id) FROM sensors GROUP BY location ORDER BY location" \
  rs_where "SELECT regr_slope(reading, score) FROM sensors WHERE location = 'madrid'" \
  ri_read "SELECT regr_intercept(reading, score) FROM sensors" \
  ri_id "SELECT regr_intercept(reading, id) FROM sensors" \
  ri_flags "SELECT regr_intercept(id, flags) FROM sensors" \
  ri_byte "SELECT regr_intercept(b, code) FROM sensors" \
  ri_score "SELECT regr_intercept(score, flags) FROM sensors" \
  ri_code "SELECT regr_intercept(flags, code) FROM sensors" \
  ri_one "SELECT regr_intercept(reading, score) FROM sensors WHERE id = 1" \
  ri_none "SELECT regr_intercept(reading, score) FROM sensors WHERE id > 100" \
  ri_neg "SELECT regr_intercept(reading, score) FROM sensors WHERE id = -1" \
  ri_null "SELECT regr_intercept(reading, score) FROM sensors WHERE id = NULL" \
  ri_case "SELECT REGR_INTERCEPT(reading, score) FROM sensors" \
  ri_lim0 "SELECT regr_intercept(reading, score) FROM sensors LIMIT 0" \
  ri_grp "SELECT regr_intercept(reading, score) FROM sensors GROUP BY location ORDER BY location" \
  ri_grp_id "SELECT regr_intercept(reading, id) FROM sensors GROUP BY location ORDER BY location" \
  ri_where "SELECT regr_intercept(reading, score) FROM sensors WHERE location = 'madrid'" \
  r2_read "SELECT regr_r2(reading, score) FROM sensors" \
  r2_id "SELECT regr_r2(reading, id) FROM sensors" \
  r2_flags "SELECT regr_r2(id, flags) FROM sensors" \
  r2_byte "SELECT regr_r2(b, code) FROM sensors" \
  r2_score "SELECT regr_r2(score, flags) FROM sensors" \
  r2_ts "SELECT regr_r2(ts, id) FROM sensors" \
  r2_day "SELECT regr_r2(day, id) FROM sensors" \
  r2_code "SELECT regr_r2(flags, code) FROM sensors" \
  r2_one "SELECT regr_r2(reading, score) FROM sensors WHERE id = 1" \
  r2_none "SELECT regr_r2(reading, score) FROM sensors WHERE id > 100" \
  r2_neg "SELECT regr_r2(reading, score) FROM sensors WHERE id = -1" \
  r2_null "SELECT regr_r2(reading, score) FROM sensors WHERE id = NULL" \
  r2_case "SELECT REGR_R2(reading, score) FROM sensors" \
  r2_lim0 "SELECT regr_r2(reading, score) FROM sensors LIMIT 0" \
  r2_grp "SELECT regr_r2(reading, score) FROM sensors GROUP BY location ORDER BY location" \
  r2_grp_id "SELECT regr_r2(reading, id) FROM sensors GROUP BY location ORDER BY location" \
  r2_where "SELECT regr_r2(reading, score) FROM sensors WHERE location = 'madrid'" \
  ss_read "SELECT stddev_samp(reading) FROM sensors" \
  ss_score "SELECT stddev_samp(score) FROM sensors" \
  ss_id "SELECT stddev_samp(id) FROM sensors" \
  ss_flags "SELECT stddev_samp(flags) FROM sensors" \
  ss_byte "SELECT stddev_samp(b) FROM sensors" \
  ss_code "SELECT stddev_samp(code) FROM sensors" \
  ss_ts "SELECT stddev_samp(ts) FROM sensors" \
  ss_one "SELECT stddev_samp(reading) FROM sensors WHERE id = 1" \
  ss_none "SELECT stddev_samp(reading) FROM sensors WHERE id > 100" \
  ss_neg "SELECT stddev_samp(reading) FROM sensors WHERE id = -1" \
  ss_case "SELECT STDDEV_SAMP(reading) FROM sensors" \
  ss_lim0 "SELECT stddev_samp(reading) FROM sensors LIMIT 0" \
  ss_grp "SELECT stddev_samp(reading) FROM sensors GROUP BY location ORDER BY location" \
  vs_read "SELECT var_samp(reading) FROM sensors" \
  vs_score "SELECT var_samp(score) FROM sensors" \
  vs_id "SELECT var_samp(id) FROM sensors" \
  vs_flags "SELECT var_samp(flags) FROM sensors" \
  vs_byte "SELECT var_samp(b) FROM sensors" \
  vs_one "SELECT var_samp(reading) FROM sensors WHERE id = 1" \
  vs_none "SELECT var_samp(reading) FROM sensors WHERE id > 100" \
  vs_neg "SELECT var_samp(reading) FROM sensors WHERE id = -1" \
  vs_case "SELECT VAR_SAMP(reading) FROM sensors" \
  vs_lim0 "SELECT var_samp(reading) FROM sensors LIMIT 0" \
  vs_grp "SELECT var_samp(reading) FROM sensors GROUP BY location ORDER BY location" \
  ba_ok "SELECT bool_and(ok) FROM sensors" \
  ba_one "SELECT bool_and(ok) FROM sensors WHERE id = 1" \
  ba_none "SELECT bool_and(ok) FROM sensors WHERE id > 100" \
  ba_neg "SELECT bool_and(ok) FROM sensors WHERE id = -1" \
  ba_case "SELECT BOOL_AND(ok) FROM sensors" \
  ba_lim0 "SELECT bool_and(ok) FROM sensors LIMIT 0" \
  ba_mad "SELECT bool_and(ok) FROM sensors WHERE location = 'madrid'" \
  ba_grp "SELECT bool_and(ok) FROM sensors GROUP BY location ORDER BY location" \
  bo_ok "SELECT bool_or(ok) FROM sensors" \
  bo_one "SELECT bool_or(ok) FROM sensors WHERE id = 1" \
  bo_none "SELECT bool_or(ok) FROM sensors WHERE id > 100" \
  bo_neg "SELECT bool_or(ok) FROM sensors WHERE id = -1" \
  bo_case "SELECT BOOL_OR(ok) FROM sensors" \
  bo_lim0 "SELECT bool_or(ok) FROM sensors LIMIT 0" \
  bo_mad "SELECT bool_or(ok) FROM sensors WHERE location = 'madrid'" \
  bo_grp "SELECT bool_or(ok) FROM sensors GROUP BY location ORDER BY location" \
  fn_note "SELECT first_not_null(note) FROM sensors" \
  fn_id "SELECT first_not_null(id) FROM sensors" \
  fn_read "SELECT first_not_null(reading) FROM sensors" \
  fn_score "SELECT first_not_null(score) FROM sensors" \
  fn_flags "SELECT first_not_null(flags) FROM sensors" \
  fn_byte "SELECT first_not_null(b) FROM sensors" \
  fn_code "SELECT first_not_null(code) FROM sensors" \
  fn_ch "SELECT first_not_null(ch) FROM sensors" \
  fn_loc "SELECT first_not_null(location) FROM sensors" \
  fn_tag "SELECT first_not_null(tag) FROM sensors" \
  fn_day "SELECT first_not_null(day) FROM sensors" \
  fn_ts "SELECT first_not_null(ts) FROM sensors" \
  fn_empty "SELECT first_not_null(note) FROM sensors WHERE id = NULL" \
  fn_nullid "SELECT first_not_null(id) FROM sensors WHERE id = NULL" \
  fn_none "SELECT first_not_null(note) FROM sensors WHERE id > 100" \
  fn_neg "SELECT first_not_null(note) FROM sensors WHERE id = -1" \
  fn_case "SELECT FIRST_NOT_NULL(note) FROM sensors" \
  fn_lim0 "SELECT first_not_null(note) FROM sensors LIMIT 0" \
  fn_grp "SELECT first_not_null(note) FROM sensors GROUP BY location ORDER BY location" \
  fn_grp_id "SELECT first_not_null(id) FROM sensors GROUP BY location ORDER BY location" \
  lz_note "SELECT last_not_null(note) FROM sensors" \
  lz_id "SELECT last_not_null(id) FROM sensors" \
  lz_read "SELECT last_not_null(reading) FROM sensors" \
  lz_score "SELECT last_not_null(score) FROM sensors" \
  lz_flags "SELECT last_not_null(flags) FROM sensors" \
  lz_byte "SELECT last_not_null(b) FROM sensors" \
  lz_code "SELECT last_not_null(code) FROM sensors" \
  lz_ch "SELECT last_not_null(ch) FROM sensors" \
  lz_loc "SELECT last_not_null(location) FROM sensors" \
  lz_tag "SELECT last_not_null(tag) FROM sensors" \
  lz_day "SELECT last_not_null(day) FROM sensors" \
  lz_ts "SELECT last_not_null(ts) FROM sensors" \
  lz_empty "SELECT last_not_null(note) FROM sensors WHERE id = NULL" \
  lz_nullid "SELECT last_not_null(id) FROM sensors WHERE id = NULL" \
  lz_none "SELECT last_not_null(note) FROM sensors WHERE id > 100" \
  lz_neg "SELECT last_not_null(note) FROM sensors WHERE id = -1" \
  lz_case "SELECT LAST_NOT_NULL(note) FROM sensors" \
  lz_lim0 "SELECT last_not_null(note) FROM sensors LIMIT 0" \
  lz_grp "SELECT last_not_null(note) FROM sensors GROUP BY location ORDER BY location" \
  lz_grp_id "SELECT last_not_null(id) FROM sensors GROUP BY location ORDER BY location" \
  bw_flags "SELECT bit_and(flags) FROM sensors" \
  bw_id "SELECT bit_and(id) FROM sensors" \
  bw_byte "SELECT bit_and(b) FROM sensors" \
  bw_code "SELECT bit_and(code) FROM sensors" \
  bw_one "SELECT bit_and(flags) FROM sensors WHERE id = 1" \
  bw_none "SELECT bit_and(flags) FROM sensors WHERE id > 100" \
  bw_byte0 "SELECT bit_and(b) FROM sensors WHERE id > 100" \
  bw_short0 "SELECT bit_and(code) FROM sensors WHERE id > 100" \
  bw_ch0 "SELECT bit_and(ch) FROM sensors WHERE id > 100" \
  bw_nullid "SELECT bit_and(id) FROM sensors WHERE id = NULL" \
  bw_neg "SELECT bit_and(id) FROM sensors WHERE id = -1" \
  bw_case "SELECT BIT_AND(flags) FROM sensors" \
  bw_lim0 "SELECT bit_and(flags) FROM sensors LIMIT 0" \
  bw_loc "SELECT bit_and(location) FROM sensors" \
  bw_loc_grp "SELECT bit_and(location) FROM sensors GROUP BY location ORDER BY location" \
  bw_day "SELECT bit_and(day) FROM sensors" \
  bw_ts "SELECT bit_and(ts) FROM sensors" \
  bw_grp "SELECT bit_and(flags) FROM sensors GROUP BY location ORDER BY location" \
  bw_grp_id "SELECT bit_and(id) FROM sensors GROUP BY location ORDER BY location" \
  br_flags "SELECT bit_or(flags) FROM sensors" \
  br_id "SELECT bit_or(id) FROM sensors" \
  br_byte "SELECT bit_or(b) FROM sensors" \
  br_code "SELECT bit_or(code) FROM sensors" \
  br_one "SELECT bit_or(flags) FROM sensors WHERE id = 1" \
  br_none "SELECT bit_or(flags) FROM sensors WHERE id > 100" \
  br_byte0 "SELECT bit_or(b) FROM sensors WHERE id > 100" \
  br_short0 "SELECT bit_or(code) FROM sensors WHERE id > 100" \
  br_ch0 "SELECT bit_or(ch) FROM sensors WHERE id > 100" \
  br_nullid "SELECT bit_or(id) FROM sensors WHERE id = NULL" \
  br_neg "SELECT bit_or(id) FROM sensors WHERE id = -1" \
  br_case "SELECT BIT_OR(flags) FROM sensors" \
  br_lim0 "SELECT bit_or(flags) FROM sensors LIMIT 0" \
  br_loc "SELECT bit_or(location) FROM sensors" \
  br_loc_grp "SELECT bit_or(location) FROM sensors GROUP BY location ORDER BY location" \
  br_day "SELECT bit_or(day) FROM sensors" \
  br_ts "SELECT bit_or(ts) FROM sensors" \
  br_grp "SELECT bit_or(flags) FROM sensors GROUP BY location ORDER BY location" \
  br_grp_id "SELECT bit_or(id) FROM sensors GROUP BY location ORDER BY location" \
  bx_flags "SELECT bit_xor(flags) FROM sensors" \
  bx_id "SELECT bit_xor(id) FROM sensors" \
  bx_byte "SELECT bit_xor(b) FROM sensors" \
  bx_code "SELECT bit_xor(code) FROM sensors" \
  bx_one "SELECT bit_xor(flags) FROM sensors WHERE id = 1" \
  bx_none "SELECT bit_xor(flags) FROM sensors WHERE id > 100" \
  bx_byte0 "SELECT bit_xor(b) FROM sensors WHERE id > 100" \
  bx_short0 "SELECT bit_xor(code) FROM sensors WHERE id > 100" \
  bx_ch0 "SELECT bit_xor(ch) FROM sensors WHERE id > 100" \
  bx_nullid "SELECT bit_xor(id) FROM sensors WHERE id = NULL" \
  bx_neg "SELECT bit_xor(id) FROM sensors WHERE id = -1" \
  bx_case "SELECT BIT_XOR(flags) FROM sensors" \
  bx_lim0 "SELECT bit_xor(flags) FROM sensors LIMIT 0" \
  bx_loc "SELECT bit_xor(location) FROM sensors" \
  bx_loc_grp "SELECT bit_xor(location) FROM sensors GROUP BY location ORDER BY location" \
  bx_day "SELECT bit_xor(day) FROM sensors" \
  bx_ts "SELECT bit_xor(ts) FROM sensors" \
  bx_grp "SELECT bit_xor(flags) FROM sensors GROUP BY location ORDER BY location" \
  bx_grp_id "SELECT bit_xor(id) FROM sensors GROUP BY location ORDER BY location" \
  gm_read "SELECT geomean(reading) FROM sensors" \
  gm_one "SELECT geomean(reading) FROM sensors WHERE id = 1" \
  gm_none "SELECT geomean(reading) FROM sensors WHERE id > 100" \
  gm_score "SELECT geomean(score) FROM sensors WHERE id = 1" \
  gm_pos "SELECT geomean(score) FROM sensors WHERE score > 0" \
  gm_flags "SELECT geomean(flags) FROM sensors WHERE flags > 0" \
  gm_byte "SELECT geomean(b) FROM sensors WHERE b > 0" \
  gm_idpos "SELECT geomean(id) FROM sensors WHERE id > 0" \
  gm_grp "SELECT geomean(reading) FROM sensors GROUP BY location ORDER BY location" \
  gm_one_grp "SELECT geomean(score) FROM sensors WHERE id = 1 GROUP BY location ORDER BY location" \
  gm_lim0 "SELECT geomean(reading) FROM sensors LIMIT 0" \
  gm_case "SELECT GEOMEAN(reading) FROM sensors WHERE id = 1" \
  gm_day "SELECT geomean(day) FROM sensors" \
  gm_nullread "SELECT geomean(reading) FROM sensors WHERE id = 2" \
  gm_neg "SELECT geomean(score) FROM sensors WHERE id = 2" \
  gm_flag1 "SELECT geomean(flags) FROM sensors WHERE id = 1" \
  gm_byte1 "SELECT geomean(b) FROM sensors WHERE id = 1" \
  sk_flags "SELECT skewness(flags) FROM sensors" \
  sk_one "SELECT skewness(flags) FROM sensors WHERE id = 1" \
  sk_none "SELECT skewness(flags) FROM sensors WHERE id > 100" \
  sk_read "SELECT skewness(reading) FROM sensors" \
  sk_score "SELECT skewness(score) FROM sensors" \
  sk_byte "SELECT skewness(b) FROM sensors" \
  sk_code "SELECT skewness(code) FROM sensors" \
  sk_id "SELECT skewness(id) FROM sensors" \
  sk_idpos "SELECT skewness(id) FROM sensors WHERE id > 0" \
  sk_flagpos "SELECT skewness(flags) FROM sensors WHERE flags > 0" \
  sk_grp "SELECT skewness(flags) FROM sensors GROUP BY location ORDER BY location" \
  sk_read_grp "SELECT skewness(reading) FROM sensors GROUP BY location ORDER BY location" \
  sk_lim0 "SELECT skewness(flags) FROM sensors LIMIT 0" \
  sk_case "SELECT SKEWNESS(flags) FROM sensors" \
  sk_day "SELECT skewness(day) FROM sensors" \
  sk_ts "SELECT skewness(ts) FROM sensors" \
  me_loc "SELECT mode(location) FROM sensors" \
  me_one "SELECT mode(location) FROM sensors WHERE id = 1" \
  me_none "SELECT mode(location) FROM sensors WHERE id > 100" \
  me_lim0 "SELECT mode(location) FROM sensors LIMIT 0" \
  me_case "SELECT MODE(location) FROM sensors" \
  me_grp "SELECT mode(location) FROM sensors GROUP BY location ORDER BY location" \
  me_flag1 "SELECT mode(flags) FROM sensors WHERE id = 1" \
  me_id "SELECT mode(id) FROM sensors" \
  me_ok "SELECT mode(ok) FROM sensors" \
  me_ch "SELECT mode(ch) FROM sensors" \
  am_flags "SELECT approx_median(flags) FROM sensors" \
  am_one "SELECT approx_median(flags) FROM sensors WHERE id = 1" \
  am_none "SELECT approx_median(flags) FROM sensors WHERE id > 100" \
  am_pos "SELECT approx_median(flags) FROM sensors WHERE flags > 0" \
  am_forty "SELECT approx_median(flags) FROM sensors WHERE id = -1" \
  am_prec0 "SELECT approx_median(flags, 0) FROM sensors WHERE id = -1" \
  am_prec3 "SELECT approx_median(flags, 3) FROM sensors WHERE id = -1" \
  am_read "SELECT approx_median(reading) FROM sensors" \
  am_read1 "SELECT approx_median(reading) FROM sensors WHERE id = 1" \
  am_score1 "SELECT approx_median(score) FROM sensors WHERE id = 1" \
  am_grp "SELECT approx_median(flags) FROM sensors GROUP BY location ORDER BY location" \
  am_read_grp "SELECT approx_median(reading) FROM sensors GROUP BY location ORDER BY location" \
  am_lim0 "SELECT approx_median(flags) FROM sensors LIMIT 0" \
  am_case "SELECT APPROX_MEDIAN(flags) FROM sensors" \
  pc_flags "SELECT approx_percentile(flags, 0.5) FROM sensors" \
  pc_one "SELECT approx_percentile(flags, 0.5) FROM sensors WHERE id = 1" \
  pc_none "SELECT approx_percentile(flags, 0.5) FROM sensors WHERE id > 100" \
  pc_high "SELECT approx_percentile(flags, 1) FROM sensors" \
  pc_low "SELECT approx_percentile(flags, 0) FROM sensors" \
  pc_p25 "SELECT approx_percentile(flags, 0.25) FROM sensors" \
  pc_p75 "SELECT approx_percentile(flags, 0.75) FROM sensors" \
  pc_forty "SELECT approx_percentile(flags, 0.5) FROM sensors WHERE id = -1" \
  pc_prec0 "SELECT approx_percentile(flags, 0.5, 0) FROM sensors WHERE id = -1" \
  pc_prec3 "SELECT approx_percentile(flags, 0.5, 3) FROM sensors WHERE id = -1" \
  pc_prec1 "SELECT approx_percentile(flags, 0.5, 1) FROM sensors" \
  pc_pos "SELECT approx_percentile(flags, 0.5) FROM sensors WHERE flags > 0" \
  pc_read "SELECT approx_percentile(reading, 0.5) FROM sensors" \
  pc_read0 "SELECT approx_percentile(reading, 0) FROM sensors" \
  pc_read1p "SELECT approx_percentile(reading, 1) FROM sensors" \
  pc_read1 "SELECT approx_percentile(reading, 0.5) FROM sensors WHERE id = 1" \
  pc_score1 "SELECT approx_percentile(score, 0.5) FROM sensors WHERE id = 1" \
  pc_byte "SELECT approx_percentile(b, 0.5) FROM sensors WHERE b >= 0" \
  pc_code "SELECT approx_percentile(code, 0.5) FROM sensors WHERE code > 0" \
  pc_grp "SELECT approx_percentile(flags, 0.5) FROM sensors GROUP BY location ORDER BY location" \
  pc_read_grp "SELECT approx_percentile(reading, 0.5) FROM sensors GROUP BY location ORDER BY location" \
  pc_lim0 "SELECT approx_percentile(flags, 0.5) FROM sensors LIMIT 0" \
  pc_case "SELECT APPROX_PERCENTILE(flags, 0.5) FROM sensors" \
  ax_note "SELECT arg_max(note, flags) FROM sensors" \
  ax_one "SELECT arg_max(note, flags) FROM sensors WHERE id = 1" \
  ax_none "SELECT arg_max(note, flags) FROM sensors WHERE id > 100" \
  ax_pos "SELECT arg_max(note, flags) FROM sensors WHERE flags > 0" \
  ax_loc "SELECT arg_max(location, flags) FROM sensors" \
  ax_tag "SELECT arg_max(tag, flags) FROM sensors" \
  ax_ch "SELECT arg_max(ch, flags) FROM sensors" \
  ax_id "SELECT arg_max(id, flags) FROM sensors" \
  ax_byte "SELECT arg_max(b, flags) FROM sensors" \
  ax_code "SELECT arg_max(code, flags) FROM sensors" \
  ax_score "SELECT arg_max(score, flags) FROM sensors" \
  ax_read "SELECT arg_max(reading, flags) FROM sensors" \
  ax_day "SELECT arg_max(day, flags) FROM sensors" \
  ax_ts "SELECT arg_max(ts, flags) FROM sensors" \
  ax_flags_id "SELECT arg_max(flags, id) FROM sensors" \
  ax_note_id "SELECT arg_max(note, id) FROM sensors" \
  ax_note_read "SELECT arg_max(note, reading) FROM sensors" \
  ax_note_score "SELECT arg_max(note, score) FROM sensors" \
  ax_note_ts "SELECT arg_max(note, ts) FROM sensors" \
  ax_note_day "SELECT arg_max(note, day) FROM sensors" \
  ax_note_b "SELECT arg_max(note, b) FROM sensors" \
  ax_note_loc "SELECT arg_max(note, location) FROM sensors" \
  ax_grp "SELECT arg_max(note, flags) FROM sensors GROUP BY location ORDER BY location" \
  ax_lim0 "SELECT arg_max(note, flags) FROM sensors LIMIT 0" \
  ax_case "SELECT ARG_MAX(note, flags) FROM sensors" \
  ax_notecode "SELECT arg_max(note, code) FROM sensors" \
  ax_readmax "SELECT arg_max(reading, reading) FROM sensors" \
  ax_low "SELECT arg_max(note, reading) FROM sensors WHERE reading <= 0" \
  ax_locloc "SELECT arg_max(location, location) FROM sensors" \
  ax_same "SELECT arg_max(id, id) FROM sensors WHERE id = 1" \
  ax_flags_ts "SELECT arg_max(flags, ts) FROM sensors" \
  ax_flags_read "SELECT arg_max(flags, reading) FROM sensors" \
  ax_byte_ts "SELECT arg_max(b, ts) FROM sensors" \
  ax_ch_id "SELECT arg_max(ch, id) FROM sensors" \
  im_read "SELECT arg_min(reading, flags) FROM sensors" \
  im_one "SELECT arg_min(reading, flags) FROM sensors WHERE id = 1" \
  im_none "SELECT arg_min(reading, flags) FROM sensors WHERE id > 100" \
  im_pos "SELECT arg_min(reading, flags) FROM sensors WHERE flags > 0" \
  im_score "SELECT arg_min(score, flags) FROM sensors" \
  im_byte "SELECT arg_min(b, flags) FROM sensors" \
  im_code "SELECT arg_min(code, flags) FROM sensors" \
  im_id "SELECT arg_min(id, flags) FROM sensors" \
  im_flags "SELECT arg_min(flags, flags) FROM sensors" \
  im_flags_id "SELECT arg_min(flags, id) FROM sensors" \
  im_same "SELECT arg_min(id, id) FROM sensors WHERE id = 1" \
  im_id_ts "SELECT arg_min(id, ts) FROM sensors" \
  im_flags_ts "SELECT arg_min(flags, ts) FROM sensors" \
  im_flags_read "SELECT arg_min(flags, reading) FROM sensors" \
  im_byte_ts "SELECT arg_min(b, ts) FROM sensors" \
  im_byte_id "SELECT arg_min(b, id) FROM sensors" \
  im_day "SELECT arg_min(day, flags) FROM sensors" \
  im_ts "SELECT arg_min(ts, flags) FROM sensors" \
  im_ch "SELECT arg_min(ch, flags) FROM sensors" \
  im_ch_id "SELECT arg_min(ch, id) FROM sensors" \
  im_loc "SELECT arg_min(location, flags) FROM sensors" \
  im_loc_id "SELECT arg_min(location, id) FROM sensors" \
  im_loc_ts "SELECT arg_min(location, ts) FROM sensors" \
  im_readmax "SELECT arg_min(reading, reading) FROM sensors" \
  im_grp "SELECT arg_min(reading, flags) FROM sensors GROUP BY location ORDER BY location" \
  im_lim0 "SELECT arg_min(reading, flags) FROM sensors LIMIT 0" \
  im_case "SELECT ARG_MIN(reading, flags) FROM sensors" \
  im_score1 "SELECT arg_min(score, id) FROM sensors WHERE id = 1" \
  im_day_id "SELECT arg_min(day, id) FROM sensors" \
  ud_note "SELECT string_distinct_agg(note, ',') FROM sensors" \
  ud_tag "SELECT string_distinct_agg(tag, ',') FROM sensors" \
  ud_location "SELECT string_distinct_agg(location, ',') FROM sensors" \
  ud_ch "SELECT string_distinct_agg(ch, ',') FROM sensors" \
  ud_one "SELECT string_distinct_agg(note, ',') FROM sensors WHERE id = 1" \
  ud_none "SELECT string_distinct_agg(note, ',') FROM sensors WHERE id > 100" \
  ud_neg "SELECT string_distinct_agg(note, ',') FROM sensors WHERE id = -1" \
  ud_empty "SELECT string_distinct_agg(note, ',') FROM sensors WHERE note = ''" \
  ud_case "SELECT STRING_DISTINCT_AGG(note, ',') FROM sensors" \
  ud_lim0 "SELECT string_distinct_agg(note, ',') FROM sensors LIMIT 0" \
  ud_grp "SELECT string_distinct_agg(note, ',') FROM sensors GROUP BY location ORDER BY location" \
  ud_by_note "SELECT string_distinct_agg(location, ',') FROM sensors GROUP BY note ORDER BY note" \
  ud_pipe "SELECT string_distinct_agg(note, '|') FROM sensors" \
  ud_beta "SELECT string_distinct_agg(note, 'β') FROM sensors" \
  ud_nul "SELECT string_distinct_agg(note, '') FROM sensors" \
  ud_nullsep "SELECT string_distinct_agg(note, null) FROM sensors" \
  ud_madrid "SELECT string_distinct_agg(location, ',') FROM sensors WHERE location = 'madrid'" \
  kp_flags "SELECT kurtosis_pop(flags) FROM sensors" \
  kp_one "SELECT kurtosis_pop(flags) FROM sensors WHERE id = 1" \
  kp_none "SELECT kurtosis_pop(flags) FROM sensors WHERE id > 100" \
  kp_read "SELECT kurtosis_pop(reading) FROM sensors" \
  kp_score "SELECT kurtosis_pop(score) FROM sensors" \
  kp_byte "SELECT kurtosis_pop(b) FROM sensors" \
  kp_code "SELECT kurtosis_pop(code) FROM sensors" \
  kp_id "SELECT kurtosis_pop(id) FROM sensors" \
  kp_idpos "SELECT kurtosis_pop(id) FROM sensors WHERE id > 0" \
  kp_flagpos "SELECT kurtosis_pop(flags) FROM sensors WHERE flags > 0" \
  kp_grp "SELECT kurtosis_pop(flags) FROM sensors GROUP BY location ORDER BY location" \
  kp_read_grp "SELECT kurtosis_pop(reading) FROM sensors GROUP BY location ORDER BY location" \
  kp_lim0 "SELECT kurtosis_pop(flags) FROM sensors LIMIT 0" \
  kp_case "SELECT KURTOSIS_POP(flags) FROM sensors" \
  kp_day "SELECT kurtosis_pop(day) FROM sensors" \
  kp_ts "SELECT kurtosis_pop(ts) FROM sensors" \
  wp_flags "SELECT skewness_pop(flags) FROM sensors" \
  wp_one "SELECT skewness_pop(flags) FROM sensors WHERE id = 1" \
  wp_none "SELECT skewness_pop(flags) FROM sensors WHERE id > 100" \
  wp_read "SELECT skewness_pop(reading) FROM sensors" \
  wp_score "SELECT skewness_pop(score) FROM sensors" \
  wp_byte "SELECT skewness_pop(b) FROM sensors" \
  wp_code "SELECT skewness_pop(code) FROM sensors" \
  wp_id "SELECT skewness_pop(id) FROM sensors" \
  wp_idpos "SELECT skewness_pop(id) FROM sensors WHERE id > 0" \
  wp_flagpos "SELECT skewness_pop(flags) FROM sensors WHERE flags > 0" \
  wp_grp "SELECT skewness_pop(flags) FROM sensors GROUP BY location ORDER BY location" \
  wp_read_grp "SELECT skewness_pop(reading) FROM sensors GROUP BY location ORDER BY location" \
  wp_lim0 "SELECT skewness_pop(flags) FROM sensors LIMIT 0" \
  wp_case "SELECT SKEWNESS_POP(flags) FROM sensors" \
  wp_day "SELECT skewness_pop(day) FROM sensors" \
  wp_ts "SELECT skewness_pop(ts) FROM sensors" \
  vw_read "SELECT vwap(reading, flags) FROM sensors" \
  vw_one "SELECT vwap(reading, flags) FROM sensors WHERE id = 1" \
  vw_none "SELECT vwap(reading, flags) FROM sensors WHERE id > 100" \
  vw_pos "SELECT vwap(reading, flags) FROM sensors WHERE flags > 0" \
  vw_score "SELECT vwap(score, flags) FROM sensors" \
  vw_id "SELECT vwap(id, flags) FROM sensors" \
  vw_byte "SELECT vwap(b, flags) FROM sensors" \
  vw_code "SELECT vwap(code, flags) FROM sensors" \
  vw_read_id "SELECT vwap(reading, id) FROM sensors" \
  vw_flags "SELECT vwap(flags, reading) FROM sensors" \
  vw_grp "SELECT vwap(reading, flags) FROM sensors GROUP BY location ORDER BY location" \
  vw_lim0 "SELECT vwap(reading, flags) FROM sensors LIMIT 0" \
  vw_case "SELECT VWAP(reading, flags) FROM sensors" \
  vw_neg "SELECT vwap(reading, flags) FROM sensors WHERE id = -1" \
  tw_read "SELECT twap(reading, ts) FROM sensors" \
  tw_one "SELECT twap(reading, ts) FROM sensors WHERE id = 1" \
  tw_none "SELECT twap(reading, ts) FROM sensors WHERE id > 100" \
  tw_neg "SELECT twap(reading, ts) FROM sensors WHERE id = -1" \
  tw_score "SELECT twap(score, ts) FROM sensors" \
  tw_id "SELECT twap(id, ts) FROM sensors" \
  tw_byte "SELECT twap(b, ts) FROM sensors" \
  tw_flags "SELECT twap(flags, ts) FROM sensors" \
  tw_grp "SELECT twap(reading, ts) FROM sensors GROUP BY location ORDER BY location" \
  tw_lim0 "SELECT twap(reading, ts) FROM sensors LIMIT 0" \
  tw_case "SELECT TWAP(reading, ts) FROM sensors" \
  sl_flags "SELECT sparkline(flags) FROM sensors" \
  sl_one "SELECT sparkline(flags) FROM sensors WHERE id = 1" \
  sl_none "SELECT sparkline(flags) FROM sensors WHERE id > 100" \
  sl_read "SELECT sparkline(reading) FROM sensors" \
  sl_score "SELECT sparkline(score) FROM sensors" \
  sl_byte "SELECT sparkline(b) FROM sensors" \
  sl_id "SELECT sparkline(id) FROM sensors" \
  sl_code "SELECT sparkline(code) FROM sensors" \
  sl_day "SELECT sparkline(day) FROM sensors" \
  sl_ts "SELECT sparkline(ts) FROM sensors" \
  sl_grp "SELECT sparkline(flags) FROM sensors GROUP BY location ORDER BY location" \
  sl_lim0 "SELECT sparkline(flags) FROM sensors LIMIT 0" \
  sl_case "SELECT SPARKLINE(flags) FROM sensors" \
  sl_width "SELECT sparkline(flags, 0, 42, 2) FROM sensors" \
  sl_wide "SELECT sparkline(flags, 0, 100, 2) FROM sensors" \
  sl_read_w "SELECT sparkline(reading, 0, 1.5, 2) FROM sensors" \
  sl_full "SELECT sparkline(flags, 0, 42, 10) FROM sensors" \
  sl_auto "SELECT sparkline(flags, null, null, 3) FROM sensors" \
  sl_neg "SELECT sparkline(flags) FROM sensors WHERE id = -1" \
  wa_read "SELECT weighted_avg(reading, flags) FROM sensors" \
  wa_one "SELECT weighted_avg(reading, flags) FROM sensors WHERE id = 1" \
  wa_none "SELECT weighted_avg(reading, flags) FROM sensors WHERE id > 100" \
  wa_neg "SELECT weighted_avg(reading, flags) FROM sensors WHERE id = -1" \
  wa_score "SELECT weighted_avg(score, flags) FROM sensors" \
  wa_id "SELECT weighted_avg(id, flags) FROM sensors" \
  wa_byte "SELECT weighted_avg(b, flags) FROM sensors" \
  wa_code "SELECT weighted_avg(code, flags) FROM sensors" \
  wa_flags "SELECT weighted_avg(flags, reading) FROM sensors" \
  wa_read_b "SELECT weighted_avg(reading, b) FROM sensors" \
  wa_flags_b "SELECT weighted_avg(flags, b) FROM sensors" \
  wa_score_b "SELECT weighted_avg(score, b) FROM sensors" \
  wa_cancel "SELECT weighted_avg(reading, id) FROM sensors" \
  wa_grp "SELECT weighted_avg(reading, flags) FROM sensors GROUP BY location ORDER BY location" \
  wa_lim0 "SELECT weighted_avg(reading, flags) FROM sensors LIMIT 0" \
  wa_case "SELECT WEIGHTED_AVG(reading, flags) FROM sensors" \
  wa_pos "SELECT weighted_avg(reading, flags) FROM sensors WHERE flags > 0" \
  wa_zero "SELECT weighted_avg(reading, flags) FROM sensors WHERE flags = 0" \
  wd_read "SELECT weighted_stddev(reading, flags) FROM sensors" \
  wd_one "SELECT weighted_stddev(reading, flags) FROM sensors WHERE id = 1" \
  wd_none "SELECT weighted_stddev(reading, flags) FROM sensors WHERE id > 100" \
  wd_neg "SELECT weighted_stddev(reading, flags) FROM sensors WHERE id = -1" \
  wd_score "SELECT weighted_stddev(score, flags) FROM sensors" \
  wd_id "SELECT weighted_stddev(id, flags) FROM sensors" \
  wd_byte "SELECT weighted_stddev(b, flags) FROM sensors" \
  wd_code "SELECT weighted_stddev(code, flags) FROM sensors" \
  wd_flags "SELECT weighted_stddev(flags, reading) FROM sensors" \
  wd_neg_w "SELECT weighted_stddev(reading, b) FROM sensors" \
  wd_self "SELECT weighted_stddev(flags, flags) FROM sensors" \
  wd_ts "SELECT weighted_stddev(ts, flags) FROM sensors" \
  wd_grp "SELECT weighted_stddev(reading, flags) FROM sensors GROUP BY location ORDER BY location" \
  wd_lim0 "SELECT weighted_stddev(reading, flags) FROM sensors LIMIT 0" \
  wd_case "SELECT WEIGHTED_STDDEV(reading, flags) FROM sensors" \
  wd_pos "SELECT weighted_stddev(reading, flags) FROM sensors WHERE flags > 0" \
  wf_read "SELECT weighted_stddev_freq(reading, flags) FROM sensors" \
  wf_one "SELECT weighted_stddev_freq(reading, flags) FROM sensors WHERE id = 1" \
  wf_none "SELECT weighted_stddev_freq(reading, flags) FROM sensors WHERE id > 100" \
  wf_neg "SELECT weighted_stddev_freq(reading, flags) FROM sensors WHERE id = -1" \
  wf_score "SELECT weighted_stddev_freq(score, flags) FROM sensors" \
  wf_id "SELECT weighted_stddev_freq(id, flags) FROM sensors" \
  wf_byte "SELECT weighted_stddev_freq(b, flags) FROM sensors" \
  wf_code "SELECT weighted_stddev_freq(code, flags) FROM sensors" \
  wf_flags "SELECT weighted_stddev_freq(flags, reading) FROM sensors" \
  wf_neg_w "SELECT weighted_stddev_freq(reading, b) FROM sensors" \
  wf_self "SELECT weighted_stddev_freq(flags, flags) FROM sensors" \
  wf_ts "SELECT weighted_stddev_freq(ts, flags) FROM sensors" \
  wf_grp "SELECT weighted_stddev_freq(reading, flags) FROM sensors GROUP BY location ORDER BY location" \
  wf_lim0 "SELECT weighted_stddev_freq(reading, flags) FROM sensors LIMIT 0" \
  wf_case "SELECT WEIGHTED_STDDEV_FREQ(reading, flags) FROM sensors" \
  wf_pos "SELECT weighted_stddev_freq(reading, flags) FROM sensors WHERE flags > 0" \
  wr_read "SELECT weighted_stddev_rel(reading, flags) FROM sensors" \
  wr_one "SELECT weighted_stddev_rel(reading, flags) FROM sensors WHERE id = 1" \
  wr_none "SELECT weighted_stddev_rel(reading, flags) FROM sensors WHERE id > 100" \
  wr_neg "SELECT weighted_stddev_rel(reading, flags) FROM sensors WHERE id = -1" \
  wr_score "SELECT weighted_stddev_rel(score, flags) FROM sensors" \
  wr_id "SELECT weighted_stddev_rel(id, flags) FROM sensors" \
  wr_byte "SELECT weighted_stddev_rel(b, flags) FROM sensors" \
  wr_code "SELECT weighted_stddev_rel(code, flags) FROM sensors" \
  wr_flags "SELECT weighted_stddev_rel(flags, reading) FROM sensors" \
  wr_neg_w "SELECT weighted_stddev_rel(reading, b) FROM sensors" \
  wr_self "SELECT weighted_stddev_rel(flags, flags) FROM sensors" \
  wr_ts "SELECT weighted_stddev_rel(ts, flags) FROM sensors" \
  wr_grp "SELECT weighted_stddev_rel(reading, flags) FROM sensors GROUP BY location ORDER BY location" \
  wr_lim0 "SELECT weighted_stddev_rel(reading, flags) FROM sensors LIMIT 0" \
  wr_case "SELECT WEIGHTED_STDDEV_REL(reading, flags) FROM sensors" \
  wr_pos "SELECT weighted_stddev_rel(reading, flags) FROM sensors WHERE flags > 0" \
  io_ts "SELECT isOrdered(ts) FROM sensors" \
  io_id "SELECT isOrdered(id) FROM sensors" \
  io_flags "SELECT isOrdered(flags) FROM sensors" \
  io_day "SELECT isOrdered(day) FROM sensors" \
  io_byte "SELECT isOrdered(b) FROM sensors" \
  io_code "SELECT isOrdered(code) FROM sensors" \
  io_one "SELECT isOrdered(id) FROM sensors WHERE id = 1" \
  io_none "SELECT isOrdered(id) FROM sensors WHERE id > 100" \
  io_null "SELECT isOrdered(note) FROM sensors WHERE id = 2" \
  io_flag1 "SELECT isOrdered(flags) FROM sensors WHERE id = 1" \
  io_madrid "SELECT isOrdered(id) FROM sensors WHERE location = 'madrid'" \
  io_day_mad "SELECT isOrdered(day) FROM sensors WHERE location = 'madrid'" \
  io_digit "SELECT isOrdered(ch) FROM sensors WHERE ch = '1'" \
  io_grp "SELECT isOrdered(id) FROM sensors GROUP BY location ORDER BY location" \
  io_grp_ts "SELECT isOrdered(ts) FROM sensors GROUP BY location ORDER BY location" \
  io_lim0 "SELECT isOrdered(id) FROM sensors LIMIT 0" \
  io_case "SELECT ISORDERED(ts) FROM sensors" \
  pd_flags "SELECT percentile_disc(flags, 0.5) FROM sensors" \
  pd_one "SELECT percentile_disc(flags, 0.5) FROM sensors WHERE id = 1" \
  pd_none "SELECT percentile_disc(flags, 0.5) FROM sensors WHERE id > 100" \
  pd_zero "SELECT percentile_disc(flags, 0) FROM sensors" \
  pd_high "SELECT percentile_disc(flags, 1) FROM sensors" \
  pd_neg "SELECT percentile_disc(flags, 0.5) FROM sensors WHERE id = -1" \
  pd_read "SELECT percentile_disc(reading, 0.5) FROM sensors" \
  pd_read0 "SELECT percentile_disc(reading, 0) FROM sensors" \
  pd_read1 "SELECT percentile_disc(reading, 1) FROM sensors" \
  pd_score "SELECT percentile_disc(score, 0.5) FROM sensors" \
  pd_id "SELECT percentile_disc(id, 0.5) FROM sensors" \
  pd_byte "SELECT percentile_disc(b, 0.5) FROM sensors" \
  pd_code "SELECT percentile_disc(code, 0.5) FROM sensors" \
  pd_day "SELECT percentile_disc(day, 0.5) FROM sensors" \
  pd_ts "SELECT percentile_disc(ts, 0.5) FROM sensors" \
  pd_grp "SELECT percentile_disc(flags, 0.5) FROM sensors GROUP BY location ORDER BY location" \
  pd_grp_r "SELECT percentile_disc(reading, 0.5) FROM sensors GROUP BY location ORDER BY location" \
  pd_lim0 "SELECT percentile_disc(flags, 0.5) FROM sensors LIMIT 0" \
  pd_case "SELECT PERCENTILE_DISC(flags, 0.5) FROM sensors" \
  pn_flags "SELECT percentile_cont(flags, 0.5) FROM sensors" \
  pn_quart "SELECT percentile_cont(flags, 0.25) FROM sensors" \
  pn_zero "SELECT percentile_cont(flags, 0) FROM sensors" \
  pn_high "SELECT percentile_cont(flags, 1) FROM sensors" \
  pn_one "SELECT percentile_cont(flags, 0.5) FROM sensors WHERE id = 1" \
  pn_none "SELECT percentile_cont(flags, 0.5) FROM sensors WHERE id > 100" \
  pn_read "SELECT percentile_cont(reading, 0.5) FROM sensors" \
  pn_read0 "SELECT percentile_cont(reading, 0) FROM sensors" \
  pn_read1 "SELECT percentile_cont(reading, 1) FROM sensors" \
  pn_score "SELECT percentile_cont(score, 0.5) FROM sensors" \
  pn_id "SELECT percentile_cont(id, 0.5) FROM sensors" \
  pn_byte "SELECT percentile_cont(b, 0.5) FROM sensors" \
  pn_code "SELECT percentile_cont(code, 0.5) FROM sensors" \
  pn_grp "SELECT percentile_cont(flags, 0.5) FROM sensors GROUP BY location ORDER BY location" \
  pn_grp_r "SELECT percentile_cont(reading, 0.5) FROM sensors GROUP BY location ORDER BY location" \
  pn_lim0 "SELECT percentile_cont(flags, 0.5) FROM sensors LIMIT 0" \
  pn_case "SELECT PERCENTILE_CONT(flags, 0.5) FROM sensors" \
  sy_flags "SELECT skewness_samp(flags) FROM sensors" \
  sy_one "SELECT skewness_samp(flags) FROM sensors WHERE id = 1" \
  sy_none "SELECT skewness_samp(flags) FROM sensors WHERE id > 100" \
  sy_read "SELECT skewness_samp(reading) FROM sensors" \
  sy_score "SELECT skewness_samp(score) FROM sensors" \
  sy_byte "SELECT skewness_samp(b) FROM sensors" \
  sy_code "SELECT skewness_samp(code) FROM sensors" \
  sy_id "SELECT skewness_samp(id) FROM sensors" \
  sy_idpos "SELECT skewness_samp(id) FROM sensors WHERE id > 0" \
  sy_flagpos "SELECT skewness_samp(flags) FROM sensors WHERE flags > 0" \
  sy_grp "SELECT skewness_samp(flags) FROM sensors GROUP BY location ORDER BY location" \
  sy_read_grp "SELECT skewness_samp(reading) FROM sensors GROUP BY location ORDER BY location" \
  sy_lim0 "SELECT skewness_samp(flags) FROM sensors LIMIT 0" \
  sy_day "SELECT skewness_samp(day) FROM sensors" \
  sy_ts "SELECT skewness_samp(ts) FROM sensors" \
  sy_case "SELECT SKEWNESS_SAMP(flags) FROM sensors" \
  qc_flags "SELECT quantile_cont(flags, 0.5) FROM sensors" \
  qc_quart "SELECT quantile_cont(flags, 0.25) FROM sensors" \
  qc_zero "SELECT quantile_cont(flags, 0) FROM sensors" \
  qc_high "SELECT quantile_cont(flags, 1) FROM sensors" \
  qc_one "SELECT quantile_cont(flags, 0.5) FROM sensors WHERE id = 1" \
  qc_none "SELECT quantile_cont(flags, 0.5) FROM sensors WHERE id > 100" \
  qc_read "SELECT quantile_cont(reading, 0.5) FROM sensors" \
  qc_read0 "SELECT quantile_cont(reading, 0) FROM sensors" \
  qc_read1 "SELECT quantile_cont(reading, 1) FROM sensors" \
  qc_score "SELECT quantile_cont(score, 0.5) FROM sensors" \
  qc_id "SELECT quantile_cont(id, 0.5) FROM sensors" \
  qc_byte "SELECT quantile_cont(b, 0.5) FROM sensors" \
  qc_code "SELECT quantile_cont(code, 0.5) FROM sensors" \
  qc_grp "SELECT quantile_cont(flags, 0.5) FROM sensors GROUP BY location ORDER BY location" \
  qc_grp_r "SELECT quantile_cont(reading, 0.5) FROM sensors GROUP BY location ORDER BY location" \
  qc_lim0 "SELECT quantile_cont(flags, 0.5) FROM sensors LIMIT 0" \
  qc_case "SELECT QUANTILE_CONT(flags, 0.5) FROM sensors" \
  qd_flags "SELECT quantile_disc(flags, 0.5) FROM sensors" \
  qd_one "SELECT quantile_disc(flags, 0.5) FROM sensors WHERE id = 1" \
  qd_none "SELECT quantile_disc(flags, 0.5) FROM sensors WHERE id > 100" \
  qd_zero "SELECT quantile_disc(flags, 0) FROM sensors" \
  qd_high "SELECT quantile_disc(flags, 1) FROM sensors" \
  qd_neg "SELECT quantile_disc(flags, 0.5) FROM sensors WHERE id = -1" \
  qd_read "SELECT quantile_disc(reading, 0.5) FROM sensors" \
  qd_read0 "SELECT quantile_disc(reading, 0) FROM sensors" \
  qd_read1 "SELECT quantile_disc(reading, 1) FROM sensors" \
  qd_score "SELECT quantile_disc(score, 0.5) FROM sensors" \
  qd_id "SELECT quantile_disc(id, 0.5) FROM sensors" \
  qd_byte "SELECT quantile_disc(b, 0.5) FROM sensors" \
  qd_code "SELECT quantile_disc(code, 0.5) FROM sensors" \
  qd_day "SELECT quantile_disc(day, 0.5) FROM sensors" \
  qd_ts "SELECT quantile_disc(ts, 0.5) FROM sensors" \
  qd_grp "SELECT quantile_disc(flags, 0.5) FROM sensors GROUP BY location ORDER BY location" \
  qd_grp_r "SELECT quantile_disc(reading, 0.5) FROM sensors GROUP BY location ORDER BY location" \
  qd_lim0 "SELECT quantile_disc(flags, 0.5) FROM sensors LIMIT 0" \
  qd_case "SELECT QUANTILE_DISC(flags, 0.5) FROM sensors" \
  gl_all "SELECT glob(note, 'a*') FROM sensors" \
  gl_one "SELECT glob(note, 'a*') FROM sensors WHERE id = 1" \
  gl_null "SELECT glob(note, 'a*') FROM sensors WHERE id = 2" \
  gl_z "SELECT glob(note, 'z*') FROM sensors WHERE id = 1" \
  gl_empty "SELECT glob(note, '*') FROM sensors WHERE note = ''" \
  gl_q "SELECT glob(note, '?lpha') FROM sensors WHERE id = 1" \
  gl_beta "SELECT glob(note, 'β*') FROM sensors WHERE id = -1" \
  gl_class "SELECT glob(note, '[a-z]*') FROM sensors WHERE id = 1" \
  gl_tag "SELECT glob(tag, 'v*') FROM sensors WHERE id = 1" \
  gl_sym "SELECT glob(location, 'ma*') FROM sensors WHERE id = 1" \
  gl_char "SELECT glob(ch, 'A') FROM sensors WHERE id = 1" \
  gl_lit "SELECT glob('alpha', 'a*') FROM sensors LIMIT 1" \
  gl_case "SELECT GLOB(note, 'a*') FROM sensors WHERE id = 1" \
  gl_none "SELECT glob(note, 'a*') FROM sensors WHERE id > 100" \
  gl_lim0 "SELECT glob(note, 'a*') FROM sensors LIMIT 0" \
  ik_all "SELECT ilike(note, 'A%') FROM sensors" \
  ik_one "SELECT ilike(note, 'A%') FROM sensors WHERE id = 1" \
  ik_null "SELECT ilike(note, 'A%') FROM sensors WHERE id = 2" \
  ik_z "SELECT ilike(note, 'Z%') FROM sensors WHERE id = 1" \
  ik_blank "SELECT ilike(note, '') FROM sensors WHERE id = 1" \
  ik_nullpat "SELECT ilike(note, null) FROM sensors WHERE id = 1" \
  ik_pct "SELECT ilike(note, '%') FROM sensors WHERE note = ''" \
  ik_under "SELECT ilike(note, '_') FROM sensors WHERE id = 1" \
  ik_mid "SELECT ilike(note, 'A_PHA') FROM sensors WHERE id = 1" \
  ik_tag "SELECT ilike(tag, 'v%') FROM sensors WHERE id = 1" \
  ik_sym "SELECT ilike(location, 'M%') FROM sensors WHERE id = 1" \
  ik_char "SELECT ilike(ch, 'a') FROM sensors WHERE id = 1" \
  ik_lit "SELECT ilike('Alpha', 'a%') FROM sensors LIMIT 1" \
  ik_beta "SELECT ilike(note, 'Β%') FROM sensors WHERE id = -1" \
  ik_esc "SELECT ilike(note, 'a\\%') FROM sensors WHERE id = 1" \
  ik_case "SELECT ILIKE(note, 'A%') FROM sensors WHERE id = 1" \
  ik_none "SELECT ilike(note, 'A%') FROM sensors WHERE id > 100" \
  ik_lim0 "SELECT ilike(note, 'A%') FROM sensors LIMIT 0" \
  lk_all "SELECT like(note, 'a%') FROM sensors" \
  lk_one "SELECT like(note, 'a%') FROM sensors WHERE id = 1" \
  lk_upper "SELECT like(note, 'A%') FROM sensors WHERE id = 1" \
  lk_null "SELECT like(note, 'a%') FROM sensors WHERE id = 2" \
  lk_blank "SELECT like(note, '') FROM sensors WHERE id = 1" \
  lk_nullpat "SELECT like(note, null) FROM sensors WHERE id = 1" \
  lk_pct "SELECT like(note, '%') FROM sensors WHERE note = ''" \
  lk_under "SELECT like(note, '_') FROM sensors WHERE id = 1" \
  lk_mid "SELECT like(note, 'a_pha') FROM sensors WHERE id = 1" \
  lk_tag "SELECT like(tag, 'v%') FROM sensors WHERE id = 1" \
  lk_sym "SELECT like(location, 'ma%') FROM sensors WHERE id = 1" \
  lk_char "SELECT like(ch, 'A') FROM sensors WHERE id = 1" \
  lk_lit "SELECT like('Alpha', 'a%') FROM sensors LIMIT 1" \
  lk_beta "SELECT like(note, 'β%') FROM sensors WHERE id = -1" \
  lk_esc "SELECT like(note, 'a\\%') FROM sensors WHERE id = 1" \
  lk_case "SELECT LIKE(note, 'a%') FROM sensors WHERE id = 1" \
  lk_none "SELECT like(note, 'a%') FROM sensors WHERE id > 100" \
  lk_lim0 "SELECT like(note, 'a%') FROM sensors LIMIT 0" \
  sx_all "SELECT switch(id, 1, 'one', 'other') FROM sensors" \
  sx_one "SELECT switch(id, 1, 'one', 'other') FROM sensors WHERE id = 1" \
  sx_noelse "SELECT switch(id, 1, 'one') FROM sensors" \
  sx_two "SELECT switch(id, 1, 'one', 2, 'two', 'other') FROM sensors" \
  sx_note "SELECT switch(note, 'alpha', 7, 0) FROM sensors" \
  sx_sym "SELECT switch(location, 'madrid', 'es', 'other') FROM sensors" \
  sx_case "SELECT SWITCH(id, 1, 'one', 'other') FROM sensors WHERE id = 1" \
  sx_none "SELECT switch(id, 1, 'one', 'other') FROM sensors WHERE id > 100" \
  sx_lim0 "SELECT switch(id, 1, 'one', 'other') FROM sensors LIMIT 0" \
  bn_all "SELECT bar(flags, 0, 42, 4) FROM sensors" \
  bn_read "SELECT bar(reading, 0, 2, 4) FROM sensors WHERE id = 1" \
  bn_reads "SELECT bar(reading, 0, 2, 4) FROM sensors" \
  bn_null "SELECT bar(flags, 0, 42, 4) FROM sensors WHERE id = 2" \
  bn_flat "SELECT bar(flags, 5, 5, 4) FROM sensors WHERE id = 1" \
  bn_zero "SELECT bar(flags, 0, 42, 0) FROM sensors WHERE id = 1" \
  bn_case "SELECT BAR(flags, 0, 42, 4) FROM sensors WHERE id = 1" \
  bn_none "SELECT bar(flags, 0, 42, 4) FROM sensors WHERE id > 100" \
  bn_lim0 "SELECT bar(flags, 0, 42, 4) FROM sensors LIMIT 0" \
  hv_all "SELECT haversine_dist_deg(reading, score, ts) FROM sensors" \
  hv_one "SELECT haversine_dist_deg(reading, score, ts) FROM sensors WHERE id = 1" \
  hv_none "SELECT haversine_dist_deg(reading, score, ts) FROM sensors WHERE id > 100" \
  hv_case "SELECT HAVERSINE_DIST_DEG(reading, score, ts) FROM sensors WHERE id = 1" \
  hv_lim0 "SELECT haversine_dist_deg(reading, score, ts) FROM sensors LIMIT 0"

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
  em_days "SELECT is_end_of_month(ts) FROM days ORDER BY ts" \
  ds_days "SELECT day_of_week_sunday_first(ts) FROM days ORDER BY ts" \
  tt_days "SELECT to_timestamp(note) FROM days ORDER BY ts" \
  tp_days "SELECT to_timestamp(note, 'yyyy-MM-dd') FROM days ORDER BY ts" \
  xt_days "SELECT extract(day FROM ts) FROM days ORDER BY ts" \
  td_days "SELECT to_date(note, 'yyyy-MM-dd') FROM days ORDER BY ts" \
  m5_days "SELECT md5(note) FROM days ORDER BY ts" \
  s1_days "SELECT sha1(note) FROM days ORDER BY ts" \
  s2_days "SELECT sha256(note) FROM days ORDER BY ts" \
  sn_days "SELECT sin(id) FROM days ORDER BY ts" \
  sn_days_ts "SELECT sin(ts) FROM days ORDER BY ts" \
  co_days "SELECT cos(id) FROM days ORDER BY ts" \
  tn_days "SELECT tan(id) FROM days ORDER BY ts" \
  tn_days_ts "SELECT tan(ts) FROM days ORDER BY ts" \
  an_days "SELECT asin(id) FROM days ORDER BY ts" \
  ac_days "SELECT acos(id) FROM days ORDER BY ts" \
  at_days "SELECT atan(id) FROM days ORDER BY ts" \
  a2_days "SELECT atan2(id, 1) FROM days ORDER BY ts" \
  dg_days "SELECT degrees(id) FROM days ORDER BY ts" \
  ra_days "SELECT radians(id) FROM days ORDER BY ts" \
  ct_days "SELECT cot(id) FROM days ORDER BY ts" \
  ct_days_ts "SELECT cot(ts) FROM days ORDER BY ts" \
  ty_days "SELECT typeof(id) FROM days ORDER BY ts" \
  ty_days_note "SELECT typeof(note) FROM days ORDER BY ts" \
  ty_days_loc "SELECT typeof(location) FROM days ORDER BY ts" \
  nt_days "SELECT not(null) FROM days ORDER BY ts" \
  sd_days "SELECT stddev(id) FROM days" \
  sd_days_grp "SELECT stddev(id) FROM days GROUP BY location ORDER BY location" \
  vr_days "SELECT variance(id) FROM days" \
  vr_days_grp "SELECT variance(id) FROM days GROUP BY location ORDER BY location" \
  vp_days "SELECT var_pop(id) FROM days" \
  vp_days_grp "SELECT var_pop(id) FROM days GROUP BY location ORDER BY location" \
  dp_days "SELECT stddev_pop(id) FROM days" \
  dp_days_grp "SELECT stddev_pop(id) FROM days GROUP BY location ORDER BY location" \
  ks_days "SELECT ksum(id) FROM days" \
  ks_days_grp "SELECT ksum(id) FROM days GROUP BY location ORDER BY location" \
  ns_days "SELECT nsum(id) FROM days" \
  ns_days_grp "SELECT nsum(id) FROM days GROUP BY location ORDER BY location" \
  ap_days "SELECT approx_count_distinct(id) FROM days" \
  ap_days_grp "SELECT approx_count_distinct(id) FROM days GROUP BY location ORDER BY location" \
  ap_days_loc "SELECT approx_count_distinct(location) FROM days" \
  ap_days_ts "SELECT approx_count_distinct(ts) FROM days" \
  sa_days "SELECT string_agg(note, ',') FROM days" \
  sa_days_grp "SELECT string_agg(note, ',') FROM days GROUP BY location ORDER BY location" \
  cr_days "SELECT corr(id, ts) FROM days" \
  cr_days_grp "SELECT corr(id, ts) FROM days GROUP BY location ORDER BY location" \
  ri_days "SELECT regr_intercept(id, ts) FROM days" \
  ri_days_grp "SELECT regr_intercept(id, ts) FROM days GROUP BY location ORDER BY location" \
  r2_days "SELECT regr_r2(id, ts) FROM days" \
  r2_days_grp "SELECT regr_r2(id, ts) FROM days GROUP BY location ORDER BY location" \
  ss_days "SELECT stddev_samp(id) FROM days" \
  vs_days "SELECT var_samp(id) FROM days" \
  fn_days "SELECT first_not_null(id) FROM days" \
  fn_days_note "SELECT first_not_null(note) FROM days" \
  fn_days_grp "SELECT first_not_null(note) FROM days GROUP BY location ORDER BY location" \
  lz_days "SELECT last_not_null(id) FROM days" \
  lz_days_note "SELECT last_not_null(note) FROM days" \
  lz_days_grp "SELECT last_not_null(note) FROM days GROUP BY location ORDER BY location" \
  bw_days "SELECT bit_and(id) FROM days" \
  bw_days_grp "SELECT bit_and(id) FROM days GROUP BY location ORDER BY location" \
  br_days "SELECT bit_or(id) FROM days" \
  br_days_grp "SELECT bit_or(id) FROM days GROUP BY location ORDER BY location" \
  bx_days "SELECT bit_xor(id) FROM days" \
  bx_days_grp "SELECT bit_xor(id) FROM days GROUP BY location ORDER BY location" \
  gm_days "SELECT geomean(id) FROM days" \
  gm_days_grp "SELECT geomean(id) FROM days GROUP BY location ORDER BY location" \
  sk_days "SELECT skewness(id) FROM days" \
  sk_days_grp "SELECT skewness(id) FROM days GROUP BY location ORDER BY location" \
  me_days_loc "SELECT mode(location) FROM days" \
  me_days_id "SELECT mode(id) FROM days" \
  me_days_grp "SELECT mode(id) FROM days GROUP BY location ORDER BY location" \
  me_days_loc_grp "SELECT mode(location) FROM days GROUP BY location ORDER BY location" \
  am_days "SELECT approx_median(id) FROM days" \
  am_days_grp "SELECT approx_median(id) FROM days GROUP BY location ORDER BY location" \
  pc_days "SELECT approx_percentile(id, 0.5) FROM days" \
  pc_days0 "SELECT approx_percentile(id, 0) FROM days" \
  pc_days1 "SELECT approx_percentile(id, 1) FROM days" \
  pc_days_grp "SELECT approx_percentile(id, 0.5) FROM days GROUP BY location ORDER BY location" \
  ax_days "SELECT arg_max(note, id) FROM days" \
  ax_days_loc "SELECT arg_max(location, id) FROM days" \
  ax_days_grp "SELECT arg_max(note, id) FROM days GROUP BY location ORDER BY location" \
  ax_days_ts "SELECT arg_max(ts, id) FROM days" \
  im_days "SELECT arg_min(id, id) FROM days" \
  im_days_loc "SELECT arg_min(location, id) FROM days" \
  im_days_grp "SELECT arg_min(id, id) FROM days GROUP BY location ORDER BY location" \
  im_days_ts "SELECT arg_min(ts, id) FROM days" \
  ud_days "SELECT string_distinct_agg(note, ',') FROM days" \
  ud_days_loc "SELECT string_distinct_agg(location, ',') FROM days" \
  ud_days_grp "SELECT string_distinct_agg(note, ',') FROM days GROUP BY location ORDER BY location" \
  ud_days_id "SELECT string_distinct_agg(location, ',') FROM days GROUP BY id ORDER BY id" \
  kp_days "SELECT kurtosis_pop(id) FROM days" \
  kp_days_grp "SELECT kurtosis_pop(id) FROM days GROUP BY location ORDER BY location" \
  wp_days "SELECT skewness_pop(id) FROM days" \
  wp_days_grp "SELECT skewness_pop(id) FROM days GROUP BY location ORDER BY location" \
  vw_days "SELECT vwap(id, id) FROM days" \
  vw_days_grp "SELECT vwap(id, id) FROM days GROUP BY location ORDER BY location" \
  tw_days "SELECT twap(id, ts) FROM days" \
  tw_days_grp "SELECT twap(id, ts) FROM days GROUP BY location ORDER BY location" \
  sl_days "SELECT sparkline(id) FROM days" \
  sl_days_grp "SELECT sparkline(id) FROM days GROUP BY location ORDER BY location" \
  wa_days "SELECT weighted_avg(id, id) FROM days" \
  wa_days_grp "SELECT weighted_avg(id, id) FROM days GROUP BY location ORDER BY location" \
  wd_days "SELECT weighted_stddev(id, id) FROM days" \
  wd_days_grp "SELECT weighted_stddev(id, id) FROM days GROUP BY location ORDER BY location" \
  wf_days "SELECT weighted_stddev_freq(id, id) FROM days" \
  wf_days_grp "SELECT weighted_stddev_freq(id, id) FROM days GROUP BY location ORDER BY location" \
  wr_days "SELECT weighted_stddev_rel(id, id) FROM days" \
  wr_days_grp "SELECT weighted_stddev_rel(id, id) FROM days GROUP BY location ORDER BY location" \
  io_days "SELECT isOrdered(id) FROM days" \
  io_days_ts "SELECT isOrdered(ts) FROM days" \
  io_days_grp "SELECT isOrdered(id) FROM days GROUP BY location ORDER BY location" \
  pd_days "SELECT percentile_disc(id, 0.5) FROM days" \
  pd_days_grp "SELECT percentile_disc(id, 0.5) FROM days GROUP BY location ORDER BY location" \
  pn_days "SELECT percentile_cont(id, 0.5) FROM days" \
  pn_days_grp "SELECT percentile_cont(id, 0.5) FROM days GROUP BY location ORDER BY location" \
  sy_days "SELECT skewness_samp(id) FROM days" \
  sy_days_grp "SELECT skewness_samp(id) FROM days GROUP BY location ORDER BY location" \
  qc_days "SELECT quantile_cont(id, 0.5) FROM days" \
  qc_days_grp "SELECT quantile_cont(id, 0.5) FROM days GROUP BY location ORDER BY location" \
  qd_days "SELECT quantile_disc(id, 0.5) FROM days" \
  qd_days_grp "SELECT quantile_disc(id, 0.5) FROM days GROUP BY location ORDER BY location" \
  gl_days "SELECT glob(note, 'a*') FROM days" \
  gl_days_one "SELECT glob(note, 'a*') FROM days WHERE id = 1" \
  ik_days "SELECT ilike(note, 'A%') FROM days" \
  ik_days_one "SELECT ilike(note, 'A%') FROM days WHERE id = 1" \
  lk_days "SELECT like(note, 'a%') FROM days" \
  lk_days_one "SELECT like(note, 'a%') FROM days WHERE id = 1" \
  sx_days "SELECT switch(id, 1, 'one', 'other') FROM days" \
  sx_days_one "SELECT switch(id, 1, 'one', 'other') FROM days WHERE id = 1" \
  bn_days "SELECT bar(id, 0, 3, 3) FROM days" \
  bn_days_one "SELECT bar(id, 0, 3, 3) FROM days WHERE id = 1" \
  hv_days "SELECT haversine_dist_deg(id, id, ts) FROM days" \
  hv_days_one "SELECT haversine_dist_deg(id, id, ts) FROM days WHERE id = 1"
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
    (os.environ["DIFF_H1"], "SELECT day_of_week_sunday_first(ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT day_of_week_sunday_first(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT day_of_week_sunday_first('2020-01-05T00:00:00.000000Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT day_of_week_sunday_first('2020-01-06T00:00:00.000000Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT day_of_week_sunday_first('2020-01-04T00:00:00.000000Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT day_of_week_sunday_first(null) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT DAY_OF_WEEK_SUNDAY_FIRST(TS) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT day_of_week_sunday_first(id) FROM sensors ORDER BY day_of_week_sunday_first(id)"),
    (os.environ["DIFF_H1"], "SELECT day_of_week_sunday_first(ts) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT day_of_week_sunday_first(ts) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT to_timestamp('1') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT to_timestamp('-1') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT to_timestamp('0') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT to_timestamp('1_000') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT to_timestamp('1L') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT to_timestamp('') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT to_timestamp('9223372036854775807') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT to_timestamp('-9223372036854775808') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT to_timestamp(note) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT to_timestamp(ch) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT to_timestamp(location) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT to_timestamp(null) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT TO_TIMESTAMP('1') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT to_timestamp(note) FROM sensors ORDER BY to_timestamp(note)"),
    (os.environ["DIFF_H1"], "SELECT to_timestamp('1') FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT to_timestamp('1') FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT to_timestamp('2020-01-02', 'yyyy-MM-dd') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT to_timestamp('2020-1-2', 'yyyy-M-d') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT to_timestamp('20', 'yy') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT to_timestamp('69', 'yy') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT to_timestamp('2020-01-02 15:07:09', 'yyyy-MM-dd HH:mm:ss') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT to_timestamp('03:04:05.123456', 'HH:mm:ss.SSSUUU') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT to_timestamp('Jan 2, 2020', 'MMM d, yyyy') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT to_timestamp('January 2, 2020', 'MMMM d, yyyy') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT to_timestamp('3 PM', 'h a') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT to_timestamp('12 PM', 'h a') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT to_timestamp('12 AM', 'h a') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT to_timestamp(note, 'yyyy-MM-dd') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT to_timestamp(null, 'yyyy-MM-dd') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT to_timestamp('nope', 'yyyy-MM-dd') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT TO_TIMESTAMP('2020-01-02', 'yyyy-MM-dd') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT to_timestamp(note, 'yyyy-MM-dd') FROM sensors ORDER BY to_timestamp(note, 'yyyy-MM-dd')"),
    (os.environ["DIFF_H1"], "SELECT to_timestamp('2020-01-02', 'yyyy-MM-dd') FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT to_timestamp('2020-01-02', 'yyyy-MM-dd') FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT extract(year FROM ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT extract(year FROM id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT extract(hour FROM ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT extract(dow FROM ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT extract(dow FROM '2020-01-05T00:00:00.000000Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT extract(isodow FROM '2020-01-05T00:00:00.000000Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT extract(doy FROM '2020-03-01T00:00:00.000000Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT extract(week FROM '2019-12-31T00:00:00.000000Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT extract(isoyear FROM '2019-12-31T00:00:00.000000Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT extract(century FROM '2000-01-01T00:00:00.000000Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT extract(decade FROM '2008-06-01T00:00:00.000000Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT extract(millennium FROM '2000-01-01T00:00:00.000000Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT extract(quarter FROM '2020-05-15T00:00:00.000000Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT extract(epoch FROM id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT extract(microseconds FROM '2020-01-02T03:04:05.123456Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT extract(milliseconds FROM '2020-01-02T03:04:05.123456Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT extract(nanoseconds FROM '2020-01-02T03:04:05.123456Z') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT extract(year FROM null) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT extract('YEAR' FROM ts) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT EXTRACT(YEAR FROM TS) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT extract(year FROM id) FROM sensors ORDER BY extract(year FROM id)"),
    (os.environ["DIFF_H1"], "SELECT extract(year FROM ts) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT extract(year FROM ts) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT to_date('2020-01-02', 'yyyy-MM-dd') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT to_date('2020-1-2', 'yyyy-M-d') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT to_date('20', 'yy') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT to_date('69', 'yy') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT to_date('2020-01-02 15:07:09', 'yyyy-MM-dd HH:mm:ss') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT to_date('03:04:05.123', 'HH:mm:ss.SSS') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT to_date('03:04:05.123456', 'HH:mm:ss.SSSUUU') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT to_date('Jan 2, 2020', 'MMM d, yyyy') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT to_date('3 PM', 'h a') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT to_date('12 AM', 'h a') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT to_date('U', 'U') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT to_date('2020-02-29', 'yyyy-MM-dd') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT to_date('2019-02-29', 'yyyy-MM-dd') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT to_date(note, 'yyyy-MM-dd') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT to_date(location, 'yyyy-MM-dd') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT to_date(null, 'yyyy-MM-dd') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT to_date('nope', 'yyyy-MM-dd') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT TO_DATE('2020-01-02', 'yyyy-MM-dd') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT to_date(note, 'yyyy-MM-dd') FROM sensors ORDER BY to_date(note, 'yyyy-MM-dd')"),
    (os.environ["DIFF_H1"], "SELECT to_date('2020-01-02', 'yyyy-MM-dd') FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT to_date('2020-01-02', 'yyyy-MM-dd') FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT md5('abc') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT md5('') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT md5(null) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT md5(note) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT md5(location) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT md5('β') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT md5('東京') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT MD5('abc') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT md5(note) FROM sensors ORDER BY md5(note)"),
    (os.environ["DIFF_H1"], "SELECT md5(note) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT md5(note) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT sha1('abc') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT sha1('') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT sha1(null) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT sha1(note) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT sha1(location) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT sha1('β') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT sha1('東京') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT SHA1('abc') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT sha1(note) FROM sensors ORDER BY sha1(note)"),
    (os.environ["DIFF_H1"], "SELECT sha1(note) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT sha1(note) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT sha256('abc') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT sha256('') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT sha256(null) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT sha256(note) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT sha256(location) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT sha256('β') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT sha256('東京') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT SHA256('abc') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT sha256(note) FROM sensors ORDER BY sha256(note)"),
    (os.environ["DIFF_H1"], "SELECT sha256(note) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT sha256(note) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT sin(0) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT sin(1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT sin(-1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT sin(-0.0) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT sin(null) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT sin(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT sin(reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT sin(score) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT sin(flags) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT sin(b) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT sin(code) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT sin(ts) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT sin(day) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT sin('1.5') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT sin(' 1') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT sin('1.5d') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT sin('NaN') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT SIN(0) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT sin(note) FROM sensors WHERE id = 2"),
    (os.environ["DIFF_H1"], "SELECT sin(id) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT sin(id) FROM sensors ORDER BY sin(id)"),
    (os.environ["DIFF_H1"], "SELECT sin(id) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT sin(ch) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT cos(0) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT cos(1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT cos(-1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT cos(-0.0) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT cos(null) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT cos(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT cos(reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT cos(score) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT cos(flags) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT cos(b) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT cos(code) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT cos(ts) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT cos(day) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT cos('1.5') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT cos(' 1') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT cos('1.5d') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT cos('NaN') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT COS(0) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT cos(note) FROM sensors WHERE id = 2"),
    (os.environ["DIFF_H1"], "SELECT cos(id) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT cos(id) FROM sensors ORDER BY cos(id)"),
    (os.environ["DIFF_H1"], "SELECT cos(id) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT cos(ch) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT tan(0) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT tan(1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT tan(-1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT tan(-0.0) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT tan(null) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT tan(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT tan(reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT tan(score) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT tan(flags) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT tan(b) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT tan(ts) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT tan(day) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT tan('1.5') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT tan(' 1') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT tan('1.5d') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT tan('NaN') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT TAN(0) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT tan(note) FROM sensors WHERE id = 2"),
    (os.environ["DIFF_H1"], "SELECT tan(id) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT tan(id) FROM sensors ORDER BY tan(id)"),
    (os.environ["DIFF_H1"], "SELECT tan(id) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT tan(ch) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT asin(0) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT asin(1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT asin(-1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT asin(0.5) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT asin(-0.0) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT asin(2) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT asin(null) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT asin(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT asin(reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT asin(b) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT asin(ts) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT asin('0.5') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT asin('1.5d') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT asin(' 1') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT asin('NaN') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT ASIN(0) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT asin(note) FROM sensors WHERE id = 2"),
    (os.environ["DIFF_H1"], "SELECT asin(id) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT asin(id) FROM sensors ORDER BY asin(id)"),
    (os.environ["DIFF_H1"], "SELECT asin(id) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT asin(ch) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT acos(0) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT acos(1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT acos(-1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT acos(0.5) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT acos(-0.0) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT acos(2) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT acos(null) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT acos(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT acos(reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT acos(b) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT acos(ts) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT acos('0.5') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT acos('1.5d') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT acos(' 1') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT acos('NaN') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT ACOS(1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT acos(note) FROM sensors WHERE id = 2"),
    (os.environ["DIFF_H1"], "SELECT acos(id) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT acos(id) FROM sensors ORDER BY acos(id)"),
    (os.environ["DIFF_H1"], "SELECT acos(id) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT acos(ch) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT atan(0) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT atan(1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT atan(-1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT atan(0.5) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT atan(-0.0) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT atan(2) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT atan(null) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT atan(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT atan(reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT atan(b) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT atan(ts) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT atan('0.5') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT atan('1.5d') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT atan(' 1') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT atan('NaN') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT ATAN(1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT atan(note) FROM sensors WHERE id = 2"),
    (os.environ["DIFF_H1"], "SELECT atan(id) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT atan(id) FROM sensors ORDER BY atan(id)"),
    (os.environ["DIFF_H1"], "SELECT atan(id) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT atan(ch) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT atan2(0, 1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT atan2(0, -1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT atan2(-0.0, 1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT atan2(0, -0.0) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT atan2(-0.0, -0.0) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT atan2(1, 0) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT atan2(-1, 0) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT atan2(0, 0) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT atan2(0.5, 0.5) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT atan2(null, 1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT atan2(1, null) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT atan2(id, 1) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT atan2(reading, id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT atan2(ts, 1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT atan2('0.5', '1') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT atan2('1.5d', 1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT atan2(' 1', 1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT atan2('NaN', 1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT ATAN2(0, 1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT atan2(note, 1) FROM sensors WHERE id = 2"),
    (os.environ["DIFF_H1"], "SELECT atan2(id, 1) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT atan2(id, 1) FROM sensors ORDER BY atan2(id, 1)"),
    (os.environ["DIFF_H1"], "SELECT atan2(id, 1) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT atan2(ch, 1) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT degrees(0) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT degrees(1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT degrees(-1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT degrees(0.5) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT degrees(-0.0) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT degrees(3.141592653589793) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT degrees(1.5707963267948966) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT degrees(2) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT degrees(null) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT degrees(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT degrees(reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT degrees(score) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT degrees(b) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT degrees(code) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT degrees(flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT degrees('1') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT degrees('1.5d') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT degrees(' 1') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT degrees('NaN') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT DEGREES(1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT degrees(note) FROM sensors WHERE id = 2"),
    (os.environ["DIFF_H1"], "SELECT degrees(id) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT degrees(id) FROM sensors ORDER BY degrees(id)"),
    (os.environ["DIFF_H1"], "SELECT degrees(id) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT degrees(ch) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT radians(0) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT radians(1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT radians(-1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT radians(0.5) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT radians(-0.0) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT radians(180) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT radians(90) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT radians(2) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT radians(null) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT radians(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT radians(reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT radians(score) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT radians(b) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT radians(code) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT radians(flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT radians('1') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT radians('1.5d') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT radians(' 1') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT radians('NaN') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT RADIANS(1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT radians(note) FROM sensors WHERE id = 2"),
    (os.environ["DIFF_H1"], "SELECT radians(id) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT radians(id) FROM sensors ORDER BY radians(id)"),
    (os.environ["DIFF_H1"], "SELECT radians(id) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT radians(ch) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT cot(0) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT cot(1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT cot(-1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT cot(-0.0) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT cot(0.5) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT cot(2) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT cot(null) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT cot(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT cot(reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT cot(score) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT cot(b) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT cot(code) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT cot(flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT cot(ts) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT cot('1') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT cot('1.5d') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT cot(' 1') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT cot('NaN') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT COT(1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT cot(note) FROM sensors WHERE id = 2"),
    (os.environ["DIFF_H1"], "SELECT cot(id) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT cot(id) FROM sensors ORDER BY cot(id)"),
    (os.environ["DIFF_H1"], "SELECT cot(id) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT cot(ch) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT typeof(1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT typeof(2147483647) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT typeof(2147483648) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT typeof(-2147483648) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT typeof(-2147483647) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT typeof(1.5) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT typeof(null) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT typeof('A') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT typeof('ab') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT typeof('') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT typeof(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT typeof(reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT typeof(score) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT typeof(b) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT typeof(code) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT typeof(flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT typeof(ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT typeof(day) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT typeof(ok) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT typeof(note) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT typeof(location) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT typeof(tag) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT typeof(ch) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT TYPEOF(1) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT typeof(id) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT typeof(id) FROM sensors ORDER BY typeof(id)"),
    (os.environ["DIFF_H1"], "SELECT typeof(id) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT not(ok) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT not(null) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT NOT(ok) FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT not(ok) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT not(ok) FROM sensors ORDER BY not(ok)"),
    (os.environ["DIFF_H1"], "SELECT not(ok) FROM sensors ORDER BY not(ok) DESC"),
    (os.environ["DIFF_H1"], "SELECT not(ok) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT not(1, ok) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT not(missing, ok) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT not(ok, null) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT stddev(reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT stddev(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT stddev(score) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT stddev(b) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT stddev(code) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT stddev(flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT stddev(ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT stddev(reading) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT stddev(reading) FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT stddev(reading) FROM sensors WHERE id = -1"),
    (os.environ["DIFF_H1"], "SELECT STDDEV(reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT stddev(reading) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT stddev(reading) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT stddev(id) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT stddev(reading) FROM sensors WHERE location = 'madrid'"),
    (os.environ["DIFF_H1"], "SELECT variance(reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT variance(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT variance(score) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT variance(b) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT variance(flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT variance(reading) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT variance(reading) FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT variance(reading) FROM sensors WHERE id = -1"),
    (os.environ["DIFF_H1"], "SELECT VARIANCE(reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT variance(reading) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT variance(reading) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT variance(id) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT variance(reading) FROM sensors WHERE location = 'madrid'"),
    (os.environ["DIFF_H1"], "SELECT var_pop(reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT var_pop(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT var_pop(score) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT var_pop(b) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT var_pop(flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT var_pop(reading) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT var_pop(reading) FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT var_pop(reading) FROM sensors WHERE id = -1"),
    (os.environ["DIFF_H1"], "SELECT VAR_POP(reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT var_pop(reading) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT var_pop(reading) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT var_pop(id) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT var_pop(reading) FROM sensors WHERE location = 'madrid'"),
    (os.environ["DIFF_H1"], "SELECT stddev_pop(reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT stddev_pop(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT stddev_pop(score) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT stddev_pop(b) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT stddev_pop(code) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT stddev_pop(flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT stddev_pop(ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT stddev_pop(reading) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT stddev_pop(reading) FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT stddev_pop(reading) FROM sensors WHERE id = -1"),
    (os.environ["DIFF_H1"], "SELECT STDDEV_POP(reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT stddev_pop(reading) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT stddev_pop(reading) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT stddev_pop(id) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT stddev_pop(reading) FROM sensors WHERE location = 'madrid'"),
    (os.environ["DIFF_H1"], "SELECT ksum(reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT ksum(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT ksum(score) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT ksum(b) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT ksum(code) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT ksum(flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT ksum(reading) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT ksum(reading) FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT ksum(reading) FROM sensors WHERE id = -1"),
    (os.environ["DIFF_H1"], "SELECT KSUM(reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT ksum(reading) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT ksum(reading) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT ksum(id) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT ksum(reading) FROM sensors WHERE location = 'madrid'"),
    (os.environ["DIFF_H1"], "SELECT nsum(reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT nsum(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT nsum(score) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT nsum(b) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT nsum(code) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT nsum(flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT nsum(reading) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT nsum(reading) FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT nsum(reading) FROM sensors WHERE id = -1"),
    (os.environ["DIFF_H1"], "SELECT NSUM(reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT nsum(reading) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT nsum(reading) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT nsum(id) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT nsum(reading) FROM sensors WHERE location = 'madrid'"),
    (os.environ["DIFF_H1"], "SELECT approx_count_distinct(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT approx_count_distinct(flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT approx_count_distinct(code) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT approx_count_distinct(b) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT approx_count_distinct(ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT approx_count_distinct(day) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT approx_count_distinct(location) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT approx_count_distinct(id) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT approx_count_distinct(id) FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT approx_count_distinct(id) FROM sensors WHERE id = -1"),
    (os.environ["DIFF_H1"], "SELECT approx_count_distinct(id) FROM sensors WHERE id = NULL"),
    (os.environ["DIFF_H1"], "SELECT APPROX_COUNT_DISTINCT(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT approx_count_distinct(id) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT approx_count_distinct(id) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT approx_count_distinct(id) FROM sensors WHERE location = 'madrid'"),
    (os.environ["DIFF_H1"], "SELECT approx_count_distinct(id, 14) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT approx_count_distinct(id, 4) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT string_agg(note, ',') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT string_agg(tag, ',') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT string_agg(location, ',') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT string_agg(ch, ',') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT string_agg(note, ',') FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT string_agg(note, ',') FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT string_agg(note, ',') FROM sensors WHERE id = -1"),
    (os.environ["DIFF_H1"], "SELECT string_agg(note, ',') FROM sensors WHERE note = ''"),
    (os.environ["DIFF_H1"], "SELECT STRING_AGG(note, ',') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT string_agg(note, ',') FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT string_agg(note, ',') FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT string_agg(note, '|') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT string_agg(note, 'β') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT string_agg(note, '') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT string_agg(note, null) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT covar_pop(reading, score) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT covar_pop(reading, id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT covar_pop(id, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT covar_pop(b, code) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT covar_pop(score, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT covar_pop(ts, id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT covar_pop(flags, code) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT covar_pop(reading, score) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT covar_pop(reading, score) FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT covar_pop(reading, score) FROM sensors WHERE id = -1"),
    (os.environ["DIFF_H1"], "SELECT covar_pop(reading, score) FROM sensors WHERE id = NULL"),
    (os.environ["DIFF_H1"], "SELECT COVAR_POP(reading, score) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT covar_pop(reading, score) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT covar_pop(reading, score) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT covar_pop(reading, id) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT covar_pop(reading, score) FROM sensors WHERE location = 'madrid'"),
    (os.environ["DIFF_H1"], "SELECT covar_samp(reading, score) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT covar_samp(reading, id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT covar_samp(id, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT covar_samp(b, code) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT covar_samp(score, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT covar_samp(ts, id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT covar_samp(flags, code) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT covar_samp(reading, score) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT covar_samp(reading, score) FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT covar_samp(reading, score) FROM sensors WHERE id = -1"),
    (os.environ["DIFF_H1"], "SELECT covar_samp(reading, score) FROM sensors WHERE id = NULL"),
    (os.environ["DIFF_H1"], "SELECT COVAR_SAMP(reading, score) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT covar_samp(reading, score) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT covar_samp(reading, score) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT covar_samp(reading, id) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT covar_samp(reading, score) FROM sensors WHERE location = 'madrid'"),
    (os.environ["DIFF_H1"], "SELECT corr(reading, score) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT corr(reading, id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT corr(id, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT corr(b, code) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT corr(score, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT corr(ts, id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT corr(day, id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT corr(flags, code) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT corr(reading, score) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT corr(reading, score) FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT corr(reading, score) FROM sensors WHERE id = -1"),
    (os.environ["DIFF_H1"], "SELECT corr(reading, score) FROM sensors WHERE id = NULL"),
    (os.environ["DIFF_H1"], "SELECT CORR(reading, score) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT corr(reading, score) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT corr(reading, score) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT corr(reading, id) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT corr(reading, score) FROM sensors WHERE location = 'madrid'"),
    (os.environ["DIFF_H1"], "SELECT regr_slope(reading, score) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT regr_slope(reading, id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT regr_slope(id, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT regr_slope(b, code) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT regr_slope(score, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT regr_slope(ts, id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT regr_slope(reading, score) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT regr_slope(reading, score) FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT regr_slope(reading, score) FROM sensors WHERE id = -1"),
    (os.environ["DIFF_H1"], "SELECT regr_slope(reading, score) FROM sensors WHERE id = NULL"),
    (os.environ["DIFF_H1"], "SELECT REGR_SLOPE(reading, score) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT regr_slope(reading, score) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT regr_slope(reading, score) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT regr_slope(reading, id) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT regr_slope(reading, score) FROM sensors WHERE location = 'madrid'"),
    (os.environ["DIFF_H1"], "SELECT regr_intercept(reading, score) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT regr_intercept(reading, id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT regr_intercept(id, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT regr_intercept(b, code) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT regr_intercept(score, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT regr_intercept(flags, code) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT regr_intercept(reading, score) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT regr_intercept(reading, score) FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT regr_intercept(reading, score) FROM sensors WHERE id = -1"),
    (os.environ["DIFF_H1"], "SELECT regr_intercept(reading, score) FROM sensors WHERE id = NULL"),
    (os.environ["DIFF_H1"], "SELECT REGR_INTERCEPT(reading, score) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT regr_intercept(reading, score) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT regr_intercept(reading, score) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT regr_intercept(reading, id) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT regr_intercept(reading, score) FROM sensors WHERE location = 'madrid'"),
    (os.environ["DIFF_H1"], "SELECT regr_r2(reading, score) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT regr_r2(reading, id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT regr_r2(id, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT regr_r2(b, code) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT regr_r2(score, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT regr_r2(ts, id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT regr_r2(day, id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT regr_r2(flags, code) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT regr_r2(reading, score) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT regr_r2(reading, score) FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT regr_r2(reading, score) FROM sensors WHERE id = -1"),
    (os.environ["DIFF_H1"], "SELECT regr_r2(reading, score) FROM sensors WHERE id = NULL"),
    (os.environ["DIFF_H1"], "SELECT REGR_R2(reading, score) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT regr_r2(reading, score) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT regr_r2(reading, score) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT regr_r2(reading, id) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT regr_r2(reading, score) FROM sensors WHERE location = 'madrid'"),
    (os.environ["DIFF_H1"], "SELECT stddev_samp(reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT stddev_samp(score) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT stddev_samp(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT stddev_samp(flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT stddev_samp(b) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT stddev_samp(code) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT stddev_samp(ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT stddev_samp(reading) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT stddev_samp(reading) FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT stddev_samp(reading) FROM sensors WHERE id = -1"),
    (os.environ["DIFF_H1"], "SELECT STDDEV_SAMP(reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT stddev_samp(reading) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT stddev_samp(reading) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT var_samp(reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT var_samp(score) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT var_samp(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT var_samp(flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT var_samp(b) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT var_samp(reading) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT var_samp(reading) FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT var_samp(reading) FROM sensors WHERE id = -1"),
    (os.environ["DIFF_H1"], "SELECT VAR_SAMP(reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT var_samp(reading) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT var_samp(reading) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT bool_and(ok) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT bool_and(ok) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT bool_and(ok) FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT bool_and(ok) FROM sensors WHERE id = -1"),
    (os.environ["DIFF_H1"], "SELECT BOOL_AND(ok) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT bool_and(ok) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT bool_and(ok) FROM sensors WHERE location = 'madrid'"),
    (os.environ["DIFF_H1"], "SELECT bool_and(ok) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT bool_or(ok) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT bool_or(ok) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT bool_or(ok) FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT bool_or(ok) FROM sensors WHERE id = -1"),
    (os.environ["DIFF_H1"], "SELECT BOOL_OR(ok) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT bool_or(ok) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT bool_or(ok) FROM sensors WHERE location = 'madrid'"),
    (os.environ["DIFF_H1"], "SELECT bool_or(ok) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT first_not_null(note) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT first_not_null(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT first_not_null(reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT first_not_null(score) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT first_not_null(flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT first_not_null(b) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT first_not_null(code) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT first_not_null(ch) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT first_not_null(location) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT first_not_null(tag) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT first_not_null(day) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT first_not_null(ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT first_not_null(note) FROM sensors WHERE id = NULL"),
    (os.environ["DIFF_H1"], "SELECT first_not_null(id) FROM sensors WHERE id = NULL"),
    (os.environ["DIFF_H1"], "SELECT first_not_null(note) FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT first_not_null(note) FROM sensors WHERE id = -1"),
    (os.environ["DIFF_H1"], "SELECT FIRST_NOT_NULL(note) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT first_not_null(note) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT first_not_null(note) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT first_not_null(id) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT last_not_null(note) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT last_not_null(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT last_not_null(reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT last_not_null(score) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT last_not_null(flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT last_not_null(b) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT last_not_null(code) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT last_not_null(ch) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT last_not_null(location) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT last_not_null(tag) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT last_not_null(day) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT last_not_null(ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT last_not_null(note) FROM sensors WHERE id = NULL"),
    (os.environ["DIFF_H1"], "SELECT last_not_null(id) FROM sensors WHERE id = NULL"),
    (os.environ["DIFF_H1"], "SELECT last_not_null(note) FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT last_not_null(note) FROM sensors WHERE id = -1"),
    (os.environ["DIFF_H1"], "SELECT LAST_NOT_NULL(note) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT last_not_null(note) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT last_not_null(note) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT last_not_null(id) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT bit_and(flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT bit_and(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT bit_and(b) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT bit_and(code) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT bit_and(flags) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT bit_and(flags) FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT bit_and(b) FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT bit_and(code) FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT bit_and(ch) FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT bit_and(id) FROM sensors WHERE id = NULL"),
    (os.environ["DIFF_H1"], "SELECT bit_and(id) FROM sensors WHERE id = -1"),
    (os.environ["DIFF_H1"], "SELECT BIT_AND(flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT bit_and(flags) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT bit_and(location) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT bit_and(location) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT bit_and(day) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT bit_and(ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT bit_and(flags) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT bit_and(id) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT bit_or(flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT bit_or(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT bit_or(b) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT bit_or(code) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT bit_or(flags) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT bit_or(flags) FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT bit_or(b) FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT bit_or(code) FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT bit_or(ch) FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT bit_or(id) FROM sensors WHERE id = NULL"),
    (os.environ["DIFF_H1"], "SELECT bit_or(id) FROM sensors WHERE id = -1"),
    (os.environ["DIFF_H1"], "SELECT BIT_OR(flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT bit_or(flags) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT bit_or(location) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT bit_or(location) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT bit_or(day) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT bit_or(ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT bit_or(flags) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT bit_or(id) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT bit_xor(flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT bit_xor(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT bit_xor(b) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT bit_xor(code) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT bit_xor(flags) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT bit_xor(flags) FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT bit_xor(b) FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT bit_xor(code) FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT bit_xor(ch) FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT bit_xor(id) FROM sensors WHERE id = NULL"),
    (os.environ["DIFF_H1"], "SELECT bit_xor(id) FROM sensors WHERE id = -1"),
    (os.environ["DIFF_H1"], "SELECT BIT_XOR(flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT bit_xor(flags) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT bit_xor(location) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT bit_xor(location) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT bit_xor(day) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT bit_xor(ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT bit_xor(flags) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT bit_xor(id) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT geomean(reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT geomean(reading) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT geomean(reading) FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT geomean(score) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT geomean(score) FROM sensors WHERE score > 0"),
    (os.environ["DIFF_H1"], "SELECT geomean(flags) FROM sensors WHERE flags > 0"),
    (os.environ["DIFF_H1"], "SELECT geomean(b) FROM sensors WHERE b > 0"),
    (os.environ["DIFF_H1"], "SELECT geomean(id) FROM sensors WHERE id > 0"),
    (os.environ["DIFF_H1"], "SELECT geomean(reading) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT geomean(score) FROM sensors WHERE id = 1 GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT geomean(reading) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT GEOMEAN(reading) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT geomean(day) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT geomean(reading) FROM sensors WHERE id = 2"),
    (os.environ["DIFF_H1"], "SELECT geomean(score) FROM sensors WHERE id = 2"),
    (os.environ["DIFF_H1"], "SELECT geomean(flags) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT geomean(b) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT skewness(flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT skewness(flags) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT skewness(flags) FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT skewness(reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT skewness(score) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT skewness(b) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT skewness(code) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT skewness(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT skewness(id) FROM sensors WHERE id > 0"),
    (os.environ["DIFF_H1"], "SELECT skewness(flags) FROM sensors WHERE flags > 0"),
    (os.environ["DIFF_H1"], "SELECT skewness(flags) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT skewness(reading) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT skewness(flags) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT SKEWNESS(flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT skewness(day) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT skewness(ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT mode(location) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT mode(location) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT mode(location) FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT mode(location) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT MODE(location) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT mode(location) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT mode(flags) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT mode(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT mode(ok) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT mode(ch) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT approx_median(flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT approx_median(flags) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT approx_median(flags) FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT approx_median(flags) FROM sensors WHERE flags > 0"),
    (os.environ["DIFF_H1"], "SELECT approx_median(flags) FROM sensors WHERE id = -1"),
    (os.environ["DIFF_H1"], "SELECT approx_median(flags, 0) FROM sensors WHERE id = -1"),
    (os.environ["DIFF_H1"], "SELECT approx_median(flags, 3) FROM sensors WHERE id = -1"),
    (os.environ["DIFF_H1"], "SELECT approx_median(reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT approx_median(reading) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT approx_median(score) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT approx_median(flags) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT approx_median(reading) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT approx_median(flags) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT APPROX_MEDIAN(flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT approx_percentile(flags, 0.5) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT approx_percentile(flags, 0.5) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT approx_percentile(flags, 0.5) FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT approx_percentile(flags, 1) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT approx_percentile(flags, 0) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT approx_percentile(flags, 0.25) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT approx_percentile(flags, 0.75) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT approx_percentile(flags, 0.5) FROM sensors WHERE id = -1"),
    (os.environ["DIFF_H1"], "SELECT approx_percentile(flags, 0.5, 0) FROM sensors WHERE id = -1"),
    (os.environ["DIFF_H1"], "SELECT approx_percentile(flags, 0.5, 3) FROM sensors WHERE id = -1"),
    (os.environ["DIFF_H1"], "SELECT approx_percentile(flags, 0.5, 1) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT approx_percentile(flags, 0.5) FROM sensors WHERE flags > 0"),
    (os.environ["DIFF_H1"], "SELECT approx_percentile(reading, 0.5) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT approx_percentile(reading, 0) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT approx_percentile(reading, 1) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT approx_percentile(reading, 0.5) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT approx_percentile(score, 0.5) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT approx_percentile(b, 0.5) FROM sensors WHERE b >= 0"),
    (os.environ["DIFF_H1"], "SELECT approx_percentile(code, 0.5) FROM sensors WHERE code > 0"),
    (os.environ["DIFF_H1"], "SELECT approx_percentile(flags, 0.5) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT approx_percentile(reading, 0.5) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT approx_percentile(flags, 0.5) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT APPROX_PERCENTILE(flags, 0.5) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT arg_max(note, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT arg_max(note, flags) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT arg_max(note, flags) FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT arg_max(note, flags) FROM sensors WHERE flags > 0"),
    (os.environ["DIFF_H1"], "SELECT arg_max(location, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT arg_max(tag, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT arg_max(ch, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT arg_max(id, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT arg_max(b, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT arg_max(code, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT arg_max(score, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT arg_max(reading, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT arg_max(day, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT arg_max(ts, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT arg_max(flags, id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT arg_max(note, id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT arg_max(note, reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT arg_max(note, score) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT arg_max(note, ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT arg_max(note, day) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT arg_max(note, b) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT arg_max(note, location) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT arg_max(note, flags) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT arg_max(note, flags) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT ARG_MAX(note, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT arg_max(note, code) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT arg_max(reading, reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT arg_max(note, reading) FROM sensors WHERE reading <= 0"),
    (os.environ["DIFF_H1"], "SELECT arg_max(location, location) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT arg_max(id, id) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT arg_max(flags, ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT arg_max(flags, reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT arg_max(b, ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT arg_max(ch, id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT arg_min(reading, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT arg_min(reading, flags) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT arg_min(reading, flags) FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT arg_min(reading, flags) FROM sensors WHERE flags > 0"),
    (os.environ["DIFF_H1"], "SELECT arg_min(score, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT arg_min(b, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT arg_min(code, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT arg_min(id, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT arg_min(flags, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT arg_min(flags, id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT arg_min(id, id) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT arg_min(id, ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT arg_min(flags, ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT arg_min(flags, reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT arg_min(b, ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT arg_min(b, id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT arg_min(day, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT arg_min(ts, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT arg_min(ch, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT arg_min(ch, id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT arg_min(location, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT arg_min(location, id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT arg_min(location, ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT arg_min(reading, reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT arg_min(reading, flags) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT arg_min(reading, flags) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT ARG_MIN(reading, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT arg_min(score, id) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT arg_min(day, id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT string_distinct_agg(note, ',') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT string_distinct_agg(tag, ',') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT string_distinct_agg(location, ',') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT string_distinct_agg(ch, ',') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT string_distinct_agg(note, ',') FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT string_distinct_agg(note, ',') FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT string_distinct_agg(note, ',') FROM sensors WHERE id = -1"),
    (os.environ["DIFF_H1"], "SELECT string_distinct_agg(note, ',') FROM sensors WHERE note = ''"),
    (os.environ["DIFF_H1"], "SELECT STRING_DISTINCT_AGG(note, ',') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT string_distinct_agg(note, ',') FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT string_distinct_agg(note, ',') FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT string_distinct_agg(location, ',') FROM sensors GROUP BY note ORDER BY note"),
    (os.environ["DIFF_H1"], "SELECT string_distinct_agg(note, '|') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT string_distinct_agg(note, 'β') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT string_distinct_agg(note, '') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT string_distinct_agg(note, null) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT string_distinct_agg(location, ',') FROM sensors WHERE location = 'madrid'"),
    (os.environ["DIFF_H1"], "SELECT kurtosis_pop(flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT kurtosis_pop(flags) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT kurtosis_pop(flags) FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT kurtosis_pop(reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT kurtosis_pop(score) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT kurtosis_pop(b) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT kurtosis_pop(code) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT kurtosis_pop(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT kurtosis_pop(id) FROM sensors WHERE id > 0"),
    (os.environ["DIFF_H1"], "SELECT kurtosis_pop(flags) FROM sensors WHERE flags > 0"),
    (os.environ["DIFF_H1"], "SELECT kurtosis_pop(flags) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT kurtosis_pop(reading) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT kurtosis_pop(flags) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT KURTOSIS_POP(flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT kurtosis_pop(day) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT kurtosis_pop(ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT skewness_pop(flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT skewness_pop(flags) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT skewness_pop(flags) FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT skewness_pop(reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT skewness_pop(score) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT skewness_pop(b) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT skewness_pop(code) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT skewness_pop(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT skewness_pop(id) FROM sensors WHERE id > 0"),
    (os.environ["DIFF_H1"], "SELECT skewness_pop(flags) FROM sensors WHERE flags > 0"),
    (os.environ["DIFF_H1"], "SELECT skewness_pop(flags) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT skewness_pop(reading) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT skewness_pop(flags) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT SKEWNESS_POP(flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT skewness_pop(day) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT skewness_pop(ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT vwap(reading, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT vwap(reading, flags) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT vwap(reading, flags) FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT vwap(reading, flags) FROM sensors WHERE flags > 0"),
    (os.environ["DIFF_H1"], "SELECT vwap(score, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT vwap(id, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT vwap(b, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT vwap(code, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT vwap(reading, id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT vwap(flags, reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT vwap(reading, flags) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT vwap(reading, flags) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT VWAP(reading, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT vwap(reading, flags) FROM sensors WHERE id = -1"),
    (os.environ["DIFF_H1"], "SELECT twap(reading, ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT twap(reading, ts) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT twap(reading, ts) FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT twap(reading, ts) FROM sensors WHERE id = -1"),
    (os.environ["DIFF_H1"], "SELECT twap(score, ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT twap(id, ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT twap(b, ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT twap(flags, ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT twap(reading, ts) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT twap(reading, ts) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT TWAP(reading, ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT sparkline(flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT sparkline(flags) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT sparkline(flags) FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT sparkline(reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT sparkline(score) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT sparkline(b) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT sparkline(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT sparkline(code) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT sparkline(day) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT sparkline(ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT sparkline(flags) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT sparkline(flags) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT SPARKLINE(flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT sparkline(flags, 0, 42, 2) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT sparkline(flags, 0, 100, 2) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT sparkline(reading, 0, 1.5, 2) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT sparkline(flags, 0, 42, 10) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT sparkline(flags, null, null, 3) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT sparkline(flags) FROM sensors WHERE id = -1"),
    (os.environ["DIFF_H1"], "SELECT weighted_avg(reading, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT weighted_avg(reading, flags) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT weighted_avg(reading, flags) FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT weighted_avg(reading, flags) FROM sensors WHERE id = -1"),
    (os.environ["DIFF_H1"], "SELECT weighted_avg(score, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT weighted_avg(id, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT weighted_avg(b, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT weighted_avg(code, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT weighted_avg(flags, reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT weighted_avg(reading, b) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT weighted_avg(flags, b) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT weighted_avg(score, b) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT weighted_avg(reading, id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT weighted_avg(reading, flags) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT weighted_avg(reading, flags) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT WEIGHTED_AVG(reading, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT weighted_avg(reading, flags) FROM sensors WHERE flags > 0"),
    (os.environ["DIFF_H1"], "SELECT weighted_avg(reading, flags) FROM sensors WHERE flags = 0"),
    (os.environ["DIFF_H1"], "SELECT weighted_stddev(reading, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT weighted_stddev(reading, flags) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT weighted_stddev(reading, flags) FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT weighted_stddev(reading, flags) FROM sensors WHERE id = -1"),
    (os.environ["DIFF_H1"], "SELECT weighted_stddev(score, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT weighted_stddev(id, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT weighted_stddev(b, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT weighted_stddev(code, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT weighted_stddev(flags, reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT weighted_stddev(reading, b) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT weighted_stddev(flags, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT weighted_stddev(ts, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT weighted_stddev(reading, flags) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT weighted_stddev(reading, flags) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT WEIGHTED_STDDEV(reading, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT weighted_stddev(reading, flags) FROM sensors WHERE flags > 0"),
    (os.environ["DIFF_H1"], "SELECT weighted_stddev_freq(reading, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT weighted_stddev_freq(reading, flags) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT weighted_stddev_freq(reading, flags) FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT weighted_stddev_freq(reading, flags) FROM sensors WHERE id = -1"),
    (os.environ["DIFF_H1"], "SELECT weighted_stddev_freq(score, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT weighted_stddev_freq(id, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT weighted_stddev_freq(b, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT weighted_stddev_freq(code, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT weighted_stddev_freq(flags, reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT weighted_stddev_freq(reading, b) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT weighted_stddev_freq(flags, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT weighted_stddev_freq(ts, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT weighted_stddev_freq(reading, flags) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT weighted_stddev_freq(reading, flags) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT WEIGHTED_STDDEV_FREQ(reading, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT weighted_stddev_freq(reading, flags) FROM sensors WHERE flags > 0"),
    (os.environ["DIFF_H1"], "SELECT weighted_stddev_rel(reading, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT weighted_stddev_rel(reading, flags) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT weighted_stddev_rel(reading, flags) FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT weighted_stddev_rel(reading, flags) FROM sensors WHERE id = -1"),
    (os.environ["DIFF_H1"], "SELECT weighted_stddev_rel(score, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT weighted_stddev_rel(id, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT weighted_stddev_rel(b, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT weighted_stddev_rel(code, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT weighted_stddev_rel(flags, reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT weighted_stddev_rel(reading, b) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT weighted_stddev_rel(flags, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT weighted_stddev_rel(ts, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT weighted_stddev_rel(reading, flags) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT weighted_stddev_rel(reading, flags) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT WEIGHTED_STDDEV_REL(reading, flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT weighted_stddev_rel(reading, flags) FROM sensors WHERE flags > 0"),
    (os.environ["DIFF_H1"], "SELECT isOrdered(ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT isOrdered(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT isOrdered(flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT isOrdered(day) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT isOrdered(b) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT isOrdered(code) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT isOrdered(id) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT isOrdered(id) FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT isOrdered(note) FROM sensors WHERE id = 2"),
    (os.environ["DIFF_H1"], "SELECT isOrdered(flags) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT isOrdered(id) FROM sensors WHERE location = 'madrid'"),
    (os.environ["DIFF_H1"], "SELECT isOrdered(day) FROM sensors WHERE location = 'madrid'"),
    (os.environ["DIFF_H1"], "SELECT isOrdered(ch) FROM sensors WHERE ch = '1'"),
    (os.environ["DIFF_H1"], "SELECT isOrdered(id) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT isOrdered(ts) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT isOrdered(id) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT ISORDERED(ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT percentile_disc(flags, 0.5) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT percentile_disc(flags, 0.5) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT percentile_disc(flags, 0.5) FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT percentile_disc(flags, 0) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT percentile_disc(flags, 1) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT percentile_disc(flags, 0.5) FROM sensors WHERE id = -1"),
    (os.environ["DIFF_H1"], "SELECT percentile_disc(reading, 0.5) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT percentile_disc(reading, 0) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT percentile_disc(reading, 1) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT percentile_disc(score, 0.5) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT percentile_disc(id, 0.5) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT percentile_disc(b, 0.5) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT percentile_disc(code, 0.5) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT percentile_disc(day, 0.5) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT percentile_disc(ts, 0.5) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT percentile_disc(flags, 0.5) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT percentile_disc(reading, 0.5) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT percentile_disc(flags, 0.5) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT PERCENTILE_DISC(flags, 0.5) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT percentile_cont(flags, 0.5) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT percentile_cont(flags, 0.25) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT percentile_cont(flags, 0) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT percentile_cont(flags, 1) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT percentile_cont(flags, 0.5) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT percentile_cont(flags, 0.5) FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT percentile_cont(reading, 0.5) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT percentile_cont(reading, 0) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT percentile_cont(reading, 1) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT percentile_cont(score, 0.5) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT percentile_cont(id, 0.5) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT percentile_cont(b, 0.5) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT percentile_cont(code, 0.5) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT percentile_cont(flags, 0.5) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT percentile_cont(reading, 0.5) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT percentile_cont(flags, 0.5) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT PERCENTILE_CONT(flags, 0.5) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT skewness_samp(flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT skewness_samp(flags) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT skewness_samp(flags) FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT skewness_samp(reading) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT skewness_samp(score) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT skewness_samp(b) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT skewness_samp(code) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT skewness_samp(id) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT skewness_samp(id) FROM sensors WHERE id > 0"),
    (os.environ["DIFF_H1"], "SELECT skewness_samp(flags) FROM sensors WHERE flags > 0"),
    (os.environ["DIFF_H1"], "SELECT skewness_samp(flags) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT skewness_samp(reading) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT skewness_samp(flags) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT skewness_samp(day) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT skewness_samp(ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT SKEWNESS_SAMP(flags) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT quantile_cont(flags, 0.5) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT quantile_cont(flags, 0.25) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT quantile_cont(flags, 0) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT quantile_cont(flags, 1) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT quantile_cont(flags, 0.5) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT quantile_cont(flags, 0.5) FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT quantile_cont(reading, 0.5) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT quantile_cont(reading, 0) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT quantile_cont(reading, 1) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT quantile_cont(score, 0.5) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT quantile_cont(id, 0.5) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT quantile_cont(b, 0.5) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT quantile_cont(code, 0.5) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT quantile_cont(flags, 0.5) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT quantile_cont(reading, 0.5) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT quantile_cont(flags, 0.5) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT QUANTILE_CONT(flags, 0.5) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT quantile_disc(flags, 0.5) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT quantile_disc(flags, 0.5) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT quantile_disc(flags, 0.5) FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT quantile_disc(flags, 0) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT quantile_disc(flags, 1) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT quantile_disc(flags, 0.5) FROM sensors WHERE id = -1"),
    (os.environ["DIFF_H1"], "SELECT quantile_disc(reading, 0.5) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT quantile_disc(reading, 0) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT quantile_disc(reading, 1) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT quantile_disc(score, 0.5) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT quantile_disc(id, 0.5) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT quantile_disc(b, 0.5) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT quantile_disc(code, 0.5) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT quantile_disc(day, 0.5) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT quantile_disc(ts, 0.5) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT quantile_disc(flags, 0.5) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT quantile_disc(reading, 0.5) FROM sensors GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H1"], "SELECT quantile_disc(flags, 0.5) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT QUANTILE_DISC(flags, 0.5) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT glob(note, 'a*') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT glob(note, 'a*') FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT glob(note, 'a*') FROM sensors WHERE id = 2"),
    (os.environ["DIFF_H1"], "SELECT glob(note, 'z*') FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT glob(note, '*') FROM sensors WHERE note = ''"),
    (os.environ["DIFF_H1"], "SELECT glob(note, '?lpha') FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT glob(note, 'β*') FROM sensors WHERE id = -1"),
    (os.environ["DIFF_H1"], "SELECT glob(note, '[a-z]*') FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT glob(tag, 'v*') FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT glob(location, 'ma*') FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT glob(ch, 'A') FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT glob('alpha', 'a*') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT GLOB(note, 'a*') FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT glob(note, 'a*') FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT glob(note, 'a*') FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT ilike(note, 'A%') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT ilike(note, 'A%') FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT ilike(note, 'A%') FROM sensors WHERE id = 2"),
    (os.environ["DIFF_H1"], "SELECT ilike(note, 'Z%') FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT ilike(note, '') FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT ilike(note, null) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT ilike(note, '%') FROM sensors WHERE note = ''"),
    (os.environ["DIFF_H1"], "SELECT ilike(note, '_') FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT ilike(note, 'A_PHA') FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT ilike(tag, 'v%') FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT ilike(location, 'M%') FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT ilike(ch, 'a') FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT ilike('Alpha', 'a%') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT ilike(note, 'Β%') FROM sensors WHERE id = -1"),
    (os.environ["DIFF_H1"], "SELECT ilike(note, 'a\\%') FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT ILIKE(note, 'A%') FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT ilike(note, 'A%') FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT ilike(note, 'A%') FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT like(note, 'a%') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT like(note, 'a%') FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT like(note, 'A%') FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT like(note, 'a%') FROM sensors WHERE id = 2"),
    (os.environ["DIFF_H1"], "SELECT like(note, '') FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT like(note, null) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT like(note, '%') FROM sensors WHERE note = ''"),
    (os.environ["DIFF_H1"], "SELECT like(note, '_') FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT like(note, 'a_pha') FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT like(tag, 'v%') FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT like(location, 'ma%') FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT like(ch, 'A') FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT like('Alpha', 'a%') FROM sensors LIMIT 1"),
    (os.environ["DIFF_H1"], "SELECT like(note, 'β%') FROM sensors WHERE id = -1"),
    (os.environ["DIFF_H1"], "SELECT like(note, 'a\\%') FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT LIKE(note, 'a%') FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT like(note, 'a%') FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT like(note, 'a%') FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT switch(id, 1, 'one', 'other') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT switch(id, 1, 'one', 'other') FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT switch(id, 1, 'one') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT switch(id, 1, 'one', 2, 'two', 'other') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT switch(note, 'alpha', 7, 0) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT switch(location, 'madrid', 'es', 'other') FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT SWITCH(id, 1, 'one', 'other') FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT switch(id, 1, 'one', 'other') FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT switch(id, 1, 'one', 'other') FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT bar(flags, 0, 42, 4) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT bar(reading, 0, 2, 4) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT bar(reading, 0, 2, 4) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT bar(flags, 0, 42, 4) FROM sensors WHERE id = 2"),
    (os.environ["DIFF_H1"], "SELECT bar(flags, 5, 5, 4) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT bar(flags, 0, 42, 0) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT BAR(flags, 0, 42, 4) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT bar(flags, 0, 42, 4) FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT bar(flags, 0, 42, 4) FROM sensors LIMIT 0"),
    (os.environ["DIFF_H1"], "SELECT haversine_dist_deg(reading, score, ts) FROM sensors"),
    (os.environ["DIFF_H1"], "SELECT haversine_dist_deg(reading, score, ts) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT haversine_dist_deg(reading, score, ts) FROM sensors WHERE id > 100"),
    (os.environ["DIFF_H1"], "SELECT HAVERSINE_DIST_DEG(reading, score, ts) FROM sensors WHERE id = 1"),
    (os.environ["DIFF_H1"], "SELECT haversine_dist_deg(reading, score, ts) FROM sensors LIMIT 0"),
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
    (os.environ["DIFF_H2"], "SELECT day_of_week_sunday_first(ts) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT to_timestamp(note) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT to_timestamp(note, 'yyyy-MM-dd') FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT extract(day FROM ts) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT to_date(note, 'yyyy-MM-dd') FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT md5(note) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT sha1(note) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT sha256(note) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT sin(id) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT sin(ts) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT cos(id) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT tan(id) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT tan(ts) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT asin(id) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT acos(id) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT atan(id) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT atan2(id, 1) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT degrees(id) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT radians(id) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT cot(id) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT cot(ts) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT typeof(id) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT typeof(note) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT typeof(location) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT not(null) FROM days ORDER BY ts"),
    (os.environ["DIFF_H2"], "SELECT stddev(id) FROM days"),
    (os.environ["DIFF_H2"], "SELECT stddev(id) FROM days GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H2"], "SELECT variance(id) FROM days"),
    (os.environ["DIFF_H2"], "SELECT variance(id) FROM days GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H2"], "SELECT var_pop(id) FROM days"),
    (os.environ["DIFF_H2"], "SELECT var_pop(id) FROM days GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H2"], "SELECT stddev_pop(id) FROM days"),
    (os.environ["DIFF_H2"], "SELECT stddev_pop(id) FROM days GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H2"], "SELECT ksum(id) FROM days"),
    (os.environ["DIFF_H2"], "SELECT ksum(id) FROM days GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H2"], "SELECT nsum(id) FROM days"),
    (os.environ["DIFF_H2"], "SELECT nsum(id) FROM days GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H2"], "SELECT approx_count_distinct(id) FROM days"),
    (os.environ["DIFF_H2"], "SELECT approx_count_distinct(id) FROM days GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H2"], "SELECT approx_count_distinct(location) FROM days"),
    (os.environ["DIFF_H2"], "SELECT approx_count_distinct(ts) FROM days"),
    (os.environ["DIFF_H2"], "SELECT string_agg(note, ',') FROM days"),
    (os.environ["DIFF_H2"], "SELECT string_agg(note, ',') FROM days GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H2"], "SELECT corr(id, ts) FROM days"),
    (os.environ["DIFF_H2"], "SELECT corr(id, ts) FROM days GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H2"], "SELECT regr_intercept(id, ts) FROM days"),
    (os.environ["DIFF_H2"], "SELECT regr_intercept(id, ts) FROM days GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H2"], "SELECT regr_r2(id, ts) FROM days"),
    (os.environ["DIFF_H2"], "SELECT regr_r2(id, ts) FROM days GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H2"], "SELECT stddev_samp(id) FROM days"),
    (os.environ["DIFF_H2"], "SELECT var_samp(id) FROM days"),
    (os.environ["DIFF_H2"], "SELECT first_not_null(id) FROM days"),
    (os.environ["DIFF_H2"], "SELECT first_not_null(note) FROM days"),
    (os.environ["DIFF_H2"], "SELECT first_not_null(note) FROM days GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H2"], "SELECT last_not_null(id) FROM days"),
    (os.environ["DIFF_H2"], "SELECT last_not_null(note) FROM days"),
    (os.environ["DIFF_H2"], "SELECT last_not_null(note) FROM days GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H2"], "SELECT bit_and(id) FROM days"),
    (os.environ["DIFF_H2"], "SELECT bit_and(id) FROM days GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H2"], "SELECT bit_or(id) FROM days"),
    (os.environ["DIFF_H2"], "SELECT bit_or(id) FROM days GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H2"], "SELECT bit_xor(id) FROM days"),
    (os.environ["DIFF_H2"], "SELECT bit_xor(id) FROM days GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H2"], "SELECT geomean(id) FROM days"),
    (os.environ["DIFF_H2"], "SELECT geomean(id) FROM days GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H2"], "SELECT skewness(id) FROM days"),
    (os.environ["DIFF_H2"], "SELECT skewness(id) FROM days GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H2"], "SELECT mode(location) FROM days"),
    (os.environ["DIFF_H2"], "SELECT mode(id) FROM days"),
    (os.environ["DIFF_H2"], "SELECT mode(id) FROM days GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H2"], "SELECT mode(location) FROM days GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H2"], "SELECT approx_median(id) FROM days"),
    (os.environ["DIFF_H2"], "SELECT approx_median(id) FROM days GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H2"], "SELECT approx_percentile(id, 0.5) FROM days"),
    (os.environ["DIFF_H2"], "SELECT approx_percentile(id, 0) FROM days"),
    (os.environ["DIFF_H2"], "SELECT approx_percentile(id, 1) FROM days"),
    (os.environ["DIFF_H2"], "SELECT approx_percentile(id, 0.5) FROM days GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H2"], "SELECT arg_max(note, id) FROM days"),
    (os.environ["DIFF_H2"], "SELECT arg_max(location, id) FROM days"),
    (os.environ["DIFF_H2"], "SELECT arg_max(note, id) FROM days GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H2"], "SELECT arg_max(ts, id) FROM days"),
    (os.environ["DIFF_H2"], "SELECT arg_min(id, id) FROM days"),
    (os.environ["DIFF_H2"], "SELECT arg_min(location, id) FROM days"),
    (os.environ["DIFF_H2"], "SELECT arg_min(id, id) FROM days GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H2"], "SELECT arg_min(ts, id) FROM days"),
    (os.environ["DIFF_H2"], "SELECT string_distinct_agg(note, ',') FROM days"),
    (os.environ["DIFF_H2"], "SELECT string_distinct_agg(location, ',') FROM days"),
    (os.environ["DIFF_H2"], "SELECT string_distinct_agg(note, ',') FROM days GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H2"], "SELECT string_distinct_agg(location, ',') FROM days GROUP BY id ORDER BY id"),
    (os.environ["DIFF_H2"], "SELECT kurtosis_pop(id) FROM days"),
    (os.environ["DIFF_H2"], "SELECT kurtosis_pop(id) FROM days GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H2"], "SELECT skewness_pop(id) FROM days"),
    (os.environ["DIFF_H2"], "SELECT skewness_pop(id) FROM days GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H2"], "SELECT vwap(id, id) FROM days"),
    (os.environ["DIFF_H2"], "SELECT vwap(id, id) FROM days GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H2"], "SELECT twap(id, ts) FROM days"),
    (os.environ["DIFF_H2"], "SELECT twap(id, ts) FROM days GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H2"], "SELECT sparkline(id) FROM days"),
    (os.environ["DIFF_H2"], "SELECT sparkline(id) FROM days GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H2"], "SELECT weighted_avg(id, id) FROM days"),
    (os.environ["DIFF_H2"], "SELECT weighted_avg(id, id) FROM days GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H2"], "SELECT weighted_stddev(id, id) FROM days"),
    (os.environ["DIFF_H2"], "SELECT weighted_stddev(id, id) FROM days GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H2"], "SELECT weighted_stddev_freq(id, id) FROM days"),
    (os.environ["DIFF_H2"], "SELECT weighted_stddev_freq(id, id) FROM days GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H2"], "SELECT weighted_stddev_rel(id, id) FROM days"),
    (os.environ["DIFF_H2"], "SELECT weighted_stddev_rel(id, id) FROM days GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H2"], "SELECT isOrdered(id) FROM days"),
    (os.environ["DIFF_H2"], "SELECT isOrdered(ts) FROM days"),
    (os.environ["DIFF_H2"], "SELECT isOrdered(id) FROM days GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H2"], "SELECT percentile_disc(id, 0.5) FROM days"),
    (os.environ["DIFF_H2"], "SELECT percentile_disc(id, 0.5) FROM days GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H2"], "SELECT percentile_cont(id, 0.5) FROM days"),
    (os.environ["DIFF_H2"], "SELECT percentile_cont(id, 0.5) FROM days GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H2"], "SELECT skewness_samp(id) FROM days"),
    (os.environ["DIFF_H2"], "SELECT skewness_samp(id) FROM days GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H2"], "SELECT quantile_cont(id, 0.5) FROM days"),
    (os.environ["DIFF_H2"], "SELECT quantile_cont(id, 0.5) FROM days GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H2"], "SELECT quantile_disc(id, 0.5) FROM days"),
    (os.environ["DIFF_H2"], "SELECT quantile_disc(id, 0.5) FROM days GROUP BY location ORDER BY location"),
    (os.environ["DIFF_H2"], "SELECT glob(note, 'a*') FROM days"),
    (os.environ["DIFF_H2"], "SELECT glob(note, 'a*') FROM days WHERE id = 1"),
    (os.environ["DIFF_H2"], "SELECT ilike(note, 'A%') FROM days"),
    (os.environ["DIFF_H2"], "SELECT ilike(note, 'A%') FROM days WHERE id = 1"),
    (os.environ["DIFF_H2"], "SELECT like(note, 'a%') FROM days"),
    (os.environ["DIFF_H2"], "SELECT like(note, 'a%') FROM days WHERE id = 1"),
    (os.environ["DIFF_H2"], "SELECT switch(id, 1, 'one', 'other') FROM days"),
    (os.environ["DIFF_H2"], "SELECT switch(id, 1, 'one', 'other') FROM days WHERE id = 1"),
    (os.environ["DIFF_H2"], "SELECT bar(id, 0, 3, 3) FROM days"),
    (os.environ["DIFF_H2"], "SELECT bar(id, 0, 3, 3) FROM days WHERE id = 1"),
    (os.environ["DIFF_H2"], "SELECT haversine_dist_deg(id, id, ts) FROM days"),
    (os.environ["DIFF_H2"], "SELECT haversine_dist_deg(id, id, ts) FROM days WHERE id = 1"),
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
