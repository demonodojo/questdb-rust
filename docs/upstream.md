# Referencia QuestDB 10.1.0

Consulta: 2026-11-06.

| Campo | Valor |
| --- | --- |
| Tag | `10.1.0` |
| Commit | `e11c837f8a169b533540bd6fa57ef3d520b68ad9` |
| Fecha del commit | 2026-10-22 12:56:06 +0200 |
| Mensaje | `[maven-release-plugin] prepare release 10.1.0` |
| Licencia | Apache-2.0 (`reference/questdb/LICENSE.txt`) |
| Avisos de terceros | `reference/questdb/THIRD_PARTY_LICENSES.txt` |
| `NOTICE` | no existe en este tag |

El tag anotado apunta al objeto `ea109fc567c8ed3d476cbcfaa97b75f5d79c0c8a`. El commit que contiene el código es el de la tabla. No usar `master`.

## Clonar

```bash
mkdir -p reference
git clone --depth 1 --branch 10.1.0 https://github.com/questdb/questdb.git reference/questdb
git -C reference/questdb rev-parse HEAD
git -C reference/questdb describe --tags --exact-match
```

`reference/` está en `.gitignore`. No forma parte del binario Rust.

## Construir desde fuente

`CONTRIBUTING.md` del tag exige Java 25 y Maven 3:

```bash
mvn clean package -DskipTests -P build-web-console
java --add-exports=java.base/jdk.internal.vm=io.questdb \
  -p core/target/questdb-10.1.0.jar \
  -m io.questdb/io.questdb.ServerMain -d <root_dir>
```

En esta máquina Java es `25.0.4.1` (`/usr/lib/jvm/java-25-openjdk-amd64`). `mvn` no está instalado y el repositorio no trae `mvnw`, así que el árbol de fuentes no se compiló. El detalle está en `docs/blockers.md`.

## Binario usado como oráculo

Se descargó el artefacto publicado del mismo tag:

```bash
curl -fL -o /tmp/questdb-10.1.0-no-jre-bin.tar.gz \
  https://github.com/questdb/questdb/releases/download/10.1.0/questdb-10.1.0-no-jre-bin.tar.gz
tar -xzf /tmp/questdb-10.1.0-no-jre-bin.tar.gz -C reference
```

`META-INF/MANIFEST.MF` del jar declara:

- `Implementation-Version: 10.1.0`
- `Build-Jdk: 25.0.2`
- `Build-Commit-Hash: e11c837f8a169b533540bd6fa57ef3d520b68ad9`

Ese hash coincide con el clon. Arranque aislado, sin tocar ningún directorio de datos ajeno:

```bash
export JAVA_HOME=/usr/lib/jvm/java-25-openjdk-amd64
reference/questdb-10.1.0-no-jre-bin/questdb.sh start \
  -d reference/runtime/questdb-root
```

`server.conf` de esa raíz fijó `http.bind.to=127.0.0.1:19011`, desactivó PG Wire, ILP y telemetría, y puso `cairo.wal.enabled.default=false`.

Comprobaciones hechas contra ese proceso:

1. `SELECT 1` por `GET /exec` devolvió `[[1]]`.
2. `CREATE TABLE sensors ... PARTITION BY NONE` más cuatro `INSERT`.
3. `SELECT * FROM sensors ORDER BY ts` devolvió las cuatro filas.
4. `questdb.sh stop` y `start` de nuevo.
5. `SELECT count() FROM sensors` devolvió `4` después del reinicio.

La tabla copiada está en `tests/fixtures/h1/db/sensors`. El SQL está en `tests/fixtures/h1/load.sql`.
