# Encargo para agente de codificación: reescritura de QuestDB OSS en Rust

> **Uso:** entrega este archivo íntegro a un agente con acceso a terminal, editor y, si está disponible, Internet. El agente debe trabajar dentro de `./questdb-rust` (o en la ruta de `QUESTDB_RUST_DIR`). No debe limitarse a producir documentación: debe implementar código funcional e ir avanzando por hitos verificables.

## 1. Misión y alcance

Reescribe **todo el motor y servidor de QuestDB Open Source 10.1.0 en Rust**, conservando la semántica y, en la medida en que sea técnicamente viable, los formatos de almacenamiento y protocolos de red de esa versión.

**Prioridades, en este orden:** (1) corrección, (2) integridad y durabilidad de datos, (3) compatibilidad funcional, (4) rendimiento, (5) portabilidad. Primero debe funcionar en **Linux x86-64**; más adelante debe poder adaptarse a **Soso**, un sistema operativo en Rust. Diseña fronteras de plataforma claras desde el inicio, pero no ralentices la primera implementación intentando compilar de entrada para Soso.

### Alcance explícito
- **Incluido:** todos los componentes *open source* del backend de la versión fijada: tipos y funciones SQL, parser y planificación, ejecutor, catálogos, almacenamiento columnar, tablas particionadas, WAL, concurrencia, índices, ingestión, consultas, características temporales, Parquet, mantenimiento, vistas y protocolos (HTTP/REST, ILP, PostgreSQL Wire y QWP cuando correspondan al alcance OSS de 10.1.0). Descubre y registra las capacidades exactas del código antes de darlas por supuestas.
- **Excluido:** QuestDB Enterprise, características cerradas y servicios externos no disponibles en el repositorio público. **No reescribas la web console en Rust**: si es necesario, sirve sus recursos estáticos existentes. El objetivo «100 % Rust» se aplica al **motor y backend ejecutable**, no al HTML/TypeScript de la interfaz web ni a herramientas de pruebas de desarrollo.
- **Sin JVM ni dependencias del motor QuestDB compiladas desde Java o C/C++ en producción.** Se permiten llamadas a APIs del sistema (`libc`/syscalls) y crates de terceros sujetos a su licencia; cualquier excepción nativa se debe justificar y no puede esconder un componente antiguo sin portar.
- **No se exige compatibilidad binaria de los ficheros desde el primer hito.** Debes investigarla, implementarla y probarla progresivamente. Nunca declares compatibilidad de escritura sobre archivos de QuestDB hasta probarla exhaustivamente.
- **No confundir una nueva TSDB en Rust con un port de QuestDB:** el producto final exige paridad verificable con el software original, no solo SQL genérico.

## 2. Directorio y política de ejecución

Usa como directorio raíz `${QUESTDB_RUST_DIR:-./questdb-rust}`. Si ya existe, inspecciónalo y continúa sin sobrescribir trabajo ajeno. No alteres repositorios ni ficheros fuera de ese directorio. No borres ficheros o bases de datos del usuario, no ejecutes comandos destructivos y no instales paquetes globales sin permiso.

Si tienes red, clona el código de referencia inmovilizado:

```bash
mkdir -p "${QUESTDB_RUST_DIR:-./questdb-rust}" && cd "${QUESTDB_RUST_DIR:-./questdb-rust}"
mkdir -p reference
# Ejecutar solo si reference/questdb no existe:
git clone --depth 1 --branch 10.1.0 https://github.com/questdb/questdb.git reference/questdb
(cd reference/questdb && git rev-parse HEAD)
```

Guarda en `docs/upstream.md` el tag, el hash exacto del commit, la fecha de consulta y las instrucciones reproducibles para construir la referencia. **No trabajes desde `master` en tareas posteriores.** Respeta `LICENSE.txt`, `NOTICE` si existe y `THIRD_PARTY_LICENSES.txt`; revisa licencias de los crates y conserva atribuciones derivadas. La fuente `reference/questdb` es solo el oráculo y no debe formar parte del binario Rust; exclúyela de artefactos de distribución y, si es un clon independiente, del Git principal.

