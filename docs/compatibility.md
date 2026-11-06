# Compatibilidad

Fecha de la última ejecución: 2026-11-06. Estados: `not-started`, `in-progress`, `verified`, `blocked`.

| Característica | Estado | Prueba | Límite |
| --- | --- | --- | --- |
| Lectura `_meta` versión 426, `PARTITION BY NONE`, WAL off | verified | `qdb-storage` `reads_original_sensors_fixture` | Rechaza otra versión, WAL, particiones y columnas reemplazadas |
| Tipos BOOLEAN, BYTE, SHORT, CHAR, INT, LONG, DATE, TIMESTAMP µs, FLOAT, DOUBLE, STRING, SYMBOL, VARCHAR | verified | misma prueba, contra `tests/fixtures/h1` | Sin nanos, UUID, geohash, decimal, array, binary, long256, IPv4 |
| NULL de INT/LONG/DATE/TIMESTAMP/FLOAT/DOUBLE/STRING/SYMBOL/VARCHAR | verified | misma prueba y `SELECT id FROM sensors WHERE id = NULL` | BOOLEAN/BYTE/SHORT/CHAR no distinguen NULL de 0 en disco |
| `-0.0` double | verified | fila 4 de `reading` | |
| UTF-8 / UTF-16 (`β`, `東`, `東京`, `héllo`) | verified | columnas `ch`, `note`, `location`, `tag` | |
| `SELECT` columnas o `*`, `WHERE` comparación, `ORDER BY` ASC | verified | `qdb-sql` `select_star_and_filters_match_reference_rows`, `tools/run_differential.sh` | Sin joins, agregados, `SAMPLE BY`, funciones, `DESC`, DML |
| `col = NULL` / `col != NULL` como en 10.1.0 | verified | queries del fixture | Solo sobre el literal NULL |
| Orden `ORDER BY id` con NULL primero | verified | comparado con `/exec` del binario 10.1.0 | No probado en otras columnas |
| Servidor HTTP, ILP, PG Wire, QWP | not-started | — | El CLI no escucha red |
| Escritura, WAL, O3, dedup, Parquet, índices, vistas | not-started | — | |
| Compilar el fuente Java y correr su JUnit | blocked | `docs/blockers.md` | Falta Maven |

No hay compatibilidad de escritura sobre ficheros QuestDB. El lector solo abre la copia de `tests/fixtures/h1`.
