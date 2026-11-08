# Decisiones

## 2026-11-06 — Oráculo ejecutable sin Maven

No se compiló el fuente. El jar publicado de 10.1.0 declara el mismo commit que el clon, y con él se creó el fixture. Los tests Rust comparan contra esos bytes y contra el TSV capturado de `/exec`, no contra un resultado inventado.

## 2026-11-06 — Lector estricto y pequeño

Solo `PARTITION BY NONE`, WAL desactivado, `_meta` versión 426 y columnas sin sufijo de txn (`columnNameTxn = -1`). Si el fichero no cabe en ese contrato, el lector devuelve error. No rellena con ceros ni ignora cabeceras inválidas.

BOOLEAN, BYTE, SHORT y CHAR guardan el nulo como 0. El lector repite ese hecho: un byte 0 booleano es `false`, un char 0 es vacío. No se inventa un tercer estado que el disco no tiene.

## 2026-11-06 — Particiones DAY nativas

El lector abre `PARTITION BY DAY` además de `NONE`. El nombre del directorio sale del timestamp de partición en microsegundos y del name txn del `_txn`. La partición activa usa `transient_row_count` aunque el tamaño guardado en el registro sea distinto; en el fixture `days` ese tamaño es 0 y hay 2 filas. Parquet y particiones composite se rechazan. HOUR, MONTH, YEAR y WEEK siguen fuera.

## 2026-11-06 — LIMIT como en 10.1.0

`LIMIT` se aplica después del filtro y del `ORDER BY`. `LIMIT n` positivo deja el prefijo y `n` negativo deja el sufijo, en el mismo orden. `LIMIT lo, hi` es el intervalo `[lo, hi)`, con los negativos contados desde el final; si `lo` y `hi` tienen el mismo signo y `lo > hi`, se intercambian. `LIMIT <negative>, <positive>` se rechaza cuando no hay `ORDER BY` o cuando ordena el timestamp designado. Si `ORDER BY` nombra otra columna, ese caso deja las últimas `|lo|` filas. Se contrastó con `/exec` de 10.1.0 en `sensors` y `days`.

## 2026-11-06 — ORDER BY DESC

`DESC` invierte el orden ascendente de 10.1.0. En enteros, cadenas, símbolos, fechas y timestamps el NULL va primero en ASC y último en DESC. En FLOAT y DOUBLE el NULL va al final en ASC y al principio en DESC, y `-0.0` es menor que `0.0`. Se contrastó con `/exec` en `sensors` y `days`, también junto a `LIMIT`.

## 2026-11-06 — ORDER BY de varias columnas

Cada clave se compara con su propio ASC o DESC y con la regla de NULL de su tipo. Si todas empatan, se conserva el orden de llegada. `ORDER BY location` deja las dos filas `madrid` en el orden de inserción. `LIMIT <negative>, <positive>` con `ORDER BY ts, id` sigue el camino de ordenación y deja las últimas `|lo|` filas; con solo `ts` sigue rechazado. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-06 — count()

`count()` cuenta las filas que pasan el filtro, incluidos los NULL, y sin filas devuelve 0. `count(col)` omite NULL. Un `CHAR` 0 no cuenta; un `BYTE` o `SHORT` 0 sí, porque así responde 10.1.0. `count` de un `BOOLEAN` no tiene función. El resultado es una fila `LONG`, y `LIMIT` se aplica a esa fila. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-08 — segundo `JOIN`

Un segundo `JOIN` conserva los nombres cualificados del primero. `sensors` contra `days` y otra vez contra `sensors`, por `id`, devuelve `1`/`1`/`1`, `2`/`2`/`2` y `NULL`/`NULL`/`NULL`. La tercera copia de `id` se llama `id_3`. La nota del `id` nulo sigue siendo `東京`. Se contrastó con `/exec`.

## 2026-11-08 — `CROSS JOIN`

`CROSS JOIN` empareja cada fila izquierda con cada fila derecha y no admite `ON`. `sensors` contra `days` repite los `id` `1`, `2`, `NULL` y `-1` con los `id` `1`, `2`, `3` y `NULL`. Una cláusula `ON` devuelve `Cross joins cannot have join clauses`. Se contrastó con `/exec`.

## 2026-11-08 — `FULL JOIN`

`FULL JOIN` conserva las filas sin pareja de los dos lados. `sensors` contra `days` por `id` deja `-1`/`NULL` y, al final, `NULL`/`3`. La nota de esa última fila es `beta`. Se contrastó con `/exec`.

## 2026-11-08 — `RIGHT JOIN`

`RIGHT JOIN` conserva la fila derecha sin pareja al final y rellena el lado izquierdo con `NULL`. `sensors` contra `days` por `id` deja `NULL`/`3`. La nota de esa fila es `beta`. Se contrastó con `/exec`.

## 2026-11-08 — `LEFT JOIN`

`LEFT JOIN` conserva la fila izquierda sin pareja y rellena el lado derecho con `NULL`. `sensors` contra `days` por `id` deja `-1`/`NULL`. La nota de esa fila también es `NULL`. El `LEFT JOIN` de `sensors` consigo misma por `id` repite `-1`/`-1`. Se contrastó con `/exec`.

## 2026-11-08 — `JOIN` entre `sensors` y `days`

El lado derecho puede ser otra tabla. Por `location`, `sensors` contra `days` devuelve `1`/`3`, `1`/`1`, `2`/`2`, `NULL`/`3`, `NULL`/`1` y `-1`/`NULL`. Por `id` quedan tres filas, y la nota del `id` nulo es `東京`. Se contrastó con `/exec`.

## 2026-11-08 — `JOIN` por `location`

Cuando la clave se repite, las filas del lado derecho salen de la más reciente a la más antigua. `a.location = b.location` devuelve seis filas: `madrid` con el `id` nulo y después con `1`, el `NULL` con `2`, otra vez `madrid`, y `東京` con `-1`. Se contrastó con `/exec`.

## 2026-11-08 — `JOIN` por igualdad

`sensors a JOIN sensors b ON a.id = b.id` devuelve una fila por cada pareja, y `NULL` casa con `NULL`. La columna repetida pasa a llamarse `id_2`. `INNER JOIN` hace lo mismo. `WHERE a.id = 1` deja la fila `1`, `alpha`. Sin `ON` se rechaza con `'on' expected`. Se contrastó con `/exec`.

## 2026-11-08 — pasos `1H`, `1000n` y el paso vacío

`'1H'` es una hora, igual que `'1h'`. `'1000n'` son mil nanosegundos, un microsegundo, y la columna sigue siendo `TIMESTAMP`. Un paso vacío se rechaza con `empty step`. Se contrastó con `/exec`.

## 2026-11-08 — pasos `1M`, `1y`, `1T` y `1U` de `generate_series`

El mes y el año avanzan por calendario: el día se recorta al último del mes resultante y el siguiente paso sale de esa fecha, no del día original. `'1T'` es un milisegundo y `'1U'` un microsegundo. Se contrastó con `/exec`.

## 2026-11-08 — pasos `1h`, `1m` y `1w` de `generate_series`

El paso de la serie de marcas acepta horas, minutos y semanas como anchos fijos. Un paso que no es un periodo, como `'1x'`, se rechaza con `invalid period`. Un paso nulo dice `step argument must be a non-null constant or bind variable constant`. Se contrastó con `/exec`.

## 2026-11-08 — `generate_series` de marcas de tiempo

`generate_series` con dos textos de marca y un paso `'1s'` o `'1d'` produce la columna `TIMESTAMP` `generate_series`. Un día suelto `YYYY-MM-DD` es medianoche UTC. Si el final es anterior al inicio y el paso es positivo, la serie sale en orden ascendente. Dos textos de marca sin paso se rechazan con `inconvertible value`. Se contrastó con `/exec`.

## 2026-11-08 — `generate_series()`

`generate_series(start, end[, step])` es el origen de `FROM`. Con enteros la columna `generate_series` es `LONG` y recorre el intervalo inclusivo; un paso negativo invierte el sentido. Con un extremo `DOUBLE` la columna es `DOUBLE` y el paso por defecto es `1`. Un argumento nulo o no constante se rechaza. Usarla en la lista de `SELECT` dice `cursor function cannot be used as a column`. Se contrastó con `/exec`.

## 2026-11-08 — `long_sequence()`

`long_sequence(n)` es el origen de `FROM` y produce la columna `LONG` `x` con los enteros `1`…`n`. Un conteo nulo, cero o negativo no devuelve filas. Un argumento que no es constante dice `constant expected`. Usarla en la lista de `SELECT` dice `cursor function cannot be used as a column`. Se contrastó con `/exec`.

## 2026-11-08 — `pg_catalog.pg_get_expr()`

`pg_catalog.pg_get_expr` acepta un texto y un entero, o esos dos más un booleano, y devuelve `NULL`. El encabezado conserva la grafía del nombre cualificado. Sin argumentos dice `requires arguments`. Una firma que no encaja lista los tipos entre paréntesis. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-08 — `pg_catalog.pg_get_partkeydef()`

`pg_catalog.pg_get_partkeydef` comprueba un argumento compatible con `INT` y devuelve `NULL`. El encabezado conserva la grafía del nombre cualificado. Sin argumentos publica `pg_catalog.pg_get_partkeydef(INT)`. Dos argumentos dicen `expected: 1, provided: 2`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-08 — `pg_catalog.pg_is_in_recovery()`

`pg_catalog.pg_is_in_recovery` devuelve el booleano fijo `false`. El encabezado conserva la grafía del nombre cualificado. Un argumento se rechaza con `expected: 0, provided: 1`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-08 — `pg_catalog.current_database()`

`pg_catalog.current_database` devuelve la cadena fija `qdb`. El encabezado conserva la grafía del nombre cualificado. Un argumento se rechaza con `expected: 0, provided: 1`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-08 — `pg_catalog.current_schema()`

`pg_catalog.current_schema` devuelve la cadena fija `public`. El encabezado conserva la grafía del nombre cualificado. Un argumento se rechaza con `expected: 0, provided: 1`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-08 — `pg_catalog.age()`

`pg_catalog.age` devuelve el `LONG` fijo `0` cuando el argumento puede convertirse a `LONG`. Acepta enteros, texto, fechas y timestamps. Un `BOOLEAN`, un `DOUBLE` o un `SYMBOL` se rechazan. Sin el esquema `pg_catalog`, el nombre es una función desconocida. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-08 — `pg_catalog.pg_table_is_visible()`

`pg_catalog.pg_table_is_visible` devuelve el booleano fijo `true` cuando el argumento puede convertirse a `INT`. Un `LONG`, un `BOOLEAN` o un `DOUBLE` se rechazan. Sin el esquema `pg_catalog`, el nombre es una función desconocida. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-08 — `pg_catalog.pg_get_userbyid()`

`pg_catalog.pg_get_userbyid` devuelve la cadena fija `public` cuando el argumento puede convertirse a `INT`. Un `LONG`, un `BOOLEAN` o un `DOUBLE` se rechazan. Sin el esquema `pg_catalog`, el nombre es una función desconocida. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-08 — `pg_catalog.version()`

`pg_catalog.version` devuelve la cadena fija `PostgreSQL 12.3, compiled by Visual C++ build 1914, 64-bit, QuestDB`. El encabezado conserva la grafía del nombre cualificado. Un argumento se rechaza con `expected: 0, provided: 1`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-08 — `to_char()`

`to_char` vuelca un `BINARY` como hexadecimales en líneas de 16 bytes. `to_char(from_base64('YWI='))` es `00000000 61 62`. Un binario vacío es la cadena vacía y un nulo es `NULL`. Diecisiete bytes pasan a una segunda línea que empieza en `00000010`; ese caso lo cubre el test y no el diferencial, porque el recuento en vivo parte por saltos de línea. Sin argumentos publica `to_char(BINARY)`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-08 — `json_extract()`

`json_extract` de dos argumentos lee un documento JSON con una ruta `$`, `$.clave` o `$[índice]` y devuelve `VARCHAR`. Un escalar que empieza por `n` se trata como nulo JSON y la ruta no se consulta. Un nulo JSON es la cadena vacía; una clave ausente o un documento vacío es `NULL`. El tercer argumento `1` es la firma tipada y devuelve `BOOLEAN`. Sin argumentos publica `function `json_extract` requires arguments`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-08 — `to_uuid()`

`to_uuid` junta dos mitades `LONG` en un UUID en minúsculas. `to_uuid(1, 2)` es `00000000-0000-0002-0000-000000000001` y `to_uuid(-1, -1)` es `ffffffff-ffff-ffff-ffff-ffffffffffff`. Un nulo ocupa esa mitad con el centinela `LONG`; los dos nulos devuelven `NULL`. El tipo de `typeof` es `UUID`. Sin argumentos publica `to_uuid(LONG, LONG)`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-08 — `make_geohash()`

`make_geohash` convierte longitud, latitud y una precisión de 1 a 60 bits en un geohash. Si la precisión es múltiplo de cinco el texto es base32 y el tipo es `GEOHASH(Nc)`; si no, es una cadena de bits y el tipo es `GEOHASH(Nb)`. Una coordenada nula entra en el hash. Una coordenada fuera de rango, cuando viene de una columna, devuelve NULL. Sin argumentos publica `make_geohash(DOUBLE, DOUBLE, INT constant)`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-08 — `within_box()`

`within_box` dice si un punto cartesiano cae en una caja alineada a los ejes, bordes incluidos. `(0, 0)` cabe en la caja de `(-1, -1)` a `(1, 1)` y `(2, 0)` no. El borde `(1, 1)` es `true` y el origen con caja nula también. Un argumento nulo devuelve `false`. Sin argumentos publica la firma de seis `DOUBLE`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-08 — `within_radius()`

`within_radius` dice si un punto cartesiano está dentro del radio respecto del centro, borde incluido. `(0, 0)` y el centro `(3, 4)` caben en radio `5` y no en radio `4`. El mismo punto con radio `0` es `true`. Un radio negativo o un argumento nulo devuelve `false`. Sin argumentos publica la firma de cinco `DOUBLE`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-08 — `geo_within_radius_latlon()`

`geo_within_radius_latlon` dice si un punto está dentro del radio en metros respecto del centro. El radio incluye el borde: la distancia `111320` con radio `111320` es `true` y con radio `100000` es `false`. Un argumento nulo o un radio negativo devuelve `false`. El centro `(40, -3)` y el punto `(41, 2)` caben en `500000` metros y no en `400000`. Sin argumentos publica la firma de cinco `DOUBLE`. Una latitud fuera de rango se rechaza igual que `geo_distance_meters`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-08 — `geo_distance_meters()`

`geo_distance_meters` devuelve la distancia en metros con la proyección equirectangular de QuestDB. `111_320` metros por grado de latitud, y la longitud se escala con el coseno de la latitud de origen cuando los dos primeros argumentos son constantes. Si el origen es una columna, el coseno usa la latitud media. `(0, 0, 0, 0)` es `0.0` y `(0, 0, 0, 1)` es `111320.0`. `(41.0, 2.0, 40.0, -3.0)` encabeza `-(3.0)` y devuelve `434571.148912159`. Un nulo en cualquier argumento devuelve `NULL`. Una latitud fuera de `-90..=90` se rechaza con `latitude must be between -90 and 90`. Sin argumentos publica la firma de cuatro `DOUBLE`. Otra aridad dice `expected: 4`. Un texto que no es número se rechaza con `inconvertible value`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-08 — `netmask()`

`netmask` devuelve la máscara `IPv4` de un prefijo CIDR. Sin barra el resultado es `255.255.255.255`. `/24` es `255.255.255.0`, `/8` es `255.0.0.0`, `/16` es `255.255.0.0`, `/1` es `128.0.0.0`, `/7` es `254.0.0.0` y `/31` es `255.255.255.254`. El prefijo `0`, un prefijo fuera de `1..=32`, un texto inválido o un nulo devuelven `NULL`. Un espacio delante de la dirección se ignora; un espacio detrás del prefijo anula el resultado. `netmask(note)` y `netmask(location)` devuelven `255.255.255.255`, `NULL`, `255.255.255.255` y `255.255.255.255`. Sin argumentos exige argumentos y no publica firma. Un tipo que no es texto se rechaza con `there is no matching function `netmask``. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-08 — `pg_get_expr()`