Si la red, Java 25, Maven o la toolchain que pide upstream no están disponibles, anota el impedimento exacto en `docs/blockers.md`; implementa lo que sí puedas verificar localmente, pero no inventes resultados de referencia.

## 3. Estructura prevista (crear gradualmente, no todo como scaffolding vacío)

```text
questdb-rust/
├── AGENTS.md                     # instrucciones resumidas de continuación
├── Cargo.toml                    # workspace
├── Cargo.lock
├── rust-toolchain.toml           # toolchain de la reescritura
├── crates/
│   ├── qdb-types/                # tipos, NULL, timestamps, símbolos, coerciones
│   ├── qdb-platform/             # interfaces para filesystem/mmap/threads/net/time
│   ├── qdb-catalog/              # esquema, metadatos y versiones
│   ├── qdb-storage/              # lectura/escritura columnar y particiones
│   ├── qdb-wal/                  # log, commit, replay, crash recovery
│   ├── qdb-sql/                  # lexer, parser, AST, funciones y semántica
│   ├── qdb-planner/              # planificación y optimización
│   ├── qdb-executor/             # ejecución vectorizada y operadores
│   ├── qdb-ingest/               # ingestión, O3, deduplicación
│   ├── qdb-parquet/              # compatibilidad con Parquet
│   ├── qdb-protocol-http/        # API HTTP/REST e ILP sobre HTTP
│   ├── qdb-protocol-pgwire/      # protocolo PostgreSQL
│   ├── qdb-protocol-qwp/         # QuestDB Wire Protocol
│   └── qdb-server/               # binario, configuración, arranque y shutdown
├── tests/
│   ├── fixtures/                 # datasets originales deterministas
│   ├── differential/             # misma entrada contra original y port
│   ├── interoperability/         # formato on-disk y protocolos
│   ├── crash/                    # inyección de fallos y reinicio
│   └── concurrency/              # carreras y cargas concurrentes
├── benches/                      # benchmarks reproducibles
├── tools/                        # scripts de referencia / fixtures / reportes
├── docs/
│   ├── upstream.md
│   ├── architecture.md
│   ├── inventory.md
│   ├── compatibility.md
│   ├── decisions.md
│   ├── portability.md
│   ├── progress.md
│   └── blockers.md
└── reference/questdb/            # copia fijada de QuestDB, no distribuir
```

Empieza con **pocos crates reales** (`qdb-types`, `qdb-platform`, `qdb-storage`, `qdb-sql`, `qdb-server`) y separa más módulos cuando haya código y pruebas que lo justifiquen. Evita capas genéricas que añadan costes a los caminos críticos.

## 4. Reglas innegociables de implementación

