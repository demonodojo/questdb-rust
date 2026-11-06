# Arquitectura del slice actual

El binario `qdb` no arranca un servidor. `qdb query --data-dir DIR --sql SQL` parsea un `SELECT`, abre `<dir>/db/<tabla>` y escribe TSV por stdout.

```text
qdb-server  CLI
    → qdb-sql     parser y filtro/orden en memoria
        → qdb-storage   _meta, _txn, columnas, mapa de símbolos
            → qdb-platform  FileSystem (hoy std::fs)
                → qdb-types  tags, centinelas y formato temporal
```

`qdb-storage` lee el fichero entero y comprueba longitudes antes de decodificar. No usa `mmap` ni `unsafe`. Las filas de la partición `default` se materializan en `Vec<Scalar>`. Sirve para la tabla de cuatro filas del fixture; no es el ejecutor vectorizado de QuestDB.

La frontera de sistema operativo es el trait `qdb_platform::FileSystem`. Un adaptador Soso tendría que implementar `read`, `is_dir` y `list_dir`. El resto del slice no llama a libc.
