# Continuar el port

Directorio de trabajo: este repositorio. No toques ficheros fuera de él. `reference/` es el oráculo QuestDB 10.1.0 y está ignorado por git.

Al empezar una sesión:

1. Lee `PLAN_QUESTDB_RUST_PARA_AGENTE.md`, este archivo, `docs/progress.md`, `docs/compatibility.md` y `docs/blockers.md`.
2. Ejecuta `tools/check.sh` y no asumas que el estado anterior sigue en verde.
3. Lo cerrado es H1, la lectura de `tests/fixtures/h2` (`PARTITION BY DAY`), `LIMIT`, `ORDER BY`, `count()`, `min()`, `max()`, `sum()`, `avg()`, `first()`, `last()`, `count_distinct()`, `concat()`, `length()`, `lower()`, `upper()`, `left()`, `right()`, `substring()`, `trim()`, `ltrim()`, `rtrim()`, `replace()`, `starts_with()`, `strpos()`, `position()`, `split_part()`, `to_lowercase()`, `to_uppercase()`, `lpad()`, `rpad()`, `quote_ident()`, `length_bytes()`, `size_pretty()`, `abs()`, `round()`, `floor()`, `ceil()`, `ceiling()`, `sign()`, `sqrt()`, `power()`, `ln()`, `log()`, `exp()`, `pi()`, `greatest()`, `least()`, `round_down()`, `round_up()`, `round_half_even()`, `coalesce()`, `nullif()`, `cast()`, `dateadd()`, `datediff()`, `year()`, `month()`, `day()`, `hour()`, `minute()`, `second()`, `millis()`, `micros()`, `day_of_week()`, `week_of_year()`, `date_trunc()`, `timestamp_floor()`, `timestamp_ceil()`, `is_leap_year()`, `days_in_month()`, `to_str()`, `is_end_of_month()`, `day_of_week_sunday_first()`, `to_timestamp()` de un texto numérico o con un patrón, `extract()`, `to_date()` y `GROUP BY` de una columna. No está portado el motor.
4. La siguiente tarea está al final de `docs/progress.md`. No saltes a WAL ni a HTTP mientras esa tarea no tenga tests.

Reglas que ya valen: corrección y durabilidad antes que velocidad; `unsafe` prohibido salvo un bloque local justificado; una función no está hecha hasta que un test la ve pasar; no declares compatibilidad de escritura sobre disco QuestDB.

Comandos:

```bash
tools/check.sh
cargo run -p qdb-server --bin qdb -- query --data-dir tests/fixtures/h1 --sql "SELECT id FROM sensors WHERE id > 1"
tools/run_differential.sh
```
