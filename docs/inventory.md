# Inventario QuestDB 10.1.0

Medido el 2026-11-06 sobre `reference/questdb` en `e11c837f8a169b533540bd6fa57ef3d520b68ad9`. Los recuentos son `find -name '*.java' | wc -l` de ese árbol. No son una estimación de progreso del port.

## Árbol de producto

| Ruta | Qué es |
| --- | --- |
| `core/` | Motor, servidor y consola embebida. `pom.xml` del módulo. |
| `core/src/main/java/io/questdb` | Código Java del backend. 3918 ficheros `.java`. |
| `core/src/test/java/io/questdb/test` | Tests. 2427 ficheros `.java`. |
| `core/src/main/c` | Nativo: `linux`, `aarch64`, `osx`, `windows`, `freebsd`, `share` (simdjson, zlib, jemalloc, fsst, vcl, asmlib, jit, pdqsort, async-profiler). 157 ficheros. |
| `core/rust/qdbr` | JNI de lectura/escritura Parquet (`parquet_read`, `parquet_write`, `parquet_metadata`). No es el motor SQL. |
| `core/rust/qdb-core` | Soporte columnar Rust del lado Parquet (`col_driver`). |
| `core/rust/parquet2`, `parquet-format-safe`, `qdb-parquet-meta`, `qdb-sqllogictest` | Dependencias y harness de Parquet / SQLLogicTest. |
| `java-questdb-client`, `benchmarks`, `compat`, `utils`, `win64svc`, `pkg`, `ci` | Cliente, benches, compatibilidad, empaquetado. Fuera del binario del motor. |

Entrada del servidor: `io.questdb.ServerMain` y `io.questdb.Bootstrap`. Motor de tablas: `io.questdb.cairo.CairoEngine`.

## Java de producción, por paquete

| Paquete | `.java` | Papel comprobado en el tag |
| --- | --- | --- |
| `cairo` | 648 | Almacenamiento, metadatos, WAL, índices, vistas |
| `griffin` | 2347 | SQL: `SqlParser` (434 KiB), `SqlOptimiser` (787 KiB), `SqlCompilerImpl` (337 KiB), `SqlCodeGenerator` (878 KiB), `WhereClauseParser`, `engine/`, `model/` |
| `cutlass` | 327 | Protocolos |
| `std` | 324 | Primitivos, UTF-8, tiempo, `Numbers` |
| `mp` | 83 | Colas y workers |
| `network` | 43 | IO |
| `metrics` | 35 | Métricas |
| `log` | 22 | Log |
| `tasks` | 18 | Tareas de fondo |
| `lifecycle` | 8 | Arranque |
| `jit` | 5 | Filtros JIT |
| `preferences` | 4 | Preferencias |

Subpaquetes `cairo` contados: `lv` 102, `sql` 85, `wal` 62, `vm` 36, `idx` 28, `map` 27, `pool` 20, `mv` 19, `frm` 18, `mig` 17, `arr` 14, `view` 11, `security` 10, `file` 6.

`cutlass` contiene `http`, `pgwire`, `line` (ILP), `qwp`, `text` (CSV), `parquet`, `json`, `auth`. `Services.java` registra los servicios.

Tests Java bajo `io.questdb.test`: `griffin` 1253, `cairo` 479, `cutlass` 314, `std` 189, más `fuzz` 25 y harness (`AbstractCairoTest`, `ServerMainTest`). No se ejecutaron: falta Maven.

## Formato de tabla nativa leído en H1

Fuentes: `TableUtils.writeMetadata`, `TxReader.unsafeLoadBaseOffset`, `StringTypeDriver`, `VarcharTypeDriver`, `SymbolMapWriter`, `ColumnType`, `TableWriter.configureNullSetters`. Comprobado con la tabla `sensors` generada por el binario 10.1.0.

Directorio: `<root>/db/<tabla>/` cuando la tabla no es WAL. `_name` es un string QuestDB (i32 longitud UTF-16 + chars). En el fixture dice `sensors`.

`_meta`, little-endian:

