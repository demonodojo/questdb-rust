# Compatibilidad

Fecha de la última ejecución: 2026-11-06. Estados: `not-started`, `in-progress`, `verified`, `blocked`.

| Característica | Estado | Prueba | Límite |
| --- | --- | --- | --- |
| Lectura `_meta` versión 426, `PARTITION BY NONE` y `DAY`, WAL off | verified | `reads_original_sensors_fixture`, `reads_two_day_partitions_including_name_txn_suffix` | Rechaza otra versión, WAL, HOUR/MONTH/YEAR/WEEK, Parquet, particiones composite y columnas reemplazadas |
| Tipos BOOLEAN, BYTE, SHORT, CHAR, INT, LONG, DATE, TIMESTAMP µs, FLOAT, DOUBLE, STRING, SYMBOL, VARCHAR | verified | misma prueba, contra `tests/fixtures/h1` | Sin nanos, UUID, geohash, decimal, array, binary, long256, IPv4 |
| NULL de INT/LONG/DATE/TIMESTAMP/FLOAT/DOUBLE/STRING/SYMBOL/VARCHAR | verified | misma prueba y `SELECT id FROM sensors WHERE id = NULL` | BOOLEAN/BYTE/SHORT/CHAR no distinguen NULL de 0 en disco |
| `-0.0` double | verified | fila 4 de `reading` | |
| UTF-8 / UTF-16 (`β`, `東`, `東京`, `héllo`) | verified | columnas `ch`, `note`, `location`, `tag` | |
| `SELECT` columnas o `*`, `count()`, `min(col)`, `max(col)`, `WHERE`, `GROUP BY` de una columna, `ORDER BY`, `LIMIT` | verified | `min_max_matches_captured_exec`, `tools/run_differential.sh` | Un solo agregado por `SELECT`. Sin varias columnas de `GROUP BY`, joins ni DML. `count(BOOLEAN)` se rechaza. `min` y `max` de `BOOLEAN` devuelven `DOUBLE`. `CHAR` 0 no entra en el agregado. Sin `ORDER BY`, el orden de los grupos es el de primera aparición |
| `col = NULL` / `col != NULL` como en 10.1.0 | verified | queries del fixture | Solo sobre el literal NULL |
| Orden `ORDER BY`: NULL primero en enteros, texto, símbolo, fecha y timestamp; NULL al final en FLOAT y DOUBLE | verified | `descending_order_matches_captured_exec` | `DESC` invierte ese orden. `-0.0` queda antes que `0.0` |
| Servidor HTTP, ILP, PG Wire, QWP | not-started | — | El CLI no escucha red |
| Escritura, WAL, O3, dedup, Parquet, índices, vistas | not-started | — | |
| Compilar el fuente Java y correr su JUnit | blocked | `docs/blockers.md` | Falta Maven |

No hay compatibilidad de escritura sobre ficheros QuestDB. El lector abre las copias de `tests/fixtures/h1` (`NONE`) y `tests/fixtures/h2` (`DAY`, dos días, el segundo directorio con sufijo `.{nameTxn}`). La última partición cuenta filas con `transient_row_count`.
