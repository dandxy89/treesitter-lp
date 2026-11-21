minimize
 obj: 2 x + [ x ^ 2 + 4 x * y ] / 2
\           ^ punctuation.bracket
\               ^ operator
\                         ^ operator
\                               ^ operator
subject to
 q1: [ x ^ 2 ] <= 25
\^ label
\ block comments carry through
\ <- comment
bounds
 -inf <= x <= +inf
\^ operator
\ ^ constant.builtin
 y >= 0
\^ variable
semi-continuous
\ <- keyword.type
 x
\^ variable
binaries
\ <- keyword.type
 b
end
