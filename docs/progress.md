# Progreso

## Hito actual

H0 y H1 cumplidos. La lectura de `PARTITION BY DAY` de dos días también, el 2026-11-06. El port no está terminado.

## Qué hay

- Referencia fijada: tag `10.1.0`, commit `e11c837f8a169b533540bd6fa57ef3d520b68ad9`. Detalle en `docs/upstream.md`.
- Inventario medido en `docs/inventory.md`.
- Fixture real `tests/fixtures/h1`: tabla `sensors`, 4 filas, `PARTITION BY NONE`.
- Fixture real `tests/fixtures/h2`: tabla `days`, 4 filas en `2020-01-02` y `2020-01-03.1`, `PARTITION BY DAY`, WAL off. El sufijo `.1` es el name txn de la segunda partición.
- Workspace `qdb-types`, `qdb-platform`, `qdb-storage`, `qdb-sql`, `qdb-server`.
- CLI: `qdb query --data-dir tests/fixtures/h1 --sql "..."`.

## Comandos y resultados

`cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --offline -- -D warnings` y `cargo test --workspace --offline` terminaron con código 0.

Tests que pasaron:

| Crate | Resultado |
| --- | --- |
| qdb-types | 3 ok |
| qdb-platform | 1 ok |
| qdb-storage | 4 ok, incluido `reads_original_sensors_fixture` y `reads_two_day_partitions_including_name_txn_suffix` |
| qdb-sql | 263 ok, incluido `sv_matches_captured_exec` |
| qdb-server | 2 ok (`query_fixture`) |

`tools/run_differential.sh`:

```text
captured-oracle: 3203 queries match tests/fixtures/h1/expected
captured-oracle: 376 queries match tests/fixtures/h2/expected
live-reference: 3579 queries matched row counts against http://127.0.0.1:19011/exec
```

Las salidas de `h1` y de `h2` se capturaron con el servidor vivo, desde `GET /exec`, antes de copiar los directorios. El TSV de `sensors` incluye `NULL`, cadena vacía, `-0.0`, `héllo` y `東京`. El de `days` incluye el símbolo `東京` compartido entre los dos días y el `NULL` de `id`.

`cargo tree -p qdb-server` solo muestra los cinco crates del workspace.

## Limitaciones

- `PARTITION BY NONE` y `DAY`, WAL desactivado, `_meta` versión 426, ficheros de columna sin sufijo de txn.
- HOUR, MONTH, YEAR, WEEK, Parquet y particiones composite se rechazan.
- La última partición usa `transient_row_count`. El tamaño guardado en su registro puede ser 0.
- `LIMIT`, `ORDER BY`, `JOIN`, `LEFT JOIN`, `RIGHT JOIN`, `FULL JOIN`, `CROSS JOIN`, `AS`, `count()`, `min()`, `max()`, `sum()`, `avg()`, `first()`, `last()`, `count_distinct()`, `concat()`, `||`, `length()`, `lower()`, `upper()`, `left()`, `right()`, `substring()`, `trim()`, `ltrim()`, `rtrim()`, `replace()`, `starts_with()`, `strpos()`, `position()`, `split_part()`, `to_lowercase()`, `to_uppercase()`, `lpad()`, `rpad()`, `quote_ident()`, `length_bytes()`, `size_pretty()`, `abs()`, `round()`, `floor()`, `ceil()`, `ceiling()`, `sign()`, `sqrt()`, `power()`, `ln()`, `log()`, `exp()`, `pi()`, `greatest()`, `least()`, `round_down()`, `round_up()`, `round_half_even()`, `coalesce()`, `nullif()`, `cast()`, `dateadd()`, `datediff()`, `year()`, `month()`, `day()`, `hour()`, `minute()`, `second()`, `millis()`, `micros()`, `nanos()`, `to_utc()`, `to_timezone()`, `to_pg_date()`, `current_schema()`, `current_database()`, `current_user()`, `session_user()`, `current_catalog()`, `version()`, `pg_is_in_recovery()`, `format_type()`, `regexp_replace()`, `current_setting()`, `pg_get_partkeydef()`, `pg_get_expr()`, `netmask()`, `geo_distance_meters()`, `geo_within_radius_latlon()`, `within_radius()`, `within_box()`, `make_geohash()`, `to_uuid()`, `json_extract()`, `to_char()`, `pg_catalog.version()`, `pg_catalog.pg_get_userbyid()`, `pg_catalog.pg_table_is_visible()`, `pg_catalog.age()`, `pg_catalog.current_schema()`, `pg_catalog.current_database()`, `pg_catalog.pg_is_in_recovery()`, `pg_catalog.pg_get_partkeydef()`, `pg_catalog.pg_get_expr()`, `long_sequence()`, `generate_series()`, `day_of_week()`, `week_of_year()`, `date_trunc()`, `timestamp_floor()`, `timestamp_ceil()`, `is_leap_year()`, `days_in_month()`, `to_str()`, `is_end_of_month()`, `day_of_week_sunday_first()`, `to_timestamp()` de un texto numérico o con un patrón constante, `extract()`, `to_date()`, `md5()`, `sha1()`, `sha256()`, `sin()`, `cos()`, `tan()`, `asin()`, `acos()`, `atan()`, `atan2()`, `degrees()`, `radians()`, `cot()`, `typeof()`, `not()`, `stddev()`, `variance()`, `var_pop()`, `stddev_pop()`, `ksum()`, `nsum()`, `approx_count_distinct()`, `string_agg()`, `covar_pop()`, `covar_samp()`, `corr()`, `regr_slope()`, `regr_intercept()`, `regr_r2()`, `stddev_samp()`, `var_samp()`, `bool_and()`, `bool_or()`, `first_not_null()`, `last_not_null()`, `bit_and()`, `bit_or()`, `bit_xor()`, `geomean()`, `skewness()`, `mode()`, `approx_median()`, `approx_percentile()`, `arg_max()`, `arg_min()`, `string_distinct_agg()`, `kurtosis_pop()`, `kurtosis_samp()`, `kurtosis()`, `skewness_pop()`, `vwap()`, `twap()`, `sparkline()`, `weighted_avg()`, `weighted_stddev()`, `weighted_stddev_rel()`, `weighted_stddev_freq()`, `isOrdered()`, `percentile_disc()`, `percentile_cont()`, `skewness_samp()`, `quantile_cont()`, `quantile_disc()`, `glob()`, `ilike()`, `like()`, `switch()`, `bar()`, `haversine_dist_deg()`, `timestamp_sequence()`, `mid()`, `spread()`, `spread_bps()`, `wmid()`, `between()`, `in`, `=`, `<>`, `<`, `>`, `>=`, `<=`, `+`, `-`, `*`, `/`, `%`, `-value`, `~value`, `and`, `or`, `||`, `IS NULL`, `IS NOT NULL`, `BETWEEN`, `NOT BETWEEN`, `LIKE`, `NOT LIKE`, `ILIKE`, `NOT ILIKE`, `~`, `!~`, `&`, `|`, `^`, `=`, `<>`, `!=`, `<`, `>`, `>=`, `<=`, `::string`, `::long`, `::int`, `::double`, `::float`, `::boolean`, `::byte`, `::short`, `::char`, `::symbol`, `::varchar`, `::date`, `::timestamp` y `GROUP BY` de una columna están verificados. Un `SELECT` admite un solo agregado. El resto de SQL no.
- Sin escritura, sin red, sin el resto de tipos ni de SQL.
- BOOLEAN/BYTE/SHORT/CHAR no separan NULL de 0; es el formato upstream.
- Maven no está instalado, así que la suite Java no se ejecutó (`docs/blockers.md`).

## 2026-11-14 — skills del proyecto

Añadidas cuatro skills locales en `.agents/skills/`: `qdb-port`, `qdb-oracle`, `qdb-storage` y `qdb-status`, con guía de uso en `docs/skills.md` y enlace desde `AGENTS.md`. Recogen los procedimientos del repositorio y distinguen la comparación TSV capturada de las cuentas de filas del runner vivo. Este cambio no amplía la compatibilidad del motor.

Comprobaciones de esta sesión:

- `tools/check.sh` terminó con código 0: formato, Clippy y 273 tests.
- El validador `quick_validate.py` de `skill-creator` aceptó las cuatro skills. Se comprobaron además nombres iguales a sus directorios, enlaces locales y ausencia de plantillas pendientes. Los casos de selección de `docs/skills.md` se revisaron manualmente; la activación automática en una sesión nueva de Codex no se probó.
- `mkdir -p target/oracle-tmp` y `TMPDIR="$PWD/target/oracle-tmp" tools/run_differential.sh` terminaron con código 0: 3203 consultas de h1 y 376 de h2 coinciden con los TSV capturados. El runner informó `live-reference: NOT RUNNING`; no se ejecutó comparación viva. Log local ignorado: `target/skills-differential.log`.

## 2026-11-14 — `trim` con columna cualificada

`strip_alias_refs` ya quita el alias de tabla de los argumentos de `trim`, como ya hacía para otras funciones de texto. La etiqueta de salida conserva `trim(s.note)`. Se añadió `tests/fixtures/h1/expected/trim_qualified_note.tsv` con la respuesta de QuestDB 10.1.0 para `SELECT trim(s.note) FROM sensors AS s`: columna `STRING`, filas `alpha`, NULL, cadena vacía y `βeta`. El test `trim_matches_captured_exec` consume esa captura y `tools/run_differential.sh` incluye el caso.

Verificación: `tools/check.sh` terminó con código 0 (273 tests). Las tres consultas se observaron por `/exec` en el binario 10.1.0; el runner diferencial terminó con código 0: 3206 consultas h1 y 376 h2 coinciden con capturas. El tramo de referencia viva no estaba activo durante ese runner.

## 2026-11-14 — `ltrim` y `rtrim` con columna cualificada

Las dos funciones usan el mismo recorrido de `Proj::Trim`, así que el arreglo del alias cubre ambas. Se añadieron capturas independientes de `/exec` para `SELECT ltrim(s.note)` y `SELECT rtrim(s.note)` sobre `sensors AS s`. QuestDB declara `STRING` en ambos casos y devuelve `alpha`, NULL, cadena vacía y `βeta`. `ltrim_rtrim_matches_captured_exec` y el runner cubren las salidas.

Verificación: `cargo test -p qdb-sql ltrim_rtrim_matches_captured_exec --offline` y `tools/check.sh` terminaron con código 0 (suite: 273 tests). El runner terminó con código 0: 3206 consultas h1 y 376 h2 coinciden con capturas; el tramo vivo no estaba activo.

## Próxima tarea

## 2026-11-14 — `right` con columna cualificada

Se comprobó `SELECT right(s.note, 2) FROM sensors AS s` contra QuestDB 10.1.0 (`STRING`, encabezado `right(s.note, 2)`, valores `ha`, NULL, cadena vacía y `ta`). La función comparte la ruta `Proj::Side`, que ya quita el alias. Se añadió la captura a `expected/right_qualified_note.tsv`, a `left_right_matches_captured_exec` y al runner diferencial.

Verificación: `cargo test -p qdb-sql right_matches_captured_exec --offline` y `tools/check.sh` terminaron con código 0 (273 tests). El runner confirmó 3207 consultas h1 y 376 h2 contra capturas. La comparación viva no estaba activa.

## 2026-11-14 — `starts_with` con columna cualificada

Se añadió el manejo de `Proj::StartsWith` al retiro del alias de tabla. La captura compara `starts_with(s.note, 'a')` y el prefijo vacío: QuestDB declara `BOOLEAN`; el texto NULL da `false`, y el prefijo vacío devuelve `true` para cadena vacía y `βeta`. El caso está en `starts_with_matches_captured_exec` y en el runner.

Verificación: el test enfocado y `tools/check.sh` terminaron con código 0 (273 tests). El runner terminó con código 0: 3208 consultas h1 y 376 h2 coinciden con capturas; comparación viva no activa.

## Próxima tarea

## 2026-11-14 — `replace` con columna cualificada

Se añadió el manejo de `Proj::Replace` al retiro del alias. `/exec` confirma `STRING`, encabezado `replace(s.note, 'a', 'A')` y los valores `AlphA`, NULL, cadena vacía y `βetA`. La captura está en `rep_qualified_note.tsv`; `replace_matches_captured_exec` y el runner diferencial la comprueban.

Verificación: `cargo test -p qdb-sql replace_matches_captured_exec --offline` y `tools/check.sh` terminaron con código 0 (273 tests). El runner terminó con código 0: 3209 consultas h1 y 376 h2 coinciden con capturas; no había referencia viva activa.

## Próxima tarea

## 2026-11-14 — `regexp_replace` con columna cualificada

Se añadió el retiro del alias para todos los argumentos de `Proj::RegexpReplace`. `/exec` confirma `STRING`, encabezado `regexp_replace(s.note, 'a', 'A')` y las filas `AlphA`, NULL, cadena vacía y `βetA`. El caso tiene captura y regresión en `rr_matches_captured_exec`, además del runner.

Verificación: `cargo test -p qdb-sql rr_matches_captured_exec --offline` y `tools/check.sh` terminaron con código 0 (273 tests). El runner terminó con código 0: 3210 consultas h1 y 376 h2 coinciden con capturas; no había referencia viva activa.

## Próxima tarea

## 2026-11-14 — `strpos` con columna cualificada

Se añadió el retiro del alias para `Proj::StrPos`. `/exec` confirmó tipo `INT`, encabezados cualificados y las posiciones para `strpos(s.note, 'a')` y `strpos(s.note, '')`: `1/1`, NULL/NULL, `0/1` y `4/1`. La captura `sp_qualified_note.tsv` está en el test y el runner diferencial.

Verificación: `cargo test -p qdb-sql strpos_matches_captured_exec --offline` y `tools/check.sh` terminaron con código 0 (273 tests). El runner terminó con código 0: 3211 consultas h1 y 376 h2 coinciden con capturas; no había referencia viva activa.

## Próxima tarea

## 2026-11-14 — `position` con columna cualificada

Se reutilizó la ruta `Proj::StrPos`, ya cubierta para aliases. La sintaxis soportada por este parser y verificada contra QuestDB es `position(s.note, 'a')`; el caso captura también la búsqueda vacía. `/exec` declara `INT` y devuelve `1/1`, NULL/NULL, `0/1` y `4/1`. Se añadió a `strpos_matches_captured_exec` y a los dos tramos de h1 del runner.

Verificación: `cargo test -p qdb-sql strpos_matches_captured_exec --offline` y `tools/check.sh` terminaron con código 0 (273 tests). El runner terminó con código 0: 3212 consultas h1 y 376 h2 coinciden con capturas; no había referencia viva activa.

## Próxima tarea

## 2026-11-14 — `split_part` con columna cualificada

Se añadió el retiro del alias a los argumentos de `Proj::SplitPart`. `/exec` confirmó dos columnas `STRING`, `split_part(s.note, 'a', 1)` y `(s.note, 'a', 2)`, con filas `('', 'lph')`, `(NULL, NULL)`, `('', '')` y `('βet', '')`. Se añadió la captura `part_qualified_note.tsv` al test y al runner.

Verificación: `cargo test -p qdb-sql split_part_matches_captured_exec --offline` y `tools/check.sh` terminaron con código 0 (273 tests). El runner terminó con código 0: 3213 consultas h1 y 376 h2 coinciden con capturas; la comparación viva no estaba activa.

## Próxima tarea

## 2026-11-14 — `quote_ident` con columna cualificada

Se añadió el retiro del alias para el argumento de `Proj::QuoteIdent`. `/exec` confirma tipo `STRING`, encabezado `quote_ident(s.note)` y valores `alpha`, NULL, cadena vacía y `βeta`. La captura está en `qi_qualified_note.tsv`, consumida por `quote_ident_matches_captured_exec` y el runner.

Verificación: `cargo test -p qdb-sql quote_ident_matches_captured_exec --offline` y `tools/check.sh` terminaron con código 0 (273 tests). El runner terminó con código 0: 3214 consultas h1 y 376 h2 coinciden con capturas; no había referencia viva activa.

## Próxima tarea

## 2026-11-14 — `size_pretty` con columna cualificada

Se añadió el retiro del alias para `Proj::SizePretty`. `/exec` confirma tipo `STRING`, encabezado `size_pretty(s.id)` y valores `1.0 B`, `2.0 B`, NULL y `-0.0 EiB`. El caso quedó capturado en `sz_qualified_id.tsv` y agregado al test y runner.

Verificación: `cargo test -p qdb-sql size_pretty_matches_captured_exec --offline` y `tools/check.sh` terminaron con código 0 (273 tests). El runner terminó con código 0: 3215 consultas h1 y 376 h2 coinciden con capturas; la comparación viva no estaba activa.

## Próxima tarea

## 2026-11-14 — `abs` con columna cualificada

Se añadió el retiro del alias para `Proj::Abs`. `/exec` confirma tipo `LONG`, encabezado `abs(s.id)` y valores `1`, `2`, NULL y `1`. La captura `abs_qualified_id.tsv` está cubierta por el test y el runner.

Verificación: `cargo test -p qdb-sql abs_matches_captured_exec --offline` y `tools/check.sh` terminaron con código 0 (273 tests). El runner terminó con código 0: 3216 consultas h1 y 376 h2 coinciden con capturas; la comparación viva no estaba activa.

## Próxima tarea

## 2026-11-14 — `round` con columna cualificada

Se añadió el retiro del alias para los argumentos de `Proj::Round`. `/exec` confirma `DOUBLE`, encabezado `round(s.reading)` y valores `2.0`, NULL, `0.0`, `0.0`; el `-0.0` de entrada da `0.0`. La captura está cubierta en el test y el runner.

Verificación: `cargo test -p qdb-sql round_matches_captured_exec --offline` y `tools/check.sh` terminaron con código 0 (273 tests). El runner terminó con código 0: 3217 consultas h1 y 376 h2 coinciden con capturas; no había referencia viva activa.

## Próxima tarea

## 2026-11-14 — `round_down` con columna cualificada

`strip_alias_refs` ahora elimina el alias de tabla en los argumentos de `Proj::RoundDown`. `/exec` confirmó tipo `DOUBLE`, encabezado `round_down(s.reading, 1)`, NULL en la segunda fila y conservación de `-0.0` en la cuarta. La captura `rdn_qualified_reading.tsv` se consume desde `round_down_matches_captured_exec` y `tools/run_differential.sh`.

Verificación: `cargo test -p qdb-sql round_down_matches_captured_exec --offline` y `tools/check.sh` terminaron con código 0 (263 tests de qdb-sql). El runner terminó con código 0: 3218 consultas h1 y 376 h2 coinciden con capturas; la referencia viva no estaba activa.

## Próxima tarea

## 2026-11-14 — `round_up` con columna cualificada

`strip_alias_refs` también elimina el alias en los argumentos de `Proj::RoundUp`. `/exec` confirmó tipo `DOUBLE`, encabezado `round_up(s.reading, 1)`, NULL en la segunda fila y conservación de `-0.0` en la cuarta. La captura `rup_qualified_reading.tsv` se consume desde `round_up_matches_captured_exec` y `tools/run_differential.sh`.

Verificación: `cargo test -p qdb-sql round_up_matches_captured_exec --offline` y `tools/check.sh` terminaron con código 0 (263 tests de qdb-sql). El runner terminó con código 0: 3219 consultas h1 y 376 h2 coinciden con capturas; la referencia viva no estaba activa.

## Próxima tarea

## 2026-11-14 — `round_half_even` con columna cualificada

`strip_alias_refs` ahora procesa los argumentos de `Proj::RoundHalfEven`. `/exec` confirmó tipo `DOUBLE`, encabezado `round_half_even(s.reading, 1)`, NULL en la segunda fila y `-0.0` en las dos últimas. La captura `rhe_qualified_reading.tsv` se consume desde `round_half_even_matches_captured_exec` y el runner.

Verificación: `cargo test -p qdb-sql round_half_even_matches_captured_exec --offline` y `tools/check.sh` terminaron con código 0 (263 tests de qdb-sql). El runner terminó con código 0: 3220 consultas h1 y 376 h2 coinciden con capturas; la referencia viva no estaba activa.