`pg_get_expr` devuelve `NULL` de tipo `STRING` con dos o tres argumentos aceptados. El primero es texto, símbolo, varchar, char o nulo. El segundo es un entero de 32 bits, byte, short o nulo. El tercero, si está, es booleano o nulo. `true` y `false` dentro de la llamada son literales booleanos. `pg_get_expr(note, 1)` son cuatro `NULL`. `PG_GET_EXPR` conserva las mayúsculas. `pg_get_expr(note, -1)` encabeza `pg_get_expr(note, -(1))`. Sin argumentos exige argumentos y no publica firma. Otra aridad o tipos distintos se rechazan con `there is no matching function `pg_get_expr``. Una columna ausente es `Invalid column: missing`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-08 — `pg_get_partkeydef()`

`pg_get_partkeydef` devuelve `NULL` de tipo `STRING` cuando el argumento se acepta. Acepta enteros de 32 bits, byte, short, char, texto, símbolo, varchar y nulos. Rechaza long, float, double, boolean, timestamp y date con `argument type mismatch ... expected: INT`. `pg_get_partkeydef(1)` son cuatro `NULL`. `PG_GET_PARTKEYDEF` conserva las mayúsculas. `pg_get_partkeydef(-1)` encabeza `pg_get_partkeydef(-(1))`. Sin argumentos exige `pg_get_partkeydef(INT)`. Dos argumentos dicen `expected: 1, provided: 2`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-08 — `current_setting()`

`current_setting` devuelve una cadena vacía de tipo `STRING` para una constante de texto o un nulo. `server_version_num`, sin distinguir mayúsculas, devuelve `123000`. Una columna de texto se rechaza porque el argumento tiene que ser una constante: `expected: STRING constant, actual: STRING`. Un entero se rechaza con `expected: STRING, actual: INT`. Sin argumentos exige `current_setting(STRING constant)`. Dos argumentos dicen `expected: 1, provided: 2`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-08 — `regexp_replace()`

`regexp_replace` sustituye cada aparición literal del patrón. El tipo es `STRING`. `regexp_replace(note, 'a', 'b')` devuelve `blphb`, `NULL`, una cadena vacía y `βetb`. `REGEXP_REPLACE` conserva las mayúsculas. En `days` devuelve `blphb`, `NULL`, `betb` y `東京`. Un valor, un patrón o un reemplazo nulos dejan `NULL`. `regexp_replace('alpha', 'a', 'b')` devuelve `blphb`. Sin argumentos exige argumentos y no publica firma. Dos argumentos, un `LONG` o un patrón entero se rechazan con `there is no matching function `regexp_replace``. Una columna ausente es `Invalid column: missing`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-08 — `format_type()`

`format_type` devuelve `NULL` de tipo `STRING` cuando los dos argumentos se aceptan. Acepta enteros de 32 bits, byte, short, char, texto, símbolo, varchar y nulos. Rechaza long, double, boolean, timestamp, date y float con `expression type mismatch, expected: INT`. `format_type(20, 0)` son cuatro `NULL`. `FORMAT_TYPE` conserva las mayúsculas. `format_type(1114, -1)` encabeza `format_type(1114, -(1))`. Sin argumentos exige `format_type(INT, INT)`. Un argumento dice `expected: 2, provided: 1` y tres dicen `provided: 3`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-08 — `pg_is_in_recovery()`

`pg_is_in_recovery` devuelve el booleano `false` en cada fila y el tipo es `BOOLEAN`. En `sensors` y en `days` son cuatro `false`. Con `id = 1` queda una fila. `PG_IS_IN_RECOVERY` conserva las mayúsculas del encabezado. Un argumento se rechaza con `wrong number of arguments for function `pg_is_in_recovery`; expected: 0, provided: 1`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-08 — `version()`

`version` devuelve el texto `PostgreSQL 12.3, compiled by Visual C++ build 1914, 64-bit, QuestDB` en cada fila y el tipo es `STRING`. En `sensors` y en `days` son cuatro copias de ese texto. Con `id = 1` queda una fila. `VERSION` conserva las mayúsculas del encabezado. Un argumento se rechaza con `wrong number of arguments for function `version`; expected: 0, provided: 1`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-08 — `current_catalog()`

`current_catalog` devuelve el texto `qdb` en cada fila y el tipo es `STRING`. En `sensors` y en `days` son cuatro `qdb`. Con `id = 1` queda una fila. `CURRENT_CATALOG` conserva las mayúsculas del encabezado. Un argumento se rechaza con `wrong number of arguments for function `current_catalog`; expected: 0, provided: 1`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-08 — `session_user()`

`session_user` devuelve el texto `admin` en cada fila y el tipo es `STRING`. En `sensors` y en `days` son cuatro `admin`. Con `id = 1` queda una fila. `SESSION_USER` conserva las mayúsculas del encabezado. Un argumento se rechaza con `wrong number of arguments for function `session_user`; expected: 0, provided: 1`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-08 — `current_user()`

`current_user` devuelve el texto `admin` en cada fila y el tipo es `STRING`. En `sensors` y en `days` son cuatro `admin`. Con `id = 1` queda una fila. `CURRENT_USER` conserva las mayúsculas del encabezado. Un argumento se rechaza con `wrong number of arguments for function `current_user`; expected: 0, provided: 1`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-08 — `current_database()`

`current_database` devuelve el texto `qdb` en cada fila y el tipo es `STRING`. En `sensors` y en `days` son cuatro `qdb`. Con `id = 1` queda una fila. `CURRENT_DATABASE` conserva las mayúsculas del encabezado. Un argumento se rechaza con `wrong number of arguments for function `current_database`; expected: 0, provided: 1`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-08 — `current_schema()`

`current_schema` devuelve el texto `public` en cada fila y el tipo es `STRING`. En `sensors` y en `days` son cuatro `public`. Con `id = 1` queda una fila. `CURRENT_SCHEMA` conserva las mayúsculas del encabezado. Un argumento se rechaza con `wrong number of arguments for function `current_schema`; expected: 0, provided: 1`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-08 — `to_pg_date()`

`to_pg_date` interpreta un texto `y-MM-dd` como `DATE` a medianoche. `to_pg_date('2020-01-02')` es `2020-01-02T00:00:00.000Z`. `1970-01-01`, `1969-12-31` y `2020-06-15` siguen ese mismo corte. Un texto con hora, vacío, ilegible o nulo sale `NULL`. `note`, `ch`, `location` y `tag` salen `NULL` en las cuatro filas, igual que `note` en `days`. `TO_PG_DATE` conserva las mayúsculas del encabezado. Sin argumentos exige argumentos. Un timestamp, una fecha, un long, un int, un booleano, un double o un byte no tienen función. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-08 — `to_timezone()`

`to_timezone` suma el desplazamiento local de la zona y devuelve `TIMESTAMP`. `UTC` deja el valor igual. En enero, `Europe/Madrid` suma una hora y `America/New_York` resta cinco. El 15 de junio Madrid suma dos horas y Nueva York resta cuatro. Un `DATE` nulo no es NULL: Madrid imprime `294247-01-10T03:46:10.775808Z` y Nueva York `294247-01-09T23:04:52.775808Z`. `to_timezone(ts, 'Europe/Madrid')` es `2020-01-02T04:04:05.000000Z` en la primera fila. `TO_TIMEZONE` conserva las mayúsculas del encabezado. Una zona desconocida se rechaza con `invalid timezone`. Un booleano o un `CHAR` no encajan. Un símbolo o un texto se rechaza como inconvertible a `TIMESTAMP_NS`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-08 — `to_utc()`

`to_utc` resta el desplazamiento local de la zona y devuelve `TIMESTAMP`. `UTC` deja el valor igual. En enero, `Europe/Madrid` resta una hora y `America/New_York` suma cinco. El 15 de junio Madrid resta dos horas. Un `DATE` nulo con Madrid no es NULL: el centinela menos el huso histórico `-00:14:44` se imprime `-290308-01-01T20:13:49.224192Z`. `to_utc(ts, 'Europe/Madrid')` es `2020-01-02T02:04:05.000000Z` en la primera fila. `TO_UTC` conserva las mayúsculas del encabezado. Una zona desconocida se rechaza con `invalid timezone`. Un booleano o un `CHAR` no encajan. Un símbolo o un texto se rechaza como inconvertible a `TIMESTAMP_NS`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-08 — `nanos()`

`nanos` devuelve los nanosegundos dentro del microsegundo como `INT`. Un timestamp en microsegundos no tiene ese resto, así que un valor presente es `0` y un nulo sigue `NULL`. `nanos(ts)` es `0` en las cuatro filas. `nanos(day)` es `0`, `NULL`, `0` y `0`. `nanos(id)` es `0`, `0`, `NULL` y `0`. `nanos(flags)` es `0`, `NULL`, `0` y `0`. En `days`, `nanos(id)` es `0`, `0`, `0` y `NULL`. `NANOS` conserva las mayúsculas del encabezado. Un booleano o un `CHAR` no encajan. Un símbolo o un texto se rechaza como inconvertible a `TIMESTAMP_NS`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-08 — `kurtosis()`

`kurtosis` devuelve la misma curtosis excesiva muestral que `kurtosis_samp`, como `DOUBLE`. Con menos de cuatro valores finitos, o si todos son iguales, el resultado es `NULL`. `kurtosis(b)` es `3.9926202441231036`, `kurtosis(code)` es `3.9999992882590982` y `kurtosis(ts)` es `-1.2000000000000024`. `flags`, `reading`, `score`, `id` y `day` salen `NULL`. Los grupos por `location` salen `NULL`. `KURTOSIS` conserva las mayúsculas del encabezado. Un booleano o un símbolo no encajan. Un texto que no es número se rechaza en la fila. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-08 — `kurtosis_samp()`

`kurtosis_samp` devuelve la curtosis excesiva muestral como `DOUBLE`. Con menos de cuatro valores finitos, o si todos son iguales, el resultado es `NULL`. `kurtosis_samp(b)` es `3.9926202441231036`, `kurtosis_samp(code)` es `3.9999992882590982` y `kurtosis_samp(ts)` es `-1.2000000000000024`. `flags`, `reading`, `score`, `id` y `day` salen `NULL`. Los grupos por `location` salen `NULL`. `KURTOSIS_SAMP` conserva las mayúsculas del encabezado. Un booleano o un símbolo no encajan. Un texto que no es número se rechaza en la fila. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-08 — `::timestamp`

`column::timestamp` convierte una columna a `TIMESTAMP` y el encabezado conserva la grafía. Un nulo sigue siendo `NULL` y un texto es `NULL`. Un entero es microsegundos desde la época y una fecha conserva el instante en microsegundos. Un timestamp se queda igual. `id::timestamp` es `1970-01-01T00:00:00.000001Z`, `1970-01-01T00:00:00.000002Z`, `NULL` y `1969-12-31T23:59:59.999999Z`. En `days` es `1970-01-01T00:00:00.000001Z`, `1970-01-01T00:00:00.000002Z`, `1970-01-01T00:00:00.000003Z` y `NULL`. `ts::timestamp` es `2020-01-02T03:04:05.000000Z`, `2020-01-02T03:04:06.000000Z`, `2020-01-02T03:04:07.000000Z` y `2020-01-02T03:04:08.000000Z`. `day::timestamp` es `2020-01-02T00:00:00.000000Z`, `NULL`, `1970-01-01T00:00:00.000000Z` y `2020-06-15T12:30:00.000000Z`. `note::timestamp` es `NULL` en las cuatro filas. `flags::timestamp` es `1970-01-01T00:00:00.000010Z`, `NULL`, `1970-01-01T00:00:00.000000Z` y `1970-01-01T00:00:00.000042Z`. Un tipo desconocido se rechaza con `invalid constant: missing`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-08 — `::date`

`column::date` convierte una columna a `DATE` y el encabezado conserva la grafía. Un nulo sigue siendo `NULL` y un texto es `NULL`. Un entero es milisegundos desde la época y un timestamp conserva el instante hasta el milisegundo. `id::date` es `1970-01-01T00:00:00.001Z`, `1970-01-01T00:00:00.002Z`, `NULL` y `1969-12-31T23:59:59.999Z`. En `days` es `1970-01-01T00:00:00.001Z`, `1970-01-01T00:00:00.002Z`, `1970-01-01T00:00:00.003Z` y `NULL`. `day::date` es `2020-01-02T00:00:00.000Z`, `NULL`, `1970-01-01T00:00:00.000Z` y `2020-06-15T12:30:00.000Z`. `ts::date` es `2020-01-02T03:04:05.000Z`, `2020-01-02T03:04:06.000Z`, `2020-01-02T03:04:07.000Z` y `2020-01-02T03:04:08.000Z`. `note::date` es `NULL` en las cuatro filas. `flags::date` es `1970-01-01T00:00:00.010Z`, `NULL`, `1970-01-01T00:00:00.000Z` y `1970-01-01T00:00:00.042Z`. Un tipo desconocido se rechaza con `invalid constant: missing`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-08 — `::varchar`

`column::varchar` convierte una columna a `VARCHAR` y el encabezado conserva la grafía. Un nulo sigue siendo `NULL`, el carácter cero es `NULL` y una cadena vacía sigue vacía. Un número se escribe en decimal. `note::varchar` es `alpha`, `NULL`, vacío y `βeta`. En `days` es `alpha`, `NULL`, `beta` y `東京`. `location::varchar` es `madrid`, `NULL`, `madrid` y `東京`. `id::varchar` es `1`, `2`, `NULL` y `-1`. `ch::varchar` es `A`, `NULL`, `β` y `東`. `tag::varchar` es `v1`, `NULL`, `héllo` y `0123456789ABCDEF`. Un tipo desconocido se rechaza con `invalid constant: missing`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-08 — `::symbol`

`column::symbol` convierte una columna a `SYMBOL` y el encabezado conserva la grafía. Un nulo sigue siendo `NULL`, el carácter cero es `NULL` y una cadena vacía sigue vacía. Un número se escribe en decimal. `location::symbol` es `madrid`, `NULL`, `madrid` y `東京`. En `days` es lo mismo. `note::symbol` es `alpha`, `NULL`, vacío y `βeta`. `id::symbol` es `1`, `2`, `NULL` y `-1`. `ch::symbol` es `A`, `NULL`, `β` y `東`. Un tipo desconocido se rechaza con `invalid constant: missing`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-08 — `::char`

`column::char` convierte una columna a `CHAR` y el encabezado conserva la grafía. Un nulo es una cadena vacía y el cero también. `-1` es U+FFFF. Un entero positivo es ese punto de código y un texto aporta su primer carácter. `id::char` es U+0001, U+0002, vacío y U+FFFF. En `days` es U+0001, U+0002, U+0003 y vacío. `ch::char` es `A`, vacío, `β` y `東`. `note::char` es `a`, vacío, vacío y `β`. `flags::char` es un salto de línea, vacío, vacío y `*`; esa consulta queda en el test y fuera del diferencial en vivo porque el salto de línea parte la fila TSV. Un tipo desconocido se rechaza con `invalid constant: missing`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-08 — `::short`

`column::short` convierte una columna a `SHORT` y el encabezado conserva la grafía. Un nulo es `0` y un texto es `0`. `id::short` es `1`, `2`, `0` y `-1`. En `days` es `1`, `2`, `3` y `0`. `flags::short` es `10`, `0`, `0` y `42`. `note::short` es `0`, `0`, `0` y `0`. `code::short` es `7`, `0`, `-32768` y `1`. Un tipo desconocido se rechaza con `invalid constant: missing`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-08 — `::byte`

