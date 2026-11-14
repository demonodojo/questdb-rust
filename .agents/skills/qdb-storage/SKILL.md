---
name: qdb-storage
description: Implementa o revisa el lector de archivos QuestDB del port Rust, sus metadatos, columnas, símbolos y particiones. Úsala para ampliar formatos de lectura o diagnosticar corrupción, offsets inválidos y discrepancias de fixtures binarios.
---

# Lectura de disco con límites comprobados

Trabaja desde la raíz y aplica [AGENTS.md](../../../AGENTS.md). Consulta [compatibilidad](../../../docs/compatibility.md), [arquitectura](../../../docs/architecture.md) y [upstream](../../../docs/upstream.md) para determinar el formato y comportamiento soportados antes de extenderlos.

## Localizar el contrato

El lector está en `crates/qdb-storage/src/`: `bytes.rs` valida acceso a bytes, `meta.rs` interpreta `_meta`, `txn.rs` interpreta transacciones/particiones, `column.rs` lee columnas y `table.rs` ensambla la tabla. Los tags, sentinelas NULL y unidades están en `crates/qdb-types/src/lib.rs`; el acceso a plataforma está en `qdb-platform`.

Busca en el tag fijado de `reference/questdb` las constantes, rutinas de escritura/lectura y tests que justifican cada offset. No deduzcas un layout universal de una muestra ni asumas que igualdad lógica implica igualdad binaria.

## Mantener invariantes

- Comprueba versión, flags y modos antes de interpretar datos. Mantén el rechazo explícito de formatos no soportados; no elimines una comprobación sin implementar y probar el caso que protegía.
- Valida tamaños, offsets y multiplicaciones/sumas antes de indexar o reservar memoria. Comprueba truncamiento, índices de símbolos, longitudes de texto y codificación contra el tamaño del archivo.
- Respeta los sentinelas de cada tipo, incluido que BOOLEAN/BYTE/SHORT/CHAR no distinguen NULL de cero en el formato upstream. No los uniformes a la representación de INT o LONG.
- En particiones DAY contempla directorios con sufijo de name txn y el uso de `transient_row_count` en la última partición; no tomes siempre su tamaño del registro de particiones.
- Conserva los fixtures originales. Para corrupción y archivos incompletos usa copias aisladas bajo `target/`; nunca escribas sobre el fixture compartido o sobre la raíz activa de referencia.
- Evita `unsafe`. Una excepción exige un bloque local con invariantes justificados y pruebas de borde, según las reglas del proyecto.

## Demostrar la ampliación

Añade un fixture auténtico o un caso derivado reproducible que pruebe el nuevo modo, más rechazos relevantes: versión/flag no soportado, cabecera truncada o longitud fuera de rango. Incluye una lectura por la API de tabla cuando el cambio altere cómo se ensamblan las filas.

Ejecuta `cargo test -p qdb-storage`, las pruebas de tipos o plataforma afectadas y `tools/check.sh`. Para filas resultantes compara con la referencia usando [qdb-oracle](../qdb-oracle/SKILL.md). En [compatibilidad](../../../docs/compatibility.md) declara únicamente versiones, tipos y modos cubiertos por tests. La lectura correcta no demuestra compatibilidad de escritura; escritura, WAL y recuperación requieren sus propios hitos y pruebas.
