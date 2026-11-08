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
| qdb-sql | 207 ok, incluido `na_matches_captured_exec` |
| qdb-server | 2 ok (`query_fixture`) |

`tools/run_differential.sh`:

```text
captured-oracle: 2736 queries match tests/fixtures/h1/expected
captured-oracle: 342 queries match tests/fixtures/h2/expected
live-reference: 3078 queries matched row counts against http://127.0.0.1:19011/exec
```

Las salidas de `h1` y de `h2` se capturaron con el servidor vivo, desde `GET /exec`, antes de copiar los directorios. El TSV de `sensors` incluye `NULL`, cadena vacía, `-0.0`, `héllo` y `東京`. El de `days` incluye el símbolo `東京` compartido entre los dos días y el `NULL` de `id`.

`cargo tree -p qdb-server` solo muestra los cinco crates del workspace.

## Limitaciones

- `PARTITION BY NONE` y `DAY`, WAL desactivado, `_meta` versión 426, ficheros de columna sin sufijo de txn.
- HOUR, MONTH, YEAR, WEEK, Parquet y particiones composite se rechazan.
- La última partición usa `transient_row_count`. El tamaño guardado en su registro puede ser 0.
- `LIMIT`, `ORDER BY`, `count()`, `min()`, `max()`, `sum()`, `avg()`, `first()`, `last()`, `count_distinct()`, `concat()`, `||`, `length()`, `lower()`, `upper()`, `left()`, `right()`, `substring()`, `trim()`, `ltrim()`, `rtrim()`, `replace()`, `starts_with()`, `strpos()`, `position()`, `split_part()`, `to_lowercase()`, `to_uppercase()`, `lpad()`, `rpad()`, `quote_ident()`, `length_bytes()`, `size_pretty()`, `abs()`, `round()`, `floor()`, `ceil()`, `ceiling()`, `sign()`, `sqrt()`, `power()`, `ln()`, `log()`, `exp()`, `pi()`, `greatest()`, `least()`, `round_down()`, `round_up()`, `round_half_even()`, `coalesce()`, `nullif()`, `cast()`, `dateadd()`, `datediff()`, `year()`, `month()`, `day()`, `hour()`, `minute()`, `second()`, `millis()`, `micros()`, `nanos()`, `day_of_week()`, `week_of_year()`, `date_trunc()`, `timestamp_floor()`, `timestamp_ceil()`, `is_leap_year()`, `days_in_month()`, `to_str()`, `is_end_of_month()`, `day_of_week_sunday_first()`, `to_timestamp()` de un texto numérico o con un patrón constante, `extract()`, `to_date()`, `md5()`, `sha1()`, `sha256()`, `sin()`, `cos()`, `tan()`, `asin()`, `acos()`, `atan()`, `atan2()`, `degrees()`, `radians()`, `cot()`, `typeof()`, `not()`, `stddev()`, `variance()`, `var_pop()`, `stddev_pop()`, `ksum()`, `nsum()`, `approx_count_distinct()`, `string_agg()`, `covar_pop()`, `covar_samp()`, `corr()`, `regr_slope()`, `regr_intercept()`, `regr_r2()`, `stddev_samp()`, `var_samp()`, `bool_and()`, `bool_or()`, `first_not_null()`, `last_not_null()`, `bit_and()`, `bit_or()`, `bit_xor()`, `geomean()`, `skewness()`, `mode()`, `approx_median()`, `approx_percentile()`, `arg_max()`, `arg_min()`, `string_distinct_agg()`, `kurtosis_pop()`, `kurtosis_samp()`, `kurtosis()`, `skewness_pop()`, `vwap()`, `twap()`, `sparkline()`, `weighted_avg()`, `weighted_stddev()`, `weighted_stddev_rel()`, `weighted_stddev_freq()`, `isOrdered()`, `percentile_disc()`, `percentile_cont()`, `skewness_samp()`, `quantile_cont()`, `quantile_disc()`, `glob()`, `ilike()`, `like()`, `switch()`, `bar()`, `haversine_dist_deg()`, `timestamp_sequence()`, `mid()`, `spread()`, `spread_bps()`, `wmid()`, `between()`, `in`, `=`, `<>`, `<`, `>`, `>=`, `<=`, `+`, `-`, `*`, `/`, `%`, `-value`, `~value`, `and`, `or`, `||`, `IS NULL`, `IS NOT NULL`, `BETWEEN`, `NOT BETWEEN`, `LIKE`, `NOT LIKE`, `ILIKE`, `NOT ILIKE`, `~`, `!~`, `&`, `|`, `^`, `=`, `<>`, `!=`, `<`, `>`, `>=`, `<=`, `::string`, `::long`, `::int`, `::double`, `::float`, `::boolean`, `::byte`, `::short`, `::char`, `::symbol`, `::varchar`, `::date`, `::timestamp` y `GROUP BY` de una columna están verificados. Un `SELECT` admite un solo agregado. El resto de SQL no.
- Sin escritura, sin red, sin el resto de tipos ni de SQL.
- BOOLEAN/BYTE/SHORT/CHAR no separan NULL de 0; es el formato upstream.
- Maven no está instalado, así que la suite Java no se ejecutó (`docs/blockers.md`).

