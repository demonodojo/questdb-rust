# Portabilidad hacia Soso

Estado: diseño de frontera, sin port. La ejecución en Soso no es parte de este backend Linux.

## Lo que el slice usa del sistema

| Necesidad | Implementación actual | Contrato para otro OS |
| --- | --- | --- |
| Leer un fichero entero | `std::fs::read` | `FileSystem::read -> Vec<u8>` |
| Saber si una ruta es directorio | `Path::is_dir` | `FileSystem::is_dir` |
| Listar un directorio | `std::fs::read_dir` | `FileSystem::list_dir`, ordenado |
| Reloj, red, hilos, señales, mmap, `fsync` | no se usan | no añadirlos dentro de `qdb-storage` |

Tiempo de TIMESTAMP y DATE se formatea con aritmética UTC en `qdb-types`, sin `chrono` ni tzdata. El proceso CLI usa `std::env::args` y stdout.

## Dependencias

El workspace no declara crates externos. `cargo tree` del binario `qdb` solo contiene `qdb-server`, `qdb-sql`, `qdb-storage`, `qdb-platform` y `qdb-types`. No hay JVM ni `libquestdb` en el enlace. Eso describe este slice, no el port completo.

## Qué no prometer

No es `no_std`. `std::fs` y el allocator del sistema siguen dentro de `qdb-platform`. Un adaptador `qdb-platform-soso` puede existir cuando Soso ofrezca lectura de ficheros equivalente; hasta entonces el código de almacenamiento no debe importar módulos Linux fuera de ese crate.
