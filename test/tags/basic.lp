minimize
 obj: 2 x + 3 y
\^ definition.constant
\       ^ reference.variable
subject to
 cap: x + y <= 10
\^ definition.constant
\         ^ reference.variable
 c2: z - x >= -4
\^ definition.constant
bounds
 0 <= x <= 40
\     ^ definition.variable
generals
 y
\^ definition.variable
sos
 s1: S1 ::
\^ definition.constant
 x : 1
\^ reference.variable
end
