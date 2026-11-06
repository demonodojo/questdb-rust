# Continuar el port

Directorio de trabajo: este repositorio. No toques ficheros fuera de él. `reference/` es el oráculo QuestDB 10.1.0 y está ignorado por git.

Al empezar una sesión:

1. Lee `PLAN_QUESTDB_RUST_PARA_AGENTE.md`, este archivo, `docs/progress.md`, `docs/compatibility.md` y `docs/blockers.md`.
2. Ejecuta `tools/check.sh` y no asumas que el estado anterior sigue en verde.
3. Lo cerrado es H1, la lectura de `tests/fixtures/h2` (`PARTITION BY DAY`), `LIMIT`, `ORDER BY`, `count()`, `min()`, `max()`, `sum()` y `GROUP BY` de una columna. No está portado el motor.
4. La siguiente tarea está al final de `docs/progress.md`. No saltes a WAL ni a HTTP mientras esa tarea no tenga tests.

Reglas que ya valen: corrección y durabilidad antes que velocidad; `unsafe` prohibido salvo un bloque local justificado; una función no está hecha hasta que un test la ve pasar; no declares compatibilidad de escritura sobre disco QuestDB.

Comandos:

```bash
tools/check.sh
cargo run -p qdb-server --bin qdb -- query --data-dir tests/fixtures/h1 --sql "SELECT id FROM sensors WHERE id > 1"
tools/run_differential.sh
```