1. **Haz código, no solo planes.** En cada iteración termina una unidad pequeña, ejecutable y comprobable.
2. **Implementa por comportamiento**, apoyándote en el código de referencia y sus pruebas; no traduzcas clases Java mecánicamente. No sustituyas características complejas por una librería SQL genérica y declares paridad. Si reutilizas librerías, documenta exactamente qué comportamiento cubren y qué falta.
3. **Test-driven diferencial:** antes de portar una característica, localiza las pruebas originales, extrae casos observables y escribe pruebas Rust comparables. Siempre que sea posible, ejecuta la misma entrada en QuestDB original y en el port.
4. **Una funcionalidad no es «implementada» hasta que tenga pruebas que pasen.** Etiquetas permitidas en `docs/compatibility.md`: `not-started`, `in-progress`, `verified`, `blocked`; acompáñalas con rutas de pruebas y fecha. No uses porcentajes arbitrarios.
5. **Fidelidad semántica:** vigila `NULL`, `NaN`, overflow y casts, TIMESTAMP/zonas horarias, precisión temporal, orden no especificado, errores SQL, transacciones, identificadores y codificación de texto. Compara los resultados normalizados únicamente donde la semántica permite normalización; no disimules diferencias significativas.
6. **Durabilidad antes que velocidad:** es preferible devolver `Unsupported` explícito a reconocer una operación que pueda corromper datos. No escribas en un directorio de datos del servidor de referencia; utiliza copias aisladas. Nunca abras en modo escritura el mismo directorio desde ambos motores.
7. **`unsafe` mínimo, localizado y auditado**; incluye invariantes de seguridad en comentarios, pruebas de bordes, fuzzing y herramientas de análisis cuando proceda.
8. **Rendimiento medido, no supuesto.** Conserva un benchmark del QuestDB original en el mismo hardware/configuración. Primero iguala resultados; luego optimiza y documenta regresiones.
9. **Backend autocontenido en Rust:** al terminar debe arrancar sin JVM, sin `libquestdb` C/C++ de upstream y sin librerías Java. Inspecciona también dependencias nativas transitivas antes de hacer afirmaciones de «100 % Rust».
10. **No declares terminado el proyecto mientras haya funcionalidades OSS relevantes no verificadas.** Registra carencias y divergencias, no las ocultes.
11. Si el tiempo o el contexto de la sesión se agota, actualiza `docs/progress.md` y termina con un siguiente paso concreto, sin sustituir código por resúmenes extensos.

## 5. Hitos obligatorios y criterios de salida

### H0. Auditoría de código y referencia ejecutable

**Tareas:**
- Explora la fuente real y registra un inventario de componentes, dependencias, formatos, APIs, opciones de configuración, SQL, tests y licencias. Rutas iniciales a explorar: `core/src/main/java`, `core/src/test/java`, `core/src/main/c`, `core/rust/qdbr`, `benchmarks` y `CONTRIBUTING.md`. Verifica cada ubicación contra el tag fijado.
- Localiza el servidor principal, las clases de almacenamiento (incluidas las rutas de datos y metadatos), el WAL, el parser/optimizador SQL, el planificador, las implementaciones nativas, las vistas y cada protocolo. Traza un mapa de dependencias y tamaño aproximado por subsistema; no presupongas cómo son los archivos binarios.
- Construye y arranca QuestDB original si hay dependencias disponibles. Prueba consultas con datos conocidos, una inserción, reinicio y lectura tras reinicio.
- Genera fixtures con fecha/hora fija y sin aleatoriedad implícita: distintos tipos, `NULL`, cadenas UTF-8, símbolos, filas ordenadas/desordenadas, particiones y WAL. Conserva SQL/HTTP/ILP de entrada, resultados esperados, versiones y checksums.
- Crea scripts `tools/run_reference.sh`, `tools/generate_fixtures.sh`, `tools/run_differential.sh` (inicialmente pueden ofrecer errores claros si faltan componentes, pero no falsos éxitos).

**Criterio de salida:** inventario real en `docs/inventory.md`; referencia fijada; un conjunto mínimo reproducible de pruebas de referencia, o un blocker explícito con comandos intentados y errores exactos. **No presentes un inventario imaginado como comprobado.**

### H1. Workspace Rust y slice vertical mínimo de lectura

**Tareas:**
- Inicializa el workspace con `cargo fmt`, `cargo clippy`, `cargo test`; configura CI local o script `tools/check.sh` que haga fallar si alguna comprobación falla.
- Implementa tipos primitivos y semántica mínima (`NULL`, timestamps, cadenas, números) basados en upstream, sin inventar un formato propio si se requiere compatibilidad.
- Implementa lectura **read-only** de al menos un fixture de tabla original, tras inspeccionar su formato y validar cabeceras, límites y versiones. No mapees datos corruptos con `unsafe` sin checks.
- Implementa una interfaz SQL real y pequeña para `SELECT` de las columnas del fixture con filtro simple, y emite el resultado por CLI o API que puedas testear; añade `qdb-server` con `--data-dir` y `--http-bind` cuando el servidor esté listo.
- Crea pruebas diferenciales para columnas, filas, `NULL` y filtros. Rechaza explícitamente instrucciones no implementadas.

