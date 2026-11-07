# Compatibilidad

Fecha de la última ejecución: 2026-11-06. Estados: `not-started`, `in-progress`, `verified`, `blocked`.

| Característica | Estado | Prueba | Límite |
| --- | --- | --- | --- |
| Lectura `_meta` versión 426, `PARTITION BY NONE` y `DAY`, WAL off | verified | `reads_original_sensors_fixture`, `reads_two_day_partitions_including_name_txn_suffix` | Rechaza otra versión, WAL, HOUR/MONTH/YEAR/WEEK, Parquet, particiones composite y columnas reemplazadas |
| Tipos BOOLEAN, BYTE, SHORT, CHAR, INT, LONG, DATE, TIMESTAMP µs, FLOAT, DOUBLE, STRING, SYMBOL, VARCHAR | verified | misma prueba, contra `tests/fixtures/h1` | Sin nanos, UUID, geohash, decimal, array, binary, long256, IPv4 |
| NULL de INT/LONG/DATE/TIMESTAMP/FLOAT/DOUBLE/STRING/SYMBOL/VARCHAR | verified | misma prueba y `SELECT id FROM sensors WHERE id = NULL` | BOOLEAN/BYTE/SHORT/CHAR no distinguen NULL de 0 en disco |
| `-0.0` double | verified | fila 4 de `reading` | |
| UTF-8 / UTF-16 (`β`, `東`, `東京`, `héllo`) | verified | columnas `ch`, `note`, `location`, `tag` | |
| `SELECT` columnas o `*`, `count()`, `min(col)`, `max(col)`, `sum(col)`, `avg(col)`, `first(col)`, `last(col)`, `count_distinct(col)`, `concat(...)`, `length(...)`, `lower(...)`, `upper(...)`, `left(...)`, `right(...)`, `substring(...)`, `trim(...)`, `WHERE`, `GROUP BY` de una columna, `ORDER BY`, `LIMIT` | verified | `trim_matches_captured_exec`, `tools/run_differential.sh` | Un solo agregado por `SELECT`. Sin varias columnas de `GROUP BY`, joins ni DML. `count` y `sum` de `BOOLEAN` se rechazan. `min`, `max` y `avg` de `BOOLEAN` devuelven `DOUBLE`. `avg` siempre es `DOUBLE`. `first` y `last` conservan el tipo y también un NULL o un `CHAR` 0. `count_distinct` devuelve `LONG`, omite NULL y `CHAR` 0, y distingue `-0.0` de `0.0`. Dentro de `GROUP BY` no admite `FLOAT`, `DOUBLE` ni `BOOLEAN`. `concat` devuelve `STRING`: un NULL de texto no aporta, un NULL numérico es `null` y un float nulo es `NaN`. `length` devuelve `INT` y usa `-1` para un texto nulo. `lower` y `upper` devuelven `STRING`, o NULL si el texto es NULL. `upper('βeta')` es `ΒETA`. `left` y `right` devuelven `STRING` y cuentan caracteres: un entero negativo recorta por el otro extremo, y un NULL en el texto o en la cuenta deja NULL. `substring` toma un inicio en base 1 y una longitud. Una longitud constante 0 es vacía, un inicio constante que no alcanza el texto es NULL, y una longitud negativa se rechaza. `trim` devuelve `STRING` y solo quita el espacio ASCII ` ` de los dos extremos. Un tabulador se conserva. NULL sigue siendo NULL. `CHAR` 0 no entra en `count`, `min`, `max`, `sum` ni `avg`. `sum` y `avg` de texto no convertible fallan. Sin `ORDER BY`, el orden de los grupos es el de primera aparición |
| `col = NULL` / `col != NULL` como en 10.1.0 | verified | queries del fixture | Solo sobre el literal NULL |
| Orden `ORDER BY`: NULL primero en enteros, texto, símbolo, fecha y timestamp; NULL al final en FLOAT y DOUBLE | verified | `descending_order_matches_captured_exec` | `DESC` invierte ese orden. `-0.0` queda antes que `0.0` |
| Servidor HTTP, ILP, PG Wire, QWP | not-started | — | El CLI no escucha red |
| Escritura, WAL, O3, dedup, Parquet, índices, vistas | not-started | — | |
| Compilar el fuente Java y correr su JUnit | blocked | `docs/blockers.md` | Falta Maven |

No hay compatibilidad de escritura sobre ficheros QuestDB. El lector abre las copias de `tests/fixtures/h1` (`NONE`) y `tests/fixtures/h2` (`DAY`, dos días, el segundo directorio con sufijo `.{nameTxn}`). La última partición cuenta filas con `transient_row_count`.