## Próxima tarea

## 2026-11-14 — `greatest` con columnas cualificadas

`strip_alias_refs` retira el alias de cada argumento de `Proj::Greatest`. `/exec` confirmó tipo `DOUBLE`, encabezado `greatest(s.reading, s.score)`, NULL en la segunda fila y `-0.0` en la última. La captura `gt_qualified_reading_score.tsv` se consume desde `greatest_matches_captured_exec` y el runner diferencial.

Verificación: `cargo test -p qdb-sql greatest_matches_captured_exec --offline` y `tools/check.sh` terminaron con código 0 (263 tests de qdb-sql). El runner terminó con código 0: 3221 consultas h1 y 376 h2 coinciden con capturas; la referencia viva no estaba activa.

## Próxima tarea

## 2026-11-14 — `least` con columnas cualificadas

`strip_alias_refs` también retira el alias de los argumentos de `Proj::Least`. `/exec` confirmó tipo `DOUBLE`, encabezado `least(s.reading, s.score)`, NULL en la segunda fila y resultados `0.0` y `-1.5` en las últimas filas. La captura `ls_qualified_reading_score.tsv` se consume desde `least_matches_captured_exec` y el runner.

Verificación: `cargo test -p qdb-sql least_matches_captured_exec --offline` y `tools/check.sh` terminaron con código 0 (263 tests de qdb-sql). El runner terminó con código 0: 3222 consultas h1 y 376 h2 coinciden con capturas; la referencia viva no estaba activa.

## Próxima tarea

## 2026-11-14 — `coalesce` con columnas cualificadas

`strip_alias_refs` ahora elimina los alias de los argumentos de `Proj::Coalesce`. `/exec` confirmó tipo `DOUBLE`, encabezado `coalesce(s.reading, s.score)`, NULL en la segunda fila y conservación de `-0.0` en la última. La captura `ca_qualified_reading_score.tsv` se consume desde `coalesce_matches_captured_exec` y el runner.

Verificación: `cargo test -p qdb-sql coalesce_matches_captured_exec --offline` y `tools/check.sh` terminaron con código 0 (263 tests de qdb-sql). El runner terminó con código 0: 3223 consultas h1 y 376 h2 coinciden con capturas; la referencia viva no estaba activa.

## Próxima tarea

## 2026-11-14 — `nullif` con columnas cualificadas

`strip_alias_refs` ahora retira el alias de los argumentos de `Proj::NullIf`. `/exec` confirmó tipo `DOUBLE`, encabezado `nullif(s.reading, s.score)`, NULL cuando los dos valores son iguales y `-0.0` en la última fila. La captura `ni_qualified_reading_score.tsv` se consume desde `nullif_matches_captured_exec` y el runner diferencial.

Verificación: `cargo test -p qdb-sql nullif_matches_captured_exec --offline` y `tools/check.sh` terminaron con código 0 (263 tests de qdb-sql). El runner terminó con código 0: 3224 consultas h1 y 376 h2 coinciden con capturas; la referencia viva no estaba activa.

## Próxima tarea

## 2026-11-14 — `glob`, `like` e `ilike` con columna cualificada

Se añadieron las referencias cualificadas de `Proj::Glob`, `Proj::Like` y `Proj::Ilike` a `strip_alias_refs`, con capturas `/exec` para `s.note`. QuestDB devuelve los encabezados `s.note like 'a%'` y `s.note ilike 'A%'`. La comparación reveló que el port rotulaba las llamadas `ilike(...)` con sintaxis de función; se ajustó el rótulo a la forma `valor ilike patrón` que usa QuestDB y se corrigieron los encabezados capturados afectados en h1 y h2.

Verificación: las pruebas focales `glob_matches_captured_exec`, `like_matches_captured_exec` e `ilike_matches_captured_exec` pasan; `tools/check.sh` terminó con código 0 (263 tests de qdb-sql). El runner terminó con código 0: 3227 consultas h1 y 376 h2 coinciden con capturas; la referencia viva no estaba activa en el runner. Los casos cualificados y las variantes ILIKE normalizadas se contrastaron directamente con `/exec`.

## Próxima tarea

## 2026-11-14 — `switch` con columna cualificada

`strip_alias_refs` ahora elimina alias en los argumentos de `Proj::Switch`. `/exec` confirmó tipo `STRING`, encabezado `switch(s.id, 1, 'one', 'other')`, y valores `one`, `other`, `other`, `other`. La captura `sx_qualified_id.tsv` se verifica en `switch_matches_captured_exec` y el runner.

Verificación: `cargo test -p qdb-sql switch_matches_captured_exec --offline` y `tools/check.sh` terminaron con código 0 (263 tests de qdb-sql). El runner terminó con código 0: 3228 consultas h1 y 376 h2 coinciden con capturas; la referencia viva no estaba activa.

## Próxima tarea

## 2026-11-14 — `bar` con columna cualificada

`strip_alias_refs` elimina el alias de los argumentos de `Proj::Bar`. `/exec` confirmó tipo `VARCHAR`, encabezado `bar(s.flags, 0, 42, 4)` y valores `▉`, NULL, cadena vacía y `████`. La captura `bn_qualified_flags.tsv` se verifica en `bar_matches_captured_exec` y el runner.

Verificación: `cargo test -p qdb-sql bar_matches_captured_exec --offline` y `tools/check.sh` terminaron con código 0 (263 tests de qdb-sql). El runner terminó con código 0: 3229 consultas h1 y 376 h2 coinciden con capturas; la referencia viva no estaba activa.

## Próxima tarea

## 2026-11-14 — `timestamp_sequence` con columna inicial cualificada

`strip_alias_refs` elimina el alias de los argumentos de `Proj::TimestampSequence`. `/exec` confirmó tipo `TIMESTAMP`, encabezado `timestamp_sequence(s.ts, 1)` y una secuencia que avanza un microsegundo por fila desde `s.ts`. El caso está capturado en `tq_qualified_ts.tsv`, cubierto por `timestamp_sequence_matches_captured_exec` y el runner.

Verificación: `cargo test -p qdb-sql timestamp_sequence_matches_captured_exec --offline` y `tools/check.sh` terminaron con código 0 (263 tests de qdb-sql). El runner terminó con código 0: 3230 consultas h1 y 376 h2 coinciden con capturas; la referencia viva no estaba activa.

## Próxima tarea

## 2026-11-14 — `lpad` con columna cualificada

`strip_alias_refs` retira el alias de los argumentos de `Proj::Pad`. `/exec` confirmó tipo `STRING`, encabezado `lpad(s.note, 5, '_')`, NULL conservado y relleno de `''` y `βeta` como `_____` y `_βeta`. La captura `lp_qualified_note.tsv` se consume desde `pad_matches_captured_exec` y el runner.

Verificación: `cargo test -p qdb-sql pad_matches_captured_exec --offline` y `tools/check.sh` terminaron con código 0 (263 tests de qdb-sql). El runner terminó con código 0: 3231 consultas h1 y 376 h2 coinciden con capturas; la referencia viva no estaba activa.

## Próxima tarea

## 2026-11-14 — `rpad` con columna cualificada

La cobertura de `Proj::Pad` incluye ahora ambas direcciones. `/exec` confirmó tipo `STRING`, encabezado `rpad(s.note, 5, '_')`, NULL conservado y salida `_____` y `βeta_` para los valores vacío y Unicode. La captura `rp_qualified_note.tsv` se verifica desde `pad_matches_captured_exec` y el runner.

Verificación: `cargo test -p qdb-sql pad_matches_captured_exec --offline` y `tools/check.sh` terminaron con código 0 (263 tests de qdb-sql). El runner terminó con código 0: 3232 consultas h1 y 376 h2 coinciden con capturas; la referencia viva no estaba activa.

## Próxima tarea

## 2026-11-14 — `dateadd` con timestamp cualificado

`strip_alias_refs` elimina el alias de los argumentos de `Proj::DateAdd`. `/exec` confirmó tipo `TIMESTAMP`, encabezado `dateadd('d', 1, s.ts)` y desplazamiento de un día que conserva microsegundos. La captura `da_qualified_ts.tsv` se consume desde `dateadd_matches_captured_exec` y el runner.

Verificación: `cargo test -p qdb-sql dateadd_matches_captured_exec --offline` y `tools/check.sh` terminaron con código 0 (263 tests de qdb-sql). El runner terminó con código 0: 3233 consultas h1 y 376 h2 coinciden con capturas; la referencia viva no estaba activa.

## Próxima tarea

## 2026-11-14 — `datediff` con timestamps cualificados

`strip_alias_refs` elimina alias de los argumentos de `Proj::DateDiff`. `/exec` confirmó tipo `LONG`, encabezado `datediff('d', s.ts, s.ts)` y cuatro ceros para timestamps iguales. La captura `df_qualified_ts.tsv` se verifica desde `datediff_matches_captured_exec` y el runner diferencial.

Verificación: `cargo test -p qdb-sql datediff_matches_captured_exec --offline` y `tools/check.sh` terminaron con código 0 (263 tests de qdb-sql). El runner terminó con código 0: 3234 consultas h1 y 376 h2 coinciden con capturas; la referencia viva no estaba activa.

## Próxima tarea

## 2026-11-14 — `year` con timestamp cualificado

`strip_alias_refs` elimina alias de los argumentos de `Proj::Year`. `/exec` confirmó tipo `INT`, encabezado `year(s.ts)` y valor 2020 en las cuatro filas. La captura `yr_qualified_ts.tsv` se verifica desde `year_matches_captured_exec` y el runner.

Verificación: `cargo test -p qdb-sql year_matches_captured_exec --offline` y `tools/check.sh` terminaron con código 0 (263 tests de qdb-sql). El runner terminó con código 0: 3235 consultas h1 y 376 h2 coinciden con capturas; la referencia viva no estaba activa.

## Próxima tarea

## 2026-11-14 — `month` con timestamp cualificado

`strip_alias_refs` elimina alias de los argumentos de `Proj::Month`. `/exec` confirmó tipo `INT`, encabezado `month(s.ts)` y valor 1 en las cuatro filas. La captura `mo_qualified_ts.tsv` está cubierta por `month_matches_captured_exec` y el runner.

Verificación: `cargo test -p qdb-sql month_matches_captured_exec --offline` y `tools/check.sh` terminaron con código 0 (263 tests de qdb-sql). El runner terminó con código 0: 3236 consultas h1 y 376 h2 coinciden con capturas; la referencia viva no estaba activa.

## Próxima tarea

## 2026-11-14 — `day` con timestamp cualificado

`strip_alias_refs` elimina alias de los argumentos de `Proj::Day`. `/exec` confirmó tipo `INT`, encabezado `day(s.ts)` y valor 2 en las cuatro filas. La captura `dy_qualified_ts.tsv` se verifica desde `day_matches_captured_exec` y el runner.

Verificación: `cargo test -p qdb-sql day_matches_captured_exec --offline` y `tools/check.sh` terminaron con código 0 (263 tests de qdb-sql). El runner terminó con código 0: 3237 consultas h1 y 376 h2 coinciden con capturas; la referencia viva no estaba activa.

## 2026-11-14 — Partes de tiempo con timestamp cualificado

`strip_alias_refs` cubre `Proj::Hour`, `Minute`, `Second`, `Millis`, `Micros` y `Nanos`. `/exec` confirmó tipo `INT`; los valores son hora 3, minuto 4, segundos 5–8, y milisegundos, microsegundos y nanosegundos 0 en este fixture. Las capturas `_qualified_ts.tsv` de cada función se prueban y se comparan desde el runner.

Verificación: las seis pruebas focales terminaron con código 0; `tools/check.sh` terminó con código 0 (263 tests de qdb-sql). El runner terminó con código 0: 3243 consultas h1 y 376 h2 coinciden con capturas; la referencia viva no estaba activa.

## Próxima tarea

## 2026-11-14 — `day_of_week` con timestamp cualificado

`strip_alias_refs` elimina el alias de `Proj::DayOfWeek`. `/exec` confirmó tipo `INT`, encabezado `day_of_week(s.ts)` y valor 4 para jueves (lunes = 1). La captura `dw_qualified_ts.tsv` está cubierta por `day_of_week_matches_captured_exec` y el runner.

Verificación: `cargo test -p qdb-sql day_of_week_matches_captured_exec --offline` y `tools/check.sh` terminaron con código 0 (263 tests de qdb-sql). El runner terminó con código 0: 3244 consultas h1 y 376 h2 coinciden con capturas; la referencia viva no estaba activa.

## Próxima tarea

## 2026-11-14 — `week_of_year` con timestamp cualificado

`strip_alias_refs` retira el alias de `Proj::WeekOfYear`. `/exec` confirmó tipo `INT`, encabezado `week_of_year(s.ts)` y semana 1 para las cuatro filas. La captura `wy_qualified_ts.tsv` se verifica en `week_of_year_matches_captured_exec` y el runner.

Verificación: `cargo test -p qdb-sql week_of_year_matches_captured_exec --offline` y `tools/check.sh` terminaron con código 0 (263 tests de qdb-sql). El runner terminó con código 0: 3245 consultas h1 y 376 h2 coinciden con capturas; la referencia viva no estaba activa.

## Próxima tarea

## 2026-11-14 — `date_trunc` con timestamp cualificado

`strip_alias_refs` retira el alias de los argumentos de `Proj::DateTrunc`. `/exec` confirmó tipo `TIMESTAMP`, encabezado `date_trunc('day', s.ts)` y truncamiento de cada fila a medianoche UTC del mismo día. La captura `dt_qualified_ts.tsv` se verifica en `date_trunc_matches_captured_exec` y el runner.

Verificación: `cargo test -p qdb-sql date_trunc_matches_captured_exec --offline` y `tools/check.sh` terminaron con código 0 (263 tests de qdb-sql). El runner terminó con código 0: 3246 consultas h1 y 376 h2 coinciden con capturas; la referencia viva no estaba activa.

## Próxima tarea

## 2026-11-14 — `timestamp_floor` con timestamp cualificado

`strip_alias_refs` retira el alias de `Proj::TimestampFloor`. `/exec` confirmó tipo `TIMESTAMP`, encabezado `timestamp_floor('d', s.ts)` y truncamiento a medianoche UTC. La captura `tf_qualified_ts.tsv` se verifica en `timestamp_floor_matches_captured_exec` y el runner.

Verificación: `cargo test -p qdb-sql timestamp_floor_matches_captured_exec --offline` y `tools/check.sh` terminaron con código 0 (263 tests de qdb-sql). El runner terminó con código 0: 3247 consultas h1 y 376 h2 coinciden con capturas; la referencia viva no estaba activa.

## Próxima tarea

## 2026-11-14 — `timestamp_ceil` con timestamp cualificado

`strip_alias_refs` retira el alias de `Proj::TimestampCeil`. `/exec` confirmó tipo `TIMESTAMP`, encabezado `timestamp_ceil('d', s.ts)` y redondeo de cada timestamp del fixture a la medianoche del día siguiente. La captura `tc_qualified_ts.tsv` se verifica desde `timestamp_ceil_matches_captured_exec` y el runner.

Verificación: `cargo test -p qdb-sql timestamp_ceil_matches_captured_exec --offline` y `tools/check.sh` terminaron con código 0 (263 tests de qdb-sql). El runner terminó con código 0: 3248 consultas h1 y 376 h2 coinciden con capturas; la referencia viva no estaba activa.

## Próxima tarea

## 2026-11-14 — `is_leap_year` con timestamp cualificado

`strip_alias_refs` retira el alias de `Proj::IsLeapYear`. `/exec` confirmó tipo `BOOLEAN`, encabezado `is_leap_year(s.ts)` y `true` en todas las filas del año 2020. La captura `ly_qualified_ts.tsv` se verifica en `is_leap_year_matches_captured_exec` y el runner.

Verificación: `cargo test -p qdb-sql is_leap_year_matches_captured_exec --offline` y `tools/check.sh` terminaron con código 0 (263 tests de qdb-sql). El runner terminó con código 0: 3249 consultas h1 y 376 h2 coinciden con capturas; la referencia viva no estaba activa.

## Próxima tarea

## 2026-11-14 — `days_in_month` con timestamp cualificado

`strip_alias_refs` retira el alias de `Proj::DaysInMonth`. `/exec` confirmó tipo `INT`, encabezado `days_in_month(s.ts)` y 31 días en todas las filas de enero. La captura `dm_qualified_ts.tsv` se verifica en `days_in_month_matches_captured_exec` y el runner.

Verificación: `cargo test -p qdb-sql days_in_month_matches_captured_exec --offline` y `tools/check.sh` terminaron con código 0 (263 tests de qdb-sql). El runner terminó con código 0: 3250 consultas h1 y 376 h2 coinciden con capturas; la referencia viva no estaba activa.

## Próxima tarea

## 2026-11-14 — `is_end_of_month` con timestamp cualificado

`strip_alias_refs` retira el alias de `Proj::IsEndOfMonth`. `/exec` confirmó tipo `BOOLEAN`, encabezado `is_end_of_month(s.ts)` y `false` para las cuatro fechas de enero 2. La captura `em_qualified_ts.tsv` se verifica en `is_end_of_month_matches_captured_exec` y el runner.

Verificación: `cargo test -p qdb-sql is_end_of_month_matches_captured_exec --offline` y `tools/check.sh` terminaron con código 0 (263 tests de qdb-sql). El runner terminó con código 0: 3251 consultas h1 y 376 h2 coinciden con capturas; la referencia viva no estaba activa.

## Próxima tarea

## 2026-11-14 — `day_of_week_sunday_first` con timestamp cualificado

`strip_alias_refs` elimina el alias de `Proj::DayOfWeekSundayFirst`. `/exec` confirmó tipo `INT`, encabezado `day_of_week_sunday_first(s.ts)` y valor 5 para jueves (domingo = 1). La captura `ds_qualified_ts.tsv` se verifica desde `day_of_week_sunday_first_matches_captured_exec` y el runner.

Verificación: `cargo test -p qdb-sql day_of_week_sunday_first_matches_captured_exec --offline` y `tools/check.sh` terminaron con código 0 (263 tests de qdb-sql). El runner terminó con código 0: 3252 consultas h1 y 376 h2 coinciden con capturas; la referencia viva no estaba activa.

## Próxima tarea

## 2026-11-14 — `to_utc` con timestamp cualificado

`strip_alias_refs` retira el alias de los argumentos de `Proj::ToUtc`. `/exec` confirmó tipo `TIMESTAMP`, encabezado `to_utc(s.ts, 'UTC')` y los mismos instantes en UTC. La captura `uc_qualified_ts.tsv` está cubierta por `uc_matches_captured_exec` y el runner.

Verificación: `cargo test -p qdb-sql uc_matches_captured_exec --offline` y `tools/check.sh` terminaron con código 0 (263 tests de qdb-sql). El runner terminó con código 0: 3253 consultas h1 y 376 h2 coinciden con capturas; la referencia viva no estaba activa.

## Próxima tarea

## 2026-11-14 — `to_timezone` con timestamp cualificado

`strip_alias_refs` retira el alias de los argumentos de `Proj::ToTimezone`. `/exec` confirmó tipo `TIMESTAMP`, encabezado `to_timezone(s.ts, 'Europe/Madrid')` y hora mostrada adelantada una hora en enero. La captura `tz_qualified_ts.tsv` se verifica desde `tz_matches_captured_exec` y el runner.

Verificación: `cargo test -p qdb-sql tz_matches_captured_exec --offline` y `tools/check.sh` terminaron con código 0 (263 tests de qdb-sql). El runner terminó con código 0: 3254 consultas h1 y 376 h2 coinciden con capturas; la referencia viva no estaba activa.

## Próxima tarea

## 2026-11-14 — `to_pg_date` con texto cualificado