## Próxima tarea

Añadir `to_utc(value, zone)` y comprobarlo contra `/exec` en `sensors` y `days`. El encabezado conserva la grafía y el tipo es `TIMESTAMP`. `to_utc(ts, 'UTC')` devuelve `2020-01-02T03:04:05.000000Z`, `2020-01-02T03:04:06.000000Z`, `2020-01-02T03:04:07.000000Z` y `2020-01-02T03:04:08.000000Z`. `TO_UTC(ts, 'UTC')` usa ese mismo cuerpo. `to_utc(ts, 'Europe/Madrid')` devuelve `2020-01-02T02:04:05.000000Z`, `2020-01-02T02:04:06.000000Z`, `2020-01-02T02:04:07.000000Z` y `2020-01-02T02:04:08.000000Z`. Con `id = 1` devuelve `2020-01-02T02:04:05.000000Z`. `to_utc(ts, 'America/New_York')` devuelve `2020-01-02T08:04:05.000000Z`, `2020-01-02T08:04:06.000000Z`, `2020-01-02T08:04:07.000000Z` y `2020-01-02T08:04:08.000000Z`. `to_utc(day, 'UTC')` devuelve `2020-01-02T00:00:00.000000Z`, `NULL`, `1970-01-01T00:00:00.000000Z` y `2020-06-15T12:30:00.000000Z`. `to_utc(day, 'Europe/Madrid')` devuelve `2020-01-01T23:00:00.000000Z`, `-290308-01-01T20:13:49.224192Z`, `1969-12-31T23:00:00.000000Z` y `2020-06-15T10:30:00.000000Z`. `to_utc(id, 'UTC')` devuelve `1970-01-01T00:00:00.000001Z`, `1970-01-01T00:00:00.000002Z`, `NULL` y `1969-12-31T23:59:59.999999Z`. `to_utc(flags, 'UTC')` devuelve `1970-01-01T00:00:00.000010Z`, `NULL`, `1970-01-01T00:00:00.000000Z` y `1970-01-01T00:00:00.000042Z`. Sin filas no devuelve ninguna. `LIMIT 0` no devuelve filas. En `days`, `to_utc(ts, 'UTC')` devuelve `2020-01-02T03:04:05.000000Z`, `2020-01-02T23:00:00.000000Z`, `2020-01-03T00:00:00.000000Z` y `2020-01-03T12:30:00.000000Z`. `to_utc(ts, 'Mars')` se rechaza con `invalid timezone: Mars`. `to_utc(ts, null)` se rechaza con `timezone must not be null`. Sin argumentos se rechaza con `function `to_utc` requires arguments: to_utc(TIMESTAMP, STRING)`. Con un argumento se rechaza con `wrong number of arguments for function `to_utc`; expected: 2, provided: 1`. Con tres argumentos se rechaza con `wrong number of arguments for function `to_utc`; expected: 2, provided: 3`. `ok` se rechaza con `expression type mismatch, expected: TIMESTAMP, actual: BOOLEAN`. `ch` se rechaza con `expression type mismatch, expected: TIMESTAMP, actual: CHAR`. `location` se rechaza con `inconvertible value: `madrid` [SYMBOL -> TIMESTAMP_NS]`. `note` se rechaza con `inconvertible value: `alpha` [STRING -> TIMESTAMP_NS]`. `to_utc(missing, 'UTC')` se rechaza con `Invalid column: missing`. No abrir WAL en ese paso.
