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
| qdb-sql | 227 ok, incluido `wb_matches_captured_exec` |
| qdb-server | 2 ok (`query_fixture`) |

`tools/run_differential.sh`:

```text
captured-oracle: 2959 queries match tests/fixtures/h1/expected
captured-oracle: 363 queries match tests/fixtures/h2/expected
live-reference: 3322 queries matched row counts against http://127.0.0.1:19011/exec
```

Las salidas de `h1` y de `h2` se capturaron con el servidor vivo, desde `GET /exec`, antes de copiar los directorios. El TSV de `sensors` incluye `NULL`, cadena vacía, `-0.0`, `héllo` y `東京`. El de `days` incluye el símbolo `東京` compartido entre los dos días y el `NULL` de `id`.

`cargo tree -p qdb-server` solo muestra los cinco crates del workspace.

## Limitaciones

- `PARTITION BY NONE` y `DAY`, WAL desactivado, `_meta` versión 426, ficheros de columna sin sufijo de txn.
- HOUR, MONTH, YEAR, WEEK, Parquet y particiones composite se rechazan.
- La última partición usa `transient_row_count`. El tamaño guardado en su registro puede ser 0.
- `LIMIT`, `ORDER BY`, `count()`, `min()`, `max()`, `sum()`, `avg()`, `first()`, `last()`, `count_distinct()`, `concat()`, `||`, `length()`, `lower()`, `upper()`, `left()`, `right()`, `substring()`, `trim()`, `ltrim()`, `rtrim()`, `replace()`, `starts_with()`, `strpos()`, `position()`, `split_part()`, `to_lowercase()`, `to_uppercase()`, `lpad()`, `rpad()`, `quote_ident()`, `length_bytes()`, `size_pretty()`, `abs()`, `round()`, `floor()`, `ceil()`, `ceiling()`, `sign()`, `sqrt()`, `power()`, `ln()`, `log()`, `exp()`, `pi()`, `greatest()`, `least()`, `round_down()`, `round_up()`, `round_half_even()`, `coalesce()`, `nullif()`, `cast()`, `dateadd()`, `datediff()`, `year()`, `month()`, `day()`, `hour()`, `minute()`, `second()`, `millis()`, `micros()`, `nanos()`, `to_utc()`, `to_timezone()`, `to_pg_date()`, `current_schema()`, `current_database()`, `current_user()`, `session_user()`, `current_catalog()`, `version()`, `pg_is_in_recovery()`, `format_type()`, `regexp_replace()`, `current_setting()`, `pg_get_partkeydef()`, `pg_get_expr()`, `netmask()`, `geo_distance_meters()`, `geo_within_radius_latlon()`, `within_radius()`, `within_box()`, `day_of_week()`, `week_of_year()`, `date_trunc()`, `timestamp_floor()`, `timestamp_ceil()`, `is_leap_year()`, `days_in_month()`, `to_str()`, `is_end_of_month()`, `day_of_week_sunday_first()`, `to_timestamp()` de un texto numérico o con un patrón constante, `extract()`, `to_date()`, `md5()`, `sha1()`, `sha256()`, `sin()`, `cos()`, `tan()`, `asin()`, `acos()`, `atan()`, `atan2()`, `degrees()`, `radians()`, `cot()`, `typeof()`, `not()`, `stddev()`, `variance()`, `var_pop()`, `stddev_pop()`, `ksum()`, `nsum()`, `approx_count_distinct()`, `string_agg()`, `covar_pop()`, `covar_samp()`, `corr()`, `regr_slope()`, `regr_intercept()`, `regr_r2()`, `stddev_samp()`, `var_samp()`, `bool_and()`, `bool_or()`, `first_not_null()`, `last_not_null()`, `bit_and()`, `bit_or()`, `bit_xor()`, `geomean()`, `skewness()`, `mode()`, `approx_median()`, `approx_percentile()`, `arg_max()`, `arg_min()`, `string_distinct_agg()`, `kurtosis_pop()`, `kurtosis_samp()`, `kurtosis()`, `skewness_pop()`, `vwap()`, `twap()`, `sparkline()`, `weighted_avg()`, `weighted_stddev()`, `weighted_stddev_rel()`, `weighted_stddev_freq()`, `isOrdered()`, `percentile_disc()`, `percentile_cont()`, `skewness_samp()`, `quantile_cont()`, `quantile_disc()`, `glob()`, `ilike()`, `like()`, `switch()`, `bar()`, `haversine_dist_deg()`, `timestamp_sequence()`, `mid()`, `spread()`, `spread_bps()`, `wmid()`, `between()`, `in`, `=`, `<>`, `<`, `>`, `>=`, `<=`, `+`, `-`, `*`, `/`, `%`, `-value`, `~value`, `and`, `or`, `||`, `IS NULL`, `IS NOT NULL`, `BETWEEN`, `NOT BETWEEN`, `LIKE`, `NOT LIKE`, `ILIKE`, `NOT ILIKE`, `~`, `!~`, `&`, `|`, `^`, `=`, `<>`, `!=`, `<`, `>`, `>=`, `<=`, `::string`, `::long`, `::int`, `::double`, `::float`, `::boolean`, `::byte`, `::short`, `::char`, `::symbol`, `::varchar`, `::date`, `::timestamp` y `GROUP BY` de una columna están verificados. Un `SELECT` admite un solo agregado. El resto de SQL no.
- Sin escritura, sin red, sin el resto de tipos ni de SQL.
- BOOLEAN/BYTE/SHORT/CHAR no separan NULL de 0; es el formato upstream.
- Maven no está instalado, así que la suite Java no se ejecutó (`docs/blockers.md`).