La consulta candidata con timestamp fue rechazada por QuestDB (`TIMESTAMP` no encaja); se probó el tipo real admitido, `STRING`. `strip_alias_refs` elimina alias de `Proj::ToPgDate`. `/exec` confirmó tipo `DATE`, encabezado `to_pg_date(s.note)` y NULL para los cuatro textos del fixture. La captura `pg_qualified_note.tsv` se verifica desde `pg_matches_captured_exec` y el runner.

Verificación: `cargo test -p qdb-sql pg_matches_captured_exec --offline` y `tools/check.sh` terminaron con código 0 (263 tests de qdb-sql). El runner terminó con código 0: 3255 consultas h1 y 376 h2 coinciden con capturas; la referencia viva no estaba activa.

## 2026-11-14 — `to_date` con texto cualificado

`strip_alias_refs` retira el alias de `Proj::ToDate`. La respuesta viva de `/exec` para `SELECT to_date(s.note, 'yyyy-MM-dd') FROM sensors AS s ORDER BY s.id` declaró tipo `DATE`, encabezado `to_date(s.note, 'yyyy-MM-dd')` y cuatro NULL; los valores de `note` del fixture no son fechas. La salida de fecha válida conserva el formato ya capturado en `td_date.tsv` (`2020-01-02T00:00:00.000Z`). La captura `td_qualified_note.tsv` se verifica en `to_date_matches_captured_exec` y el runner.

Verificación: `cargo test -p qdb-sql to_date_matches_captured_exec --offline` y `tools/check.sh` terminaron con código 0 (263 tests de qdb-sql). `git diff --check` pasó. El runner terminó con código 0: 3256 consultas h1 y 376 h2 coinciden con capturas; la comparación viva no se ejecutó.

## Próxima tarea

## 2026-11-14 — `to_str` con timestamp cualificado

`strip_alias_refs` retira el alias de `Proj::ToStr`. La respuesta viva de `/exec` para `SELECT to_str(s.ts, 'yyyy-MM-dd HH:mm:ss.SSSUUU') FROM sensors AS s ORDER BY s.ts` declaró tipo `STRING`, encabezado `to_str(s.ts, 'yyyy-MM-dd HH:mm:ss.SSSUUU')` y las cuatro marcas `2020-01-02 03:04:05.000000` a `2020-01-02 03:04:08.000000`. La captura `tostr_qualified_ts.tsv` se verifica en `to_str_matches_captured_exec` y el runner.

Verificación: antes del arreglo, la regresión falló con `Invalid column: s.ts`; después, `cargo test -p qdb-sql to_str_matches_captured_exec --offline` y `tools/check.sh` terminaron con código 0 (263 tests de qdb-sql). `git diff --check` pasó. El runner terminó con código 0: 3257 consultas h1 y 376 h2 coinciden con capturas; la comparación viva no se ejecutó.

## Próxima tarea

## 2026-11-14 — `to_timestamp` con texto cualificado y patrón

`strip_alias_refs` retira el alias de `Proj::ToTimestamp`. La respuesta viva de `/exec` para `SELECT to_timestamp(s.note, 'yyyy-MM-dd') FROM sensors AS s ORDER BY s.id` declaró tipo `TIMESTAMP`, encabezado `to_timestamp(s.note, 'yyyy-MM-dd')` y cuatro NULL; ninguno de los textos del fixture coincide con el patrón. Las fechas válidas y su formato UTC se cubren por separado en `tp_date.tsv`. La captura `tp_qualified_note.tsv` se verifica en `to_timestamp_format_matches_captured_exec` y el runner.

Verificación: antes del arreglo, la regresión falló con `Invalid column: s.note`; después, `cargo test -p qdb-sql to_timestamp_format_matches_captured_exec --offline` y `tools/check.sh` terminaron con código 0 (263 tests de qdb-sql). `git diff --check` pasó. El runner terminó con código 0: 3258 consultas h1 y 376 h2 coinciden con capturas; la comparación viva no se ejecutó.

## Próxima tarea

## 2026-11-14 — `extract` con timestamp cualificado

`strip_alias_refs` elimina el alias de `ExtractExpr.arg` cuando es una columna. La respuesta viva de `/exec` para `SELECT extract(year FROM s.ts) FROM sensors AS s ORDER BY s.ts` declaró tipo `INT`, encabezado normalizado `extract(year, s.ts)` y valor `2020` en las cuatro filas. La captura `xt_qualified_ts.tsv` se verifica en `extract_matches_captured_exec` y el runner.

Verificación: antes del arreglo, la regresión falló con `Invalid column: s.ts`; después, `cargo test -p qdb-sql extract_matches_captured_exec --offline` y `tools/check.sh` terminaron con código 0 (263 tests de qdb-sql). `git diff --check` pasó. El runner terminó con código 0: 3259 consultas h1 y 376 h2 coinciden con capturas; la comparación viva no se ejecutó.

## Próxima tarea

## 2026-11-14 — firma candidata de `to_char` descartada

La consulta `SELECT to_char(s.ts, 'yyyy-MM-dd') FROM sensors AS s ORDER BY s.ts` fue rechazada por QuestDB 10.1.0 con el error `wrong number of arguments for function to_char; expected: 1, provided: 2`. No se añade esa forma; la función `to_char` probada en el port es el conversor binario de un argumento.

## 2026-11-14 — `typeof` con columna cualificada

`strip_alias_refs` retira el alias de los argumentos de `Proj::Typeof`. La respuesta viva de `/exec` para `SELECT typeof(s.ts) FROM sensors AS s ORDER BY s.ts` declaró tipo de salida `STRING`, encabezado `typeof(s.ts)` y `TIMESTAMP` en las cuatro filas. La captura `ty_qualified_ts.tsv` se verifica en `typeof_matches_captured_exec` y el runner.

Verificación: antes del arreglo, la regresión falló con `Invalid column: s.ts`; después, `cargo test -p qdb-sql typeof_matches_captured_exec --offline` y `tools/check.sh` terminaron con código 0 (263 tests de qdb-sql). `git diff --check` pasó. El runner terminó con código 0: 3260 consultas h1 y 376 h2 coinciden con capturas; la comparación viva no se ejecutó.

## Próxima tarea

## 2026-11-14 — `json_extract` con texto cualificado y alias

`strip_alias_refs` elimina el alias de los argumentos de `Proj::JsonExtract`. `/exec` confirmó tipo `VARCHAR`, encabezado `json_extract(s.tag, '$')` y filas NULL, NULL, NULL, `0123456789ABCDEF`. El último valor evidenció que QuestDB conserva un texto completo que empieza por dígitos aunque el parser de JSON pueda leer un prefijo numérico; `extract_json_varchar` ahora devuelve el texto entero si tras ese prefijo queda contenido no blanco. `/exec` también confirmó `json_extract('123abc', '$')` → `123abc`. Las capturas `je_qualified_tag.tsv` y `je_num_suffix.tsv` se verifican en `je_matches_captured_exec` y el runner.

Verificación: la regresión primero falló por `Invalid column: s.tag` y, tras retirar el alias, detectó la discrepancia `0123456789` frente a `0123456789ABCDEF`; ambos casos pasan con el arreglo. `cargo test -p qdb-sql je_matches_captured_exec --offline` y `tools/check.sh` terminaron con código 0 (263 tests de qdb-sql). `git diff --check` pasó. El runner terminó con código 0: 3262 consultas h1 y 376 h2 coinciden con capturas; la comparación viva no se ejecutó.

## Próxima tarea

## 2026-11-14 — `typeof(json_extract(...))` con alias anidado

Dentro de `Proj::Typeof`, `strip_alias_refs` también elimina el alias de las columnas en `TypeofExpr.json.args`. La respuesta viva de `/exec` para `SELECT typeof(json_extract(s.tag, '$')) FROM sensors AS s ORDER BY s.ts` declaró tipo de salida `VARCHAR`, encabezado `typeof(json_extract(s.tag, '$'))` y `VARCHAR` en las cuatro filas. La captura `ty_json_qualified_tag.tsv` se verifica en `typeof_matches_captured_exec` y el runner.

Verificación: antes del arreglo, la regresión falló con `Invalid column: s.tag`; después, `cargo test -p qdb-sql typeof_matches_captured_exec --offline` y `tools/check.sh` terminaron con código 0 (263 tests de qdb-sql). `git diff --check` pasó. El runner terminó con código 0: 3263 consultas h1 y 376 h2 coinciden con capturas; la comparación viva no se ejecutó.

## Próxima tarea

## 2026-11-14 — `typeof(to_uuid(...))` con alias anidado

Dentro de `Proj::Typeof`, `strip_alias_refs` también retira el alias de `TypeofExpr.uuid.args`. La respuesta viva de `/exec` para `SELECT typeof(to_uuid(s.id, s.id)) FROM sensors AS s ORDER BY s.ts` declaró tipo de salida `STRING`, encabezado `typeof(to_uuid(s.id, s.id))` y `UUID` en las cuatro filas. La captura `ty_uuid_qualified_id.tsv` se verifica en `typeof_matches_captured_exec` y el runner.

Verificación: antes del arreglo, la regresión falló con `Invalid column: s.id`; después, `cargo test -p qdb-sql typeof_matches_captured_exec --offline` y `tools/check.sh` terminaron con código 0 (263 tests de qdb-sql). `git diff --check` pasó. El runner terminó con código 0: 3264 consultas h1 y 376 h2 coinciden con capturas; la comparación viva no se ejecutó.

## Próxima tarea

## 2026-11-14 — `typeof(make_geohash(...))` con alias anidado

Dentro de `Proj::Typeof`, `strip_alias_refs` también elimina el alias de las columnas en `TypeofExpr.geohash.args`. La respuesta viva de `/exec` para `SELECT typeof(make_geohash(s.reading, 41.0, 5)) FROM sensors AS s ORDER BY s.ts` declaró tipo de salida `STRING`, encabezado `typeof(make_geohash(s.reading, 41.0, 5))` y `GEOHASH(1c)` en las cuatro filas. La captura `ty_geohash_qualified_reading.tsv` se verifica en `typeof_matches_captured_exec` y el runner.

Verificación: antes del arreglo, la regresión falló con `Invalid column: s.reading`; después, `cargo test -p qdb-sql typeof_matches_captured_exec --offline` y `tools/check.sh` terminaron con código 0 (263 tests de qdb-sql). `git diff --check` pasó. El runner terminó con código 0: 3265 consultas h1 y 376 h2 coinciden con capturas; la comparación viva no se ejecutó.

## Próxima tarea

## 2026-11-14 — `typeof(to_char(from_base64(...)))` con alias anidado

Dentro de `Proj::Typeof`, `strip_alias_refs` retira el alias de `TypeofExpr.to_char.args` y también del argumento de `FromBase64Expr` que contiene. La respuesta viva de `/exec` para `SELECT typeof(to_char(from_base64(s.tag))) FROM sensors AS s ORDER BY s.ts` declaró tipo de salida `STRING`, encabezado `typeof(to_char(from_base64(s.tag)))` y `STRING` en las cuatro filas. La captura `ty_to_char_qualified_tag.tsv` se verifica en `typeof_matches_captured_exec` y el runner.

Verificación: antes del arreglo, la regresión falló con `Invalid column: s.tag`; después, `cargo test -p qdb-sql typeof_matches_captured_exec --offline` y `tools/check.sh` terminaron con código 0 (263 tests de qdb-sql). `git diff --check` pasó. El runner terminó con código 0: 3266 consultas h1 y 376 h2 coinciden con capturas; la comparación viva no se ejecutó.

## Próxima tarea

Añadir `pg_catalog.pg_get_userbyid(s.flags)` con alias; verificar nombre, tipo y valor frente a `/exec`.

## 2026-11-14 — `pg_catalog.pg_get_userbyid` con alias

`strip_alias_refs` ahora elimina el alias de tabla de las columnas en los argumentos de `PgGetUserByIdExpr`. La respuesta viva de QuestDB 10.1.0 para `SELECT pg_catalog.pg_get_userbyid(s.flags) FROM sensors AS s ORDER BY s.ts` declaró el encabezado `pg_catalog.pg_get_userbyid(s.flags)`, tipo `STRING`, cuatro filas y el valor `public` en cada una. Se guardó la respuesta JSON completa en `target/pg_get_userbyid_qualified_flags.json`; la captura TSV `ub_qualified_flags.tsv` se comprueba en `ub_matches_captured_exec` y en el runner diferencial.

Verificación: antes del arreglo, la regresión falló con `Invalid column: s.flags`; después, `cargo test -p qdb-sql ub_matches_captured_exec --offline`, `tools/check.sh` y `git diff --check` terminaron con código 0 (263 pruebas de `qdb-sql`). El runner comparó 3267 consultas h1 y 376 h2 con capturas; su comparación viva por conteo no se ejecutó. La respuesta completa de `/exec` quedó guardada en `target/pg_get_userbyid_qualified_flags.json`. La identidad verificada del oráculo es QuestDB 10.1.0, commit `e11c837f8a169b533540bd6fa57ef3d520b68ad9`.

## Próxima tarea

Añadir `pg_catalog.pg_table_is_visible(s.flags)` con alias; verificar nombre, tipo y valor frente a `/exec`.

## 2026-11-14 — `pg_catalog.pg_table_is_visible` con alias

`strip_alias_refs` también elimina el alias de tabla de las columnas en los argumentos de `PgTableIsVisibleExpr`. La respuesta viva de QuestDB 10.1.0 para `SELECT pg_catalog.pg_table_is_visible(s.flags) FROM sensors AS s ORDER BY s.ts` declaró el encabezado `pg_catalog.pg_table_is_visible(s.flags)`, tipo `BOOLEAN`, cuatro filas y `true` en cada una. La respuesta JSON completa se guardó en `target/pg_table_is_visible_qualified_flags.json`; la captura `tv_qualified_flags.tsv` se comprueba en `tv_matches_captured_exec` y en el runner diferencial.

Verificación: antes del arreglo, la regresión falló con `Invalid column: s.flags`; después, `cargo test -p qdb-sql tv_matches_captured_exec --offline` y `git diff --check` terminaron con código 0. La identidad del oráculo coincide con QuestDB 10.1.0, commit `e11c837f8a169b533540bd6fa57ef3d520b68ad9`.

El runner volvió a pasar con 3268 capturas h1 y 376 h2; su comprobación viva de conteos no encontró el servidor activo. La respuesta completa de `/exec` quedó guardada en `target/pg_table_is_visible_qualified_flags.json`.

## Próxima tarea

Añadir `pg_catalog.age(s.id)` con alias; verificar nombre, tipo y valor frente a `/exec`.

## 2026-11-14 — `pg_catalog.age` con alias

`strip_alias_refs` también retira el alias de las columnas de los argumentos de `PgAgeExpr`. La respuesta viva de QuestDB 10.1.0 para `SELECT pg_catalog.age(s.id) FROM sensors AS s ORDER BY s.ts` declaró encabezado `pg_catalog.age(s.id)`, tipo `LONG`, cuatro filas y el valor `0` en cada una. La respuesta completa se guardó en `target/pg_age_qualified_id.json`; la captura `ag_qualified_id.tsv` se comprueba en `ag_matches_captured_exec` y en el runner diferencial.

Verificación: antes del arreglo, el test falló con `Invalid column: s.id`; después, `cargo test -p qdb-sql ag_matches_captured_exec --offline`, `tools/check.sh` (263 pruebas de `qdb-sql`), el runner y `git diff --check` terminaron con código 0. El runner comparó 3269 consultas h1 y 376 h2 con capturas; su comprobación viva de conteos no encontró el servidor activo. Oráculo: QuestDB 10.1.0, commit `e11c837f8a169b533540bd6fa57ef3d520b68ad9`.

## Próxima tarea

Añadir `pg_catalog.pg_get_expr(s.note, s.flags)` con alias; verificar nombre, tipo y valor frente a `/exec`.

## 2026-11-14 — `pg_catalog.pg_get_expr` con argumentos cualificados y alias

`strip_alias_refs` también normaliza los argumentos de `PgGetExprExpr`, incluidos ambos nombres de columna. La respuesta de QuestDB 10.1.0 para `SELECT pg_catalog.pg_get_expr(s.note, s.flags) FROM sensors AS s ORDER BY s.ts` declaró encabezado `pg_catalog.pg_get_expr(s.note, s.flags)`, tipo `STRING`, cuatro filas y NULL en cada una. La respuesta completa está guardada en `target/pg_get_expr_qualified_args.json`; `ge_qualified_args.tsv` se comprueba en `ge_matches_captured_exec` y en el runner diferencial.

Verificación: antes del arreglo, la regresión falló con `Invalid column: s.note`; después, `cargo test -p qdb-sql ge_matches_captured_exec --offline`, `tools/check.sh` (263 pruebas de `qdb-sql`), el runner y `git diff --check` terminaron con código 0. El runner comparó 3270 consultas h1 y 376 h2 con capturas; no ejecutó la comparación viva de conteos. Oráculo: QuestDB 10.1.0, commit `e11c837f8a169b533540bd6fa57ef3d520b68ad9`.

## Próxima tarea

Añadir `pg_catalog.pg_get_partkeydef(s.flags)` con alias; verificar nombre, tipo y valor frente a `/exec`.

## 2026-11-14 — `pg_catalog.pg_get_partkeydef` con alias

`strip_alias_refs` también retira el alias de las columnas en `PgGetPartKeyDefExpr.args`. La respuesta de QuestDB 10.1.0 para `SELECT pg_catalog.pg_get_partkeydef(s.flags) FROM sensors AS s ORDER BY s.ts` declaró encabezado `pg_catalog.pg_get_partkeydef(s.flags)`, tipo `STRING`, cuatro filas y NULL en cada una. La respuesta completa está en `target/pg_get_partkeydef_qualified_flags.json`; `kf_qualified_flags.tsv` se comprueba en `kf_matches_captured_exec` y en el runner diferencial.

Verificación: antes del arreglo, el test falló con `Invalid column: s.flags`; después, `cargo test -p qdb-sql kf_matches_captured_exec --offline`, `tools/check.sh` (263 pruebas de `qdb-sql`), el runner y `git diff --check` terminaron con código 0. El runner comparó 3271 consultas h1 y 376 h2 con capturas; no ejecutó la comparación viva por conteo. Oráculo: QuestDB 10.1.0, commit `e11c837f8a169b533540bd6fa57ef3d520b68ad9`.

## Próxima tarea

Verificar `pg_catalog.pg_get_expr(s.note, s.flags, s.ok)` para cubrir el tercer argumento opcional cualificado frente a `/exec`.

## 2026-11-14 — `pg_catalog.pg_get_expr` con tercer argumento cualificado

La regresión amplía `ge_matches_captured_exec` para cubrir los dos argumentos obligatorios y el tercero opcional con alias de tabla. La respuesta de QuestDB 10.1.0 para `SELECT pg_catalog.pg_get_expr(s.note, s.flags, s.ok) FROM sensors AS s ORDER BY s.ts` declaró encabezado completo, tipo `STRING`, cuatro filas y NULL en cada una. La respuesta JSON está en `target/pg_get_expr_qualified_optional.json`; la captura `ge_qualified_optional.tsv` quedó añadida al runner.

Verificación: `cargo test -p qdb-sql ge_matches_captured_exec --offline`, `tools/check.sh` (263 pruebas de `qdb-sql`), el runner y `git diff --check` terminaron con código 0. El runner comparó 3272 consultas h1 y 376 h2 con capturas; no ejecutó la comparación viva por conteo. El alias de los argumentos se normaliza mediante la rama ya añadida para `PgGetExprExpr`. Oráculo: QuestDB 10.1.0, commit `e11c837f8a169b533540bd6fa57ef3d520b68ad9`.

## Próxima tarea

Verificar `current_setting(s.note)` con alias; confirmar si QuestDB acepta la columna como nombre de setting y capturar su respuesta completa.

## 2026-11-14 — `current_setting` con argumento cualificado

