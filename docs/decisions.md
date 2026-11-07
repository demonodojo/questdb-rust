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