`column::byte` convierte una columna a `BYTE` y el encabezado conserva la grafía. Un nulo es `0` y un texto es `0`. `id::byte` es `1`, `2`, `0` y `-1`. En `days` es `1`, `2`, `3` y `0`. `flags::byte` es `10`, `0`, `0` y `42`. `note::byte` es `0`, `0`, `0` y `0`. `b::byte` es `3`, `0`, `-128` y `1`. Un tipo desconocido se rechaza con `invalid constant: missing`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-08 — `::boolean`

`column::boolean` convierte una columna a `BOOLEAN` y el encabezado conserva la grafía. Un nulo es `false`, el cero es `false` y un texto es `false`. Cualquier otro número es `true`. `id::boolean` es `true`, `true`, `false` y `true`. En `days` es `true`, `true`, `true` y `false`. `ok::boolean` es `true`, `false`, `true` y `false`. `flags::boolean` es `true`, `false`, `false` y `true`. `note::boolean` es `false`, `false`, `false` y `false`. Un tipo desconocido se rechaza con `invalid constant: missing`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-08 — `::float`

`column::float` y `column::FLOAT` se resuelven como `DOUBLE`. El encabezado es `id::DOUBLE`. Un nulo sigue siendo `NULL` y un texto que no es un número es `NULL`. `id::float` es `1.0`, `2.0`, `NULL` y `-1.0`. En `days` es `1.0`, `2.0`, `3.0` y `NULL`. `flags::float` es `10.0`, `NULL`, `0.0` y `42.0`. `note::float` es `NULL`, `NULL`, `NULL` y `NULL`. `reading::float` es `1.5`, `NULL`, `0.0` y `-0.0`. `score::float` es `1.25`, `NULL`, `0.0` y `-1.5`. Un tipo desconocido se rechaza con `invalid constant: missing`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-08 — `::double`

`column::double` convierte una columna a `DOUBLE`. El encabezado es `id::double`. Un nulo sigue siendo `NULL` y un texto que no es un número es `NULL`. `id::double` es `1.0`, `2.0`, `NULL` y `-1.0`. En `days` es `1.0`, `2.0`, `3.0` y `NULL`. `flags::double` es `10.0`, `NULL`, `0.0` y `42.0`. `note::double` es `NULL`, `NULL`, `NULL` y `NULL`. `reading::double` es `1.5`, `NULL`, `0.0` y `-0.0`. Un tipo desconocido se rechaza con `invalid constant: missing`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-08 — `::int`

`column::int` convierte una columna a `INT`. El encabezado es `id::int`. Un nulo sigue siendo `NULL` y un texto que no es un número es `NULL`. `id::int` es `1`, `2`, `NULL` y `-1`. En `days` es `1`, `2`, `3` y `NULL`. `flags::int` es `10`, `NULL`, `0` y `42`. `note::int` es `NULL`, `NULL`, `NULL` y `NULL`. Un tipo desconocido se rechaza con `invalid constant: missing`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-08 — `::long`

`column::long` convierte una columna a `LONG`. El encabezado es `id::long`. Un nulo sigue siendo `NULL` y un texto que no es un número es `NULL`. `id::long` es `1`, `2`, `NULL` y `-1`. En `days` es `1`, `2`, `3` y `NULL`. `flags::long` es `10`, `NULL`, `0` y `42`. `note::long` es `NULL`, `NULL`, `NULL` y `NULL`. Un tipo desconocido se rechaza con `invalid constant: missing`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-08 — `::string`

`column::string` convierte una columna a `STRING`. El encabezado es `id::string` y conserva el nombre de tipo escrito. Un nulo sigue siendo `NULL`. `id::string` es `1`, `2`, `NULL` y `-1`. En `days` es `1`, `2`, `3` y `NULL`. `flags::string` es `10`, `NULL`, `0` y `42`. `note::string` es `alpha`, `NULL`, la cadena vacía y `βeta`. Un tipo desconocido se rechaza con `invalid constant: missing`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-08 — `!=`

`left != right` compara dos columnas y devuelve `BOOLEAN`. El encabezado es `id != id`. Un nulo no es distinto de un nulo. `id != id` es `false`, `false`, `false` y `false`. En `days` es `false`, `false`, `false` y `false`. `id != flags` y `flags != id` son `true`, `true`, `true` y `true`. `note != note` es `false`, `false`, `false` y `false`. Una columna ausente se rechaza con `Invalid column: missing`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-08 — `<=`

`left <= right` compara dos columnas y devuelve `BOOLEAN`. El encabezado es `id <= id`. Un nulo es menor o igual que un nulo y no es menor o igual que un valor. `id <= id` es `true`, `true`, `true` y `true`. En `days` es `true`, `true`, `true` y `true`. `id <= flags` es `true`, `false`, `false` y `true`. `flags <= id` es `false`, `false`, `false` y `false`. `note <= note` es `true`, `true`, `true` y `true`. Una columna ausente se rechaza con `Invalid column: missing`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-08 — `>=`

`left >= right` compara dos columnas y devuelve `BOOLEAN`. El encabezado es `id >= id`. Un nulo es mayor o igual que un nulo y no es mayor o igual que un valor. `id >= id` es `true`, `true`, `true` y `true`. En `days` es `true`, `true`, `true` y `true`. `id >= flags` es `false`, `false`, `false` y `false`. `flags >= id` es `true`, `false`, `false` y `true`. `note >= note` es `true`, `true`, `true` y `true`. Una columna ausente se rechaza con `Invalid column: missing`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-08 — `>`

`left > right` compara dos columnas y devuelve `BOOLEAN`. El encabezado es `id > id`. Un nulo no es mayor que un nulo ni que un valor. `id > id` es `false`, `false`, `false` y `false`. En `days` es `false`, `false`, `false` y `false`. `id > flags` es `false`, `false`, `false` y `false`. `flags > id` es `true`, `false`, `false` y `true`. `note > note` es `false`, `false`, `false` y `false`. Una columna ausente se rechaza con `Invalid column: missing`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-08 — `<`

`left < right` compara dos columnas y devuelve `BOOLEAN`. El encabezado es `id < id`. Un nulo no es menor que un nulo ni que un valor. `id < id` es `false`, `false`, `false` y `false`. En `days` es `false`, `false`, `false` y `false`. `id < flags` es `true`, `false`, `false` y `true`. `flags < id` es `false`, `false`, `false` y `false`. `note < note` es `false`, `false`, `false` y `false`. Una columna ausente se rechaza con `Invalid column: missing`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-08 — `<>`

`left <> right` compara dos columnas y devuelve `BOOLEAN`. El encabezado es `id <> id`. Un nulo no es distinto de un nulo. `id <> id` es `false`, `false`, `false` y `false`. En `days` es `false`, `false`, `false` y `false`. `id <> flags` y `flags <> id` son `true`, `true`, `true` y `true`. `note <> note` es `false`, `false`, `false` y `false`. Una columna ausente se rechaza con `Invalid column: missing`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-08 — `=`

`left = right` compara dos columnas y devuelve `BOOLEAN`. El encabezado es `id = id`. Un nulo es igual a un nulo y un nulo no es igual a un valor. `id = id` es `true`, `true`, `true` y `true`. En `days` es `true`, `true`, `true` y `true`. `id = flags` y `flags = id` son `false`, `false`, `false` y `false`. `note = note` es `true`, `true`, `true` y `true`. Una columna ausente se rechaza con `Invalid column: missing`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-08 — `^`

`left ^ right` es el XOR bit a bit de dos columnas enteras. El encabezado es `id ^ id` y el tipo es `LONG`. Un operando nulo es `NULL`. `id ^ id` es `0`, `0`, `NULL` y `0`. En `days` es `0`, `0`, `0` y `NULL`. `id ^ flags` y `flags ^ id` son `11`, `NULL`, `NULL` y `-43`. Una columna ausente se rechaza con `Invalid column: missing`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-08 — `|`

`left | right` es el OR bit a bit de dos columnas enteras. El encabezado es `id | id` y el tipo es `LONG`. Un operando nulo es `NULL`. `id | id` es `1`, `2`, `NULL` y `-1`. En `days` es `1`, `2`, `3` y `NULL`. `id | flags` y `flags | id` son `11`, `NULL`, `NULL` y `-1`. Una columna ausente se rechaza con `Invalid column: missing`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-08 — `&`

`left & right` es el AND bit a bit de dos columnas enteras. El encabezado es `id & id` y el tipo es `LONG`. Un operando nulo es `NULL`. `id & id` es `1`, `2`, `NULL` y `-1`. En `days` es `1`, `2`, `3` y `NULL`. `id & flags` y `flags & id` son `0`, `NULL`, `NULL` y `42`. Una columna ausente se rechaza con `Invalid column: missing`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-08 — `!~`

`value !~ pattern` niega `~` y el encabezado es `note !~ 'a.*'`. El tipo es `BOOLEAN`. `note !~ 'a.*'` es `false`, `true`, `true` y `false`. En `days` es `false`, `true`, `false` y `true`. `note !~ '.*eta'` es `true`, `true`, `true` y `false`. `location !~ 'mad.*'` es `false`, `true`, `false` y `true`. Un patrón que es columna se rechaza con `there is no matching operator `!~` with the argument types: STRING !~ STRING`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-08 — `~`

`value ~ pattern` busca una expresión regular constante. El encabezado es `note ~ 'a.*'` y el tipo es `BOOLEAN`. Un nulo es `false`. `note ~ 'a.*'` es `true`, `false`, `false` y `true`. En `days` es `true`, `false`, `true` y `false`. `note ~ '.*eta'` es `false`, `false`, `false` y `true`. `location ~ 'mad.*'` es `true`, `false`, `true` y `false`. Un patrón que es columna se rechaza con `not implemented: dynamic pattern would be very slow to execute`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-08 — `NOT ILIKE`

`value NOT ILIKE pattern` niega `ilike`. El encabezado es `not(note ilike 'A%')` y el tipo es `BOOLEAN`. `note NOT ILIKE 'A%'` es `false`, `true`, `true` y `true`. En `days` es `false`, `true`, `true` y `true`. `note NOT ILIKE '%ETA'` es `true`, `true`, `true` y `false`. `location NOT ILIKE 'MAD%'` es `false`, `true`, `false` y `true`. Un patrón que es columna se rechaza con `use constant or bind variable`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-08 — `ILIKE`

`value ILIKE pattern` es la función `ilike`. El encabezado es `note ilike 'A%'` y el tipo es `BOOLEAN`. Un nulo es `false` y la comparación no distingue mayúsculas. `note ILIKE 'A%'` es `true`, `false`, `false` y `false`. En `days` es `true`, `false`, `false` y `false`. `note ILIKE '%ETA'` es `false`, `false`, `false` y `true`. `location ILIKE 'MAD%'` es `true`, `false`, `true` y `false`. Un patrón que es columna se rechaza con `use constant or bind variable`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — `NOT LIKE`

`value NOT LIKE pattern` niega `like`. El encabezado es `not(note like 'a%')` y el tipo es `BOOLEAN`. `note NOT LIKE 'a%'` es `false`, `true`, `true` y `true`. En `days` es `false`, `true`, `true` y `true`. `note NOT LIKE '%eta'` es `true`, `true`, `true` y `false`. `location NOT LIKE 'mad%'` es `false`, `true`, `false` y `true`. Un patrón que es columna se rechaza con `use constant or bind variable`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — `LIKE`

`value LIKE pattern` es la función `like`. El encabezado es `note like 'a%'` y el tipo es `BOOLEAN`. Un nulo es `false`. `note LIKE 'a%'` es `true`, `false`, `false` y `false`. En `days` es `true`, `false`, `false` y `false`. `note LIKE '%eta'` es `false`, `false`, `false` y `true`. `location LIKE 'mad%'` es `true`, `false`, `true` y `false`. Un patrón que es columna se rechaza con `use constant or bind variable`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — `NOT BETWEEN`

`value NOT BETWEEN low AND high` niega `between`. El encabezado es `not(id between (1, 2))` y el tipo es `BOOLEAN`. `id NOT BETWEEN 1 AND 2` es `false`, `false`, `true` y `true`. En `days` es `false`, `false`, `true` y `true`. `flags NOT BETWEEN 0 AND 10` es `false`, `true`, `false` y `true`. `id NOT BETWEEN id AND id` es `false`, `false`, `true` y `false`. Un `DOUBLE` se rechaza con el mismo texto que `between`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — `BETWEEN`

`value BETWEEN low AND high` es la función `between`. El encabezado es `id between (1, 2)` y el tipo es `BOOLEAN`. Los límites se incluyen y un nulo es `false`. `id BETWEEN 1 AND 2` es `true`, `true`, `false` y `false`. En `days` es `true`, `true`, `false` y `false`. `flags BETWEEN 0 AND 10` es `true`, `false`, `true` y `false`. `id BETWEEN id AND id` es `true`, `true`, `false` y `true`. Un `DOUBLE` se rechaza. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — `IS NOT NULL`

`value IS NOT NULL` es la comparación `value != NULL`. El encabezado es `id != NULL` y el tipo es `BOOLEAN`. `id IS NOT NULL` es `true`, `true`, `false` y `true`. En `days` es `true`, `true`, `true` y `false`. `note IS NOT NULL` es `true`, `false`, `true` y `true`. `ok IS NOT NULL` es `true` en las cuatro filas. `reading IS NOT NULL` es `true`, `false`, `true` y `true`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — `IS NULL`

`value IS NULL` es la comparación `value = NULL`. El encabezado es `id = NULL` y el tipo es `BOOLEAN`. `id IS NULL` es `false`, `false`, `true` y `false`. En `days` es `false`, `false`, `false` y `true`. `note IS NULL` es `false`, `true`, `false` y `false`. `ok IS NULL` es `false` en las cuatro filas. `reading IS NULL` es `false`, `true`, `false` y `false`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — `not` sin paréntesis

`not value` es la función `not` con un solo argumento. El encabezado es `not(ok)`. `not ok` es `false`, `true`, `false` y `true`. Con `id = 1` es `false` y con `id = 2` es `true`. En `days` se rechaza porque no existe `ok`. Se contrastó con `/exec` en `sensors`.

## 2026-11-07 — operador de concatenación

`left || right` es `concat` de dos columnas. El encabezado es `concat(left, right)` y el tipo es `STRING`. Un número nulo se escribe `null` y un texto nulo desaparece. `id || note` es `1alpha`, `2`, `null` y `-1βeta`. En `days` es `1alpha`, `2`, `3beta` y `null東京`. `note || note` es `alphaalpha`, una cadena vacía, otra cadena vacía y `βetaβeta`. `note || location` es `alphamadrid`, una cadena vacía, `madrid` y `βeta東京`. `id || id` es `11`, `22`, `nullnull` y `-1-1`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — resto de dos columnas

`value % other` también calcula el resto de dos columnas. Un nulo hace nulo el resultado y el resto de cero también, incluido `-0.0 % -0.0`. El signo sigue al dividendo. El tipo se conserva si las dos columnas lo comparten. `LONG` con `INT` ensancha a `LONG`. `id % id` es `0`, `0`, NULL y `0`. En `days` es `0`, `0`, `0` y NULL. `reading % reading` es `0.0`, NULL, NULL y NULL. `flags % flags` es `0`, NULL, NULL y `0`. `id % flags` es `1`, NULL, NULL y `-1`. `flags % id` es `0`, NULL, NULL y `0`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — división de dos columnas

`value / other` también divide dos columnas. Un nulo hace nulo el resultado y dividir por cero también, incluido `-0.0 / -0.0`. El tipo se conserva si las dos columnas lo comparten. `LONG` con `INT` ensancha a `LONG`. `id / id` es `1`, `1`, NULL y `1`. En `days` es `1`, `1`, `1` y NULL. `reading / reading` es `1.0`, NULL, NULL y NULL. `flags / flags` es `1`, NULL, NULL y `1`. `id / flags` es `0`, NULL, NULL y `0`. `flags / id` es `10`, NULL, NULL y `-42`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — producto de dos columnas