QuestDB 10.1.0 rechaza `SELECT current_setting(s.note) FROM sensors AS s ORDER BY s.ts`: la respuesta `/exec` contiene el error `argument type mismatch for function current_setting at #1 expected: STRING constant, actual: STRING`, posición 23. La respuesta íntegra se guardó en `target/current_setting_qualified_note.json`. `strip_alias_refs` ahora elimina el alias de `CurrentSettingExpr.args`, permitiendo que el ejecutor reporte el mismo error de tipo que el oráculo.

Verificación: antes del cambio, `cw_matches_captured_exec` falló con `Invalid column: s.note`; después, la prueba enfocada, `tools/check.sh` (263 pruebas de `qdb-sql`) y `git diff --check` pasaron. El runner comparó 3272 consultas h1 y 376 h2; no incluye esta consulta de error porque compara resultados TSV. La respuesta completa del oráculo, con posición 23, está guardada en `target/current_setting_qualified_note.json`.

## Próxima tarea

Verificar `format_type(s.flags, s.flags)` con alias frente a `/exec` y comparar encabezado, tipo, valores o error.

## 2026-11-14 — `format_type` con argumentos cualificados y alias

`strip_alias_refs` también normaliza los dos argumentos de `FormatTypeExpr`. La respuesta de QuestDB 10.1.0 para `SELECT format_type(s.flags, s.flags) FROM sensors AS s ORDER BY s.ts` declaró encabezado `format_type(s.flags, s.flags)`, tipo `STRING`, cuatro filas y NULL en cada una. La respuesta completa está en `target/format_type_qualified_flags.json`; la captura `ft_qualified_flags.tsv` se prueba en `ft_matches_captured_exec` y el runner.

Verificación: antes del arreglo, el test falló con `Invalid column: s.flags`; después, `cargo test -p qdb-sql ft_matches_captured_exec --offline`, `tools/check.sh` (263 pruebas de `qdb-sql`), el runner y `git diff --check` terminaron con código 0. El runner comparó 3273 consultas h1 y 376 h2 con capturas; no ejecutó la comparación viva por conteo. Oráculo: QuestDB 10.1.0, commit `e11c837f8a169b533540bd6fa57ef3d520b68ad9`.

## Próxima tarea

Verificar `netmask(s.tag)` con alias frente a `/exec` y comparar encabezado, tipo y valores.

## 2026-11-14 — `netmask` con texto cualificado y alias

`strip_alias_refs` también normaliza `NetmaskExpr.args`. La respuesta de QuestDB 10.1.0 para `SELECT netmask(s.tag) FROM sensors AS s ORDER BY s.ts` declaró encabezado `netmask(s.tag)`, tipo `IPv4`, cuatro filas y los valores `255.255.255.255`, NULL, `255.255.255.255`, `255.255.255.255`. La respuesta íntegra está en `target/netmask_qualified_tag.json`; la captura `nm_qualified_tag.tsv` quedó conectada a `nm_matches_captured_exec` y al runner.

Verificación: antes del arreglo, el test falló con `Invalid column: s.tag`; después, `cargo test -p qdb-sql nm_matches_captured_exec --offline`, `tools/check.sh` (263 pruebas de `qdb-sql`), el runner y `git diff --check` terminaron con código 0. El runner comparó 3274 consultas h1 y 376 h2 con capturas; no ejecutó la comparación viva por conteo. Oráculo: QuestDB 10.1.0, commit `e11c837f8a169b533540bd6fa57ef3d520b68ad9`.

## Próxima tarea

Verificar `geo_distance_meters` con cuatro argumentos cualificados y alias frente a `/exec`.

## 2026-11-14 — `geo_distance_meters` con cuatro columnas cualificadas

`strip_alias_refs` normaliza los cuatro argumentos de `GeoDistanceExpr`. La respuesta de QuestDB 10.1.0 para `SELECT geo_distance_meters(s.flags, s.flags, s.flags, s.flags) FROM sensors AS s ORDER BY s.ts` declaró tipo `DOUBLE`, cuatro filas con valores `0.0`, NULL, `0.0`, `0.0`. La respuesta completa está en `target/geo_distance_qualified_flags.json`; la captura `gd_qualified_flags.tsv` se compara en `gd_matches_captured_exec` y en el runner.

Verificación: antes del arreglo, el test falló con `Invalid column: s.flags`; después, `cargo test -p qdb-sql gd_matches_captured_exec --offline`, `tools/check.sh` (263 pruebas de `qdb-sql`), el runner y `git diff --check` terminaron con código 0. El runner comparó 3275 consultas h1 y 376 h2 con capturas; no ejecutó la comparación viva por conteo. Oráculo: QuestDB 10.1.0, commit `e11c837f8a169b533540bd6fa57ef3d520b68ad9`.

## Próxima tarea

Verificar `geo_within_radius_latlon` con columnas cualificadas y alias frente a `/exec`.

## 2026-11-14 — `geo_within_radius_latlon` con cinco columnas cualificadas

`strip_alias_refs` normaliza los cinco argumentos de `GeoWithinExpr`. La respuesta de QuestDB 10.1.0 para `SELECT geo_within_radius_latlon(s.id, s.id, s.id, s.id, s.id) FROM sensors AS s ORDER BY s.ts` declaró tipo `BOOLEAN`, cuatro filas y valores `true`, `true`, `false`, `false`. La respuesta completa está en `target/geo_within_radius_qualified_id.json`; la captura `gw_qualified_id.tsv` se compara en `gw_matches_captured_exec` y en el runner.

Verificación: antes del arreglo, el test falló con `Invalid column: s.id`; después, `cargo test -p qdb-sql gw_matches_captured_exec --offline`, `tools/check.sh` (263 pruebas de `qdb-sql`), el runner y `git diff --check` terminaron con código 0. El runner comparó 3276 consultas h1 y 376 h2 con capturas; no ejecutó la comparación viva por conteo. Oráculo: QuestDB 10.1.0, commit `e11c837f8a169b533540bd6fa57ef3d520b68ad9`.

## Próxima tarea

Verificar `within_radius(s.id, s.id, s.id, s.id, s.id)` con alias frente a `/exec`.

## 2026-11-14 — `within_radius` con cinco columnas cualificadas

`strip_alias_refs` normaliza los argumentos de `WithinRadiusExpr`. La respuesta de QuestDB 10.1.0 para `SELECT within_radius(s.id, s.id, s.id, s.id, s.id) FROM sensors AS s ORDER BY s.ts` declaró tipo `BOOLEAN`, cuatro filas y valores `true`, `true`, `false`, `false`. La respuesta completa está en `target/within_radius_qualified_id.json`; la captura `wn_qualified_id.tsv` se compara en `wn_matches_captured_exec` y en el runner.

Verificación: antes del arreglo, el test falló con `Invalid column: s.id`; después, `cargo test -p qdb-sql wn_matches_captured_exec --offline`, `tools/check.sh` (263 pruebas de `qdb-sql`), el runner y `git diff --check` terminaron con código 0. El runner comparó 3277 consultas h1 y 376 h2 con capturas; no ejecutó la comparación viva por conteo. Oráculo: QuestDB 10.1.0, commit `e11c837f8a169b533540bd6fa57ef3d520b68ad9`.

## Próxima tarea

Verificar `within_box` con seis argumentos cualificados y alias frente a `/exec`.

## 2026-11-14 — `within_box` con seis columnas cualificadas

`strip_alias_refs` normaliza los seis argumentos de `WithinBoxExpr`. La respuesta de QuestDB 10.1.0 para `SELECT within_box(s.id, s.id, s.id, s.id, s.id, s.id) FROM sensors AS s ORDER BY s.ts` declaró tipo `BOOLEAN`, cuatro filas y valores `true`, `true`, `false`, `true`. La respuesta completa está en `target/within_box_qualified_id.json`; la captura `wb_qualified_id.tsv` se compara en `wb_matches_captured_exec` y en el runner.

Verificación: antes del arreglo, el test falló con `Invalid column: s.id`; después, `cargo test -p qdb-sql wb_matches_captured_exec --offline`, `tools/check.sh` (263 pruebas de `qdb-sql`), el runner y `git diff --check` terminaron con código 0. El runner comparó 3278 consultas h1 y 376 h2 con capturas; no ejecutó la comparación viva por conteo. Oráculo: QuestDB 10.1.0, commit `e11c837f8a169b533540bd6fa57ef3d520b68ad9`.

## Próxima tarea

Verificar `make_geohash(s.reading, s.reading, 5)` con alias frente a `/exec`.

## 2026-11-14 — `make_geohash` con coordenadas cualificadas

`strip_alias_refs` normaliza los argumentos de `MakeGeohashExpr`, incluidas las columnas de longitud y latitud. QuestDB 10.1.0 devolvió tipo `GEOHASH(1c)` y los valores `s`, `0`, `s`, `s` para `SELECT make_geohash(s.reading, s.reading, 5) FROM sensors AS s ORDER BY s.ts`. La respuesta completa está en `target/make_geohash_qualified_reading.json`; `mg_qualified_reading.tsv` se conecta al test `mg_matches_captured_exec` y al runner.

Verificación: antes del arreglo, el test falló con `Invalid column: s.reading`; después, `cargo test -p qdb-sql mg_matches_captured_exec --offline`, `tools/check.sh` (263 pruebas de `qdb-sql`), el runner y `git diff --check` terminaron con código 0. El runner comparó 3279 consultas h1 y 376 h2 con capturas; no ejecutó la comparación viva por conteo. Oráculo: QuestDB 10.1.0, commit `e11c837f8a169b533540bd6fa57ef3d520b68ad9`.

## Próxima tarea

Verificar `to_uuid(s.id, s.id)` con columnas cualificadas y alias frente a `/exec`.

## 2026-11-14 — `to_uuid` con dos columnas cualificadas

`strip_alias_refs` normaliza ambos argumentos de `ToUuidExpr`. La respuesta de QuestDB 10.1.0 para `SELECT to_uuid(s.id, s.id) FROM sensors AS s ORDER BY s.ts` declaró tipo `UUID` y valores `00000000-0000-0001-0000-000000000001`, `00000000-0000-0002-0000-000000000002`, NULL y `ffffffff-ffff-ffff-ffff-ffffffffffff`. La respuesta íntegra está en `target/to_uuid_qualified_id.json`; la captura `uu_qualified_id.tsv` se compara en `uu_matches_captured_exec` y en el runner.

Verificación: antes del arreglo, el test falló con `Invalid column: s.id`; después, `cargo test -p qdb-sql uu_matches_captured_exec --offline`, `tools/check.sh` (263 pruebas de `qdb-sql`), el runner y `git diff --check` terminaron con código 0. El runner comparó 3280 consultas h1 y 376 h2 con capturas; no ejecutó la comparación viva por conteo. Oráculo: QuestDB 10.1.0, commit `e11c837f8a169b533540bd6fa57ef3d520b68ad9`.

## Próxima tarea

Auditar las proyecciones restantes con argumentos de columna en `strip_alias_refs` y agregar el siguiente caso cualificado comprobado contra `/exec`.

## 2026-11-14 — MD5, SHA-1 y SHA-256 con alias

La auditoría de `Proj` encontró que `Md5Expr`, `Sha1Expr` y `Sha256Expr` tenían argumentos de columna pero no ramas en `strip_alias_refs`. Se añadieron las tres. Las respuestas vivas de `/exec` para las consultas `md5(s.tag)`, `sha1(s.tag)` y `sha256(s.tag)` declararon tipo `VARCHAR`, cuatro filas cada una y el mismo NULL en la segunda fila; sus respuestas completas quedaron en `target/md5_qualified_tag.json`, `target/sha1_qualified_tag.json` y `target/sha256_qualified_tag.json`.

Verificación: `md5_matches_captured_exec`, `sha1_matches_captured_exec` y `sha256_matches_captured_exec` pasan; SHA-1 fallaba antes con `Invalid column: s.tag`. `tools/check.sh` terminó con 263 pruebas de `qdb-sql`, `git diff --check` pasó y el runner comparó 3283 consultas h1 y 376 h2 con capturas. Las tres capturas están conectadas al runner diferencial; su comparación viva por conteo no se ejecutó.

## Próxima tarea

Verificar `floor(s.reading)` con alias frente a `/exec`.

## 2026-11-14 — `floor` y `ceil` con columna cualificada

`strip_alias_refs` normaliza los argumentos de `FloorExpr` y `CeilExpr`. Para `floor(s.reading)` QuestDB 10.1.0 devolvió `DOUBLE` y `1.0`, NULL, `0.0`, `-0.0`; para `ceil(s.reading)` devolvió `DOUBLE` y `2.0`, NULL, `0.0`, `-0.0`. Las respuestas completas están en `target/floor_qualified_reading.json` y `target/ceil_qualified_reading.json`; las capturas quedaron conectadas a sus pruebas y al runner.

Verificación: `floor_matches_captured_exec` falló antes del arreglo con `Invalid column: s.reading`; ahora pasan ambas pruebas enfocadas, `tools/check.sh` (263 pruebas de `qdb-sql`), el runner y `git diff --check`. El runner comparó 3285 consultas h1 y 376 h2 con capturas; no ejecutó la comparación viva por conteo.

## Próxima tarea

Verificar `sign(s.reading)` con alias frente a `/exec`.

## 2026-11-14 — `sign` y `sqrt` con columna cualificada

`strip_alias_refs` normaliza los argumentos de `SignExpr` y `SqrtExpr`. QuestDB 10.1.0 devolvió para `sign(s.reading)` el tipo `DOUBLE` y `1.0`, NULL, `0.0`, `0.0`; para `sqrt(s.reading)` devolvió `DOUBLE` y `1.224744871391589`, NULL, `0.0`, `-0.0`. Las respuestas completas están en `target/sign_qualified_reading.json` y `target/sqrt_qualified_reading.json`, y las capturas se comparan en `sign_matches_captured_exec` y `sqrt_matches_captured_exec`.

Verificación: `sign_matches_captured_exec` falló antes con `Invalid column: s.reading`; tras el arreglo pasan ambas regresiones, `tools/check.sh` (263 pruebas de `qdb-sql`), el runner y `git diff --check`. El runner comparó 3287 consultas h1 y 376 h2 con capturas; no ejecutó la comparación viva por conteo.

## Próxima tarea

Verificar `sin(s.reading)` y `cos(s.reading)` con alias frente a `/exec`.

## 2026-11-14 — `sin` y `cos` con columna cualificada

`strip_alias_refs` normaliza los argumentos de `SinExpr` y `CosExpr`. QuestDB 10.1.0 devolvió para `sin(s.reading)` el tipo `DOUBLE` y `0.9974949866040544`, NULL, `0.0`, `-0.0`; para `cos(s.reading)` devolvió `DOUBLE` y `0.0707372016677029`, NULL, `1.0`, `1.0`. Las respuestas completas están en `target/sin_qualified_reading.json` y `target/cos_qualified_reading.json`; las capturas `sn_qualified_reading.tsv` y `co_qualified_reading.tsv` se conectaron a sus tests y al runner.

Verificación: `sin_matches_captured_exec` falló antes con `Invalid column: s.reading`; tras el arreglo pasan ambas pruebas enfocadas, `tools/check.sh` (263 pruebas de `qdb-sql`), el runner y `git diff --check`. El runner comparó 3289 consultas h1 y 376 h2 con capturas; no ejecutó la comparación viva por conteo.

## Próxima tarea

Verificar `tan(s.reading)` y las funciones trigonométricas restantes con alias frente a `/exec`.

## 2026-11-14 — Funciones trigonométricas con columna cualificada

`strip_alias_refs` normaliza ahora los argumentos de `Tan`, `Asin`, `Acos`, `Atan`, `Atan2`, `Degrees`, `Radians` y `Cot`. QuestDB 10.1.0 devolvió `DOUBLE` para cada proyección. En orden temporal, los resultados fueron: `tan`: `14.101419947171719`, NULL, `0.0`, `-0.0`; `asin`: NULL, NULL, `0.0`, `-0.0`; `acos`: NULL, NULL, `1.5707963267948966`, `1.5707963267948966`; `atan`: `0.982793723247329`, NULL, `0.0`, `-0.0`; `atan2(s.reading, s.reading)`: `0.7853981633974483`, NULL, `0.0`, `-3.141592653589793`; `degrees`: `85.94366926962348`, NULL, `0.0`, `-0.0`; `radians`: `0.026179938779914945`, NULL, `0.0`, `-0.0`; `cot`: `0.07091484430265245`, NULL, NULL, NULL. Las respuestas completas están en `target/*_qualified_reading.json` y las ocho capturas TSV tienen tests y entradas en el runner.

Verificación: `tan_matches_captured_exec` y los siete tests restantes pasan dentro de `tools/check.sh` (263 pruebas de `qdb-sql`); también pasan storage, types y CLI. `tools/run_differential.sh` comparó 3297 consultas h1 y 376 h2 con capturas y verificó conteos para 3636 consultas vivas contra QuestDB 10.1.0.

## 2026-11-14 — `power`, `ln`, `log` y `exp` con alias

`strip_alias_refs` normaliza ahora los argumentos de `PowerExpr`, `LnExpr`, `LogExpr` y `ExpExpr`. QuestDB 10.1.0 devolvió tipo `DOUBLE` para las cuatro consultas: `power(s.reading, 2)` produjo `2.25`, NULL, `0.0`, `0.0`; `ln(s.reading)` produjo `0.4054651081081644`, NULL, NULL, NULL; `log(s.reading)` produjo `0.17609125905568124`, NULL, NULL, NULL; `exp(s.reading)` produjo `4.4816890703380645`, NULL, `1.0`, `1.0`. Las respuestas completas están en `target/*_qualified_reading.json`; las capturas TSV están en los tests y el runner.

Verificación: los cuatro tests capturados pasan; `tools/check.sh` pasó con 263 pruebas de `qdb-sql`, 4 de `qdb-storage` y 3 de `qdb-types`, además de CLI. `tools/run_differential.sh` comparó 3301 consultas h1 y 376 h2 con capturas, y la comparación viva de 3636 consultas confirmó los conteos de filas frente a QuestDB 10.1.0. `git diff --check` pasó.

## Próxima tarea

Verificar `not(s.ok)`, `mid`, `spread`, `spread_bps` y `wmid` con columnas cualificadas frente a `/exec`.

## 2026-11-14 — `not`, `mid`, `spread`, `spread_bps` y `wmid` con alias

`strip_alias_refs` normaliza ahora los argumentos de estas cinco proyecciones, incluidos los cuatro argumentos de `wmid`. QuestDB 10.1.0 devolvió `BOOLEAN` para `not(s.ok)`: `false`, `true`, `false`, `true`. `mid(s.reading, s.score)` devolvió `DOUBLE`: `1.375`, NULL, `0.0`, `-0.75`; `spread` devolvió `-0.25`, NULL, `0.0`, `-1.5`; `spread_bps` devolvió `-1818.1818181818182`, NULL, NULL, `20000.0`; `wmid(s.id, s.reading, s.score, s.id)` devolvió `1.375`, NULL, NULL, `-0.75`. Las respuestas completas están en `target/not_qualified_flags.json`, `target/mid_qualified_reading_score.json`, `target/spread_qualified_reading_score.json`, `target/spread_bps_qualified_reading_score.json` y `target/wmid_qualified_args.json`; las capturas tienen pruebas y entradas del runner.

Verificación: los cinco tests pasan, al igual que `tools/check.sh` (263 pruebas de `qdb-sql`, 4 de storage y 3 de types). El runner comparó 3306 consultas h1 y 376 h2 con capturas y verificó cuentas para 3641 consultas vivas de QuestDB 10.1.0. `git diff --check` pasó.

## Próxima tarea

Revisar referencias de columnas cualificadas en `Cast`, `Between`, `InList`, operadores y `ToChar` dentro de `strip_alias_refs`.

## 2026-11-14 — Casts, operadores y `to_char` con alias

