# Fixture `trim_cases`

Tabla binaria read-only creada por QuestDB 10.1.0 (`e11c837f8a169b533540bd6fa57ef3d520b68ad9`) en un runtime aislado con WAL desactivado y `PARTITION BY NONE`. Se copió después de apagar QuestDB. `load.sql` reproduce el esquema y las ocho filas; `SHA256SUMS` identifica los ficheros copiados.

`trim_column_oracle.jsonl` conserva las respuestas completas originales de `/exec` para `trim(txt)`, `ltrim(txt)` y `rtrim(txt)` ordenadas por timestamp. `where_trim_oracle.jsonl`, `where_trim_errors_oracle.jsonl`, `where_text_scalar_oracle.jsonl`, `where_text_scalar_errors_oracle.jsonl` y `where_text_is_null_oracle.jsonl`, `where_text_in_oracle.jsonl` y `where_text_in_errors_oracle.jsonl` guardan capturas de filtros ejecutados con la misma tabla; el runner diferencial requiere que la tabla también esté cargada en el runtime QuestDB configurado. Incluye texto acentuado, bandera, marca combinante, espacios, cadena vacía, NULL, tabuladores y espacio Unicode U+2003.
