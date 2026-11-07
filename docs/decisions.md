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