`value * other` también multiplica dos columnas. Un nulo hace nulo el resultado. El tipo se conserva si las dos columnas lo comparten. `LONG` con `INT` ensancha a `LONG`. `-0.0 * -0.0` es `0.0`. `id * id` es `1`, `4`, NULL y `1`. En `days` es `1`, `4`, `9` y NULL. `reading * reading` es `2.25`, NULL, `0.0` y `0.0`. `flags * flags` es `100`, NULL, `0` y `1764`. `id * flags` y `flags * id` son `10`, NULL, NULL y `-42`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — resta de dos columnas

`value - other` también resta dos columnas. Un nulo hace nulo el resultado. El tipo se conserva si las dos columnas lo comparten. `LONG` con `INT` ensancha a `LONG`. `-0.0 - -0.0` es `0.0`. `id - id` es `0`, `0`, NULL y `0`. En `days` es `0`, `0`, `0` y NULL. `reading - reading` es `0.0`, NULL, `0.0` y `0.0`. `flags - flags` es `0`, NULL, `0` y `0`. `id - flags` es `-9`, NULL, NULL y `-43`. `flags - id` es `9`, NULL, NULL y `43`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — suma de dos columnas

`value + other` también suma dos columnas. Un nulo hace nulo el resultado. El tipo se conserva si las dos columnas lo comparten. `LONG` con `INT` ensancha a `LONG`. `-0.0 + -0.0` es `-0.0`. `id + id` es `2`, `4`, NULL y `-2`. En `days` es `2`, `4`, `6` y NULL. `reading + reading` es `3.0`, NULL, `0.0` y `-0.0`. `flags + flags` es `20`, NULL, `0` y `84`. `id + flags` es `11`, NULL, NULL y `41`. Una columna que no existe se rechaza. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — disyunción

`value or other` devuelve si una columna booleana o la otra es verdadera. El encabezado es `ok or ok`. El resultado es `BOOLEAN` y no es nulo: un nulo cuenta como falso. `ok or ok` es `true`, `false`, `true` y `false`. `ok or false` y `ok or null` repiten la columna. `ok or true` es `true` en las cuatro filas. En `days` no hay columna `ok`. Se contrastó con `/exec` en `sensors`.

## 2026-11-07 — conjunción

`value and other` devuelve si una columna booleana y otra booleana son las dos verdaderas. El encabezado es `ok and ok`. El resultado es `BOOLEAN` y no es nulo: un nulo cuenta como falso. `ok and ok` es `true`, `false`, `true` y `false`. `ok and false` y `ok and null` son `false` en las cuatro filas. `ok and true` repite la columna. En `days` no hay columna `ok`. Se contrastó con `/exec` en `sensors`.

## 2026-11-07 — complemento

`~value` es el complemento a nivel de bit de una columna. El encabezado es `~(id)`. Un nulo sigue nulo. `LONG` e `INT` conservan el tipo. `BYTE` y `SHORT` pasan a `INT`. Un símbolo pasa a `INT`, la clave del símbolo. Una fecha y un timestamp pasan a `LONG`. `~(id)` es `-2`, `-3`, NULL y `0`. En `days` es `-2`, `-3`, `-4` y NULL. `~(flags)` es `-11`, NULL, `-1` y `-43`. `~(b)` es `-4`, `-1`, `127` y `-2`. `~(code)` es `-8`, `-1`, `32767` y `-2`. `~(day)` es `-1577923200001`, NULL, `-1` y `-1592224200001`. `~(ts)` es `-1577934245000001`, `-1577934246000001`, `-1577934247000001` y `-1577934248000001`. `~(location)` es `-1`, NULL, `-1` y `-2`. `FLOAT`, `DOUBLE` y `BOOLEAN` no tienen operador. Un texto se intenta leer como `LONG` y `alpha` se rechaza. El carácter `A` se rechaza como inconvertible a `INT`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — menos unario

`-value` niega una columna y conserva el tipo numérico, también en `BYTE`, `SHORT` y `FLOAT`. El encabezado es `-(id)`. Un nulo sigue nulo. Negar `-0.0` da `0.0` y negar `0.0` da `-0.0`. El mínimo de `BYTE` y de `SHORT` no cambia. `-(id)` es `-1`, `-2`, NULL y `1`. En `days` es `-1`, `-2`, `-3` y NULL. `-(reading)` es `-1.5`, NULL, `-0.0` y `0.0`. `-(flags)` es `-10`, NULL, `0` y `-42`. `-(score)` es `-1.25`, NULL, `-0.0` y `1.5`. `-(b)` es `-3`, `0`, `-128` y `-1`. `-(code)` es `-7`, `0`, `-32768` y `-1`. `BOOLEAN` no tiene operador. Un texto se intenta leer como `DOUBLE` y `alpha` se rechaza. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — resto

`value % other` devuelve el resto de dividir una columna por una constante y conserva el tipo numérico de la columna. Un nulo hace nulo el resultado. El resto de dividir por `0` también es NULL. El signo del resto sigue al dividendo. `-1 % 2` es `-1`. `id % 2` es `1`, `0`, NULL y `-1`. En `days` es `1`, `0`, `1` y NULL. `id % null` es NULL en las cuatro filas. `id % 0` es NULL en las cuatro filas. `reading % 2` es `1.5`, NULL, `0.0` y `-0.0`. `flags % 2` es `0`, NULL, `0` y `0`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — división

`value / other` divide una columna por una constante y conserva el tipo numérico de la columna. Un nulo hace nulo el resultado. Dividir por `0` también devuelve NULL. La división entera trunca hacia cero. `-0.0 / 2` es `-0.0`. `id / 2` es `0`, `1`, NULL y `0`. En `days` es `0`, `1`, `1` y NULL. `id / null` es NULL en las cuatro filas. `id / 0` es NULL en las cuatro filas. `reading / 2` es `0.75`, NULL, `0.0` y `-0.0`. `flags / 2` es `5`, NULL, `0` y `21`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — producto

`value * other` multiplica una columna por una constante y conserva el tipo numérico de la columna. Un nulo hace nulo el resultado. `-0.0 * 0` es `-0.0`. `id * 2` es `2`, `4`, NULL y `-2`. En `days` es `2`, `4`, `6` y NULL. `id * null` es NULL en las cuatro filas. `reading * 0` es `0.0`, NULL, `0.0` y `-0.0`. `flags * 2` es `20`, NULL, `0` y `84`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — resta

`value - other` resta una constante de una columna y conserva el tipo numérico de la columna. Un nulo hace nulo el resultado. `-0.0 - 0` es `-0.0`. `id - 1` es `0`, `1`, NULL y `-2`. En `days` es `0`, `1`, `2` y NULL. `id - null` es NULL en las cuatro filas. `reading - 0` es `1.5`, NULL, `0.0` y `-0.0`. `flags - 1` es `9`, NULL, `-1` y `41`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — suma

`value + other` suma una constante a una columna y conserva el tipo numérico de la columna. Un nulo hace nulo el resultado. `-0.0 + 0` es `0.0`. `id + 1` es `2`, `3`, NULL y `0`. En `days` es `2`, `3`, `4` y NULL. `id + null` es NULL en las cuatro filas. `reading + 0` es `1.5`, NULL, `0.0` y `0.0`. `flags + 1` es `11`, NULL, `1` y `43`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — menor o igual

`value <= other` compara una columna con una constante y devuelve un `BOOLEAN`. Un nulo es menor o igual que nulo y no es menor o igual que ningún otro valor. `-0.0` es menor o igual que `0`. `id <= 1` es `true`, `false`, `false` y `true`. En `days` es `true`, `false`, `false` y `false`. `id <= null` es `false`, `false`, `true` y `false`. `reading <= 0` es `false`, `false`, `true` y `true`. `note <= 'alpha'` es `true`, `false`, `true` y `false`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — mayor o igual

`value >= other` compara una columna con una constante y devuelve un `BOOLEAN`. Un nulo es mayor o igual que nulo y no es mayor o igual que ningún otro valor. `-0.0` es mayor o igual que `0`. `id >= 1` es `true`, `true`, `false` y `false`. En `days` es `true`, `true`, `true` y `false`. `id >= null` es `false`, `false`, `true` y `false`. `reading >= 0` es `true`, `false`, `true` y `true`. `note >= 'alpha'` es `true`, `false`, `false` y `true`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — mayor que

`value > other` compara una columna con una constante y devuelve un `BOOLEAN`. Un nulo no es mayor que nada, ni siquiera que nulo. `-0.0` no es mayor que `0`. `id > 1` es `false`, `true`, `false` y `false`. En `days` es `false`, `true`, `true` y `false`. `id > null` es `false` en las cuatro filas. `reading > 0` es `true`, `false`, `false` y `false`. `note > 'alpha'` es `false`, `false`, `false` y `true`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — menor que

`value < other` compara una columna con una constante y devuelve un `BOOLEAN`. Un nulo no es menor que nada, ni siquiera que nulo. `-0.0` no es menor que `0`. `id < 1` es `false`, `false`, `false` y `true`. En `days` es `false` en las cuatro filas. `id < null` y `reading < 0` son `false` en las cuatro filas. `note < 'alpha'` es `false`, `false`, `true` y `false`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — desigualdad

`value <> other` niega la igualdad y devuelve un `BOOLEAN`. Un nulo es distinto de cualquier otro valor y no es distinto de nulo. `-0.0` no es distinto de `0`. `id <> 1` es `false`, `true`, `true` y `true`. En `days` es `false`, `true`, `true` y `true`. `id <> null` es `true`, `true`, `false` y `true`. `note <> 'alpha'` es `false`, `true`, `true` y `true`. `note <> ''` es `true`, `true`, `false` y `true`. `reading <> 0` es `true`, `true`, `false` y `false`. `id != 1` es la misma comparación y conserva el operador escrito. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — igualdad

`value = other` compara una columna con una constante y devuelve un `BOOLEAN`. Un nulo es igual a nulo y distinto de cualquier otro valor. `-0.0` es igual a `0`. `id = 1` es `true`, `false`, `false` y `false`. En `days` es `true`, `false`, `false` y `false`. `id = null` es `false`, `false`, `true` y `false`. `note = 'alpha'` es `true`, `false`, `false` y `false`. `note = ''` es `false`, `false`, `true` y `false`. `reading = 0` es `false`, `false`, `true` y `true`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — in

`value in (lista)` dice si el valor es igual a alguna constante de la lista y devuelve un `BOOLEAN`. Un valor nulo devuelve `false`. `id in (1, -1)` es `true`, `false`, `false` y `true`. En `days` es `true`, `false`, `false` y `false`. `note in ('alpha', '')` es `true`, `false`, `true` y `false` en `sensors`, y `true`, `false`, `false` y `false` en `days`. `location in ('madrid', '東京')` es `true`, `false`, `true` y `true`. `reading in (1.5, 0)` es `true`, `false`, `true` y `true`, porque `-0.0` coincide con `0`. La lista vacía responde `too few arguments for 'in'`. Un `BOOLEAN` no tiene operador `in`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — between()

`between(value, low, high)` dice si un entero, una fecha o un timestamp cae en un intervalo cerrado y devuelve un `BOOLEAN`. El orden de los extremos no importa. Un valor nulo o un extremo nulo devuelve `false`. `between(id, 1, 2)` es `true`, `true`, `false` y `false`. `between(ts, ts, ts)` es `true` en las cuatro filas. `between(day, day, day)` es `true`, `false`, `true` y `true`. En `days`, `between(id, 1, 2)` es `true`, `true`, `false` y `false`. La llamada vacía responde `too few arguments for 'between' [found=0,expected=3]`. Un `DOUBLE` no tiene operador `between`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — wmid()

`wmid(bid_size, bid_price, ask_price, ask_size)` pondera el ask y el bid por el tamaño de cada lado y devuelve un `DOUBLE`. `wmid(flags, reading, score, id)` es `1.2727272727272727`, NULL, NULL y `-1.5365853658536586`. Un argumento nulo deja el resultado en NULL. En `days`, `wmid(id, id, id, id)` es `1.0`, `2.0`, `3.0` y NULL. La llamada vacía exige `wmid(DOUBLE, DOUBLE, DOUBLE, DOUBLE)`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — spread_bps()

`spread_bps(bid, ask)` divide `ask - bid` por el punto medio y lo expresa en puntos básicos, como `DOUBLE`. `spread_bps(reading, score)` es `-1818.1818181818182`, NULL, NULL y `20000.0`. El tercer NULL aparece porque el punto medio es cero. Un argumento nulo también deja el resultado en NULL. En `days`, `spread_bps(id, id)` es `0.0` en las tres filas con `id` y NULL en la última. La llamada vacía exige `spread_bps(DOUBLE, DOUBLE)`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — spread()

`spread(bid, ask)` devuelve `ask - bid` como `DOUBLE`. `spread(reading, score)` es `-0.25`, NULL, `0.0` y `-1.5`. Un argumento nulo deja el resultado en NULL. `spread(flags, id)` con `id = 1` es `-9.0`. En `days`, `spread(id, id)` es `0.0` en las tres filas con `id` y NULL en la última. La llamada vacía exige `spread(DOUBLE, DOUBLE)`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — mid()

`mid(bid, ask)` devuelve la media de los dos números como `DOUBLE`. `mid(reading, score)` es `1.375`, NULL, `0.0` y `-0.75`. Un argumento nulo deja el resultado en NULL. `mid(flags, id)` con `id = 1` es `5.5`. En `days`, `mid(id, id)` es `1.0`, `2.0`, `3.0` y NULL. La llamada vacía exige `mid(DOUBLE, DOUBLE)`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — timestamp_sequence()

`timestamp_sequence(start, step)` devuelve un `TIMESTAMP` y avanza el paso en cada fila emitida. `timestamp_sequence(0, 1000000)` es el origen y los tres segundos siguientes. Un paso negativo retrocede. `timestamp_sequence(ts, 1000)` suma el índice de fila, en microsegundos, al timestamp de esa fila. Sin `ORDER BY`, `LIMIT -1` deja una fila y la secuencia vuelve a empezar. Con `ORDER BY id` el valor se asigna en el orden de lectura y viaja con la fila. La llamada vacía exige `timestamp_sequence(TIMESTAMP, LONG)`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — haversine_dist_deg()

`haversine_dist_deg(lat, lon, ts)` suma, en kilómetros, la distancia entre puntos consecutivos finitos y devuelve un `DOUBLE`. `haversine_dist_deg(reading, score, ts)` es `383.9025620024131`. Con una sola fila, o sin filas, el resultado es `0.0`. En `days`, `haversine_dist_deg(id, id, ts)` es `314.4073265716869`. La llamada vacía exige `haversine_dist_deg(DOUBLE, DOUBLE, TIMESTAMP)`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — bar()

`bar(value, min, max, width)` dibuja el valor entre el mínimo y el máximo con bloques y devuelve `VARCHAR`. `bar(flags, 0, 42, 4)` es `▉`, NULL, cadena vacía y `████`. `bar(reading, 0, 2, 4)` con `id = 1` es `███`. Un ancho no positivo, un mínimo que no es menor que el máximo, o un valor nulo devuelven NULL. La llamada vacía exige `bar(DOUBLE, DOUBLE, DOUBLE, INT)`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — switch()

`switch(key, case, result, ..., [else])` devuelve el resultado del caso constante que iguala la clave. Si el número de argumentos es par, el último es la rama else. `switch(id, 1, 'one', 'other')` devuelve `one`, `other`, `other`, `other`. Sin else, las filas que no coinciden son NULL. `switch(note, 'alpha', 7, 0)` es el `INT` `7` en la primera fila. Un caso que no es constante dice `constant expected`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — like()

