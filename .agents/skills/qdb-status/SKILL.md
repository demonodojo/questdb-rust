---
name: qdb-status
description: Revisa el estado real del port QuestDB Rust y la trazabilidad entre progreso, compatibilidad, bloqueos y pruebas. Úsala para pedir un balance, auditar afirmaciones de compatibilidad o preparar el cierre y siguiente paso de una sesión.
---

# Estado respaldado por evidencia

Aplica el arranque de [AGENTS.md](../../../AGENTS.md) desde la raíz. Los documentos mantenidos son el [plan](../../../PLAN_QUESTDB_RUST_PARA_AGENTE.md), [progreso](../../../docs/progress.md), [compatibilidad](../../../docs/compatibility.md), [bloqueos](../../../docs/blockers.md) e [inventario](../../../docs/inventory.md). Las skills describen procedimientos; el estado cambiante permanece en esos documentos.

## Revisar

- Inspecciona `git status --short` y el diff, preservando cambios ajenos. Relaciona cada afirmación revisada con código, nombre/ruta de test, fixture y comando ejecutado. Distingue evidencia histórica y ejecución de esta sesión.
- Verifica las salidas de `tools/check.sh`, no solo una prueba aislada. No repitas comprobaciones ya completadas si nada relevante cambió. Si falla el estado inicial, registra qué fallo precedía a tu cambio.
- Para SQL distingue el subconjunto probado de una función de su cobertura general. Las limitaciones de `docs/compatibility.md` son parte del contrato. Cuatro filas no acreditan cardinalidad grande, concurrencia ni durabilidad.
- Cuando se cite el runner diferencial, lee su comparación efectiva: TSV capturado y cuentas de filas contra la referencia viva son evidencias diferentes. No afirmes igualdad completa de resultados vivos a partir de sus cuentas.
- Revisa que los bloqueos sigan vigentes antes de declararlos resueltos. La falta de Maven y de ejecución JUnit upstream no se compensa con tests Rust; informa de ambas evidencias por separado.

## Registrar o informar

Si solo se pide una auditoría, presenta hallazgos con rutas y evidencia; no implementes automáticamente la siguiente tarea. Si se pide actualizar el estado o cerrar una implementación, ajusta los documentos afectados:

- `docs/progress.md`: hito, cambio concreto, comandos y resultados reales, límites y una próxima tarea ejecutable. Mantén `Próxima tarea` al final.
- `docs/compatibility.md`: usa `not-started`, `in-progress`, `verified` o `blocked`, acompañados de fecha y pruebas. Actualiza una fecha de ejecución solo cuando corresponda a la cobertura realmente ejecutada.
- `docs/blockers.md`: impedimento exacto, comando/error, alcance bloqueado y alternativa comprobada. No confundas una limitación aún no implementada con un impedimento del entorno.
- `docs/decisions.md`, `docs/architecture.md` o `docs/portability.md`: solo cuando haya una decisión o contrato nuevo que mantener.

El cierre debe indicar qué cambió, qué pasó, qué quedó sin comprobar y cuál es el siguiente paso. No declares porcentajes de paridad inventados, compatibilidad de escritura ni el motor terminado a partir del slice de lectura.