**Criterio de salida:** `cargo test --workspace` pasa y un test comprueba datos leídos de un fixture de QuestDB original con resultados correctos. **No aceptar como hito cumplido únicamente `SELECT 1` sin leer datos reales.**

### H2. SQL de lectura, catálogo y almacenamiento

**Tareas:**
- Amplía el lector al catálogo completo, todos los tipos OSS, particiones, metadatos, símbolos/índices y formatos documentados por el análisis; registra cada versión de formato soportada.
- Porta parser/AST, validación, funciones y operadores; después proyección, filtros, orden, límites, agrupación y agregados. Integra semántica de timestamps y time-series SQL.
- Añade joins y funciones temporales según la matriz de compatibilidad, con pruebas de borde y comportamiento; añade planificación incremental.
- Prueba bytes corruptos, longitudes inválidas, UTF-8 malformado, `NULL`, precisión temporal y archivos parcialmente escritos.

**Criterio de salida:** conjuntos relevantes de pruebas SQL de lectura pasan de forma diferencial y toda característica que no pase está listada en `docs/compatibility.md`.

### H3. Escritura, WAL, integridad y recuperación

**Tareas:**
- Implementa creación y modificación de esquemas, escritura nativa, particiones, transacciones y WAL basados en los invariantes y orden de persistencia de QuestDB 10.1.0.
- Comprueba atomicidad de metadatos/commit, `fsync`/durabilidad, reintentos, idempotencia de replay, escrituras fuera de orden (O3), deduplicación y operaciones concurrentes.
- Añade puntos controlados de inyección de fallos antes/después de cada barrera de persistencia; prueba SIGKILL/procesos separados, reinicio y recuperación.
- Compara con QuestDB original los resultados lógicos y, donde se declare compatibilidad de formato, la apertura cruzada **sobre copias nuevas y separadas**. Nunca pruebas destructivas sobre datos reales.

**Criterio de salida:** escritura e ingestión reales que sobreviven a reinicio y a inyecciones de fallo; ningún test de integridad conocido falla. No declares compatibilidad on-disk bidireccional hasta superar pruebas cruzadas específicas.

### H4. Servidor y protocolos

**Tareas:**
- Implementa API de consulta HTTP y ingestión ILP (HTTP y TCP según referencia), más Postgres Wire y QWP (WebSocket/binario) con las reglas observadas en 10.1.0.
- Implementa control de conexiones, backpressure, límites de memoria, cancelaciones, shutdown correcto y errores compatibles; comprueba clientes oficiales cuando sea posible.
- Usa los mismos tests de clientes/contratos sobre QuestDB original y Rust; prueba conexiones simultáneas, desconexiones y mensajes fragmentados/malformados.
- Integra configuración y comportamiento de inicio/parada. La web console puede servirse como contenido estático si la licencia lo permite; no introducir un frontend nuevo como requisito para declarar completo el backend.

**Criterio de salida:** los principales clientes interoperan con el servidor Rust y el conjunto de pruebas de protocolo pasa. Si alguna ruta queda sin verificar, debe constar expresamente.

### H5. Paridad OSS avanzada

**Tareas:**
- Completa el SQL específico: `SAMPLE BY`, `LATEST ON`, joins temporales (`ASOF`, etc.), ventanas, agregaciones, funciones y nuevos elementos de 10.1.0, según inventario real.
- Implementa Parquet, índices, vistas/materialized views/live views, cambios de esquema, mantenimiento, consultas vectorizadas, optimizador, estadísticas y restantes características OSS inventariadas.
- Compara formatos de exportación/importación y reglas de interoperabilidad sobre fixtures. Añade pruebas de extensiones, grandes volúmenes y presión de memoria.
- Recorre **todas** las categorías identificadas en `docs/inventory.md`; actualiza la matriz característica → prueba original → prueba Rust → resultado → limitaciones.