`like(text, pattern)` distingue mayúsculas y devuelve `BOOLEAN`. La cabecera es `note like 'a%'`. Esa consulta es `true` en la fila `id = 1` y `false` cuando `note` es NULL. `like(note, 'A%')` con `id = 1` es `false`. Un patrón vacío o NULL devuelve `false`. Un patrón que no es constante dice `use constant or bind variable`. La llamada vacía dice `too few arguments for 'like'`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — ilike()

`ilike(text, pattern)` es un `LIKE` sin distinguir mayúsculas y devuelve `BOOLEAN`. `ilike(note, 'A%')` es `true` en la fila `id = 1` y `false` cuando `note` es NULL. Un patrón vacío o NULL devuelve `false`. Un patrón que no es constante dice `use constant or bind variable`. La llamada vacía dice `too few arguments for 'ilike'`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — glob()

`glob(text, pattern)` compara el texto con un patrón constante y devuelve `BOOLEAN`. `glob(note, 'a*')` es `true` en la fila `id = 1` y `false` cuando `note` es NULL. Un patrón vacío o NULL se rechaza. Un patrón que no es constante no encuentra firma. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — quantile_disc()

`quantile_disc(value, percentile)` devuelve el mismo valor almacenado que `percentile_disc()`. `quantile_disc(flags, 0.5)` es el `LONG` `10` y `quantile_disc(reading, 0)` es el `DOUBLE` `-0.0`. Sin valores el resultado es NULL. La llamada vacía exige argumentos y no nombra una firma. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — quantile_cont()

`quantile_cont(value, percentile)` es la interpolación ya cubierta por `percentile_cont()`. `quantile_cont(flags, 0.25)` es el `DOUBLE` `5.0` y `quantile_cont(reading, 0)` es `-0.0`. Sin valores el resultado es NULL. La llamada vacía exige argumentos y no nombra una firma. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — skewness_samp()

`skewness_samp(value)` es la asimetría muestral ya cubierta por `skewness()`. `skewness_samp(flags)` es el `DOUBLE` `1.33609576072998`. Con menos de tres valores finitos, o si todos son iguales, el resultado es NULL. La llamada vacía exige `skewness_samp(DOUBLE)`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — percentile_cont()

`percentile_cont(value, percentile)` interpola entre dos valores ordenados y devuelve `DOUBLE`. `percentile_cont(flags, 0.5)` es `10.0` y `percentile_cont(flags, 0.25)` es `5.0`. Sin valores el resultado es NULL. Con valores, un percentil fuera de `[0, 1]` se rechaza. El percentil tiene que ser constante. `percentile_cont(day, 0.5)` imprime `1.5779232E12` en 10.1.0 y `percentile_cont(ts, 0.5)` imprime `1.5779342465E15`. Esas dos consultas no se capturan. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — percentile_disc()

`percentile_disc(value, percentile)` devuelve un valor almacenado. Enteros, fechas y timestamps salen como `LONG`. `FLOAT` y `DOUBLE` salen como `DOUBLE`. `percentile_disc(flags, 0.5)` es el `LONG` `10`. Sin valores el resultado es NULL, también si el percentil queda fuera de `[0, 1]`. Con valores, un percentil fuera de ese intervalo se rechaza. El percentil tiene que ser constante. Un texto se convierte a `DOUBLE` y un `CHAR` a `LONG`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — isOrdered()

`isOrdered(value)` devuelve `BOOLEAN`: cada valor convertido a `LONG` es mayor o igual que el anterior. Sin filas el resultado es `true`. Un `LONG` nulo cuenta como el mínimo de 64 bits y un `INT` nulo como el mínimo de 32 bits. Un texto nulo también cuenta como el mínimo. Un texto que no es entero se rechaza, igual que un `CHAR` que no es un dígito. `FLOAT`, `DOUBLE`, `BOOLEAN` y `SYMBOL` no tienen función. `isOrdered(ts)` es `true` e `isOrdered(id)` es `false`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — weighted_stddev_rel()

`weighted_stddev_rel(value, weight)` es el mismo cálculo de fiabilidad que `weighted_stddev` y devuelve `DOUBLE`. `weighted_stddev_rel(reading, flags)` es `1.060660171779821`. Una sola fila o un grupo sin dos pesos positivos devuelve NULL. Un peso negativo anula el agregado. `weighted_stddev_rel(day, flags)` sale en notación científica con `E` mayúscula y queda fuera del TSV capturado. `weighted_stddev_rel(reading, 1)` es `0.8660254037844386` en 10.1.0; este paso solo acepta una columna como peso. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — weighted_stddev_freq()

`weighted_stddev_freq(value, weight)` devuelve la desviación estándar ponderada por frecuencia como `DOUBLE`. Comparte la suma de `weighted_stddev` y divide por el peso total menos uno. `weighted_stddev_freq(reading, flags)` es `0.5969378875804073`. Una sola fila con peso mayor que uno es `0.0`. Sin filas el resultado es NULL. Un peso negativo anula el agregado. `weighted_stddev_freq(day, flags)` sale en notación científica con `E` mayúscula y queda fuera del TSV capturado. `weighted_stddev_freq(reading, 1)` es `0.8660254037844386` en 10.1.0; este paso solo acepta una columna como peso. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — weighted_stddev()

`weighted_stddev(value, weight)` devuelve la desviación estándar ponderada de fiabilidad como `DOUBLE`. Solo suma pesos positivos y finitos. Un peso negativo anula el agregado. `weighted_stddev(reading, flags)` es `1.060660171779821`. Una sola fila o un grupo sin dos pesos positivos devuelve NULL. `weighted_stddev(day, flags)` sale en notación científica con `E` mayúscula y queda fuera del TSV capturado. `weighted_stddev(reading, 1)` es `0.8660254037844386` en 10.1.0; este paso solo acepta una columna como peso. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — weighted_avg()

`weighted_avg(value, weight)` devuelve la media ponderada como `DOUBLE` y suma cualquier peso finito distinto de cero. `weighted_avg(reading, flags)` es `0.28846153846153844`. Una fila con `id = 1` es `1.5`. Sin filas el resultado es NULL. Un único `-0.0` se conserva. Si los pesos suman cero, el resultado es NULL. `weighted_avg(day, flags)`, `weighted_avg(ts, flags)` y `weighted_avg(reading, code)` salen en notación científica con `E` mayúscula y quedan fuera del TSV capturado. `weighted_avg(reading, 1)` es `0.5` en 10.1.0; este paso solo acepta una columna como peso. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — sparkline()

`sparkline(col)` dibuja la serie numérica como `VARCHAR` con los bloques `▁▂▃▄▅▆▇█`. Los nulos se omiten y un solo valor finito es `█`. `sparkline(flags)` es `▂▁█`. Una fila con `id = 1` es `█`. Sin filas el resultado es NULL. `sparkline(value, min, max, width)` acepta constantes: un ancho menor que el número de valores promedia cubos, y un ancho mayor o igual pinta cada valor. `min` o `max` nulos se calculan con los datos. Un ancho nulo o menor que 1 se rechaza cuando hay valores. El tope de bytes del resultado no entra en este paso. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — twap()

`twap(value, ts)` devuelve la media ponderada por el tiempo como `DOUBLE`. Cada valor se mantiene hasta la marca siguiente y el segundo argumento tiene que ser el timestamp designado. `twap(reading, ts)` es `1.0`. Una fila con `id = 1` es `1.5`. Sin filas el resultado es NULL. Si todas las marcas coinciden, el resultado es la media aritmética. `twap(day, ts)` y `twap(ts, ts)` salen en notación científica con `E` mayúscula y quedan fuera del TSV capturado. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — vwap()

`vwap(value, weight)` devuelve la media ponderada como `DOUBLE` y solo suma pesos positivos y finitos. `vwap(reading, flags)` es `0.28846153846153844`. Una fila con `id = 1` es `1.5`. Sin filas el resultado es NULL. Un único `-0.0` se conserva. `vwap(day, flags)` y `vwap(ts, flags)` salen en notación científica con `E` mayúscula y quedan fuera del TSV capturado. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — skewness_pop()

`skewness_pop(col)` devuelve la asimetría poblacional como `DOUBLE`. `skewness_pop(flags)` es `0.5454588102140288`. Una sola fila o un grupo sin valores finitos devuelve NULL. Si todos los valores finitos son iguales, el resultado también es NULL. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — kurtosis_pop()

`kurtosis_pop(col)` devuelve la curtosis poblacional en exceso como `DOUBLE`. `kurtosis_pop(flags)` es `-1.5000000000000002`. Una sola fila o un grupo sin valores finitos devuelve NULL. Si todos los valores finitos son iguales, el resultado también es NULL. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — string_distinct_agg()

`string_distinct_agg(col, sep)` une la primera aparición de cada texto con un separador de un carácter. Un nulo no aporta y una cadena vacía sí, así que `string_distinct_agg(note, ',')` es `alpha,,βeta`. Un valor repetido se queda una vez: `string_distinct_agg(location, ',')` es `madrid,東京`. El resultado es `STRING`, o `VARCHAR` si la columna lo es. Sin valores es NULL. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — arg_min()

`arg_min(value, key)` devuelve el valor de la fila con la clave menor y omite las claves nulas. Un empate se queda con la primera fila. `arg_min(reading, flags)` es el `DOUBLE` `0.0`. Una fila con `id = 1` es `1.5`. Sin filas el resultado es NULL. No hay sobrecarga de texto: un `STRING` se convierte a `DOUBLE` y un `VARCHAR` a `CHAR`. Un `SYMBOL` sale como el primer carácter, así que `arg_min(location, flags)` es el `CHAR` `m`. El resto de tipos sigue la misma promoción que `arg_max`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — arg_max()

`arg_max(value, key)` devuelve el valor de la fila con la clave mayor y omite las claves nulas. Un empate se queda con la primera fila. `arg_max(note, flags)` es el `VARCHAR` `βeta`. Una fila con `id = 1` es `alpha`. Sin filas el resultado es NULL. `STRING`, `SYMBOL` y `VARCHAR` salen como `VARCHAR`. `CHAR` se queda en `CHAR`. Una fecha pasa a `TIMESTAMP`. `LONG` con clave `INT` sigue en `LONG`, y con clave `LONG` pasa a `DOUBLE`. `BYTE`, `SHORT`, `INT`, `FLOAT` y `DOUBLE` con clave `INT` salen como `DOUBLE`. Un texto que no es número se rechaza cuando la clave no encaja. `BOOLEAN` no tiene función. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — approx_percentile()

`approx_percentile(col, percentile)` estima un percentil entre `0.0` y `1.0` y devuelve `DOUBLE`. La precisión por defecto es 1 y el tercer argumento la cambia entre 0 y 5. `approx_percentile(flags, 0.5)` es `10.0`. El percentil `0` de `flags` es `0.0` y el `1` es `43.0`. Un `42` suelto en el percentil `0.5` sale como `43.0`, con precisión 0 como `63.0` y con precisión 3 como `42.0`. `approx_percentile(id, 0.5)` en `days` es `2.0`, el percentil `0` es `1.0` y el `1` es `3.0`. El percentil tiene que ser constante. Un entero negativo se rechaza. Sin filas el resultado es NULL. `BOOLEAN` y `SYMBOL` no encajan. El histograma de coma flotante de QuestDB se aproxima aquí con la muestra ordenada de este fixture. `approx_percentile(score, 0.5)` en la columna entera imprime en 10.1.0 el rango cubierto `[1.0, 2048.0)` y aquí solo `Negative values cannot be recorded`. Esa consulta no se captura. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — approx_median()

`approx_median(col)` estima la mediana con un histograma y devuelve `DOUBLE`. La precisión por defecto es 1. `approx_median(flags)` es `10.0`. Un valor suelto de `42` sale como `43.0` y con precisión 0 como `63.0`. Precisión 3 deja `42.0`. `approx_median(reading)` es `0.0` y una fila con `id = 1` de `score` es `1.25`. `approx_median(id)` en `days` es `2.0`. Un entero negativo se rechaza. Sin filas el resultado es NULL. `BOOLEAN` y `SYMBOL` no encajan. El histograma de coma flotante de QuestDB se aproxima aquí con la muestra ordenada de este fixture. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — mode()

`mode(col)` devuelve el valor más frecuente y omite los nulos. `mode(location)` es `madrid`. `INT`, `BYTE`, `SHORT`, una fecha y un timestamp pasan a `LONG`. `FLOAT` pasa a `DOUBLE` y `CHAR` a `STRING`. Una fila con `id = 1` de `flags` es el `LONG` `10`. Sin filas el resultado es NULL. En un empate esta implementación se queda con el primer valor. QuestDB recorre su mapa hash: `mode(flags)` es `0` en 10.1.0 y `mode(note)` es la cadena vacía. Esas consultas no se capturan. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — skewness()

`skewness(col)` devuelve la asimetría muestral de Fisher-Pearson como `DOUBLE`. El estado es la media y los momentos centrales de orden 2 y 3. Con menos de tres valores finitos, o si la suma de cuadrados es cero, el resultado es NULL. `skewness(flags)` es `1.33609576072998`. `skewness(ts)` es `0.0` y `skewness(id)` en `days` es `0.0`. `BOOLEAN` y `SYMBOL` no encajan. Un texto que no es número se rechaza en la fila. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — geomean()

`geomean(col)` devuelve `exp` de la media de los logaritmos naturales, como `DOUBLE`. Los nulos se omiten. Un cero o un negativo anula el grupo. `geomean(reading)` con `id = 1` es `1.5` y la columna entera es NULL por el `-0.0`. `geomean(id)` en `days` es `1.8171205928321397`. Sin filas el resultado es NULL. `BOOLEAN` y `SYMBOL` no encajan. Un texto que no es número se rechaza en la fila. `geomean(code)` con `code > 0` imprime `2.6457513110645907` en 10.1.0 y `2.6457513110645903` en Rust. `geomean(ts)` imprime `1.5779342464999918E15`. Esas dos consultas no se capturan. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — bit_xor()

`bit_xor(col)` devuelve el XOR bit a bit con los mismos tipos que `bit_and`. `bit_xor(flags)` es `32`, `bit_xor(id)` es `-4` y `bit_xor(b)` es `-126`. `bit_xor(id)` en `days` es `0`. Sin filas, un `INT` o un `LONG` es NULL; un `BYTE`, un `SHORT` o un `CHAR` es `0`. `FLOAT`, `DOUBLE` y `BOOLEAN` no tienen función. Un texto que no es entero se rechaza en la fila. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — bit_or()

`bit_or(col)` devuelve el OR bit a bit con los mismos tipos que `bit_and`. `bit_or(flags)` es `42`, `bit_or(id)` es `-1` y `bit_or(b)` es `-125`. `bit_or(id)` en `days` es `3`. Sin filas, un `INT` o un `LONG` es NULL; un `BYTE`, un `SHORT` o un `CHAR` es `0`. `FLOAT`, `DOUBLE` y `BOOLEAN` no tienen función. Un texto que no es entero se rechaza en la fila. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — bit_and()

`bit_and(col)` devuelve el AND bit a bit y omite los nulos. `BYTE`, `SHORT` e `INT` conservan el tipo. `LONG`, `DATE` y `TIMESTAMP` pasan a `LONG`. Un `CHAR` se lee como `SHORT` y un `SYMBOL` como la clave `INT` de su orden de aparición. `bit_and(flags)` es `0` y la fila con `id = 1` es `10`. `bit_and(day)` es `0` y `bit_and(ts)` es `1577934242721792`. Sin filas, un `INT` o un `LONG` es NULL; un `BYTE`, un `SHORT` o un `CHAR` es `0`. `FLOAT`, `DOUBLE` y `BOOLEAN` no tienen función. `alpha` se rechaza como `STRING` hacia `LONG` y `A` como `CHAR` hacia `SHORT`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — last_not_null()