`strip_alias_refs` elimina el alias de `CastExpr.column`, del valor de `InList`, de los operandos de comparación y aritmética, de `AndExpr`, y de los argumentos directos y anidados de `ToCharExpr`. QuestDB 10.1.0 confirmó: `s.id::long` da `1`, `2`, NULL, `-1`; `s.id IN (1, 2)` da `true`, `true`, `false`, `false`; `s.id = 1` da `true`, `false`, `false`, `false`; `s.id + 1` da `2`, `3`, NULL, `0`; `-s.id` da `-1`, `-2`, NULL, `1`; `~s.id` da `-2`, `-3`, NULL, `0`; `s.ok AND false` da cuatro `false`. `to_char(from_base64(s.tag))` para `s.id = 1` devuelve `STRING` `00000000 bf`. Respuestas completas: `target/cast_qualified_id.json`, `in_qualified_id.json`, `eq_qualified_id.json`, `add_qualified_id.json`, `neg_qualified_id.json`, `bitnot_qualified_id.json`, `and_qualified_flags.json` y `to_char_qualified_tag.json`.

Las pruebas nuevas detectaron además que el parser cortaba `-s.id` y `~s.id` tras `s`; ambas rutinas ahora leen el identificador cualificado completo. `tools/check.sh` pasó con 263 pruebas de `qdb-sql`, 4 de storage y 3 de types. El runner comparó 3314 consultas h1 y 376 h2 con capturas y verificó cuentas para 3649 consultas vivas contra QuestDB 10.1.0. `git diff --check` pasó.

## Próxima tarea

Investigar `BETWEEN` en proyección y filtro frente a `/exec`, incluyendo alias, `NOT BETWEEN` y `LIMIT 0`.

## 2026-11-14 — `BETWEEN` en proyección y filtro

`parse_between` ahora genera el encabezado de QuestDB (`id between (1, 2)`) en vez de `between(id, 1, 2)`. `strip_alias_refs` elimina alias de los tres argumentos, y el parser de filtros acepta `WHERE col BETWEEN low AND high` y `WHERE col NOT BETWEEN low AND high`. Los límites se ordenan como en QuestDB; un límite NULL hace falso `BETWEEN` y verdadero `NOT BETWEEN`, incluso para filas con valor nulo. Las regresiones cubren proyección, alias, NOT, filtro, límites invertidos, límites NULL y `LIMIT 0`.

La auditoría estática ya normaliza las 109 variantes de `Proj` que contienen argumentos de columna; las ocho restantes son proyecciones sin argumentos de columna. Las capturas actualizadas proceden de `/exec` y están en `target/bt_*_oracle.json` y `target/where_*_oracle.json`. `tools/check.sh` pasó con 264 pruebas de `qdb-sql`, 4 de storage y 3 de types. `tools/run_differential.sh` comparó 3323 consultas h1 y 376 h2 con capturas; la comparación viva confirmó cuentas de filas para 3658 consultas. `git diff --check` pasó.

El parser ahora reproduce la regla observada en QuestDB 10.1.0: una proyección `BETWEEN` no puede ordenarse por otra columna si el operando comparado no está también proyectado; `ORDER BY` sobre ese operando sigue permitido. Las pruebas confirman el error `Invalid column: id`, las formas válidas y una proyección que incluye `s.id`. QuestDB también devuelve una posición (`7` o `15` según la forma), pero `qdb_types::Error` solo transporta el mensaje y no expone esa posición.

Verificación posterior: `between_matches_captured_exec` pasa con los casos nuevos; `tools/check.sh` pasó con 264 pruebas de `qdb-sql`, 4 de storage y 3 de types. `tools/run_differential.sh` comparó 3325 consultas h1 y 376 h2 con capturas y verificó cuentas para 3660 consultas vivas de QuestDB 10.1.0. `git diff --check` pasó.

## WHERE compuesto: AND y OR

El AST conserva condiciones booleanas anidadas. El parser da mayor precedencia a `AND` que a `OR` y acepta agrupación con paréntesis; el ejecutor evalúa ambos operadores con cortocircuito. Capturas de `/exec` y `compound_where_matches_captured_exec` cubren OR, AND, precedencia, paréntesis, alias de tabla, `BETWEEN` combinado y comparaciones con NULL.

Verificación: `cargo test -p qdb-sql compound_where_matches_captured_exec -- --nocapture` pasó; `tools/check.sh` pasó con 265 pruebas de `qdb-sql`, 4 de storage y 3 de types; `TMPDIR="$PWD/target/oracle-tmp" tools/run_differential.sh` pasó con 3332 consultas h1, 376 h2 y 3667 conteos de filas frente a la referencia viva. Próxima tarea: `WHERE IN` / `NOT IN`.

## WHERE IN / NOT IN

`WHERE` acepta `IN` y `NOT IN` con listas de literales numéricos, texto y NULL; acepta alias de tabla y mezcla con `AND`/`OR`. La prueba `in_filters_match_captured_exec` cubre listas repetidas, texto vacío, NULL a ambos lados del operador, tipos no admitidos, columna inexistente y lista vacía. Las 13 respuestas completas de `/exec` están en `tests/fixtures/h1/where_in_oracle.jsonl`; sus filas se comparan con los TSV esperados.

En QuestDB 10.1.0, NULL coincide con un elemento NULL de la lista. Por eso `id IN (NULL, 1)` incluye tanto `1` como la fila con `id` NULL; `id NOT IN (1, -1)` incluye el NULL porque no está en la lista. `IN ()` devuelve `too few arguments for 'in'`. La comparación viva confirmó encabezados, tipos, valores, orden y errores; el runner diferencial mantiene su comprobación habitual de recuentos para consultas vivas.

Verificación: `cargo test -p qdb-sql in_filters_match_captured_exec -- --nocapture` y `tools/check.sh` pasaron con 266 pruebas de `qdb-sql`, 4 de storage y 3 de types. `TMPDIR="$PWD/target/oracle-tmp" tools/run_differential.sh` comparó 3342 consultas h1 y 376 h2 con capturas y confirmó los recuentos de 3677 consultas vivas. La comparación completa de filas, tipos y errores de este operador está conservada en las capturas JSON de `/exec`.

## WHERE LIKE / ILIKE

`WHERE` acepta `LIKE`, `ILIKE`, `NOT LIKE` y `NOT ILIKE` con patrones literales y columnas textuales cualificadas. `like_filters_match_captured_exec` cubre sensibilidad de mayúsculas, `%`, `_`, escape `\\`, patrón NULL o vacío, valor NULL, `SYMBOL`, alias, AND/OR, tipo incompatible, patrón dinámico, columna inexistente, escape final y validaciones que siguen activas con `LIMIT 0`. Las 22 respuestas completas de `/exec`, incluidos errores y posiciones, están en `tests/fixtures/h1/where_like_oracle.jsonl`; los 15 resultados válidos se comparan con TSV.

QuestDB hace que un LIKE o ILIKE no negado con patrón NULL/vacío o valor NULL no coincida; su forma negada incluye esas filas. El matcher reutiliza los tokens de `LIKE`/`ILIKE` ya portados para las proyecciones. `tools/check.sh` pasó con 267 pruebas de `qdb-sql`, 4 de storage y 3 de types. `TMPDIR="$PWD/target/oracle-tmp" tools/run_differential.sh` comparó 3357 consultas h1, 376 h2 y recuentos de 3692 consultas vivas. `git diff --check` pasó.

## WHERE regex `~` / `!~`

`WHERE` acepta `~` y `!~` con patrones literales sobre `STRING`, `SYMBOL`, `VARCHAR` y `CHAR`. `regex_filters_match_captured_exec` compara 17 resultados con TSV y comprueba errores de tipo, columna inexistente, patrón dinámico y regex inválida, también con `LIMIT 0`. Se cubren búsqueda parcial, anclas, clases, alternancia, Unicode, escape hexadecimal, patrón vacío o NULL, alias, `SYMBOL` y combinaciones AND/OR. Las 26 respuestas completas de `/exec`, con tipos, valores y posiciones de error, están en `tests/fixtures/h1/where_regex_oracle.jsonl`.

El port usa el crate `regex`; el alcance verificado no demuestra compatibilidad completa con el dialecto Java de QuestDB. El patrón debe ser constante: `~` con patrón de columna informa que la ejecución dinámica no está implementada; `!~` con patrón dinámico da error de tipos. Para columnas de texto, un patrón NULL no coincide tanto en `~` como en `!~`; con columna INT se informa incompatibilidad de tipos. El matcher también se comparte con la proyección regex, cuyas regresiones pasaron en la suite completa.

Verificación: prueba focalizada y `tools/check.sh` pasaron con 268 pruebas de `qdb-sql`, 4 de storage y 3 de types. `TMPDIR="$PWD/target/oracle-tmp" tools/run_differential.sh` comparó 3374 consultas h1 y 376 h2 con capturas, y confirmó recuentos de 3709 consultas vivas. `git diff --check` pasó.

## Próxima tarea

Ampliar `WHERE` con el operador prefijo `NOT`, contrastando agrupación, precedencia con AND/OR, NULL y alias frente a QuestDB 10.1.0.

## WHERE prefijo `NOT`

El filtro admite `NOT` sobre condiciones ya soportadas y grupos entre paréntesis, además de columnas booleanas y literales booleanos. `prefix_not_filters_match_captured_exec` verifica precedencia con AND/OR, anidamiento, NULL, alias, regex, `IN`, columnas booleanas, constantes y `LIMIT 0`. Las 21 respuestas completas de `/exec` (16 resultados y 5 errores, con tipos, valores y posiciones) están en `tests/fixtures/h1/where_not_oracle.jsonl`.

`NOT NULL` coincide con todas las filas en QuestDB 10.1.0; un `NULL` sin negar falla con `boolean expression expected`. Negar una columna no booleana produce el error específico de tipo de `not`; negar una comparación invierte el resultado booleano ya evaluado, incluso si la columna comparada es NULL. Una columna inexistente dentro del grupo da `Invalid column: nombre` aun con `LIMIT 0`.

Verificación: `prefix_not_filters_match_captured_exec` y `tools/check.sh` pasaron con 269 pruebas de `qdb-sql`, 4 de storage y 3 de types. `TMPDIR="$PWD/target/oracle-tmp" tools/run_differential.sh` comparó 3390 consultas h1 y 376 h2 con capturas, y confirmó recuentos de 3725 consultas vivas. `git diff --check` pasó.

## WHERE compara dos columnas

El AST y el ejecutor aceptan comparaciones columna-columna con `=`, `!=`, `<>`, `<`, `>`, `<=` y `>=`; el parser conserva alias de ambas columnas. `column_comparison_filters_match_captured_exec` cubre alias, precedencia AND/OR, `NOT`, NULL, operadores booleanos, numéricos, texto, SYMBOL, VARCHAR, CHAR, DATE y TIMESTAMP, además de conversiones y errores con `LIMIT 0`. Las 29 respuestas completas de `/exec` (20 resultados y 9 errores) están en `tests/fixtures/h1/where_columns_oracle.jsonl`.

Se compararon tipos enteros entre sí, INT/LONG con FLOAT/DOUBLE, textos STRING/SYMBOL/VARCHAR, booleanos del mismo tipo y DATE/TIMESTAMP en ambas direcciones. El NULL de INT conserva su centinela distinto de LONG; el NULL compartido entre FLOAT/DOUBLE compara igual; DATE se lleva de milisegundos a microsegundos al compararlo con TIMESTAMP. Se reprodujeron además conversiones STRING a LONG/DOUBLE/DATE/TIMESTAMP, SYMBOL con entero para igualdad, CHAR con LONG y el error VARCHAR a CHAR frente a LONG. La matriz de todas las parejas de tipos no está verificada aún; las demás combinaciones siguen siendo cobertura pendiente.

Verificación: `column_comparison_filters_match_captured_exec` y `tools/check.sh` pasaron con 270 pruebas de `qdb-sql`, 4 de storage y 3 de types. `TMPDIR="$PWD/target/oracle-tmp" tools/run_differential.sh` comparó 3410 consultas h1 y 376 h2 con capturas, y confirmó recuentos de 3745 consultas vivas. `git diff --check` pasó.

## WHERE compara expresiones

Las comparaciones de `WHERE` aceptan una operación aritmética (`+`, `-`, `*`, `/` o `%`) en cualquier lado y expresiones de función procesadas por el parser de proyecciones. `abs`, `length` y `lower` tienen casos contrastados. La prueba `where_arithmetic_comparison_operands_match_captured_exec` compara nueve respuestas completas de `tests/fixtures/h1/where_expr_oracle.jsonl`. `where_function_comparison_operands_match_captured_exec` compara ocho resultados y un error capturados en `tests/fixtures/h1/where_expr_functions_oracle.jsonl`; cubre funciones en ambos lados, alias, NULL, combinación con OR y error de tipo con `LIMIT 0`. Se reutilizan el parser, la resolución de tipos y el evaluador de proyecciones de SELECT.

El AST de filtros admite ahora operaciones encadenadas y paréntesis; el evaluador aplica la precedencia y asociatividad de `*`, `/`, `%` frente a `+` y `-`. La prueba `where_arithmetic_precedence_matches_captured_exec` compara once resultados y un error de `tests/fixtures/h1/where_expr_chains_oracle.jsonl`: `id + flags * 2`, `id + 1 * 2`, `(id + flags) * 2`, `abs(id) + 1`, `id + 1 + flags`, NULL y casos seleccionados con INT/LONG/FLOAT/DOUBLE, división/remainder y conversión de `STRING` a `INT` que falla con `LIMIT 0`. La compatibilidad de funciones y tipos se ha probado en estos casos, no en todo el catálogo ni en toda su matriz. La captura se obtuvo de QuestDB 10.1.0, commit `e11c837f8a169b533540bd6fa57ef3d520b68ad9`, en una raíz aislada donde se cargaron los fixtures h1 y h2.

Verificación: `tools/check.sh` pasó con 273 pruebas de `qdb-sql`, 4 de storage y 3 de types. `TMPDIR="$PWD/target/oracle-tmp" tools/run_differential.sh` comparó 3438 consultas h1 y 376 h2 con capturas, y confirmó recuentos de 3745 consultas vivas. `git diff --check` pasó.

## Expresiones aritméticas de WHERE: coerciones y bordes

El evaluador de aritmética de filtros ahora convierte `CHAR` a número con el error de QuestDB (sin comillas alrededor del carácter), informa los dos tipos del operador incompatible y resuelve `SYMBOL` a su clave entera cuando se combina con un entero. La clave se conserva como `SYMBOL` durante la resolución para rechazar, por ejemplo, `SYMBOL + DOUBLE`. Se añadieron regresiones para overflow de INT y LONG, división por cero, función sobre columna cualificada con alias y estos casos de conversión/tipos. Las nueve respuestas completas de `/exec` están en `tests/fixtures/h1/where_expr_coercions_oracle.jsonl`.

Verificación: `tools/check.sh` pasó con 274 pruebas de `qdb-sql`, 4 de storage y 3 de types. `TMPDIR="$PWD/target/oracle-tmp" tools/run_differential.sh` comparó 3443 consultas h1 y 376 h2 con capturas. Con `QDB_HTTP=http://127.0.0.1:19011/exec`, el diferencial vivo confirmó los recuentos de 3745 consultas. `git diff --check` pasó.

## Expresiones de WHERE: FLOAT, NULL tipado y tiempo

El filtro conserva ahora el tipo declarado de expresiones proyectadas, incluso cuando el valor es NULL. Las operaciones permanecen en FLOAT de 32 bits si no hay un DOUBLE; la captura `score * 16777217 = 20971520.0` distingue el redondeo FLOAT del cálculo DOUBLE. Se portó aritmética seleccionada de DATE/TIMESTAMP: `temporal + entero` en el lado temporal produce TIMESTAMP, suma dos temporales tras convertir DATE a microsegundos, la resta entre dos temporales devuelve LONG con los valores internos de ambas columnas y las operaciones temporales con reales usan DOUBLE. La comparación de un resultado TIMESTAMP con un literal numérico también se contrasta.

`where_arithmetic_coercions_match_captured_exec` compara 15 resultados frente a `tests/fixtures/h1/where_expr_temporal_float_oracle.jsonl`; el script verificó las 15 respuestas completas contra el `/exec` vivo de QuestDB 10.1.0. La compatibilidad sigue siendo parcial: no está comprobada la matriz completa de operaciones temporales ni sus límites.

Verificación: `tools/check.sh` pasó con 274 pruebas de `qdb-sql`, 4 de storage y 3 de types. `TMPDIR="$PWD/target/oracle-tmp" QDB_HTTP=http://127.0.0.1:19011/exec tools/run_differential.sh` comparó 3458 consultas h1 y 376 h2 con capturas, y 3745 recuentos de filas contra la referencia viva. `cargo fmt --all -- --check` y `git diff --check` pasaron.

## Expresiones de WHERE: sentinels de overflow y posiciones

La aritmética de filtros convierte a NULL los resultados enteros que chocan con los sentinels de QuestDB (`INT_MIN` y `LONG_MIN`), también después de `MIN / -1`. El parser trata `-9223372036854775808` como DOUBLE, igual que la referencia. Se cubre overflow de TIMESTAMP al sumar `LONG_MAX` y de DATE al convertir a microsegundos. `Error` ahora puede llevar una posición opcional; el parser conserva el offset del operador aritmético y la coerción fallida de texto mantiene la posición 0. Las regresiones comprueban posiciones 0, 32 y 38. Las ocho respuestas completas de overflow están en `tests/fixtures/h1/where_expr_overflow_oracle.jsonl`.

Verificación: `tools/check.sh` pasó con 274 pruebas de `qdb-sql`, 4 de storage y 3 de types. `TMPDIR="$PWD/target/oracle-tmp" QDB_HTTP=http://127.0.0.1:19011/exec tools/run_differential.sh` comparó 3466 consultas h1 y 376 h2 con capturas, y 3745 recuentos de filas frente al oráculo vivo. Un contraste separado confirmó las 17 respuestas completas de `where_expr_coercions_oracle.jsonl` y `where_expr_overflow_oracle.jsonl`. `cargo fmt --all -- --check` y `git diff --check` pasaron.

## Expresiones de WHERE: tipos pequeños, cero real y errores anidados

La matriz cubre ahora `BYTE + SHORT`, `SHORT / BYTE` y `BYTE % INT`. Se contrastó división y remainder por cero en `FLOAT` y `DOUBLE`, además de la propagación de `NULL` tipado a través de operaciones anidadas con `FLOAT` y `DOUBLE`. Las doce respuestas completas del oráculo están en `tests/fixtures/h1/where_expr_edges_oracle.jsonl`; las diez respuestas de fila tienen TSV de regresión.

Las pruebas de error con una operación inválida anidada confirman que se informa la posición del primer operador que falla durante la evaluación: posición 38 para la subexpresión interna y 42 para la operación exterior. `where_arithmetic_coercions_match_captured_exec` verifica estos mensajes y offsets junto con los nuevos TSV.

Verificación: `tools/check.sh` pasó con 274 pruebas de `qdb-sql`, 4 de storage y 3 de types. `TMPDIR="$PWD/target/oracle-tmp" QDB_HTTP=http://127.0.0.1:19011/exec tools/run_differential.sh` comparó 3476 consultas h1 y 376 h2 con capturas, y 3745 recuentos de filas con la referencia viva. Las doce respuestas completas de `where_expr_edges_oracle.jsonl` coincidieron con `/exec`. `cargo fmt --all -- --check` y `git diff --check` pasaron.

## Expresiones de WHERE: cruces de enteros y reales

Se añadieron seis casos diferenciales para `LONG` con `DOUBLE` y `FLOAT` con `INT`: suma en ambas direcciones, división, y remainder con operandos de distinto tipo y dentro de `DOUBLE`. Las seis respuestas completas están en `tests/fixtures/h1/where_expr_numeric_oracle.jsonl` y cada resultado tiene TSV. Todas pasan con el evaluador existente.

El parser de operandos ahora reconoce literales científicos finitos, por ejemplo `1e308`, y rechaza exponentes que desbordan a infinito o subdesbordan a cero. Sus mensajes y offsets de error aún no coinciden con QuestDB. En el evaluador, `Infinity - Infinity` se convierte a NULL, igual que QuestDB; los infinitos que sí resultan de la aritmética permanecen comparables. Las capturas `tests/fixtures/h1/where_expr_nonfinite_oracle.jsonl` verifican infinito mayor que un finito, infinito igual a sí mismo, `NaN > 0` y `NaN = NULL`.

