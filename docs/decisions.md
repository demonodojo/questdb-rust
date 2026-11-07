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