`last_not_null(col)` devuelve el último valor no nulo en el orden de las filas y conserva el tipo. `BYTE` y `SHORT` se ensanchan a `INT`: `last_not_null(b)` es `1` y `last_not_null(code)` es `1`. `last_not_null(note)` es `βeta`. `last_not_null(reading)` es `-0.0`. La fila con `id` nulo tiene la nota vacía, y esa cadena cuenta. Sin filas el resultado es NULL. `BOOLEAN` no tiene función. Un segundo argumento se rechaza con la lista de tipos: `(STRING, INT)` para `last_not_null(note, 1)`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — first_not_null()

`first_not_null(col)` devuelve el primer valor no nulo en el orden de las filas y conserva el tipo. `BYTE` y `SHORT` se ensanchan a `INT`: `first_not_null(b)` es `3` y `first_not_null(code)` es `7`. `first_not_null(note)` es `alpha`. La fila con `id` nulo tiene la nota vacía, y esa cadena cuenta. Si el único `id` es nulo, el resultado es NULL. Sin filas también es NULL. `BOOLEAN` no tiene función. Un segundo argumento se rechaza con la lista de tipos: `(STRING, INT)` para `first_not_null(note, 1)`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — bool_or()

`bool_or(col)` devuelve el OR de una columna `BOOLEAN`. Sin filas el resultado es `false`. `bool_or(ok)` es `true`. La fila con `id = 1` es `true` y la de `id = -1` es `false`. El grupo `madrid` es `true`; el grupo NULL y el de `東京` son `false`. `LONG`, `DOUBLE`, `STRING` y `SYMBOL` no encajan. `days` no tiene columna `BOOLEAN`, así que `bool_or(id)` se rechaza. Se contrastó con `/exec` en `sensors`.

## 2026-11-07 — bool_and()

`bool_and(col)` devuelve el AND de una columna `BOOLEAN`. Sin filas el resultado es `false`, porque un booleano de QuestDB no representa NULL. `bool_and(ok)` es `false`. La fila con `id = 1` es `true` y la de `id = -1` es `false`. El grupo `madrid` es `true`; el grupo NULL y el de `東京` son `false`. `LONG`, `DOUBLE`, `STRING` y `SYMBOL` no encajan. `days` no tiene columna `BOOLEAN`, así que `bool_and(id)` se rechaza. Se contrastó con `/exec` en `sensors`.

## 2026-11-07 — var_samp()

`var_samp(col)` es la varianza muestral, la misma cuenta que `variance`: la suma de cuadrados dividida por el número de valores menos uno. Con menos de dos valores finitos el resultado es NULL. `var_samp(reading)` es `0.75`. `BOOLEAN` y `SYMBOL` no encajan. Un texto que no es número se rechaza en la fila. `var_samp(code)` imprime `2.68479158E8` en 10.1.0 y `268479158.0` en Rust. `var_samp(ts)` imprime `1.6666666666666667E12` y `1666666666666.6667`. `var_samp(day)` imprime `8.3753734112748E23` y `8.3753734112748e+23`. Esas tres consultas no se capturan. Se contrastó el resto con `/exec` en `sensors` y `days`.

## 2026-11-07 — stddev_samp()

`stddev_samp(col)` es la desviación típica muestral, la misma cuenta que `stddev`: la raíz de la suma de cuadrados dividida por el número de valores menos uno. Con menos de dos valores finitos el resultado es NULL. `stddev_samp(reading)` es `0.8660254037844386`. `BOOLEAN` y `SYMBOL` no encajan. Un texto que no es número se rechaza en la fila. `stddev_samp(day)` imprime `9.151706622960988E11` en 10.1.0 y `915170662296.0988` en Rust y no se captura. Se contrastó el resto con `/exec` en `sensors` y `days`.

## 2026-11-07 — regr_r2()

`regr_r2(y, x)` usa las mismas sumas y devuelve el cuadrado de la suma cruzada partido por el producto de las dos sumas de cuadrados. Sin parejas, o si la suma de cuadrados de `x` es cero, el resultado es NULL. Si `x` varía y `y` es constante, el resultado es `1.0`. `regr_r2(reading, score)` es `0.7032967032967034`. `BOOLEAN` y `SYMBOL` no encajan. Un texto que no es número se rechaza en la fila. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — regr_intercept()

`regr_intercept(y, x)` usa las mismas sumas que `regr_slope` y devuelve `media(y) - media(x) * pendiente`. Sin parejas, o si la suma de cuadrados de `x` es cero, el resultado es NULL. `regr_intercept(reading, score)` es `0.543956043956044`. `BOOLEAN` y `SYMBOL` no encajan. Un texto que no es número se rechaza en la fila. `regr_intercept(ts, id)` imprime `1.5779342468571428E15` en 10.1.0 y `1577934246857142.8` en Rust. `regr_intercept(day, id)` imprime `1.5850737E12` y `1585073700000.0`. Esas dos consultas no se capturan. Se contrastó el resto con `/exec` en `sensors` y `days`.

## 2026-11-07 — regr_slope()

`regr_slope(y, x)` usa las mismas sumas que `corr` y divide la suma de productos cruzados por la suma de cuadrados de `x`. Sin parejas, o si esa suma es cero, el resultado es NULL. `regr_slope(reading, score)` es `0.5274725274725275`. `BOOLEAN` y `SYMBOL` no encajan. Un texto que no es número se rechaza en la fila. `regr_slope(day, id)` imprime `-7.1505E9` en 10.1.0 y `-7150500000.0` en Rust. `regr_slope(flags, code)` imprime `7.932260878679345E-4` y `0.0007932260878679345`. `regr_slope(id, ts)` en `days` imprime `2.085448153480154E-11` y `2.085448153480154e-11`. El grupo `madrid` imprime `2.6541039081680048E-11` y `2.6541039081680048e-11`. Esas consultas no se capturan. Se contrastó el resto con `/exec` en `sensors`.

## 2026-11-07 — corr()

`corr(y, x)` devuelve la correlación de Pearson como `DOUBLE`: la suma de productos cruzados dividida por la raíz del producto de las dos sumas de cuadrados. Con menos de dos parejas, o con denominador cero, el resultado es NULL. El cociente se recorta a `[-1, 1]`. `corr(reading, score)` es `0.8386278693775346`. `BOOLEAN` y `SYMBOL` no encajan. Un texto que no es número se rechaza en la fila. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — covar_samp()

`covar_samp(y, x)` usa la misma suma de productos cruzados que `covar_pop` y la divide por el número de parejas menos una. Con menos de dos parejas el resultado es NULL. `covar_samp(reading, score)` es `1.0`. `BOOLEAN` y `SYMBOL` no encajan. Un texto que no es número se rechaza en la fila. `covar_samp(day, id)` imprime `-1.4301E10` en 10.1.0 y `-14301000000.0` en Rust. `covar_samp(id, ts)` en `days` imprime `3.76774999999375E10` y `37677499999.9375`. El grupo `madrid` imprime `7.5355E10` y `75355000000.0`. Esas consultas no se capturan. Se contrastó el resto con `/exec` en `sensors`.

## 2026-11-07 — covar_pop()

`covar_pop(y, x)` devuelve la covarianza poblacional como `DOUBLE`: la suma de productos cruzados dividida por el número de parejas finitas. Una sola pareja es `0.0`. Sin parejas es NULL. Un nulo en cualquiera de las dos columnas omite la pareja. `covar_pop(reading, score)` es `0.6666666666666666`. `BOOLEAN` y `SYMBOL` no encajan. Un texto que no es número se rechaza en la fila. `covar_pop(day, id)` imprime `-7.1505E9` en 10.1.0 y `-7150500000.0` en Rust. `covar_pop(id, ts)` en `days` imprime `2.5118333333291668E10` y `25118333333.291668`. El grupo `madrid` imprime `3.76775E10` y `37677500000.0`. Esas consultas no se capturan. Se contrastó el resto con `/exec` en `sensors`.

## 2026-11-07 — string_agg()

`string_agg(col, sep)` une los textos en orden de fila con un separador de un carácter. Un nulo no aporta. Una cadena vacía sí, así que `string_agg(note, ',')` es `alpha,,βeta`. Un `CHAR` 0 no cuenta: `string_agg(ch, ',')` es `A,β,東`. El resultado es `STRING`, o `VARCHAR` si la columna lo es. Sin valores es NULL. El separador vacío y el nulo insertan el carácter 0. Un separador de dos caracteres no tiene función. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — approx_count_distinct()

`approx_count_distinct(col)` devuelve un `LONG`. En `sensors` y `days` la cuenta coincide con los valores distintos no nulos: `id` es `3`, una fila es `1` y ninguna es `0`. Un `BYTE` o `SHORT` 0 cuenta. `DOUBLE`, `FLOAT` y `BOOLEAN` no tienen función. Un `STRING` o `VARCHAR` se lee como `LONG` y un `CHAR` como `INT`; si no es un entero, la fila se rechaza. `SYMBOL`, `DATE` y `TIMESTAMP` cuentan sus valores. La precisión opcional tiene que estar entre 4 y 18. En este fixture `4` y `14` devuelven la misma cuenta. QuestDB usa HyperLogLog++; aquí la cardinalidad pequeña se cuenta exactamente y el estimador denso no está portado. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — nsum()

`nsum(col)` devuelve la suma de Neumaier como `DOUBLE`. Cada valor finito actualiza la suma y la compensación, y el resultado es la suma de las dos. Un único `-0.0` se imprime `0.0`. Sin valores es NULL. Un nulo no cuenta. `nsum(reading)` es `1.5` y `nsum(score)` es `-0.25`. `BOOLEAN` y `SYMBOL` no encajan. Un texto que no es número se rechaza en la fila. `nsum(ts)` imprime `6.311736986E15` en 10.1.0 y `6311736986000000.0` en Rust. `nsum(day)` imprime `3.1701474E12` y `3170147400000.0`. Esas dos consultas no se capturan. Se contrastó el resto con `/exec` en `sensors` y `days`.

## 2026-11-07 — ksum()

`ksum(col)` devuelve la suma de Kahan como `DOUBLE`. El primer valor finito se guarda tal cual, así que un único `-0.0` sigue siendo `-0.0`. Los siguientes se compensan. Sin valores es NULL. Un nulo no cuenta. `ksum(reading)` es `1.5` y `ksum(score)` es `-0.25`. `BOOLEAN` y `SYMBOL` no encajan. Un texto que no es número se rechaza en la fila. `ksum(ts)` imprime `6.311736986E15` en 10.1.0 y `6311736986000000.0` en Rust. `ksum(day)` imprime `3.1701474E12` y `3170147400000.0`. Esas dos consultas no se capturan. Se contrastó el resto con `/exec` en `sensors` y `days`.

## 2026-11-07 — stddev_pop()

`stddev_pop(col)` devuelve la desviación típica poblacional como `DOUBLE`: la raíz cuadrada de `var_pop`. Con un solo valor el resultado es `0.0`. Sin valores es NULL. Un nulo no cuenta. `stddev_pop(reading)` es `0.7071067811865476`. `BOOLEAN` y `SYMBOL` no encajan. Un texto que no es número se rechaza en la fila. `stddev_pop(day)` imprime `7.472337167301272E11` en 10.1.0 y `747233716730.1272` en Rust. Esa consulta no se captura. Se contrastó el resto con `/exec` en `sensors` y `days`.

## 2026-11-07 — var_pop()

`var_pop(col)` devuelve la varianza poblacional como `DOUBLE`: la misma suma de cuadrados que `variance`, dividida por el número de valores finitos. Con un solo valor el resultado es `0.0`. Sin valores es NULL. Un nulo no cuenta. `var_pop(reading)` es `0.5`. `BOOLEAN` y `SYMBOL` no encajan. Un texto que no es número se rechaza en la fila. `var_pop(code)` imprime `2.013593685E8` en 10.1.0 y `201359368.5` en Rust. `var_pop(ts)` imprime `1.25E12` y `1250000000000.0`. `var_pop(day)` imprime `5.5835822741832004E23` y `5.5835822741832004e+23`. Esas tres consultas no se capturan. Se contrastó el resto con `/exec` en `sensors` y `days`.

## 2026-11-07 — variance()

`variance(col)` devuelve la varianza muestral como `DOUBLE`, el cuadrado de `stddev` antes de la raíz. Hace falta más de un valor finito; con cero o uno el resultado es NULL. Un nulo no cuenta. `variance(reading)` es `0.75`. `BOOLEAN` y `SYMBOL` no encajan. Un texto que no es número se rechaza en la fila. `variance(code)` imprime `2.68479158E8` en 10.1.0 y `268479158.0` en Rust. `variance(ts)` imprime `1.6666666666666667E12` y `1666666666666.6667`. `variance(day)` imprime `8.3753734112748E23` y `8.3753734112748e+23`. Esas tres consultas no se capturan. Se contrastó el resto con `/exec` en `sensors` y `days`.

## 2026-11-07 — stddev()

`stddev(col)` devuelve la desviación típica muestral como `DOUBLE`. Hace falta más de un valor finito; con cero o uno el resultado es NULL. Un nulo no cuenta. `stddev(reading)` es `0.8660254037844386`. `BOOLEAN` y `SYMBOL` no encajan. Un texto que no es número se rechaza en la fila. `stddev(day)` imprime `9.151706622960988E11` en 10.1.0 y `915170662296.0988` en Rust, así que esa consulta no se captura. Se contrastó el resto con `/exec` en `sensors` y `days`.

## 2026-11-07 — not()

`not` devuelve `BOOLEAN` y niega el último argumento. Los anteriores no se evalúan: `not(1, ok)` es `not(ok)` y `not(missing, ok)` también. `not(ok)` invierte la columna. `not(null)` es `true`. El encabezado se escribe `not` en minúsculas y el último argumento. Cero argumentos responden `too few arguments for 'not' [found=0,expected=1]`. Si el último argumento no es `BOOLEAN`, el error es `argument type mismatch` en `#1` con su tipo. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — typeof()

`typeof(valor)` devuelve el nombre del tipo. El resultado es `STRING`, salvo si el argumento es una columna `VARCHAR`: entonces el resultado es `VARCHAR` y el texto sigue siendo `VARCHAR`. Un literal nulo imprime `NULL`. Un nulo guardado en una columna conserva el tipo de la columna. Un entero que cabe en `INT` es `INT`, excepto `-2147483648`, que es `LONG`, igual que `2147483648`. Un texto de un carácter es `CHAR`; la cadena vacía y un texto más largo son `STRING`. Cero argumentos o más de uno responden `exactly one argument expected`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — cot()

`cot(número)` devuelve `DOUBLE`. Es el recíproco de la tangente; si el resultado no es finito se imprime NULL. `cot(1)` es `0.6420926159343306`. `cot(0)` y `cot(-0.0)` se imprimen NULL. Un nulo sigue NULL. Un texto se convierte como en `sin`. `BOOLEAN` y `SYMBOL` no encajan. `cot(1.5707963267948966)` imprime `6.123233995736766E-17` en 10.1.0 y `6.123233995736766e-17` en Rust, así que esa consulta no se captura. Se contrastó el resto con `/exec` en `sensors` y `days`.

## 2026-11-07 — radians()

`radians(número)` convierte grados a radianes y devuelve `DOUBLE`. Un nulo sigue NULL. `radians(1)` es `0.017453292519943295`, `radians(180)` es π y `radians(-0.0)` es `-0.0`. Un texto se convierte como en `sin`. `BOOLEAN` y `SYMBOL` no encajan. `radians` del timestamp `2020-01-02T03:04:05.000000Z` imprime `2.754014795522087E13` en 10.1.0 y `27540147955220.87` en Rust. `radians` de la fecha `2020-01-02` imprime `2.753995518360499E10` y `27539955183.60499`. Esas consultas no se capturan. Se contrastó el resto con `/exec` en `sensors` y `days`.

## 2026-11-07 — degrees()