| Offset | Campo | Fixture |
| --- | --- | --- |
| 0 | i32 column count | 13 |
| 4 | i32 partition by (`PartitionBy.NONE` = 3) | 3 |
| 8 | i32 designated timestamp index | 0 |
| 12 | i32 metadata version (`ColumnType.VERSION`) | 426 |
| 16 | i32 table id | 8 |
| 40 | byte WAL enabled | 0 |
| 128 | 32 bytes por columna: type i32, flags i64, index block i32, symbol capacity i32, parquet config i32, 8 bytes reservados (el int en +24 es el replacing index, 0 = ninguno) | |
| después | nombres: i32 número de chars UTF-16LE, luego los chars | `ts`, `id`, … |

`_txn` es un fichero A/B. El i64 de offset 0 es la versión de transacción. Versión par usa la sección A (i32 en offset 8 = base, i32 en offset 12 = tamaño de símbolos). Versión impar usa B (offsets 32 y 36). El registro activo debe repetir el mismo txn en su offset 0. En el fixture la versión es 4, la base es 384, `transient_row_count` es 4 y `fixed_row_count` es 0. El número de filas de la única partición `default` es esa suma. Los ficheros de columna están preasignados a 4096 bytes; la longitud del fichero no es el número de filas.

El i64 de min timestamp en ese `_txn` vale `Long.MAX_VALUE`. El max timestamp vale `1577934248000000`, que es `2020-01-02T03:04:08.000000Z`. El lector no usa el min timestamp para contar filas.

Símbolos: un slot de 8 bytes por columna SYMBOL dentro del registro (`count` i32 + transient i32) a partir de offset 132 del registro. El mapa vive en la raíz de la tabla: `<col>.o` (cabecera 64, luego i64 de inicio en `<col>.c` por clave) y `<col>.c` (strings UTF-16). La clave nula en `<col>.d` es `Integer.MIN_VALUE`.

Partición `PARTITION BY NONE`: directorio `default/`. Columna fija `<col>.d`. STRING y VARCHAR añaden `<col>.i`.

| Tag | Nombre | Ancho | Nulo en disco |
| --- | --- | --- | --- |
| 1 | BOOLEAN | 1 | byte 0, indistinguible de `false` |
| 2 | BYTE | 1 | byte 0, indistinguible de 0 |
| 3 | SHORT | 2 | 0, indistinguible de 0 |
| 4 | CHAR | 2 UTF-16 | 0, el JSON upstream lo muestra como `""` |
| 5 | INT | 4 | `Integer.MIN_VALUE` |
| 6 | LONG | 8 | `Long.MIN_VALUE` |
| 7 | DATE | 8 ms | `Long.MIN_VALUE` |
| 8 | TIMESTAMP | 8 µs | `Long.MIN_VALUE`. El bit 18 marca nanos y este lector lo rechaza |
| 9 | FLOAT | 4 | NaN |
| 10 | DOUBLE | 8 | NaN. `-0.0` se conserva |
| 11 | STRING | `.i` i64 por fila, más un centinela | longitud `-1` en `.d`, payload UTF-16LE |
| 12 | SYMBOL | i32 clave | `Integer.MIN_VALUE` |
| 26 | VARCHAR | registro aux de 16 bytes en `.i` | flag 4 en el int de cabecera. Hasta 9 bytes inline; si no, UTF-8 en `.d` y offset de 48 bits |

Esos son los únicos tags que el lector abre. El resto del sistema de tipos de `ColumnType` (LONG256, geohash, UUID, IPv4, ARRAY, DECIMAL8..256, BINARY, INTERVAL, …) está en el fuente y no tiene lector.

## SQL y protocolos vistos, no portados

El compilador real es `SqlCompilerImpl` → parser → `SqlOptimiser` → `SqlCodeGenerator`. El slice Rust no reutiliza ese código ni una librería SQL genérica: tiene un parser propio de un único `SELECT`.

Protocolos presentes en `cutlass` y todavía sin implementación Rust: HTTP `/exec`, ILP (TCP y HTTP), PostgreSQL wire, QWP. La consola web se copia a `public/` al arrancar el binario Java; no se reescribe.

WAL (`cairo/wal`, `WalWriter`, sequencer `_meta` / `_txn`), O3, dedup, Parquet (`data.parquet`, `_pm`), índices bitmap (`.k`, `.v`), materialized/live views (`mv`, `lv`) y migraciones (`mig`) están en el árbol y fuera de H1.

## Licencia

Apache-2.0. El port Rust declara la misma licencia en el workspace. La atribución está en `NOTICE`.