Verificación: `tools/check.sh` pasó con 274 pruebas de `qdb-sql`, 4 de storage y 3 de types. `TMPDIR="$PWD/target/oracle-tmp" QDB_HTTP=http://127.0.0.1:19011/exec tools/run_differential.sh` comparó 3486 consultas h1 y 376 h2 con capturas, y 3745 recuentos de filas con la referencia viva. Las respuestas completas de `where_expr_edges_oracle.jsonl`, `where_expr_numeric_oracle.jsonl` y `where_expr_nonfinite_oracle.jsonl` coincidieron con `/exec`. `cargo fmt --all -- --check` y `git diff --check` pasaron.

## Expresiones de WHERE: límites y errores de literales científicos

Los operandos aritméticos científicos ahora devuelven `invalid constant: …` con el offset del primer dígito, incluso si el signo negativo precede al literal. Se probaron también comparaciones directas con exponentes inválidos. La regresión cubre overflow (`1e999` y el límite sobre DOUBLE), underflow (`1e-999`), exponente incompleto (`1e`, `1e+`), signo negativo, exponente firmado, máximo DOUBLE finito y subnormal mínimo. Los errores tienen offsets 29, 39 y 40. Las doce respuestas completas están en `tests/fixtures/h1/where_expr_scientific_oracle.jsonl`.

Verificación: `tools/check.sh` pasó con 274 pruebas de `qdb-sql`, 4 de storage y 3 de types. `TMPDIR="$PWD/target/oracle-tmp" QDB_HTTP=http://127.0.0.1:19011/exec tools/run_differential.sh` comparó 3489 consultas h1 y 376 h2 con capturas, y 3745 recuentos de filas con la referencia viva. Las doce respuestas completas de `where_expr_scientific_oracle.jsonl` coincidieron con `/exec`. `cargo fmt --all -- --check` y `git diff --check` pasaron.

## Expresiones de WHERE: matriz cruzada de operadores

`where_arithmetic_type_matrix_matches_captured_exec` cubre `+`, `-`, `*`, `/` y `%` en las 15 parejas entre BYTE, SHORT, INT, LONG, FLOAT y DOUBLE, con ambas orientaciones para resta, división y remainder. Se añadieron sentinels INT_MIN para los cruces BYTE/SHORT/INT y LONG_MIN para BYTE/SHORT/INT con LONG. Las 123 respuestas completas están en `tests/fixtures/h1/where_expr_matrix_oracle.jsonl`. La matriz detectó y corrigió que `NULL != literal_no_nulo` debe ser verdadero en `sql_cmp`, también cuando el NULL viene de un remainder entero por cero. Se probaron además `INT != 0` y `DOUBLE != 0` directamente.

Verificación: `tools/check.sh` pasó con 275 pruebas de `qdb-sql`, 4 de storage y 3 de types. `tools/run_differential.sh` comparó 3511 capturas h1 del lote principal, 55 de la matriz inicial, 46 de la matriz inversa/sentinels y 376 h2; contrastó 3745 recuentos contra el oráculo vivo. Las 123 respuestas completas de `where_expr_matrix_oracle.jsonl` coincidieron con `/exec`. `cargo fmt --all -- --check` y `git diff --check` pasaron.

## Expresiones de WHERE: mismos tipos y overflow real en ambos órdenes

Se añadieron consultas para los cinco operadores entre operandos `BYTE`, `SHORT`, `INT`, `LONG`, `FLOAT` y `DOUBLE` del mismo tipo. Se reprodujeron cuatro casos de overflow real en INT/LONG para suma y multiplicación, con ambos órdenes de operandos. Las 38 respuestas completas de las nuevas consultas están en `tests/fixtures/h1/where_expr_overflow_matrix_oracle.jsonl`; las consultas con filas tienen TSV de regresión.

Verificación: `tools/check.sh` pasó con 275 pruebas de `qdb-sql`, 4 de storage y 3 de types. `TMPDIR="$PWD/target/oracle-tmp" QDB_HTTP=http://127.0.0.1:19011/exec tools/run_differential.sh` comparó 3511 consultas h1 del lote principal, 55 de matriz, 46 inversas/sentinels, 38 nuevas y 376 h2; el servidor vivo confirmó 3745 recuentos de filas. Las 38 respuestas completas de `where_expr_overflow_matrix_oracle.jsonl` coincidieron en columnas, dataset y cantidad. `cargo fmt --all -- --check` y `git diff --check` pasaron.

## Expresiones de WHERE: tipo estático de aritmética

`typeof(expr)` ahora acepta expresiones aritméticas y consulta su tipo estático sin evaluar sus valores. `where_arithmetic_result_types_match_captured_exec` contrasta los cinco operadores en las seis combinaciones del mismo tipo y en ambos órdenes para las quince parejas heterogéneas entre BYTE, SHORT, INT, LONG, FLOAT y DOUBLE: 180 respuestas completas en `tests/fixtures/h1/where_expr_arithmetic_type_oracle.jsonl`.

La matriz encontró que `LONG` combinado con `FLOAT` produce `DOUBLE` para los cinco operadores; se corrigió `arithmetic_result_tag`, que antes devolvía `FLOAT`. La ruta de aritmética en `WHERE` comparte esa promoción, por lo que también conserva precisión de cálculo.

Verificación: `tools/check.sh` pasó con 276 pruebas de `qdb-sql`, 4 de storage y 3 de types. `tools/run_differential.sh` comparó 3511 capturas h1 del lote principal, 55 de matriz, 46 inversas/sentinels, 38 de overflow y 180 de tipos aritméticos, además de 376 h2; el endpoint vivo confirmó 3745 recuentos de filas. Las 180 respuestas completas coincidieron con QuestDB en columnas, dataset y cantidad. `cargo fmt --all -- --check` y `git diff --check` pasaron.

## Expresiones de WHERE: tipos temporales y resta de enteros

`where_arithmetic_temporal_result_types_match_captured_exec` contrasta los cinco operadores para DATE y TIMESTAMP entre sí, consigo mismos y con BYTE, SHORT, INT, LONG, FLOAT y DOUBLE en ambos órdenes: 140 respuestas completas en `tests/fixtures/h1/where_expr_arithmetic_temporal_type_oracle.jsonl`.

`where_arithmetic_temporal_values_match_captured_exec` verifica los valores de los mismos 140 operadores con ambos órdenes en las cuatro filas del fixture. Conserva las respuestas directas de QuestDB en `where_expr_arithmetic_temporal_results_oracle.jsonl`; el test construye filtros de igualdad con esos valores y compara las filas resultantes, incluidos los NULL. Las respuestas de esos 140 filtros, cuatro consultas adicionales de sentinel `LONG_MIN`, y cuatro regresiones previas de resta están en `where_expr_arithmetic_temporal_values_oracle.jsonl` y su manifiesto SQL.

La matriz encontró dos diferencias: `DATE/TIMESTAMP - integral` produce LONG con el valor interno de fecha en milisegundos o timestamp en microsegundos; y, en suma temporal-temporal, el operando derecho DATE aporta su valor interno en milisegundos mientras que el izquierdo DATE se convierte a microsegundos. Corregí el tag y las dos rutas de cálculo.

Verificación: `tools/check.sh` pasó con 278 pruebas de `qdb-sql`, 4 de storage y 3 de types. `tools/run_differential.sh` comparó 3511 capturas h1 del lote principal, 55 de matriz, 46 inversas/sentinels, 38 de overflow, 180 de tipos numéricos, 140 de tipos temporales, 148 de valores temporales y 376 h2; el endpoint vivo confirmó 3745 recuentos de filas. Las 140 respuestas directas, las 140 de tipos y las 148 respuestas de filtros coincidieron con QuestDB en columnas, dataset y cantidad. `cargo fmt --all -- --check` y `git diff --check` pasaron.

## Proyección aritmética general en SELECT

La proyección conserva el árbol de aritmética y reutiliza la evaluación tipada ejercitada en `WHERE`. Acepta `+`, `-`, `*`, `/` y `%` encadenados con precedencia y paréntesis, literales de texto coercibles, alias `AS`, columnas cualificadas, operandos unarios y funciones como `abs(id) + reading`. Los casos incluyen propagación de NULL, división por cero, errores de tipo y overflow/sentinels enteros. Las expresiones temporales siguen las reglas de proyección de QuestDB: `DATE -` un literal entero da TIMESTAMP, `DATE -` una columna INT da LONG, y la suma DATE/TIMESTAMP conserva la asimetría según qué lado sea DATE. `typeof()` usa el tipo de proyección para distinguir esos casos.

Las pruebas comparan 28 expresiones seleccionadas, una matriz numérica de 360 combinaciones de tipos/operadores/orientaciones, una matriz temporal de 280 consultas, cuatro casos de overflow/sentinel y seis errores con mensaje y posición. Las expresiones seleccionadas incluyen `abs`, `round`, `sqrt`, menos unario, complemento bit a bit y `typeof` alrededor de aritmética. Las capturas completas están en los manifiestos `select_expr_arithmetic_*_queries.tsv` y `select_expr_arithmetic_*_oracle.jsonl` de `tests/fixtures/h1`. La matriz temporal compara valores FLOAT/DOUBLE por igualdad exacta de bits IEEE-754 cuando QuestDB y Rust serializan el mismo `f64` con distinta cadena decimal; encabezados y tipos siguen siendo exactos. La comprobación directa de 678 respuestas contra `/exec` vivo confirmó columnas, dataset y count, o error y posición.

QuestDB formatea los operadores unarios dentro de una expresión como `-(id)` y `~(id)`, también en la etiqueta de `typeof`. Se normalizaron esas etiquetas sin alterar los valores, y se añadieron siete capturas para operadores unarios, llamadas escalares y sus tipos.

Verificación: `tools/check.sh` pasó con 283 pruebas de `qdb-sql`, 4 de storage y 3 de types. `tools/run_differential.sh` comparó 3511 capturas h1 principales, 55, 46, 38, 180, 140, 148, 28, 360, 4 y 280 capturas de las matrices/regresiones, además de 376 consultas h2 (5166 capturas en total); el oráculo vivo confirmó recuentos para 4417 consultas. `cargo fmt --all -- --check`, `git diff --check` y `bash -n tools/run_differential.sh` pasan.

## `GROUP BY` con expresión aritmética

`group_by` conserva ahora la expresión de agrupación completa y `execute_group` evalúa su valor tipado por fila con el evaluador de aritmética de `SELECT`. La lista seleccionada puede incluir la misma expresión y una agregación; admite la clave antes o después del agregado, alias de tabla/columna, filtro previo, orden por clave o agregado, `LIMIT`, llamadas escalares como operando, NULL y claves DATE/TIMESTAMP. Se rechaza seleccionar una clave distinta de la expresión agrupada y se conserva el mensaje y la posición del oráculo para columnas inexistentes y errores de conversión.

`group_by_arithmetic_matches_captured_exec` compara 14 respuestas completas guardadas en `tests/fixtures/h1/group_by_arithmetic_oracle.jsonl`; `group_by_arithmetic_errors_match_captured_exec` contrasta tres errores con mensaje y posición. Las 14 respuestas se verificaron también contra el `/exec` vivo en `columns`, `dataset` y `count`; los tres errores vivos coinciden en mensaje y posición.

Verificación: `tools/check.sh` pasó con 285 pruebas de `qdb-sql`, 4 de storage y 3 de types. `tools/run_differential.sh` comparó 3511 capturas h1 principales, 55, 46, 38, 180, 140, 148, 28, 360, 4 y 280 casos previos, 14 capturas de `GROUP BY` aritmético y 376 consultas h2 (5180 capturas); el endpoint vivo confirmó 4431 consultas por cantidad de filas, más 14 respuestas completas y tres errores completos. `cargo fmt --all -- --check`, `git diff --check` y `bash -n tools/run_differential.sh` pasaron.

## `GROUP BY` con varias claves

El AST conserva todas las claves de `GROUP BY` con su posición. El ejecutor calcula la tupla por fila y compara todas sus partes antes de alimentar el agregado. Las claves seleccionadas pueden ser un subconjunto y aparecer en cualquier orden; se conservan los alias de columna, el orden explícito, el filtrado previo y `LIMIT`. Se admiten claves simples y aritméticas, repetidas, y mezclas de tipos como LONG, SYMBOL y BOOLEAN. Los errores de una columna seleccionada no agrupada y de expresiones `ORDER BY` no válidas conservan mensajes y posiciones upstream.

`group_by_multiple_keys_matches_captured_exec` compara 16 respuestas completas, y `group_by_multiple_keys_errors_match_captured_exec` compara tres errores. Las respuestas se verificaron contra el oráculo vivo en columnas, dataset y count, incluidos dos casos con claves aritméticas y uno con tres claves.

Verificación: `tools/check.sh` pasó con 287 pruebas de `qdb-sql`, 4 de storage y 3 de types. `tools/run_differential.sh` comparó 3511 capturas h1 principales, 55, 46, 38, 180, 140, 148, 28, 360, 4, 280, 14 claves aritméticas y 16 grupos multiclave, además de 376 consultas h2 (5196 capturas); el endpoint vivo confirmó 4447 consultas por cantidad de filas, 14+16 respuestas completas y seis errores completos. `cargo fmt --all -- --check`, `git diff --check` y `bash -n tools/run_differential.sh` pasaron.

## `GROUP BY` con varias agregaciones

El AST conserva una lista de agregaciones y el orden mixto de claves y agregaciones de `SELECT`. El ejecutor prepara entradas y acumuladores independientes por agregado, y usa la agregación proyectada o una expresión agregada adicional para ordenar grupos. Se preservan los alias de agregación y su orden de salida. Las consultas cubren `count`, `sum`, `avg`, `min`, `max`, `count_distinct`, `variance`, `stddev`, `first` y `last`, además de claves simples, múltiples y aritméticas, valores NULL, filtro y `LIMIT`.

`group_by_multiple_aggregates_match_captured_exec` compara 14 respuestas completas de `tests/fixtures/h1/group_by_multi_aggregate_oracle.jsonl`. Incluye ordenar por `avg(score)` aunque no aparezca en la proyección y un caso de una sola agregación con alias. `group_by_multiple_aggregate_errors_match_captured_exec` compara tres errores; las posiciones ausentes del protocolo se preservan como ausentes. Las 14 respuestas y los tres errores se contrastaron también con el `/exec` vivo en columnas, dataset, count, mensaje y posición.

Verificación de esa unidad: `tools/check.sh` pasó con 289 pruebas de `qdb-sql`, 4 de storage y 3 de types. `tools/run_differential.sh` comparó 3511 capturas h1 principales, 55, 46, 38, 180, 140, 148, 28, 360, 4 y 280 consultas de matrices, 14 claves aritméticas, 16 grupos multiclave, 14 consultas con varias agregaciones y 376 consultas h2 (5210 capturas); el endpoint vivo confirmó 4447 recuentos de filas, 44 respuestas completas de `GROUP BY` y nueve errores completos. `cargo fmt --all -- --check`, `git diff --check` y `bash -n tools/run_differential.sh` pasan.

## Agregados parametrizados y de dos columnas con `GROUP BY`

`parse_aggregate` acepta columnas cualificadas en ambos argumentos de covarianza/regresión y conserva la posición de llamada. El ejecutor devuelve posiciones upstream para errores de tipo y parámetros en `string_agg`, `string_distinct_agg` y `approx_percentile`; los errores de conversión de covarianza/percentil usan la posición 0 que devuelve QuestDB. Se documentó que QuestDB 10.1.0 rechaza `string_distinct_agg` con `VARCHAR`.

`group_by_special_multiple_aggregates_match_captured_exec` compara cinco respuestas completas con `covar_pop`, `corr`, `regr_slope`, `string_agg`, `approx_percentile` y `approx_median`, incluidas referencias cualificadas y alias. `group_by_special_aggregate_errors_match_captured_exec` compara cuatro errores por tipo/rango. Las respuestas completas y los errores se verificaron contra el endpoint vivo.

Verificación: `tools/check.sh` pasó con 291 pruebas de `qdb-sql`, 4 de storage y 3 de types. `tools/run_differential.sh` comparó 5219 capturas; el endpoint vivo confirmó 4447 recuentos de filas, 49 respuestas completas de `GROUP BY` y 13 errores completos. `cargo fmt --all -- --check`, `git diff --check` y `bash -n tools/run_differential.sh` pasan.

## Agregados ponderados, de dos columnas y de bits con `GROUP BY`

`arg_min`/`arg_max` ahora aceptan argumentos cualificados. Se verifican combinaciones de `arg_min`, `arg_max`, `weighted_avg`, `weighted_stddev`, `bit_and`, `bit_or`, `bit_xor`, `bool_and`, `bool_or`, `vwap` y `twap`, incluidos alias de tabla, NULL, claves agrupadas y errores de conversión. Las capturas están en `group_by_multi_weighted_aggregates_oracle.jsonl`; cuatro respuestas completas y tres errores se comparan en tests Rust y contra `/exec`.

## `GROUP BY` sin agregación

El ejecutor ahora agrupa filas por tuplas sin crear acumuladores. Proyecta claves simples o aritméticas y permite una proyección de subconjunto sobre varias claves, conservando `WHERE`, alias de tabla, `ORDER BY` y `LIMIT`. También reproduce el error y posición cuando se proyecta una columna ajena al grupo o se ordena por una columna no seleccionada.

`group_by_without_aggregate_matches_captured_exec` compara ocho respuestas; `group_by_without_aggregate_errors_match_captured_exec` compara dos errores. Las capturas se verificaron contra QuestDB vivo en columnas, dataset, count, mensaje y posición.

## Funciones escalares como clave de `GROUP BY` sin agregación

