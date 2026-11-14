---
name: qdb-port
description: Continúa el port de QuestDB 10.1.0 a Rust e implementa o corrige comportamiento SQL con pruebas de regresión. Úsala al pedir avanzar la siguiente tarea, portar una función u operador o corregir una consulta del port.
---

# Continuar una unidad verificable del port

Trabaja desde la raíz del repositorio. Las rutas de comandos son relativas a esa raíz.

## Elegir la unidad

- Aplica el arranque de [AGENTS.md](../../../AGENTS.md): lee también el [plan](../../../PLAN_QUESTDB_RUST_PARA_AGENTE.md), [progreso](../../../docs/progress.md), [compatibilidad](../../../docs/compatibility.md) y [bloqueos](../../../docs/blockers.md), inspecciona `git status --short` y ejecuta `tools/check.sh`. Reutiliza la ejecución de esta sesión si ya se hizo y nada la invalida.
- Si el usuario pide continuar, toma la tarea al final de `docs/progress.md`. Si pide una corrección concreta, acota esa corrección sin sustituirla por otra del plan. No adelantes WAL o HTTP mientras la tarea pendiente no tenga tests.
- Reproduce el fallo o fija el comportamiento observable antes de editar. Localiza implementación y tests en `reference/questdb` con `rg`; comprueba su identidad con `docs/upstream.md`. No uses la semántica de otro motor SQL como sustituto.

## Implementar SQL

Las fronteras actuales son `crates/qdb-sql/src/parse.rs` (parser/AST), `exec.rs` (resolución, evaluación y pruebas) y `lib.rs` (contrato público). Sigue el recorrido de la expresión; reconocer un nombre no demuestra que se ejecute correctamente.

- Extrae casos comparables de upstream. En expresiones de columna contempla nombre simple y cualificado, alias de tabla, nombre y tipo de salida, NULL y cadena vacía; añade bordes específicos sin multiplicar casos equivalentes.
- Conserva errores y coerciones de QuestDB, incluidos errores detectados con `LIMIT 0`. En texto comprueba UTF-8/UTF-16; en números, NaN, overflow y `-0.0`; en tiempo, unidad y zona.
- Añade una prueba Rust que falle antes del arreglo y pase después. Los fixtures reales y salidas capturadas viven en `tests/fixtures/h1` y `h2`; los tests `*_matches_captured_exec` de `exec.rs` muestran cómo compararlos. Añade evidencia al runner diferencial cuando se amplíe su cobertura.
- Mantén el cambio en los crates existentes salvo que una separación tenga código real que la justifique. Rechaza explícitamente los casos aún no soportados.

## Cerrar

Ejecuta primero la regresión concreta y después `tools/check.sh`. Para cambios de semántica compara con el oráculo mediante [qdb-oracle](../qdb-oracle/SKILL.md). Si falta la referencia viva, separa lo capturado de lo no comprobado.

Actualiza `docs/progress.md` con comandos, resultados, límites y próxima tarea precisa. Cambia `docs/compatibility.md` solo para el comportamiento demostrado y registra impedimentos reales en `docs/blockers.md`. Una consulta que pasa no acredita una función completa ni el motor entero.
