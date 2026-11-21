\Problem name: lazy_cuts
Minimize
 obj: x + 2 y + 3 z
Subject To
 base: x + y + z >= 3
Lazy Constraints
 lz1: x - y <= 2
 lz2: y - z <= 2
User Cuts
 uc1: x + y + z <= 20
Bounds
 0 <= x <= 10
 0 <= y <= 10
 0 <= z <= 10
General
 x y z
End
