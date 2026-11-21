\ Gurobi-style SOS and semi-continuous variables
Minimize
 obj: x + y + z
Subject To
 c0: x + y + z >= 2
Bounds
 2 <= x <= 10
 y <= 8
 z free
Semi-Continuous
 x
Generals
 y
SOS
 sos1: S1 :: x : 1 y : 2
 sos2: S2 :: y : 1 z : 2
End
