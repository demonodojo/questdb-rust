---
name: qdb-oracle
description: Contrasta el port Rust con el oráculo QuestDB 10.1.0, captura resultados de /exec e investiga discrepancias de fixtures o pruebas diferenciales. Úsala para verificar semántica upstream, generar evidencia de compatibilidad o revisar qué demuestra el runner.
---

# Evidencia contra el oráculo fijado

Ejecuta comandos desde la raíz y respeta el arranque de [AGENTS.md](../../../AGENTS.md). Lee [upstream](../../../docs/upstream.md) y [bloqueos](../../../docs/blockers.md) antes de usar Java, binarios o datos de referencia.

## Identidad y aislamiento

Comprueba `git -C reference/questdb rev-parse HEAD` contra el commit de `docs/upstream.md`; el tag es `10.1.0`. Para el binario comprueba también versión y `Build-Commit-Hash` del manifiesto del jar. Un puerto abierto aún no demuestra la identidad del proceso ni qué datos contiene.

`reference/` es el oráculo ignorado por git, no una dependencia de producción. Mantén runtime, copias y temporales dentro del repositorio; configura `TMPDIR` al invocar scripts que usan `mktemp`. No abras una misma raíz en escritura desde ambos motores ni regeneres fixtures como parte de una simple comprobación.

## Distinguir lo que se compara

Inspecciona `tools/run_differential.sh` antes de describir su cobertura:

- `captured-oracle` compara el TSV del port con archivos `expected/*.tsv` capturados de QuestDB. Es evidencia reproducible sin servidor vivo.
- El bloque inicial `live-reference` compara **solo el número de filas** para su lista general de consultas; no acredita igualdad de valores, tipos ni encabezados. Más adelante el runner tiene manifiestos específicos que comparan respuestas completas (`columns`, `dataset`, `count`) y errores (`error`, `position`). Si el servidor no está disponible, esos cotejos vivos no se ejecutan aunque pase la parte capturada.
- Para verificar una función nueva consulta `/exec` con la misma SQL y conserva la respuesta original: `columns` (nombres y tipos), `dataset` (valores), `count` y, si falla, el error con su posición. Compara esos elementos explícitamente; no basta la cuenta de filas.

Para comprobar lo ya capturado sin escribir temporales fuera del repositorio:

```bash
mkdir -p target/oracle-tmp
TMPDIR="$PWD/target/oracle-tmp" tools/run_differential.sh
```

Si hace falta arrancar la referencia, revisa `tools/run_reference.sh` y usa una raíz aislada bajo `reference/runtime/` con `QDB_REFERENCE_ROOT`. El endpoint predeterminado del runner es `http://127.0.0.1:19011/exec`; puede cambiarse con `QDB_HTTP`. Comprueba proceso y dataset antes de cargar SQL. Configura `TMPDIR="$PWD/target/oracle-tmp"`; las temporales del runner quedan bajo `target/oracle-tmp/`. Lee `tools/generate_fixtures.sh` antes de regenerar: modifica fixtures y su comportamiento no debe suponerse genérico para todos los hitos.

## Capturar y resolver diferencias

Conserva SQL de entrada, identidad del oráculo y respuesta obtenida junto a la evidencia del caso. Respeta el formato TSV y los tests existentes. La salida Rust nunca debe ser la fuente de un esperado atribuido a QuestDB.

Ordena filas solo cuando SQL no define su orden y conserva duplicados. No normalices NULL a vacío, ni cambies precisión, unidades temporales, signo de cero o errores para ocultar diferencias. Documenta tolerancias numéricas concretas si la semántica permite usarlas.

Reduce cada discrepancia a una entrada reproducible, clasifícala como `port-bug`, `incompatibility`, `known-upstream-behavior` o `test-harness-bug` y añade regresión. No retires una consulta fallida para obtener verde. Informa por separado de pruebas capturadas, comparación viva de cuentas y comparaciones completas ejecutadas; no copies cifras históricas como resultados de hoy.
