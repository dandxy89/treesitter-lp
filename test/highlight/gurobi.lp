\ Gurobi style model
\ <- comment
maximize multi-objectives
\ <- keyword.directive
\        ^ keyword.modifier
 profit: Priority=2 Weight=1.5 12 x + 9 y
\^ label
\        ^ property
\                ^ operator
\                 ^ number
\                              ^ number
\                                 ^ variable
 balance: Priority=1 x - y
\^ label
\         ^ property
subject to
 machine: 3 x + 2 y <= 120
 ind1: b = 1 -> x + y <= 3
\^ label
\      ^ variable
\            ^ operator
general constraints
\ <- keyword.type
 gmax: m = MAX ( x , y , 3 )
\^ label
\          ^ function.builtin
\              ^ punctuation.bracket
\                  ^ punctuation.delimiter
\                        ^ number
 gabs: a = ABS ( z )
\          ^ function.builtin
sos
\ <- keyword.type
 s1: S1 ::
\^ label
\    ^ type
\       ^ punctuation.delimiter
 x : 1
\^ variable
\    ^ number
end