Las proyecciones `abs(id)`, `lower(location)`, `upper(location)`, `concat(location, '!')`, `length(note)`, `trim(note)`, `replace(note, 'a', 'x')`, `date_trunc('day', ts)`, `left`, `right`, `substring`, `lpad`, `rpad`, `regexp_replace`, `starts_with`, `split_part`, `quote_ident`, `glob`, `ilike`, `like`, `switch`, `bar`, `strpos`, `mid`, `spread`, `spread_bps`, `wmid`, `round`, `floor`, `ceil`, `sqrt`, `power`, `sign`, `ln`, `log`, `exp`, `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2`, `degrees`, `radians`, `cot`, `pi`, `greatest`, `least`, `round_down`, `round_up`, `round_half_even`, `coalesce`, `nullif`, `cast`, `year`, `month`, `day`, `hour`, `minute`, `second`, `millis`, `micros`, `nanos`, `dateadd`, `datediff`, `extract`, `timestamp_floor`, `timestamp_ceil`, `to_utc`, `to_timezone`, `to_pg_date`, `to_timestamp`, `to_date`, `md5`, `sha1`, `sha256`, `to_uuid`, `json_extract`, `to_char`, `to_lowercase`, `to_uppercase`, `length_bytes`, `size_pretty`, `is_leap_year`, `days_in_month`, `to_str`, `is_end_of_month` y `day_of_week_sunday_first` pueden evaluarse a partir del primer registro de cada grupo cuando sus columnas fuente forman claves de agrupación. Para una clave seleccionada directamente, el ejecutor reutiliza el valor agrupado. `group_by_scalar_projection_keys_match_captured_exec` compara tres respuestas y `group_by_function_projection_keys_match_captured_exec` compara 203, con alias de expresión y tabla, múltiples claves, tipos, valores, cadena vacía y NULL. Noventa y ocho errores conservan mensaje y posición, incluidos argumentos fuente no agrupados, ramas no constantes de `switch` y patrones que deben ser constantes. Se cubren dominios inválidos de `asin`/`acos`, `atan2` con dos columnas y `pi()` como constante; coerciones de `greatest`/`least`; escalas de redondeo fija y por columna; empates de `round_half_even`; selección de fallback en `coalesce`; igualdad en `nullif`; casts con tipo de salida numérico y sintaxis `CAST`/`::`; extracción de componentes de DATE/TIMESTAMP, incluido NULL y timestamps con fracción `.123456` (`millis = 123`, `micros = 456`, `nanos = 0`); suma y diferencia temporal, `extract`, floor/ceil de timestamps y zonas horarias IANA de `to_utc`/`to_timezone`, calendario UTC, formato constante de `to_str`, y entradas DATE/TIMESTAMP/NULL para las funciones de calendario. `to_pg_date` conserva DATE y devuelve NULL para entradas inválidas o NULL. `to_date` y `to_timestamp` aceptan literales o columnas con formato constante, preservan NULL y rechazan una columna fuente no agrupada con la posición upstream. `md5`, `sha1` y `sha256` producen VARCHAR hexadecimal sobre texto UTF-8, preservan NULL y validan las dependencias agrupadas. `to_uuid`, `json_extract` y `to_char(from_base64(...))` conservan tipos, valores UTF-8 y dependencias; `to_uuid` reporta el último argumento fuente no agrupado según el oráculo. `to_lowercase`/`to_uppercase` conservan alias y texto UTF-8; `length_bytes` cuenta bytes UTF-8 y devuelve -1 para NULL; `size_pretty` conserva STRING, NULL y el formato de tamaños del oráculo. `netmask`, `geo_distance_meters`, `geo_within_radius_latlon`, `within_radius`, `within_box` y `make_geohash` evalúan entradas constantes o columnas agrupadas y conservan tipos IPv4, DOUBLE, BOOLEAN y GEOHASH. Las capturas cubren NULL, alias, precisión de distancia, límites que distinguen `-0.0` y `0.0`, y la posición de la última columna fuente no agrupada según QuestDB. Los errores con alias conservan la posición de la columna fuente mediante el texto original guardado en el AST. `ln(0)`/`log(0)` y el overflow de `exp(1000)` se imprimen como NULL; QuestDB agrupa `cot(0)` con NULL, regla que reproduce el port. QuestDB rechaza un reemplazo dinámico de `regexp_replace` con `not implemented: dynamic replacement would be slow to execute`; el port conserva ese error en `GROUP BY`. `spread_bps`, `wmid` y `power` conservan como claves NaN o infinito los resultados no finitos derivados de entradas válidas: se imprimen como NULL, pero no se fusionan con el NULL de entrada.

Verificación actual: `tools/check.sh` pasó con 298 pruebas de `qdb-sql`, 4 de storage y 3 de types. `tools/run_differential.sh` comparó 5466 capturas; el endpoint vivo confirmó 4447 recuentos de filas, 272 respuestas completas de `GROUP BY` y 119 errores completos. `cargo fmt --all -- --check`, `git diff --check` y `bash -n tools/run_differential.sh` pasan.

## Funciones geoespaciales como clave de `GROUP BY` sin agregación

Se añadieron `netmask`, `geo_distance_meters`, `geo_within_radius_latlon`, `within_radius`, `within_box` y `make_geohash` a las proyecciones agrupadas. Se conservan los tipos de resultado IPv4, DOUBLE, BOOLEAN y GEOHASH; las capturas cubren alias, NULL, cálculo de distancia, límites y las posiciones de columnas fuente no agrupadas. `within_box` usa el orden de punto flotante de Java para distinguir `-0.0` de `0.0`, y los errores con argumentos de columna repetidos señalan la última aparición, como QuestDB 10.1.0.

Las siete respuestas completas y seis errores se añadieron a `group_by_keys_function_oracle.jsonl` y sus manifiestos. `tools/check.sh` pasó; `tools/run_differential.sh` comparó las capturas y el endpoint vivo confirmó las 191 respuestas y 85 errores completos de funciones agrupadas.

## Funciones de catálogo como clave de `GROUP BY` sin agregación

Se añadieron `format_type`, `current_setting`, `pg_get_partkeydef` y `pg_get_expr` al ejecutor de proyecciones agrupadas. Se conservan los resultados STRING, incluidos NULL y cadena vacía; las capturas verifican argumentos constantes, columnas fuente agrupadas, alias y errores por dependencias no agrupadas. `current_setting(columna)` conserva el rechazo de QuestDB y la posición 0 observada en esta forma agrupada.

Las siete respuestas completas y cuatro errores se capturaron del oráculo en los manifiestos y `group_by_keys_function_oracle.jsonl`. `tools/check.sh` pasó; `tools/run_differential.sh` confirmó 198 respuestas y 89 errores completos de proyecciones de funciones en `GROUP BY`.

## Funciones de contexto y `pg_catalog` como clave de `GROUP BY` sin agregación

Se añadieron `current_schema`, `current_database`, `current_user`, `session_user`, `current_catalog`, `version`, `pg_is_in_recovery` y las variantes cualificadas compatibles con `pg_catalog`. Una respuesta completa cubre las once expresiones, sus alias, tipos STRING/BOOLEAN y valores (`public`, `qdb`, `admin`, la versión de QuestDB y `false`) en los tres grupos de `location`.

La respuesta se capturó de QuestDB 10.1.0 y se añadió a `group_by_keys_function_oracle.jsonl`. `tools/check.sh` pasó; el runner confirmó 199 respuestas y 89 errores de funciones como claves agrupadas, además de comparar columnas, valores y recuentos de la nueva respuesta con el endpoint vivo.

## Funciones de `pg_catalog` como clave de `GROUP BY` sin agregación

Se añadieron `pg_catalog.pg_get_userbyid`, `pg_catalog.pg_table_is_visible` y `pg_catalog.age`. Una consulta agrupada por las columnas fuente compara los tipos STRING, BOOLEAN y LONG y los valores fijos (`public`, `true`, `0`). Tres errores conservan posiciones de columnas no agrupadas con alias cualificado; otros tres validan mensaje y posición de aridad incorrecta en la llamada.

La respuesta y los seis errores se capturaron de QuestDB 10.1.0. `tools/check.sh` pasó; el runner confirmó 200 respuestas y 95 errores de funciones agrupadas, incluyendo comparación viva completa de la nueva consulta.

## `typeof` y `not` como clave de `GROUP BY` sin agregación

Se añadieron ambas funciones a las proyecciones agrupadas. Las capturas cubren los tipos STRING/BOOLEAN, `typeof(NULL)`, `not(NULL)`, alias, aritmética agrupada y argumentos agrupados. `not(a, b)` ignora los argumentos anteriores y solo depende del último, igual que QuestDB. Tres errores conservan el mensaje y la posición de columnas no agrupadas y aridad incorrecta, incluso con alias de tabla.

Tres respuestas completas y tres errores se capturaron de QuestDB 10.1.0. `tools/check.sh` pasó con 298 pruebas de `qdb-sql`; el runner confirmó las 203 respuestas y 98 errores completos de funciones agrupadas y comparó la nueva evidencia en columnas, datasets, recuentos, mensajes y posiciones.

## `day_of_week` y `week_of_year` como clave de `GROUP BY` sin agregación

Se añadieron ambas funciones a las proyecciones agrupadas. Las capturas completas verifican tipo INT, valores, argumentos NULL, alias de columna y agrupación por la expresión exacta. Las funciones también respetan la validación de columnas fuente no agrupadas: dos errores conservan mensaje y posición de QuestDB 10.1.0, incluyendo referencias cualificadas.

Tres respuestas completas y dos errores se capturaron del oráculo. Los tests de proyección agrupada pasan; `tools/check.sh` pasó con 298 pruebas de `qdb-sql`, 4 de storage y 3 de types. `tools/run_differential.sh` comparó las 206 respuestas y 100 errores completos de funciones agrupadas; el endpoint vivo verificó las nuevas columnas, datasets, recuentos y posiciones.

## `haversine_dist_deg` y `GROUP BY`

El oráculo confirma que `haversine_dist_deg` es un agregado: funciona en el `SELECT` agrupado y devuelve DOUBLE, pero QuestDB rechaza la llamada si aparece como clave en `GROUP BY`. El port ya acumulaba correctamente la distancia por grupo; se añadió la validación que produce el error upstream `aggregate functions are not allowed in GROUP BY` con su posición.

Dos respuestas completas cubren la distancia y `count()` por `location` (incluido NULL), y grupos de una fila; un error verifica el rechazo de un agregado como clave. Las capturas se comparan en `group_by_multiple_aggregates_match_captured_exec` y `group_by_multiple_aggregate_errors_match_captured_exec`, y el runner las cotejó íntegramente contra QuestDB 10.1.0.

`tools/check.sh` pasó con 298 pruebas de `qdb-sql`, 4 de storage y 3 de types. `tools/run_differential.sh` comparó 5470 capturas; el endpoint vivo confirmó 4447 recuentos de filas, 276 respuestas completas de `GROUP BY` y 125 errores completos.

## Proyecciones booleanas `BETWEEN` e `IN` en `GROUP BY` sin agregación

`BETWEEN`, `NOT BETWEEN` e `IN` se pueden proyectar desde claves fuente agrupadas. Se verificaron tipos BOOLEAN, alias de tabla, NULL en límites/listas y listas de texto UTF-8. `IN` también se admite como clave exacta de agrupación. QuestDB 10.1.0 rechaza `BETWEEN` infix como clave exacta con `literal expected`; el port reproduce el error y su posición. Cuatro respuestas completas y seis errores se comparan en los tests y contra el endpoint vivo. Los límites de `BETWEEN` pueden usar columnas agrupadas. Se reproduce el rechazo de la forma `between(...)` como clave exacta y el error de tipo STRING con su posición.

Verificación final: `tools/check.sh` pasó con 298 pruebas de `qdb-sql`, 4 de storage y 3 de types. `tools/run_differential.sh` comparó 5470 capturas; el endpoint vivo confirmó 4447 recuentos de filas, 276 respuestas completas de `GROUP BY` y 125 errores completos.

## Comparaciones y predicados de NULL en `GROUP BY` sin agregación

Se habilitaron `=`, `<>`, `<`, `>`, `<=`, `>=`, `IS NULL` e `IS NOT NULL` como proyecciones agrupadas. La matriz cubre LONG, SYMBOL, BOOLEAN, TIMESTAMP, `NULL`, operandos columna-columna, claves exactas simples y combinadas, alias de tabla y alias de salida. Se conservaron conversiones y errores de tipo del oráculo, incluido el error `STRING -> INT` que QuestDB devuelve en una respuesta HTTP 200 con cuerpo de error. El runner ahora valida ese caso y los errores HTTP 400 por el contenido del error.

Se añadieron siete respuestas completas y cinco errores; el test de proyecciones de funciones agrupadas compara 217 respuestas y 111 errores. Las respuestas nuevas se contrastaron contra QuestDB 10.1.0 en columnas, tipos, datasets y count; los errores en mensaje y posición.

Verificación: `tools/check.sh` pasó con 298 pruebas de `qdb-sql`, 4 de storage y 3 de types. `tools/run_differential.sh` comparó 5477 capturas; el endpoint vivo confirmó 4447 recuentos de filas, 283 respuestas completas de `GROUP BY` y 130 errores completos. También pasaron `cargo fmt --all -- --check`, `git diff --check` y `bash -n tools/run_differential.sh`.

## Próxima tarea

## Proyecciones booleanas `AND`, `OR` y `not(...)` en `GROUP BY` sin agregación

Se añadieron `AND` y `OR` a las proyecciones agrupadas, y se verificó `not(...)` tanto desde columnas agrupadas como como clave exacta. Las capturas cubren expresiones exactas, alias cualificados, columnas dependientes, NULL y errores de tipos. QuestDB trata NULL como falso en estas expresiones booleanas; el port reproduce los valores y tipos BOOLEAN observados. Los errores de tipos conservan el mensaje y la posición del oráculo. El parser ahora consume también el nombre cualificado del operando derecho de `AND`/`OR` antes de retirar el alias de tabla.

Se añadieron cuatro respuestas completas y tres errores, capturados de QuestDB 10.1.0. `tools/check.sh` pasó con 298 pruebas de `qdb-sql`, 4 de storage y 3 de types. `tools/run_differential.sh` comparó 5453 capturas; el endpoint vivo confirmó 4447 recuentos de filas, 287 respuestas completas de `GROUP BY` y 133 errores completos. También pasaron `cargo fmt --all -- --check` y `git diff --check`.

## Próxima tarea

## Proyecciones `LIKE`, `ILIKE` y regex en `GROUP BY` sin agregación

Se añadieron regresiones para `LIKE`, `ILIKE`, `NOT LIKE`, `NOT ILIKE`, `~` y `!~` como proyecciones sobre claves agrupadas y como claves exactas. Los casos comparan formas infix y de función, cadenas UTF-8, NULL en el valor/patrón, dependencias cualificadas, claves exactas y errores de patrón dinámico, tipo y sintaxis. QuestDB devuelve `false` tanto para `~ NULL` como para `!~ NULL`; el port ahora reproduce ese resultado. Los patrones regex inválidos de clase abierta conservan el texto y la posición del oráculo.

Se añadieron cuatro respuestas completas y cuatro errores, capturados de QuestDB 10.1.0. `tools/check.sh` pasó con 298 pruebas de `qdb-sql`, 4 de storage y 3 de types. `tools/run_differential.sh` comparó 5457 capturas; el endpoint vivo confirmó 4447 recuentos de filas, 291 respuestas completas de `GROUP BY` y 137 errores completos. En el conjunto de funciones agrupadas se verificaron en vivo 225 respuestas (columnas, tipos, datasets y count) y 118 errores (mensaje y posición). También pasaron `cargo fmt --all -- --check`, `git diff --check` y `bash -n tools/run_differential.sh`.

El dialecto regex completo de QuestDB todavía no está demostrado equivalente al crate `regex`; el resultado de esta tarea cubre los casos capturados, no la paridad de todos los patrones.

## Próxima tarea

## `NOT` prefijo en proyecciones booleanas de `GROUP BY`

Se implementó `NOT` prefijo sobre columnas booleanas, comparaciones y predicados parentizados, preservando la precedencia respecto de `AND` y `OR`. La forma `not(...)` de llamada sigue usando la semántica previa. Las regresiones cubren NULL, clave exacta, alias cualificado, tipo incompatible y columnas ausentes o no agrupadas. La comparación `NOT (id = 1) ... ORDER BY id` confirmó que se puede ordenar por una clave agrupada que no aparece en el `SELECT`; el ejecutor ahora compara ese valor de clave directamente.

Se añadieron siete respuestas completas y tres errores de QuestDB 10.1.0. `cargo test -p qdb-sql group_by_function_projection_` pasó. `tools/check.sh` pasó con 298 pruebas de `qdb-sql`, 4 de storage y 3 de types. `TMPDIR="$PWD/target/oracle-tmp" tools/run_differential.sh` comparó las capturas y contrastó en vivo 4447 recuentos de filas, 232 respuestas completas y 121 errores del grupo de funciones agrupadas. Las pruebas de `not(...)` también pasan. `cargo fmt --all` y `git diff --check` pasaron.

La forma parentizada se interpreta como prefijo si contiene un operador de predicado; una llamada simple como `not(null)` mantiene su resolución anterior. Quedan por contrastar combinaciones anidadas de NOT con `IN`, patrones y regex.

## Próxima tarea

## `NOT` prefijo anidado con `IN`, patrones y regex en `GROUP BY`

Se extendieron las regresiones a `IN`, `LIKE`, `ILIKE`, `~` y `!~`, también con alias cualificados, NULL y combinación con `OR`. Se corrigió la detección del prefijo parentizado para `ILIKE`, que comparte sintaxis con la forma de llamada `not(...)`. Los errores por columna ausente conservan ahora el nombre cualificado y su posición; también coinciden los errores de tipo, patrón dinámico o inválido y dependencia sin agrupar.

Se añadieron nueve respuestas completas y ocho errores capturados de QuestDB 10.1.0. `cargo test -p qdb-sql group_by_function_projection_` pasó y comparó 241 respuestas y 129 errores. `tools/run_differential.sh` volvió a cotejar todas las capturas, comprobó en vivo los recuentos de 4447 consultas y comparó las columnas, tipos, datasets, recuentos y errores del conjunto de funciones agrupadas (241 respuestas y 129 errores). `tools/check.sh` y `git diff --check` pasaron.

El soporte regex sigue limitado a los patrones demostrados; el dialecto completo de QuestDB no se considera compatible.

## Próxima tarea

## `NOT` prefijo con predicados `IS NULL` y `BETWEEN`

El parser ahora incorpora `IS NULL`/`IS NOT NULL` al AST de filtros y acepta `NOT` anidado (`NOT NOT`) en proyecciones agrupadas. Se contrastaron ambos predicados de NULL, `BETWEEN`, `NOT BETWEEN`, mezcla con `OR`, alias cualificados y tipo incompatible. También quedó cubierta la clave exacta `GROUP BY NOT (id BETWEEN ...)`: QuestDB la admite aunque rechaza `BETWEEN` sin envolver como clave exacta. Se conserva ese rechazo para la forma desnuda y se devuelve el error de tipo de `BETWEEN` con la posición del operador.

Se añadieron nueve respuestas completas y cuatro errores del oráculo. `tools/check.sh` pasó con 298 pruebas de `qdb-sql`, 4 de storage y 3 de types. `TMPDIR="$PWD/target/oracle-tmp" tools/run_differential.sh` comparó las capturas y, en vivo, 4447 recuentos de filas, 250 respuestas completas y 133 errores para funciones agrupadas. También pasaron `cargo fmt --all -- --check` y `git diff --check`.

## Próxima tarea

## `NOT` prefijo sobre comparaciones aritméticas en `GROUP BY`

Se añadieron comparaciones y combinaciones booleanas que contienen aritmética, claves exactas y alias cualificados. Se confirmó que `LONG + SYMBOL` produce el error de operador sin overload en QuestDB, mientras que la aritmética STRING numérica conserva la coerción por fila. `LIMIT 0` no evalúa una conversión inválida dependiente de los valores, pero sí reporta el mismatch estático `LONG + SYMBOL`. Los errores de columnas cualificadas conservan texto y posición.

Se capturaron cinco respuestas completas y cinco errores de QuestDB 10.1.0. El test de proyecciones agrupadas compara ahora 255 respuestas y 138 errores; `tools/run_differential.sh` confirmó esos resultados completos contra el endpoint y cotejó 4447 recuentos de filas. `tools/check.sh` pasó con 298 pruebas de `qdb-sql`, 4 de storage y 3 de types. También se verificó que `split_part` rechaza un índice de columna LONG y conserva el error de índice constante para INT dinámico. Pasaron `cargo fmt --all -- --check`, `git diff --check` y `bash -n tools/run_differential.sh`.

## `NOT` prefijo en proyecciones ordinarias de `SELECT`

Se añadieron siete respuestas completas y tres errores capturados de QuestDB 10.1.0 para proyecciones sin `GROUP BY`: alias, `ORDER BY`, `LIKE`, `ILIKE`, regex, precedencia booleana, NULL, mismatch de tipo y columna cualificada ausente. El ejecutor conserva el nombre cualificado y remapea los errores de resolución/evaluación a las posiciones del texto SQL.

`tools/check.sh` pasó con 300 pruebas de `qdb-sql`, 4 de storage y las pruebas del servidor/CLI. `TMPDIR="$PWD/target/oracle-tmp" tools/run_differential.sh` cotejó las capturas y confirmó en vivo los recuentos de 4447 consultas, además de las 7 respuestas completas y 3 errores de esta tarea. También pasaron `cargo fmt --all -- --check`, `git diff --check` y `bash -n tools/run_differential.sh`.

## `NOT` prefijo con predicados anidados en proyecciones ordinarias

Se extendieron las proyecciones sin `GROUP BY` a `IN`, `IS NULL`, `IS NOT NULL`, `BETWEEN`, `NOT NOT` y patrones anidados, incluidos patrones NULL. Las capturas verifican los valores BOOLEAN y sus tipos. Se ajustaron los diagnósticos ordinarios para conservar cualificadores y ubicar errores de tipo `IN` en el literal, y errores de `BETWEEN` en el operador.

