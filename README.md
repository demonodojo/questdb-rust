# questdb-rust

Reescritura en curso del motor QuestDB OSS 10.1.0 en Rust. El estado real está en `docs/progress.md`. Hoy lee tablas nativas `PARTITION BY NONE` y `PARTITION BY DAY` y responde un `SELECT` pequeño con `WHERE`, `GROUP BY` de una columna, `ORDER BY`, `LIMIT`, `count()`, `min()`, `max()`, `sum()`, `avg()`, `first()`, `last()`, `count_distinct()`, `concat()`, `length()`, `lower()`, `upper()`, `left()`, `right()`, `substring()`, `trim()`, `ltrim()`, `rtrim()`, `replace()`, `starts_with()`, `strpos()`, `position()`, `split_part()`, `to_lowercase()`, `to_uppercase()`, `lpad()`, `rpad()`, `quote_ident()`, `length_bytes()`, `size_pretty()`, `abs()`, `round()`, `floor()`, `ceil()`, `ceiling()`, `sign()`, `sqrt()`, `power()`, `ln()`, `log()`, `exp()`, `pi()`, `greatest()`, `least()`, `round_down()`, `round_up()`, `round_half_even()`, `coalesce()`, `nullif()`, `cast()`, `dateadd()`, `datediff()`, `year()`, `month()`, `day()`, `hour()`, `minute()`, `second()`, `millis()`, `micros()`, `day_of_week()`, `week_of_year()`, `date_trunc()`, `timestamp_floor()`, `timestamp_ceil()`, `is_leap_year()`, `days_in_month()`, `to_str()`, `is_end_of_month()`, `day_of_week_sunday_first()` y `to_timestamp()` de un texto numérico o con un patrón, `extract()`, `to_date()`, `md5()`, `sha1()` y `sha256()`, `sin()`, `cos()`, `tan()`, `asin()`, `acos()`, `atan()`, `atan2()`, `degrees()`, `radians()`, `cot()`, `typeof()`, `not()`, `stddev()`, `variance()`, `var_pop()`, `stddev_pop()`, `ksum()`, `nsum()`, `approx_count_distinct()`. No es un sustituto de QuestDB.

## Probar el slice

```bash
tools/check.sh
cargo run -p qdb-server --bin qdb -- query \
  --data-dir tests/fixtures/h1 \
  --sql "SELECT id, note FROM sensors WHERE id > 1"
```

La tabla `sensors` la escribió el binario QuestDB 10.1.0 (`e11c837f8a169b533540bd6fa57ef3d520b68ad9`). El SQL está en `tests/fixtures/h1/load.sql`.

Rust estable con `rustfmt` y `clippy`. No hace falta Java para los tests del port. Java 25 y el tarball `questdb-10.1.0-no-jre-bin` solo hacen falta para regenerar el fixture (`tools/generate_fixtures.sh`) o levantar el oráculo (`tools/run_reference.sh`).

Licencia Apache-2.0. Ver `NOTICE`.
