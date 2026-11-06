# Progreso

## Hito actual

H0 y H1 cumplidos el 2026-11-06. El port no está terminado.

## Qué hay

- Referencia fijada: tag `10.1.0`, commit `e11c837f8a169b533540bd6fa57ef3d520b68ad9`. Detalle en `docs/upstream.md`.
- Inventario medido en `docs/inventory.md`.
- Fixture real `tests/fixtures/h1`: tabla `sensors`, 4 filas, escrita por el binario 10.1.0, leída de nuevo después de reiniciar el servidor (`count() = 4`).
- Workspace `qdb-types`, `qdb-platform`, `qdb-storage`, `qdb-sql`, `qdb-server`.
- CLI: `qdb query --data-dir tests/fixtures/h1 --sql "..."`.

## Comandos y resultados

`tools/check.sh` (fmt --check, clippy `-D warnings`, `cargo test --workspace`) terminó con código 0.

Tests que pasaron:

| Crate | Resultado |
| --- | --- |
| qdb-types | 3 ok |
| qdb-platform | 1 ok |
| qdb-storage | 3 ok, incluido `reads_original_sensors_fixture` |
| qdb-sql | 3 ok, incluido el `SELECT` con filtros del fixture |
| qdb-server | 2 ok (`query_fixture`) |

`tools/run_differential.sh`:

```text
captured-oracle: 6 queries match tests/fixtures/h1/expected
live-reference: NOT RUNNING at http://127.0.0.1:19011/exec
compared only against captured output from QuestDB 10.1.0; this is not a live differential run
```

Esas seis salidas se capturaron antes, con el servidor vivo, desde `GET /exec`. El TSV de `SELECT *` coincide con el JSON de QuestDB, incluyendo `NULL`, cadena vacía, `-0.0`, `héllo` y `東京`.

`cargo tree -p qdb-server` solo muestra los cinco crates del workspace.

## Limitaciones

- Solo `PARTITION BY NONE`, WAL desactivado, `_meta` versión 426, ficheros de columna sin sufijo de txn.
- Sin escritura, sin red, sin el resto de tipos ni de SQL.
- BOOLEAN/BYTE/SHORT/CHAR no separan NULL de 0; es el formato upstream.
- Maven no está instalado, así que la suite Java no se ejecutó (`docs/blockers.md`).

## Próxima tarea

Generar con el binario 10.1.0 una segunda tabla `PARTITION BY DAY` de dos días, no WAL, y extender `qdb-storage` para leer la lista de particiones del `_txn` (timestamp, tamaño, name txn) y los directorios `YYYY-MM-DD`. Añadir un test que compare las filas con `/exec` o con un TSV capturado. No abrir WAL en ese paso.