**Criterio de salida:** todas las funcionalidades OSS del alcance tienen evidencia de paridad o una lista explícita de diferencias que impide declarar el port «completo».

### H6. Endurecimiento, rendimiento y portabilidad hacia Soso

**Tareas:**
- Ejecuta fuzzing de parser, WAL, archivos y protocolos; pruebas de concurrencia, filtraciones, memoria y reinicio prolongadas.
- Establece un conjunto reproducible de benchmarks frente a la versión de referencia: ingestión ordenada/O3, lectura filtrada, agrupaciones, joins, memoria y tiempos de arranque. Guarda máquina, versiones, datos, comandos, medianas y variaciones.
- Optimiza solo después de medir: procesamiento por lotes, SIMD, reducción de asignaciones, mmap y concurrencia. Documenta todas las regresiones no resueltas.
- Aísla dependencias OS tras identificar las realmente utilizadas: archivo, `mmap`, hilos, sincronización, red, relojes, aleatoriedad, señales y demás. Implementa Linux primero. En `docs/portability.md` registra contrato exacto para futura implementación `qdb-platform-soso`; evita prometer `no_std` sin auditoría de dependencias.
- Audita `ldd`/binario, cargo tree, símbolos y dependencias para confirmar ausencia de JVM y módulos C/C++ QuestDB.

**Criterio de salida:** batería de integridad y estrés superada; informe diferencial y de rendimiento reproducible; interfaces de plataforma auditadas y documento concreto de portabilidad. **La ejecución real en Soso es un proyecto posterior**, no condición para completar el backend Rust/Linux.

## 6. Marco de pruebas diferencial

Construye un runner automatizado con esta secuencia:
1. Arranca QuestDB 10.1.0 original **en directorio temporal aislado** y carga un dataset generado determinísticamente.
2. Arranca el backend Rust en **otro directorio aislado** con el mismo dataset lógico o copia de fixture *read-only* para pruebas de lector.
3. Ejecuta la misma batería de SQL y llamadas de API sobre ambos. Compara esquemas, tipos, valores, orden (solo cuando SQL lo garantiza), precision temporal, errores y efectos secundarios.
4. Guarda cada discrepancia con entrada mínima reproducible y categoría (`port-bug`, `incompatibility`, `known-upstream-behavior`, `test-harness-bug`).
5. Añade el caso al conjunto permanente de regresión. No elimines pruebas fallidas para que el conjunto se vea verde: separa las pruebas pendientes con un registro visible de resultados y cobertura.

No compares bytes de archivos como criterio universal: la igualdad lógica no implica igualdad binaria. Para los formatos que deban ser compatibles exige además pruebas específicas de lectura cruzada y, solo cuando esté seguro, escritura cruzada.

Casos mínimos: nulos, vacíos, `NaN`, extremos de enteros y decimales, tiempo y zonas, UTF-8, cardinalidad alta de símbolos, registros O3, colisiones de deduplicación, distintos tamaños de partición, consultas concurrentes a escrituras, apagado durante commit, directorios incompletos, inputs corruptos y límites de protocolo.

## 7. Modo de trabajo para sesiones sucesivas

Al **iniciar cualquier sesión**:
1. Lee este documento, `AGENTS.md`, `docs/progress.md`, `docs/compatibility.md` y `docs/blockers.md` si existen.
2. Inspecciona el árbol real; ejecuta las pruebas y reproduce el estado previo; no supongas lo que hay.
3. Selecciona **la siguiente tarea desbloqueada de menor tamaño** que produzca un avance real verificable. No saltes a la siguiente fase con el hito actual sin validar.
4. Implementa, escribe tests, ejecútalos y registra resultados reales. Haz commit si el repositorio está inicializado y hay cambios tuyos, con mensaje acotado; no alteres la historia ajena.
5. Actualiza `docs/progress.md` con: hito actual, cambios, comandos exactos, resultados, bloqueos, limitaciones y **próxima tarea precisa**.

