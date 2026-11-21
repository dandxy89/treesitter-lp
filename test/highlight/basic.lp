maximize
\ <- keyword.directive
 profit: 3 x + 2.5 y - z
\^ label
\        ^ number
\          ^ variable
\              ^ number
\            ^ operator
subject to
\ <- keyword.conditional
 cap: x + y <= 10
\^ label
\           ^ operator
\   ^ punctuation.delimiter
 inflow: x - z >= -infinity
\                  ^ constant.builtin
 rng: -5 <= x + y <= 5
\        ^ operator
bounds
\ <- keyword.type
 0 <= x <= 40
\          ^ number
 y free
\  ^ keyword.modifier
generals
\ <- keyword.type
 x
\^ variable
end
\ <- keyword.return
