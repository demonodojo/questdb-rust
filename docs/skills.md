# Skills locales del proyecto

Las skills canónicas viven en `.agents/skills/`, dentro del repositorio. No requieren instalación global. La ubicación y el formato se contrastaron con la [documentación oficial de Codex](https://developers.openai.com/codex/skills) y la [especificación Agent Skills](https://agentskills.io/specification.md).

| Skill | Uso | Ejemplo de invocación |
| --- | --- | --- |
| [qdb-port](../.agents/skills/qdb-port/SKILL.md) | Continuar una tarea o corregir SQL con una regresión | `$qdb-port continúa la próxima tarea de docs/progress.md` |
| [qdb-oracle](../.agents/skills/qdb-oracle/SKILL.md) | Capturar `/exec` y contrastar valores, tipos, errores y fixtures | `$qdb-oracle comprueba esta consulta contra QuestDB 10.1.0` |
| [qdb-storage](../.agents/skills/qdb-storage/SKILL.md) | Cambiar el lector binario y probar límites o corrupción | `$qdb-storage revisa la validación de offsets del lector` |
| [qdb-status](../.agents/skills/qdb-status/SKILL.md) | Auditar evidencia, compatibilidad, bloqueos y siguiente paso | `$qdb-status revisa el estado real del port` |

Codex puede seleccionarlas por su descripción o por `$nombre`. Si no aparecen tras el cambio, reinicia la sesión. `AGENTS.md` y el plan siguen rigiendo el trabajo; las skills no amplían permisos ni mantienen otra lista de funcionalidades terminadas.

## Comprobación de selección

Estos casos sirven para revisar el alcance de las descripciones al modificarlas. Son criterios esperados; no constituyen un resultado de activación automática del cliente.

| Skill | Petición que debe aplicar | Otra petición que debe aplicar | Petición cercana que no debe activarla por sí sola |
| --- | --- | --- | --- |
| qdb-port | Continúa la próxima tarea del port | Corrige `trim(s.note)` y añade regresión | Añade documentación de las skills |
| qdb-oracle | Captura nombres, tipos y valores de `/exec` | Explica una diferencia entre Rust y el TSV esperado | Cambia solo el formato del README |
| qdb-storage | Rechaza una cabecera `_meta` truncada | Investiga el sufijo de una partición DAY | Corrige el alias de una expresión SQL |
| qdb-status | Qué está verificado realmente | Actualiza el progreso después de estas pruebas | Implementa directamente una nueva función SQL |

Al mantenerlas, comprueba frontmatter YAML, nombre igual al directorio, rutas de enlaces y ausencia de plantillas pendientes. Revisa los casos anteriores y ejercita el procedimiento afectado con artefactos reales. Separa esa revisión manual de una prueba de selección o ejecución en una sesión nueva de Codex.

Mantén los comandos alineados con los scripts reales. En particular, `tools/run_differential.sh` comprueba valores TSV capturados y, cuando hay referencia viva, solo cuentas de filas. Una ampliación de ese runner debe reflejarse en las skills que describen su cobertura.