Se añadieron siete respuestas completas y tres errores capturados de QuestDB 10.1.0. `tools/check.sh` pasó con 302 pruebas de `qdb-sql`, 4 de storage y las pruebas del servidor/CLI. `TMPDIR="$PWD/target/oracle-tmp" tools/run_differential.sh` volvió a comprobar en vivo los recuentos de 4447 consultas; las 7 respuestas completas y 3 errores nuevos coincidieron con el oráculo. También pasaron `cargo fmt --all -- --check`, `git diff --check` y `bash -n tools/run_differential.sh`.

## `NOT` con lógica booleana anidada en proyecciones ordinarias

Se añadieron diez respuestas y tres errores capturados para `NOT` sobre agrupaciones `AND`/`OR`, precedencia con operadores externos, paréntesis anidados, `NOT NOT`, alias y ordenación. Se corrigió el parser para que un comparador situado después de un `AND`/`OR` de nivel superior no absorba toda la expresión como una comparación extendida. La validación detecta columnas no booleanas antes de evaluar filas y reporta texto y posición como QuestDB. También se capturó la coerción de `=`/`<>` con texto no numérico contra una columna numérica: el literal no coincide, mientras que los comparadores relacionales conservan su error de conversión.

`tools/check.sh` pasó con 304 pruebas de `qdb-sql`, 4 de storage y las pruebas de servidor/CLI. `TMPDIR="$PWD/target/oracle-tmp" tools/run_differential.sh` confirmó en vivo 4447 recuentos de filas y comparó 10 respuestas completas y 3 errores nuevos, además de los conjuntos previos. También pasaron `cargo fmt --all -- --check`, `git diff --check` y `bash -n tools/run_differential.sh`.

## Proyecciones booleanas ordinarias con `AND` y `OR`

Se añadieron quince respuestas completas y once errores capturados para proyecciones sin `NOT`. Los casos cubren comparaciones, `IN`, `IS NULL`, `BETWEEN`, `LIKE`, `ILIKE`, regex válida, paréntesis, operandos literales y NULL, alias, ordenación y errores de tipo, patrón dinámico, regex inválida y columna ausente con posiciones cotejadas contra QuestDB 10.1.0. El parser conserva el `AND` de `BETWEEN` como parte del predicado; NULL dentro de una operación lógica produce `false`, mientras que `WHERE NULL` directo mantiene su error. Los errores de compilación regex conservan la posición del patrón para `~` y `!~`, también dentro de paréntesis.

`tools/check.sh` pasó con 308 pruebas de `qdb-sql`, 4 de storage, 3 de types y las pruebas de servidor/CLI. `TMPDIR="$PWD/target/oracle-tmp" tools/run_differential.sh` cotejó las capturas y confirmó en vivo 4447 recuentos de filas, 15 respuestas completas (columnas, dataset y count) y 11 errores nuevos (mensaje y posición). También pasaron `cargo fmt --all -- --check`, `git diff --check` y `bash -n tools/run_differential.sh`.

## Posiciones de errores regex inválidos en `WHERE`

Se añadieron once casos capturados para `~`/`!~`: clase y grupo sin cerrar, barra invertida final y meta carácter colgante, alias cualificado, `AND`/`OR`, paréntesis, rama que no se ejecutaría por cortocircuito y `LIMIT 0`. El port conserva la posición del patrón y valida el regex antes de recorrer filas, como QuestDB. Se ajustó también el texto de errores por grupo sin cerrar y barra invertida final para que coincida con el oráculo.

`tools/check.sh` pasó con 309 pruebas de `qdb-sql`, 4 de storage, 3 de types y las pruebas de servidor/CLI. `TMPDIR="$PWD/target/oracle-tmp" tools/run_differential.sh` cotejó las capturas, confirmó en vivo 4447 recuentos y comparó los 11 mensajes y posiciones regex contra `/exec`. También pasaron `cargo fmt --all -- --check`, `git diff --check` y `bash -n tools/run_differential.sh`. El runner deja su respuesta temporal dentro de `target/oracle-tmp`.

## Dialecto regex Java en filtros `WHERE`

Se contrastaron con QuestDB 10.1.0 y se añadieron catorce casos para look-ahead/look-behind positivos y negativos, backreferences numéricas y con nombre, propiedades Unicode, `\\h`/`\\H`, `\\R`, grupos atómicos, cuantificadores posesivos e insensitive matching. El matcher conserva el camino lineal de `regex` y recurre a `fancy-regex` para construcciones de backtracking; se traduce `\\h`/`\\H` para conservar la semántica Java de espacio horizontal.

`regex_java_extensions_match_captured_exec` pasó con las catorce respuestas capturadas y `tools/run_differential.sh` las comparó en vivo por columnas, dataset y count. También coincidieron 4447 recuentos de filas y los once errores regex con mensaje y posición. `tools/check.sh` pasó con 310 pruebas de `qdb-sql`, 4 de storage, 3 de types y las pruebas de servidor/CLI. También pasaron `cargo fmt --all -- --check`, `git diff --check` y `bash -n tools/run_differential.sh`.

Esto amplía la compatibilidad demostrada, pero no prueba equivalencia completa con `java.util.regex.Pattern`; en particular, no se han inventariado todas las escapes dentro de clases de caracteres ni todas las diferencias de diagnóstico.

## Próxima tarea

## `regexp_replace` como reemplazo regex

Se reemplazó la sustitución literal por compilación del patrón constante y reemplazo de todas las coincidencias. El reemplazo interpreta referencias Java `$0`, `$n` y `${nombre}`, consume dígitos de grupo de forma codiciosa como `Matcher.appendReplacement`, copia grupos opcionales no capturados como cadena vacía y trata `\\` como escape de un carácter. Se conservó la validación de patrón/reemplazo dinámicos y se añadieron posiciones de argumentos al AST para igualar los diagnósticos de QuestDB. La ejecución cubre `SELECT` ordinario y `GROUP BY`.

Se capturaron doce respuestas completas (incluidos Unicode, patrón con look-behind, referencia `$11`, columna cualificada y agrupación) y seis errores. `regexp_replace_java_replacements_match_captured_exec` y `regexp_replace_errors_match_captured_exec` pasaron; `tools/run_differential.sh` confirmó las doce respuestas y seis errores en vivo, además de 4447 recuentos de filas. `tools/check.sh` pasó con 312 pruebas de `qdb-sql`, 4 de storage, 3 de types y las pruebas de servidor/CLI. También pasaron `cargo fmt --all -- --check`, `git diff --check` y `bash -n tools/run_differential.sh`.

La gramática completa de reemplazo Java y parámetros bind/runtime-constant aún no están cubiertos. El límite de longitud está fijado al valor por defecto observado, no es configurable como en el servidor.

## Próxima tarea

## Escapes Java regex adicionales

Se añadieron doce respuestas capturadas para `\\Q...\\E`, `\\H` en clases, `\\Z`/`\\z` y propiedades de bloque/script Unicode, incluyendo filtros, proyecciones y `regexp_replace`; también hay dos errores capturados para las anclas dentro de clases. `\\Z` se traduce como aserción de posición para no consumir el terminador final, mientras que `\\z` apunta al fin absoluto. Las pruebas de newline verifican además que `regexp_replace` deja intacto el LF final.

`tools/check.sh` pasó con 315 pruebas de `qdb-sql`, 4 de storage y 3 de types, más las pruebas de servidor/CLI. `tools/run_differential.sh` confirmó en vivo 4447 recuentos de filas; comparó las doce respuestas adicionales y las dos respuestas de anclas con newline completas (columnas, dataset y count), y los dos errores Java regex (mensaje y posición). También pasaron `cargo fmt --all -- --check`, `git diff --check` y `bash -n tools/run_differential.sh`.

La traducción de nombres de bloques Java cubre un conjunto acotado y no se ha demostrado equivalencia completa con `java.util.regex.Pattern`; la compatibilidad de estos escapes en toda forma de proyección todavía no se declara.

## Próxima tarea

## Clases Java regex anidadas y propiedades negativas

Se corrigió la traducción de `\\P{In...}` dentro de clases: ahora conserva la negación como una clase anidada en vez de convertirla en la propiedad positiva. Se añadieron nueve capturas de QuestDB 10.1.0 para clases anidadas por unión e intersección, propiedades positivas y negativas dentro de clases, y el uso en filtros, proyecciones y `regexp_replace`. Los resultados ejercitan texto ASCII y griego, cuantificadores y una intersección vacía.

`regex_java_nested_classes_and_negative_properties_match_captured_exec` pasó con las nueve respuestas. `tools/check.sh` pasó con 316 pruebas de `qdb-sql`, 4 de storage y 3 de types, además de las pruebas de servidor/CLI. `tools/run_differential.sh` confirmó las nueve respuestas en vivo por columnas, dataset y count y 4447 recuentos de filas; también pasaron `cargo fmt --all -- --check`, `git diff --check` y `bash -n tools/run_differential.sh`.

La traducción manual de nombres de bloques sigue cubriendo solo un subconjunto (`Basic_Latin`, Latin-1, Latin Extended A/B, Greek y Cyrillic); las propiedades dentro de clases fuera de esa lista dependen de lo que acepte `regex` y no acreditan el catálogo Java completo.

## Próxima tarea

## Catálogo Java de bloques Unicode

Se sustituyó la tabla parcial por el catálogo Unicode 16.0 completo: 338 rangos y 338 claves normalizadas que cubren 559 nombres/variantes aceptados por `Pattern`. Se probaron 705 nombres candidatos obtenidos de `Blocks.txt` y `PropertyValueAliases.txt` contra el oráculo Java 25; los 146 rechazados por Java producen ahora el diagnóstico `Unknown character property name` con índice del patrón y posición SQL. El traductor conserva `\\p{In...}` y `\\P{In...}` dentro y fuera de clases, incluidos alias históricos como `InGreek`, nombres con espacios/guiones y bloques suplementarios. El archivo de datos está en `crates/qdb-sql/src/unicode_blocks.txt`.

Se añadieron dieciséis respuestas completas para filtros, proyecciones y `regexp_replace`, y cuatro errores capturados. Los casos incluyen los bloques de sustitutos: sus propiedades positivas no coinciden con escalares Unicode y las negadas sí. `regex_java_unicode_blocks_match_captured_exec`, `regex_java_unknown_unicode_blocks_match_captured_errors`, la prueba de rangos representativos y la prueba que compila y comprueba ambos límites de los 338 rangos pasaron. `tools/check.sh` pasó con 320 pruebas de `qdb-sql`, 4 de storage y 3 de types, además de servidor/CLI. `tools/run_differential.sh` confirmó en vivo 4447 recuentos de filas, las dieciséis respuestas completas y los cuatro errores con mensaje/posición. También pasaron formato, `git diff --check` y `bash -n tools/run_differential.sh`.

El catálogo corresponde a Unicode 16.0, coincidente con los bloques reconocidos por el runtime Java 25 probado; esto no prueba equivalencia completa con `java.util.regex.Pattern` para el resto de la sintaxis.

## Próxima tarea

## Escapes Java regex de caracteres

Se portaron `\\a`, `\\e`, `\\cX`, `\\xhh`, `\\x{...}`, `\\uFFFF`, escapes octales y `\\N{nombre}`. Los octales siguen los límites de Java (el primer dígito después de `\\0` decide cuántos dígitos se consumen). Los nombres se resuelven con una tabla generada de Unicode 16.0 `UnicodeData` y `NameAliases`, guardada en `crates/qdb-sql/src/unicode_names.txt`; se aceptan los nombres canónicos sin distinguir mayúsculas y los alias Unicode.

Se añadieron dieciséis respuestas capturadas en filtros y `regexp_replace`, más seis errores de sintaxis/nombre con posición. `regex_java_character_escapes_match_captured_exec` y `regex_java_character_escape_errors_match_captured_exec` pasaron. `tools/check.sh` pasó con 322 pruebas de `qdb-sql`, 4 de storage y 3 de types, además de servidor/CLI. `tools/run_differential.sh` confirmó en vivo 4447 recuentos de filas, las dieciséis respuestas completas y los seis errores; también pasaron `cargo fmt --all` y la comprobación clippy de `tools/check.sh`.

La tabla de nombres usa la versión Unicode 16.0 incluida en el runtime de referencia probado; no demuestra equivalencia con versiones posteriores de Java ni con extensiones fuera de UnicodeData y NameAliases.

## Clases abreviadas y flags Java regex

Se tradujeron las clases abreviadas Java `\\d`/`\\D`, `\\s`/`\\S` y `\\w`/`\\W` con su semántica ASCII predeterminada. Los flags `(?U)` y `(?-U)` cambian las clases Unicode, tanto globalmente como en grupos de alcance limitado; `(?i)` conserva el case-insensitive ASCII de Java, mientras que `(?iu)` y `(?U)(?i)` habilitan el plegado Unicode. Las clases Unicode cubren dígitos decimales, White_Space y letras, marcas, conectores y Join_Control; las pruebas incluyen marcas combinantes, join control, puntuación de conexión, dígitos no ASCII y casos Kelvin/Greek.

Se añadieron 34 respuestas capturadas para filtros, proyecciones y `regexp_replace`, más cuatro errores de flags, rangos de repetición y clases incompletas. `regex_java_predefined_classes_match_captured_exec` y `regex_java_predefined_class_errors_match_captured_exec` pasaron. `tools/check.sh` pasó con 324 pruebas de `qdb-sql`, 4 de storage y 3 de types, además de servidor/CLI. `tools/run_differential.sh` confirmó en vivo 4447 recuentos, las 34 respuestas completas y los cuatro errores con mensaje/posición; también pasaron `cargo fmt --all`, `git diff --check` y `bash -n tools/run_differential.sh`.

La tabla Unicode de `regex` no se ha contrastado exhaustivamente carácter por carácter con `Character` de Java; las capturas acreditan las categorías y ejemplos listados, no paridad total de Unicode ni del dialecto regex.

## Flags Java `UNIX_LINES` y `COMMENTS`

Se portaron `(?d)`/`(?-d)` y `(?x)`/`(?-x)`, con grupos globales y de alcance limitado. `(?d)` hace que el punto y las anclas multilinea reconozcan solo LF; el modo Java predeterminado también trata CR, NEL, LS y PS como terminadores, y CRLF forma un límite único. `(?x)` ignora espacio ASCII y comentarios, incluidos comentarios dentro de clases; los espacios Unicode que `Character` no considera espacio de patrón permanecen literales. Los chequeos de escapes, propiedades y cuantificadores no interpretan texto dentro de esos comentarios.

Se añadieron 39 respuestas capturadas para filtros, proyecciones y `regexp_replace`, además de dos errores. `regex_java_remaining_flags_match_captured_exec` y `regex_java_remaining_flag_errors_match_captured_exec` pasaron. `tools/check.sh` pasó con 326 pruebas de `qdb-sql`, 4 de storage y 3 de types, además de servidor/CLI. `tools/run_differential.sh` confirmó en vivo 4447 recuentos, las 39 respuestas completas y los dos errores con posición. También pasaron `cargo fmt --all -- --check`, `git diff --check` y `bash -n tools/run_differential.sh`.

La comparación cubre los terminadores y casos de comentarios capturados; no acredita equivalencia general entre los analizadores de `Pattern` y Rust regex.

## Límites de palabra Java `\b` y `\B`

Se implementaron los límites ASCII predeterminados, los límites Unicode bajo `(?U)`, el tratamiento Java de marcas combinantes y la negación `\B`. Se añadieron veintiocho respuestas completas para filtros, proyecciones y `regexp_replace`, con griego, dígitos no ASCII, conectores, join control, marcas y casos Kelvin bajo `(?i)`, incluidos escapes Unicode dentro de clases; también dos errores de `\b`/`\B` dentro de clases. `regex_java_word_boundaries_match_captured_exec` y `regex_java_word_boundary_errors_match_captured_exec` pasaron. En la validación conjunta posterior, `tools/check.sh` pasó con 330 pruebas de `qdb-sql`, 4 de storage y 3 de types. `tools/run_differential.sh` confirmó en vivo 4447 recuentos, las veintiocho respuestas completas y los dos errores con posición.

La ruta de backtracking requiere retirar el flag Rust `-u`, que `fancy-regex` no admite junto con lookarounds. Se conserva el plegado ASCII para literales y clases, incluidos escapes Unicode, y las propiedades como `\p{L}` no se alteran al expandir variantes ASCII. Las capturas no demuestran equivalencia completa con `java.util.regex.Pattern`.

## Ancla Java `\G`

Se implementó `\G` como posición inicial del matcher y como final de la coincidencia anterior durante `regexp_replace` global. Se añadieron seis respuestas completas para reemplazos contiguos, inicio que no coincide, alternancia, coincidencias de longitud cero, texto suplementario Unicode y filtro `WHERE`; una captura adicional verifica el error dentro de clases. `regex_java_global_anchor_matches_captured_exec` y `regex_java_global_anchor_errors_match_captured_exec` pasaron. La validación conjunta pasó con `tools/check.sh` (330 pruebas de `qdb-sql`, 4 de storage y 3 de types) y `tools/run_differential.sh` (4447 recuentos, seis respuestas completas y el error con posición). También pasaron `cargo fmt --all -- --check`, `git diff --check` y `bash -n tools/run_differential.sh`.

## Límites de grafema Java `\X` y `\b{g}`

Se fijó la segmentación extendida de `unicode-segmentation` 1.12.0. En patrones con grafemas, `\X` se materializa como alternancia de los grafemas presentes en el texto y `\b{g}` como aserciones en los límites calculados de la cadena. Esto permite combinarlos con grupos de captura, cuantificadores, alternativas, anclas y otros look-around; el reemplazo conserva los grupos y `\b{g}` produce una coincidencia vacía en cada límite, también en la cadena vacía. Las capturas cubren marcas combinantes, emoji familiar con ZWJ, indicadores regionales, texto vacío, columnas `note` con NULL y filtros con límites al inicio y al final.

Se verificaron los 1093 casos de `GraphemeBreakTest-16.0.0.txt` contra QuestDB 10.1.0 en dos consultas agrupadas con separadores de control, y también directamente contra `unicode-segmentation` 1.12.0 (`UNICODE_VERSION = 16.0.0`). El test `unicode_16_grapheme_break_test_vectors_match_segmentation` valida los límites y spans de cada vector. `regex_java_grapheme_boundaries_match_captured_exec` compara 36 respuestas completas; el runner diferencial también coteja esos resultados en vivo. Las regresiones adicionales cubren patrones compuestos, grupos, cuantificadores, alternativas, anclas, look-around, prefijos ambiguos y 1024 grafemas ASCII y combinados.

La ruta directa `\\X`/`\\b{g}` usa los spans de Unicode. Para `\\X` compuesto, se omiten las aserciones por aparición solo si se verifica que cada aparición, incluidas las variantes ASCII case-insensitive de Java, empieza y termina en límites reales; la optimización queda desactivada para `(?iu)`, más de 64 grafemas distintos o más de 4096 caracteres. `regex_java_unanchored_grapheme_boundaries_match_long_text` compara 1024 grafemas mixtos con el patrón agrupado `\\b{g}(?:\\X){1024}\\b{g}`: pasó en 0,39 s frente a 16,90 s antes de la optimización. El caso ambiguo `áa` repetido 128 veces, que conserva guardas, pasó en 1,70 s. El camino de reemplazo que empieza con `\\b{g}` avanza un carácter Unicode después de cada coincidencia no vacía, según las respuestas capturadas de QuestDB para los casos ASCII, combinados y case-insensitive.

## Próxima tarea

Reducir el coste de los casos largos donde `\\X` aún necesita guardas por prefijos ambiguos, como `áa`, y donde `\\b{g}` aparece sin un átomo `\\X` que permita quitar aserciones redundantes. Mantener la semántica capturada de avance de búsqueda en `regexp_replace`.