`degrees(número)` convierte radianes a grados y devuelve `DOUBLE`. Un nulo sigue NULL. `degrees(1)` es `57.29577951308232` y `degrees(-0.0)` es `-0.0`. Un texto se convierte como en `sin`. `BOOLEAN` y `SYMBOL` no encajan. `degrees` del timestamp `2020-01-02T03:04:05.000000Z` imprime `9.040897258766202E16` en 10.1.0 y `9.040897258766202e+16` en Rust. `degrees` de la fecha `2020-01-02` imprime `9.04083397557773E13` y `90408339755777.3`. Esas consultas no se capturan. Se contrastó el resto con `/exec` en `sensors` y `days`.

## 2026-11-07 — atan2()

`atan2(y, x)` devuelve `DOUBLE` en radianes. Un nulo en cualquiera de los dos argumentos deja el resultado NULL. `atan2(0, 1)` es `0.0`, `atan2(-0.0, 1)` es `-0.0`, `atan2(0, -1)` es π y `atan2(-0.0, -0.0)` es -π. Si un argumento no admite conversión a `DOUBLE`, el error nombra el primer argumento cuyo tipo no es ya `DOUBLE`: un entero que cabe en `INT` se nombra `INT`, y `-2147483648` se nombra `LONG`. Un texto se convierte como en `sin`. `atan2(3, 7)` imprime `0.4048917862850834` en 10.1.0 y `0.40489178628508343` en Rust. `atan2(1.25, 1.5)` imprime `0.6947382761967033` y `0.6947382761967031`. Esas dos consultas no se capturan. Se contrastó el resto con `/exec` en `sensors` y `days`.

## 2026-11-07 — atan()

`atan(número)` devuelve `DOUBLE` en radianes. Un nulo sigue NULL. `atan(0)` es `0.0` y `atan(-0.0)` es `-0.0`. Un texto se convierte como en `sin`. `BOOLEAN` y `SYMBOL` no encajan. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — acos()

`acos(número)` devuelve `DOUBLE` en radianes. Un nulo sigue NULL. `acos(1)` es `0.0` y `acos(-0.0)` es el mismo valor que `acos(0)`. Un valor fuera de `[-1, 1]` se imprime como NULL. Un texto se convierte como en `sin`. `BOOLEAN` y `SYMBOL` no encajan. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — asin()

`asin(número)` devuelve `DOUBLE` en radianes. Un nulo sigue NULL. `asin(-0.0)` es `-0.0`. Un valor fuera de `[-1, 1]`, incluido un NaN, se imprime como NULL. Un texto se convierte como en `sin`. `BOOLEAN` y `SYMBOL` no encajan. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — tan()

`tan(número)` devuelve `DOUBLE` en radianes. Un nulo sigue NULL. `tan(-0.0)` es `-0.0`. Un texto se convierte como en `sin`. `BOOLEAN` y `SYMBOL` no encajan. `tan(7)` difiere en el último dígito del JDK (`0.8714479827243188` aquí, `0.8714479827243187` en 10.1.0), así que `tan(code)` no entra en el TSV. El resto de `sensors` y `days`, incluido `tan(ts)`, coincide. Se contrastó con `/exec`.

## 2026-11-07 — cos()

`cos(número)` devuelve `DOUBLE` en radianes. Un nulo sigue NULL. `cos(-0.0)` es `1.0`. Un texto se convierte como en `sin`. `BOOLEAN` y `SYMBOL` no encajan. En `2020-01-02T23:00:00.000000Z` el coseno del timestamp difiere en el último dígito del JDK (`-0.775358125427275` aquí, `-0.7753581254272751` en 10.1.0), así que esa fila no entra en el TSV. `cos(ts)` de la primera fila de `sensors` sí coincide. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — sin()

`sin(número)` devuelve `DOUBLE` en radianes. Un nulo sigue NULL. `-0.0` conserva el signo. Un texto se lee como `DOUBLE`: se recortan los espacios de los extremos, se admite el sufijo `d` y `NaN` o `Infinity` se imprimen como NULL. Un texto que no es número se rechaza en la fila evaluada. Un literal imposible se rechaza también con `LIMIT 0`. `BOOLEAN` y `SYMBOL` no encajan. Un timestamp o una fecha usan su cuenta en crudo. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — sha256()

`sha256(texto)` devuelve `VARCHAR` con el resumen en hexadecimal en minúsculas. El texto se lee como UTF-8. Un nulo sigue NULL. Una cadena vacía tiene resumen. Un número no tiene función. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — sha1()

`sha1(texto)` devuelve `VARCHAR` con el resumen en hexadecimal en minúsculas. El texto se lee como UTF-8. Un nulo sigue NULL. Una cadena vacía tiene resumen. Un número no tiene función. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — md5()

`md5(texto)` devuelve `VARCHAR` con el resumen en hexadecimal en minúsculas. El texto se lee como UTF-8. Un nulo sigue NULL. Una cadena vacía tiene resumen. Un número no tiene función. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — to_date()

`to_date(texto, patrón)` devuelve `DATE` en milisegundos. El patrón es una constante. Los campos que faltan salen en 1970-01-01 a medianoche. `yy` usa el siglo en curso. `U` se copia como letra. Un texto que no encaja sale NULL. Un patrón nulo se rechaza con `pattern is required`. Una columna como patrón no tiene función. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — extract()

`extract(parte FROM instante)` devuelve `INT`, y `LONG` si la parte es `epoch`. `dow` cuenta el domingo como 0. `isodow` cuenta el lunes como 1. `week` e `isoyear` siguen la semana ISO. `epoch` son segundos truncados hacia cero. `microseconds` y `milliseconds` son la fracción del segundo; `nanoseconds` es esa fracción por 1000. Un nulo sigue NULL. Una parte desconocida se rechaza. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — to_timestamp() con patrón

`to_timestamp(texto, patrón)` devuelve `TIMESTAMP`. El patrón es una constante. Los campos que faltan salen en 1970-01-01 a medianoche. `yy` suma el siglo en curso: en 2026, `20` es 2020 y `69` es 2069. `h a` lee la hora en 12 horas. Un texto que no encaja sale NULL. Un patrón nulo se rechaza con `pattern is required`. Una columna como patrón no tiene función. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — to_timestamp()

`to_timestamp(texto)` devuelve `TIMESTAMP`. El texto es un número de microsegundos: admite un signo menos, `_` entre dígitos y una `L` final. Un texto que no es ese número sale NULL. El mínimo de un `LONG` también sale NULL, porque es el centinela. Acepta `STRING`, `SYMBOL` y `CHAR`. Un número se rechaza. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — day_of_week_sunday_first()

`day_of_week_sunday_first(instante)` devuelve `INT`. El domingo es 1 y el sábado es 7. El 2 de enero de 2020 es jueves y vale 5. Un nulo sigue siendo nulo. Un `LONG` se lee como microsegundos. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — is_end_of_month()

`is_end_of_month(instante)` devuelve `BOOLEAN`. Es verdadero cuando el día del mes coincide con la longitud de ese mes: el 31 de enero, el 29 de febrero de 2020 y el 28 de febrero de 2019. El 28 de febrero de 2020 es falso. Un nulo devuelve `false`. Un `LONG` se lee como microsegundos. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — to_str()

`to_str(instante, patrón)` devuelve `STRING`. El patrón es una constante y elige la coincidencia más larga: año, mes, día, hora, fracción y nombres en inglés. `w` cuenta la semana como día del año dividido entre 7, más uno. `YYYY` es el año ISO. Las letras de zona imprimen `Z`. Una comilla del patrón se copia tal cual. Un timestamp nulo sigue NULL y un `LONG` se lee como microsegundos. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — days_in_month()

`days_in_month(instante)` devuelve `INT`. Febrero tiene 29 días en 2020 y 2000, y 28 en 2019 y 1900. Abril tiene 30. Un nulo sigue siendo nulo. Un `LONG` se lee como microsegundos. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — is_leap_year()

`is_leap_year(instante)` devuelve `BOOLEAN`. Usa la regla gregoriana: 2020 y 2000 son bisiestos, 1900 y 2019 no. Un nulo devuelve `false`. Un `LONG` se lee como microsegundos. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — timestamp_ceil()

`timestamp_ceil(unidad, instante)` devuelve `TIMESTAMP`. La unidad es una sola letra: `n`, `U`, `T`, `s`, `m`, `h`, `d`, `w`, `M` o `y`. Avanza siempre al siguiente límite, también cuando el instante ya cae en el límite. `n` y `U` se quedan en el microsegundo. Un `LONG` se lee como microsegundos. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — timestamp_floor()

`timestamp_floor(unidad, instante)` devuelve `TIMESTAMP`. La unidad es una letra: `n`, `U`, `T`, `s`, `m`, `h`, `d`, `w`, `M` o `y`. Un prefijo entero positivo marca el paso, como `2h` o `3d`. La semana cae en lunes y los pasos cuentan desde 1970. Un `LONG` se lee como microsegundos. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — date_trunc()

`date_trunc(unidad, instante)` devuelve `TIMESTAMP`. La unidad es una constante en minúsculas y admite el plural con `s`, más `centuries`. Recorta a nanosegundo, microsegundo, milisegundo, segundo, minuto, hora, día, semana (lunes), mes, trimestre, año, década, siglo y milenio. El siglo y el milenio empiezan en el año 01: 2020 cae en 2001. Un `LONG` se lee como microsegundos. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — week_of_year()

`week_of_year(instante)` devuelve `INT` con la semana ISO. El jueves decide el año: el 29 de diciembre de 2019 es la semana 52, el 31 de diciembre de 2020 es la 53 y el 1 de enero de 2021 sigue en la 53. En `sensors` y `days` los timestamps caen en la semana 1. Un `LONG` se lee como microsegundos y aquí también da 1, salvo el nulo. Una cadena con fecha UTC se acepta. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — day_of_week()

`day_of_week(instante)` devuelve `INT` con el día de la semana, lunes 1 y domingo 7. El 2 de enero de 2020 es jueves (4), el 5 es domingo (7) y el 6 es lunes (1). En `days`, ordenado por `ts`, los días son 4, 4, 5 y 5. Un `LONG` se lee como microsegundos: 1 y 2 caen en jueves, el nulo es NULL y -1 es miércoles. Una cadena con fecha UTC se acepta. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — micros()

`micros(instante)` devuelve `INT` con el microsegundo dentro del milisegundo, de 0 a 999. `000123` µs y `001123` µs dan 123. En `sensors` y `days` los timestamps caen en 0. Un `LONG` se lee como microsegundos: 1 y 2 se conservan, el nulo es NULL y -1 es 999. Una cadena con fecha UTC se acepta. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — millis()

`millis(instante)` devuelve `INT` con el milisegundo UTC, de 0 a 999, truncando los microsegundos. `123456` µs da 123 y `123999` µs también da 123. En `sensors` y `days` los timestamps caen en el milisegundo 0. Un `LONG` se lee como microsegundos: 1 y 2 son 0, el nulo es NULL y -1 es 999. Una cadena con fecha UTC se acepta. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — second()

`second(instante)` devuelve `INT` con el segundo UTC, de 0 a 59. En `sensors` los cuatro timestamps caen en los segundos 5, 6, 7 y 8. En `days`, ordenado por `ts`, los segundos son 5, 0, 0 y 0. Un `LONG` se lee como microsegundos: 1 y 2 son el segundo 0, el nulo es NULL y -1 es el segundo 59 del 31 de diciembre de 1969. Una cadena con fecha UTC se acepta. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — minute()

`minute(instante)` devuelve `INT` con el minuto UTC, de 0 a 59. En `sensors` el timestamp cae en el minuto 4. En `days`, ordenado por `ts`, los minutos son 4, 0, 0 y 30. Un `LONG` se lee como microsegundos: 1 y 2 son el minuto 0, el nulo es NULL y -1 es el minuto 59 del 31 de diciembre de 1969. Una cadena con fecha UTC se acepta. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — hour()

`hour(instante)` devuelve `INT` con la hora UTC, de 0 a 23. En `sensors` el timestamp cae a las 3. En `days`, ordenado por `ts`, las horas son 3, 23, 0 y 12. Un `LONG` se lee como microsegundos: 1 y 2 son la hora 0, el nulo es NULL y -1 es la hora 23 del 31 de diciembre de 1969. Una cadena con fecha UTC se acepta. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — day()

`day(instante)` devuelve `INT` con el día del mes. En `sensors` el timestamp cae el día 2. En `days`, ordenado por `ts`, los días son 2, 2, 3 y 3. Un `LONG` se lee como microsegundos: 1 y 2 son el día 1, el nulo es NULL y -1 es el día 31 de diciembre de 1969. Una cadena con fecha UTC se acepta. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — month()

`month(instante)` devuelve `INT` con el mes civil, de 1 a 12. Un timestamp de enero da 1. Un `LONG` se lee como microsegundos: 1 y 2 son enero, el nulo es NULL y -1 es diciembre de 1969. Una cadena con fecha UTC se acepta. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — year()

`year(instante)` devuelve `INT` con el año civil. Un timestamp de estas fixtures da 2020. Un `LONG` se lee como microsegundos: 1 y 2 son 1970, el nulo es NULL y -1 es 1969. Una cadena con fecha UTC se acepta. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — datediff()

`datediff(periodo, inicio, fin)` devuelve `LONG` con la diferencia absoluta. Acepta `n`, `u`, `T`, `s`, `m`, `h`, `d`, `w`, `M` y `y`. `H` no es hora aquí y deja la fila en NULL. Un `LONG` se lee como microsegundos. Una cadena con fecha UTC se acepta; un texto que no lo es da `Invalid date`. Si el instante final cae antes del mismo punto dentro del mes, el mes no cuenta entero. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — dateadd()

`dateadd(periodo, stride, timestamp)` devuelve `TIMESTAMP`. El periodo es un carácter: `n`, `u`, `T`, `s`, `m`, `h`/`H`, `d`, `w`, `M` y `y`. `M` suma meses de calendario y `m` suma minutos. El stride tiene que ser `INT`. Un literal nulo se rechaza con `` `null` is not a valid stride ``. Un nulo de columna deja esa fila en NULL. El día se recorta al añadir meses o años. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — cast()

`cast(columna AS tipo)` etiqueta la columna como `columna::tipo` y conserva la grafía del tipo. Cubre los tipos de las fixtures: enteros, `BOOLEAN`, `CHAR`, `FLOAT`, `DOUBLE`, `STRING`, `SYMBOL`, `VARCHAR`, `DATE` y `TIMESTAMP`. Un nulo de `LONG` truncado a `INT` sigue siendo NULL; truncado a `SHORT` o `BYTE` es 0. `-0.0` pasado a entero es 0 y pasado a `FLOAT` conserva el signo. Un texto que no es número queda en NULL. `cast(ch AS INT)` rompe la respuesta HTTP y no entra en el diferencial. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — nullif()

Exige dos argumentos del mismo tipo, con promoción numérica hacia `DOUBLE`, `LONG` o `INT`. Devuelve NULL cuando son iguales. `-0.0` y `0.0` son iguales, y dos nulos también. Un `CHAR` que coincide se guarda como el carácter 0, que se imprime vacío. Una cadena vacía es un valor: `nullif('', '')` es NULL y `nullif(note, '')` conserva el resto. Un booleano no tiene sobrecarga. `nullif(id, note)` rompe la respuesta HTTP; aquí se rechaza. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — coalesce()

Exige dos o más argumentos. Devuelve el primer valor no nulo. El tipo común sube a `DOUBLE`, `FLOAT`, `LONG` o `INT` cuando aparece ese tipo. `BYTE` con `SHORT` se queda en el tipo del primer argumento y no salta nulos; lo mismo ocurre con `CHAR` y `BOOLEAN`. Un literal de un solo carácter es `CHAR`. Una cadena vacía no es nula. `-0.0` no es nulo. Si todos los argumentos son el literal nulo, el error es `coalesce cannot be used with NULL data type`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — round_half_even()

Exige número y escala y devuelve `DOUBLE`. Un empate exacto va al dígito par: `1.5` escala 0 es `2.0` y `2.5` escala 0 es `2.0`. El camino de los no positivos niega el resultado, así que `round_half_even(0.0, 0)` es `-0.0`. La escala válida es la de `round`, no la de `round_down`: 16 queda fuera y deja la fila en NULL. Un texto rompe la respuesta HTTP; aquí se rechaza. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — round_up()