Al **terminar cualquier sesión**, responde con un resumen breve y factual de qué código se añadió, qué pasó en las pruebas, qué no funciona y qué harás después. Si el encargo completo no cabe en una sesión (lo normal), deja el repositorio compilable y listo para retomar; **no declares finalizado el port**.

No pierdas tiempo pidiendo confirmaciones no esenciales. Solicita intervención humana solo para credenciales, cambios fuera del directorio, instalaciones globales, decisiones incompatibles o bloqueos reales. Ante falta de conectividad, no inventes una ejecución upstream; registra el problema y trabaja en componentes verificables.

## 8. Primera ejecución: instrucciones imperativas

Empieza **ahora** con H0 y H1, no con un documento de arquitectura de 100 páginas:

1. Determina la ruta de trabajo y crea el directorio si falta.
2. Inicializa el repositorio Git local si no existe.
3. Clona QuestDB 10.1.0 como referencia, si la red lo permite; anota commit exacto y analiza fuentes y pruebas.
4. Crea `docs/inventory.md` con el primer mapa real del código y `docs/progress.md` con tareas ordenadas y evidencias.
5. Crea el workspace Rust mínimo y `tools/check.sh`.
6. Identifica **una tabla pequeña auténtica** del sistema original y prepara un fixture reproducible. Si puedes arrancar el servidor original, genéralo tú; si no puedes, registra el motivo y no finjas el test.
7. Implementa el primer componente concreto: lectura validada de metadatos/columnas del fixture y prueba automatizada que verifique resultados. Avanza hacia `SELECT` sobre esa tabla.
8. Ejecuta y registra `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` y `cargo test --workspace` para los crates existentes (corrige los problemas o documenta bloqueos con logs).
9. Cierra con un informe del estado real y la próxima modificación concreta.

## 9. Definition of Done del producto completo

Solo se permite afirmar **«QuestDB 10.1.0 OSS portado a Rust»** si se cumplen simultáneamente:

- [ ] Inventario exhaustivo de funcionalidades OSS de 10.1.0, con trazabilidad a código y tests upstream.
- [ ] Backend Linux compilable, ejecutable sin Java ni C/C++ del motor QuestDB.
- [ ] Matriz de SQL, funciones, datos, protocolos, Parquet, almacenamiento, WAL y vistas totalmente verificada mediante pruebas comparativas; divergencias conocidas documentadas y explícitamente aceptadas, sin presentar la paridad incompleta como completa.
- [ ] Seguridad, concurrencia, durabilidad y recuperación probadas incluyendo fallos inyectados.
- [ ] Compatibilidad con formato de disco probada solo para las versiones y operaciones expresamente declaradas.
- [ ] Benchmarks reproducibles y resultados publicados, sin afirmaciones no medidas de superioridad.
- [ ] Licencias y avisos de terceros correctos; documentación de instalación, uso, límites y diseño de adaptador Soso.

**Objetivo de esta ejecución, no del proyecto total:** dejar una implementación inicial verdadera, ejecutable y con pruebas, además del inventario y el siguiente paso. La reescritura completa requiere múltiples sesiones e iteraciones de ingeniería.

## 10. Fuentes para empezar

- Repo y licencia: https://github.com/questdb/questdb
- Release fijada: https://github.com/questdb/questdb/releases/tag/10.1.0
- Guía de desarrollo y rutas: https://github.com/questdb/questdb/blob/10.1.0/CONTRIBUTING.md
- Docs: https://questdb.com/docs/
- Rust de referencia dentro del repo: `core/rust/qdbr` (verificar dentro del tag)

**Primera respuesta esperada del agente:** informa en pocas líneas la ruta efectiva de trabajo, el commit/tag verificado, el primer componente concreto que vas a construir, y ejecuta los cambios. No te detengas tras describir el plan.