## Próxima tarea

Añadir `make_geohash(lon, lat, bits)` y comprobarlo contra `/exec` en `sensors` y `days`. El encabezado conserva la grafía. `make_geohash(2.0, 41.0, 20)` es `GEOHASH(4c)` y devuelve `sp33`. `make_geohash(0, 0, 5)` es `GEOHASH(1c)` y devuelve `s`. `make_geohash(0, 0, 1)` es `GEOHASH(1b)` y devuelve `1`. `make_geohash(0, 0, 10)` es `GEOHASH(2c)` y devuelve `s0`. `make_geohash(0, 0, 25)` es `GEOHASH(5c)` y devuelve `s0000`. `make_geohash(0, 0, 60)` es `GEOHASH(12c)` y devuelve `s00000000000`. `make_geohash(0.0, 0.0, 2)` devuelve `11`, `make_geohash(0.0, 0.0, 3)` devuelve `110`, `make_geohash(0.0, 0.0, 4)` es `GEOHASH(4b)` y devuelve `1100`, y `make_geohash(0.0, 0.0, 6)` devuelve `110000`. `make_geohash(2.0, 41.0, 15)` es `GEOHASH(3c)` y devuelve `sp3`. `make_geohash(2.0, 41.0, 16)` es `GEOHASH(16b)` y devuelve `1100010101000110`. `make_geohash(-180, -90, 10)` encabeza `make_geohash(-(180), -(90), 10)` y devuelve `00`. `make_geohash(180, 90, 10)` devuelve `00`. `make_geohash(null, 0, 5)` devuelve `8`. `make_geohash(0, null, 5)` devuelve `h`. `make_geohash(reading, reading, 5)` devuelve `s`, `0`, `s` y `s`. `MAKE_GEOHASH(2.0, 41.0, 20)` conserva las mayúsculas y devuelve `sp33`. Con `id = 1` devuelve `sp33`. Sin filas no devuelve ninguna. `LIMIT 0` no devuelve filas. En `days` devuelve cuatro `sp33`. `make_geohash(-3.0, 40.0, 20)` encabeza `make_geohash(-(3.0), 40.0, 20)` y devuelve `ezjg`. `make_geohash(2, 41, 20)` devuelve `sp33`. `make_geohash('2', '41', 5)` devuelve `s` y `make_geohash(0, 0, '5')` devuelve `s`. `make_geohash(0, score, 5)` devuelve `s`, `h`, `s` y `k`. `make_geohash(id, 0, 5)` devuelve `s`, `s`, `8` y `e`. `make_geohash(reading, 0, 20)` devuelve `s010`, `8000`, `s000` y `s000`. `make_geohash(flags, 0.0, 5)` devuelve `s`, `8`, `s` y `s`. `make_geohash(ts, 0, 5)` y `make_geohash(0, ts, 5)` devuelven cuatro `NULL`. `make_geohash(200.0, reading, 5)` y `make_geohash(reading, 100, 5)` devuelven cuatro `NULL`. `make_geohash(-0.0, 0, 5)` encabeza `make_geohash(-(0.0), 0, 5)` y devuelve `s`. `typeof(make_geohash(2.0, 41.0, 20))` devuelve `GEOHASH(4c)`. Precisión `0`, `61`, `-1` o nula se rechaza con `precision must be in [1..60] range`. `make_geohash(181, 0, 5)` se rechaza con `longitude must be in [-180.0..180.0] range`. `make_geohash(0, 91, 5)` se rechaza con `latitude must be in [-90.0..90.0] range`. Sin argumentos se rechaza con `function `make_geohash` requires arguments: make_geohash(DOUBLE, DOUBLE, INT constant)`. Dos argumentos dicen `expected: 3, provided: 2` y cuatro dicen `provided: 4`. `note` se rechaza con `inconvertible value: `alpha` [STRING -> DOUBLE]`. `ch` se rechaza con `inconvertible value: A [CHAR -> DOUBLE]`. `ok` se rechaza con `argument type mismatch for function `make_geohash` at #1 expected: DOUBLE, actual: BOOLEAN at #2 expected: DOUBLE, actual: INT`. `make_geohash(missing, 0, 5)` se rechaza con `Invalid column: missing`. `make_geohash(0, 0, flags)` se rechaza con `argument type mismatch for function `make_geohash` at #1 expected: DOUBLE, actual: INT at #2 expected: DOUBLE, actual: INT at #3 expected: INT constant, actual: INT`. `make_geohash(0.0, 0.0, 5.0)` se rechaza con `at #3 expected: INT, actual: DOUBLE`. No abrir WAL en ese paso.