Exige número y escala y devuelve `DOUBLE`. Redondea alejándose de cero: `round_up(1.5, 0)` es `2.0` y `round_up(-1.5, 0)` es `-2.0`. Un entero exacto se queda igual. `-0.0` conserva el signo. La escala válida es el intervalo abierto `(-18, 18)`. Una escala constante fuera de rango, o nula, deja todas las filas en NULL. Un texto rompe la respuesta HTTP; aquí se rechaza. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — round_down()

Exige número y escala y devuelve `DOUBLE`. Trunca hacia cero: `round_down(1.5, 0)` es `1.0` y `round_down(-1.5, 0)` es `-1.0`. `-0.0` conserva el signo. La escala válida es el intervalo abierto `(-18, 18)`. Una escala constante fuera de rango, o nula, deja todas las filas en NULL. Una escala por columna fuera de rango deja esa fila en NULL. Un texto rompe la respuesta HTTP; aquí se rechaza como tipo no admitido. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — least()

Sigue el mismo ancho de tipo que `greatest`. El nulo entero es `LONG_NULL` y se omite; si no queda ningún valor, el resultado es NULL. En coma flotante el nulo se omite; si no queda ninguno, `DOUBLE` guarda infinito positivo y `FLOAT` se convierte en NULL. `-0.0` gana a `+0.0`. Boolean, texto, char y symbol se rechazan. Una columna ausente se rechaza antes que el tipo. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — greatest()

Admite uno o más números. El tipo es el más ancho: `DOUBLE`, `FLOAT`, `LONG`, `INT`, `SHORT`, `BYTE`. Un literal nulo no cambia el tipo; si todos los argumentos son nulos, el resultado es NULL. En enteros el nulo es el mínimo del tipo y pierde frente a cualquier valor real. En coma flotante el nulo se omite; si no queda ninguno, `DOUBLE` guarda infinito negativo y `FLOAT` se convierte en NULL. `+0.0` gana a `-0.0`. Boolean, texto, char y symbol se rechazan. Una columna ausente se rechaza antes que el tipo. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — pi()

`square` no es una función de 10.1.0. `pi` no admite argumentos y devuelve `Math.PI` como `DOUBLE` en cada fila. Una columna que no existe se rechaza antes que el conteo de argumentos. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — exp()

Usa `Math.exp` y devuelve `DOUBLE`. `exp(0)` y `exp(-0.0)` son `1.0`. `exp(1000)` desborda a infinito positivo: se imprime como NULL y se ordena por encima de los finitos. Un número de 17 dígitos o más, o menor que `0.0001`, se imprime en notación científica con `e+` o `e-`. Un texto rompe la respuesta HTTP; aquí se rechaza como `DOUBLE` esperado. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — log()

Es `Math.log10` y devuelve `DOUBLE`. Un negativo es NaN y sale NULL. `log(0)` y `log(-0.0)` son infinito negativo: se ordenan antes que cualquier finito y se imprimen como NULL. `log(10)` es `1.0`. Un texto rompe la respuesta HTTP; aquí se rechaza como `DOUBLE` esperado. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — ln()

Usa `Math.log` y devuelve `DOUBLE`. Un negativo es NaN y sale NULL, así que se ordena con los NULL. `ln(0)` y `ln(-0.0)` son infinito negativo: se ordenan antes que cualquier finito y `/exec` los imprime como NULL. Un texto rompe la respuesta HTTP; aquí se rechaza como `DOUBLE` esperado. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — power()

`pow` no es una función de 10.1.0; el nombre real es `power` y firma `power(DD)`. Usa `Math.pow`. Un infinito o un NaN sale NULL en `/exec`. `power(null, 0)` es `1.0` y `power(0, 0)` también. `power(-0.0, 1)` conserva `-0.0`. Un texto rompe la respuesta HTTP; aquí se rechaza como `DOUBLE` esperado. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — sqrt()

Devuelve `DOUBLE` para enteros y para `FLOAT`. `Math.sqrt` de un negativo es NaN, así que la fila sale NULL. `-0.0` sigue siendo `-0.0`. Un texto en 10.1.0 rompe la respuesta HTTP; aquí se rechaza como `DOUBLE` esperado. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — sign()

Conserva `BYTE`, `SHORT`, `INT`, `LONG`, `FLOAT` y `DOUBLE`. El mínimo de `BYTE` y de `SHORT` sale como `-1`. Un `DOUBLE` de `-0.0` pasa a `0.0`; un `FLOAT` de `-0.0` conservaría el signo. Un literal NULL se resuelve como `BYTE` y, como el nulo de un byte es `0`, el resultado es `0`. `signum` no existe en 10.1.0. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — ceil()

Usa los mismos tipos que `floor`. `ceiling` es el mismo cálculo y conserva el nombre escrito. `ceil(1.5)` es `2.0` y `ceil(-1.5)` es `-1.0`. `-0.0` sigue siendo `-0.0`. La forma `ceil(entero, escala)` es decimal y se rechaza. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — floor()

Devuelve `FLOAT` para `BYTE`, `SHORT`, `INT` y `FLOAT`. Devuelve `DOUBLE` para `LONG`, `DOUBLE` y un literal NULL. `floor(-1.5)` es `-2.0`. `-0.0` sigue siendo `-0.0`. Un entero que cabe en `INT` sale como `FLOAT`; uno mayor sale como `DOUBLE`. La forma `floor(entero, escala)` es decimal y se rechaza. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — round()

Devuelve `DOUBLE` para enteros y para `FLOAT`. Sin escala usa el `Math.round` de Java: `1.5` es `2.0` y `-1.5` es `-1.0`. Con escala positiva o negativa redondea la magnitud y restaura el signo, así que `1.25` con escala 1 es `1.3` y `-0.0` sigue siendo `-0.0`. Una escala constante fuera de `[-17, 15]`, o NULL, deja toda la columna NULL. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — abs()

Conserva `SHORT`, `INT`, `LONG` y `DOUBLE`. Un `BYTE` se ensancha a `SHORT`, así que `abs(-128)` es `128`. Un `FLOAT` se ensancha a `DOUBLE`. `-0.0` pasa a `0.0`. `abs` del mínimo de `SHORT` sigue siendo `-32768`, porque el cast de Java desborda. NULL queda NULL. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — size_pretty()

Acepta `BYTE`, `SHORT`, `INT` y `LONG`. Elige la unidad por el bit más alto y divide en `float` de 32 bits. El número lleva un decimal. `0` es `0.0 B` y `1024` es `1.0 KiB`. Un valor negativo cae en la banda de `EiB` y se imprime como `-0.0 EiB`. NULL queda NULL. Un texto en 10.1.0 rompe la respuesta HTTP; aquí se rechaza como `LONG` esperado. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — length_bytes()

Cuenta bytes UTF-8 y devuelve `INT`. Un texto NULL, incluido un `CHAR` 0, vale `-1`, no NULL. La cadena vacía vale `0`. Acepta `STRING`, `VARCHAR` y `CHAR`. Un `SYMBOL` en 10.1.0 responde un error interno del servidor; aquí se rechaza como `VARCHAR` esperado y `SYMBOL` recibido. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — quote_ident()

10.1.0 no tiene `repeat`. `quote_ident` devuelve `STRING`. Si cada unidad es letra, dígito ASCII, `_` o `$`, el texto no cambia, incluida la cadena vacía y las mayúsculas. Si no, va entre comillas dobles y cada `"` interno se duplica. NULL queda NULL. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — lpad() y rpad()

Aceptan dos o tres argumentos. El ancho puede ser una columna `BYTE`, `SHORT` o `INT`. Con dos argumentos el relleno es un espacio. Un relleno vacío o NULL, un texto NULL o un ancho negativo dejan NULL. Un ancho 0 deja cadena vacía. Si el texto es más largo, `lpad` conserva el principio y `rpad` el final. La longitud cuenta unidades UTF-16. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — to_lowercase() y to_uppercase()

Son los mismos `lower` y `upper`. El nombre escrito se conserva en la cabecera y en el error de tipo. `to_uppercase('β')` devuelve la beta griega mayúscula. Un literal de un carácter se informa como `CHAR`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — split_part()

`split_part(text, delimiter, index)` devuelve `STRING`. El índice es constante: 1 es el primer campo y uno negativo cuenta desde el final. `0` responde `field position must not be zero`. Un índice que no es constante responde que debe serlo. Un campo ausente es cadena vacía. Un separador vacío con índice positivo también. Con índice `-1` el separador vacío devuelve el texto entero. NULL en el texto, el separador o el índice deja NULL. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — strpos() y position()

10.1.0 no tiene `ends_with`. `strpos` y `position` son la misma función y devuelven `INT`. El índice es 1-based y cuenta unidades UTF-16. Si no hay coincidencia el resultado es `0`. Un NULL en el texto o en la búsqueda deja NULL. Una búsqueda vacía vale `1`, salvo que el texto sea NULL. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — starts_with()

`starts_with` devuelve `BOOLEAN`. Un NULL en el texto o en el prefijo es `false`. Un prefijo vacío coincide con cualquier texto no nulo, incluida la cadena vacía. Distingue mayúsculas. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — replace()

`replace(value, old, new)` devuelve `STRING` y sustituye coincidencias sin solaparse, contando unidades UTF-16. Un NULL en cualquiera de los tres argumentos deja NULL, salvo una búsqueda vacía constante, que devuelve el texto original, y un texto constante vacío, que devuelve cadena vacía. `replace('aaa', 'aa', 'b')` es `ba`. Un literal de un carácter se informa como `CHAR` en el error de tipos. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — ltrim() y rtrim()

`ltrim` y `rtrim` devuelven `STRING` y quitan el espacio ASCII ` ` solo por la izquierda o por la derecha. Un tabulador se conserva. Un NULL sigue siendo NULL. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — trim()

`trim` devuelve `STRING` y solo quita el carácter espacio ` ` de los dos extremos. Un tabulador, un salto de línea y un espacio de no separación se conservan. Un NULL de texto sigue siendo NULL y una cadena vacía sigue vacía. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — substring()

`substring(text, start, length)` devuelve `STRING`. `start` es 1-based y la longitud cuenta unidades UTF-16. Una longitud constante 0 devuelve cadena vacía incluso si el texto es NULL. Un inicio y una longitud constantes cuya suma es menor que 1 devuelven NULL en todas las filas. Un NULL en el texto, el inicio o la longitud deja NULL. Una longitud negativa se rechaza, también cuando llega en una columna. Un `LONG` no tiene función. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — left() y right()

`left` y `right` devuelven `STRING` y cuentan caracteres. Un entero positivo toma ese prefijo o sufijo. Cero devuelve cadena vacía. Un entero negativo se suma a la longitud y, si queda por debajo de cero, el resultado es vacío: `left('alpha', -1)` es `alph` y `right('ab', -1)` es `b`. Un NULL en el texto o en la cuenta deja NULL. Un símbolo usado como cuenta vale su clave de primera aparición. El nombre de un literal negativo es `-(n)`. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — upper()

`upper` devuelve `STRING`. Un NULL sigue siendo NULL y una cadena vacía sigue vacía. `β` pasa a `Β`, la beta mayúscula griega, no a la `B` latina. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-07 — lower()

`lower` devuelve `STRING`. Un NULL de texto, símbolo, varchar o char sigue siendo NULL; una cadena vacía sigue vacía. Las mayúsculas ASCII pasan a minúsculas y `β` no cambia. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-06 — length()

`length` cuenta caracteres Unicode y devuelve `INT`. Un NULL de texto, símbolo, varchar o char vale `-1`. Una cadena vacía vale 0. `βeta` vale 4 y `東京` vale 2. Un entero no tiene función. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-06 — concat()

`concat` acepta uno o más argumentos y devuelve `STRING`. Un NULL de texto, símbolo o varchar no aporta caracteres. Un NULL de entero o fecha se escribe `null`. Un float o double nulo se escribe `NaN`. Una fecha no nula se escribe como milisegundos, y un timestamp con el formato `YYYY-MM-DDThh:mm:ss.mmmuuuZ`. `-0.0` se conserva. El nombre de columna conserva las mayúsculas y los literales entre comillas. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-06 — count_distinct()

`count_distinct` cuenta valores distintos y devuelve `LONG`. NULL y un `CHAR` 0 no entran; una cadena vacía y un cero numérico sí. `-0.0` y `0.0` son distintos, así que `reading = 0.0` aporta 2. Sin valores el resultado es 0. Fuera de `GROUP BY` admite `FLOAT`, `DOUBLE` y `BOOLEAN`; dentro no, y el error es el de 10.1.0. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-06 — first() y last()

`first` guarda el valor de la primera fila leída y `last` el de la última, dentro del filtro o del grupo. Un NULL y un `CHAR` 0 cuentan: el `last(id)` de `madrid` en `sensors` es NULL, y el `last(id)` de `days` también. El tipo del resultado es el de la columna, así que un `BOOLEAN` sigue siendo `true` o `false`. En estas tablas sin O3 el orden de lectura coincide con el row id de 10.1.0. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-06 — avg()

`avg` divide la misma suma que `sum` entre el número de valores que entran, y el resultado es siempre `DOUBLE`. NULL y un `CHAR` 0 no entran. Si no queda ningún valor, la fila es NULL. Un `BOOLEAN` entra como `0.0` o `1.0`. Un `-0.0` solo se conserva; `0.0` y `-0.0` juntos dan `0.0`. El texto del TSV lo escribe `format_f64`, así que `avg(ts)` de `sensors` queda `1577934246500000.0`; el JSON de `/exec` imprime `1.5779342465E15` para el mismo número. Se contrastó con `/exec` en `sensors` y `days`, también dentro del `GROUP BY` de una columna.

## 2026-11-06 — sum()

`sum` omite NULL y un `CHAR` 0. Si no queda ningún valor, la fila es NULL. BYTE, SHORT, INT, LONG, DATE y TIMESTAMP se acumulan en `LONG`. `FLOAT` y `DOUBLE` conservan su tipo. `0.0 + -0.0` da `0.0`; un `-0.0` solo se conserva. Un símbolo suma su clave, que en estos fixtures es el orden de primera aparición. `sum` de `BOOLEAN` no tiene función, y un texto no convertible devuelve el error de 10.1.0. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-06 — min() y max()

`min` y `max` omiten NULL y un `CHAR` 0. Si no queda ningún valor, la fila es NULL. Un `BOOLEAN` se lee como `0.0` o `1.0`. `-0.0` y `0.0` empatan y se conserva el que ya estaba elegido, así que el mínimo de `reading` en `sensors` es `0.0` y el máximo del grupo `id = -1` sigue siendo `-0.0`. Se contrastó con `/exec` en `sensors` y `days`, también dentro del `GROUP BY` de una columna.

## 2026-11-06 — GROUP BY de una columna

`GROUP BY` de una columna con `count()` devuelve un grupo por valor distinto, después del filtro. NULL forma un grupo. `-0.0` y `0.0` no se mezclan. Una cadena vacía tampoco es el grupo NULL. El orden sin `ORDER BY` es el de primera aparición: 10.1.0 lo deja al merge de su group-by asíncrono, así que esa secuencia no se reproduce. `ORDER BY` de la clave o de `count()` sí, y los empates conservan la primera aparición. Se contrastó con `/exec` en `sensors` y `days`.

## 2026-11-06 — SQL propio, no un motor embebido

El `SELECT` de H1 está escrito en `qdb-sql`. No envuelve DataFusion ni el parser de QuestDB. `SELECT 1` y DML responden `unsupported`.

## 2026-11-06 — Sin `unsafe`

Los ficheros se leen a `Vec<u8>`. Cuando haga falta `mmap`, el `unsafe` irá en `qdb-platform` con el invariante escrito al lado.
