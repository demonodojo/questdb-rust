# Bloqueos

## 2026-11-06 — no se puede compilar el árbol Java

- Comando no disponible: `mvn` (`No se ha encontrado la orden «mvn»`).
- El tag 10.1.0 no incluye `mvnw`.
- Java 25 sí está: `/usr/lib/jvm/java-25-openjdk-amd64`.
- Efecto: no se ejecutó `mvn clean package` ni la suite JUnit upstream.
- Alternativa usada: el binario publicado `questdb-10.1.0-no-jre-bin.tar.gz`, cuyo `Build-Commit-Hash` es `e11c837f8a169b533540bd6fa57ef3d520b68ad9`.
- No bloquea H1. Bloquea regenerar la referencia compilando el fuente y ejecutar los tests Java upstream.

No hay otro bloqueo abierto. El lector Rust no arranca el servidor HTTP (`--http-bind` queda para H4).
