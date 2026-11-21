\Problem name: ranges_sos
Minimize
 obj: 2 x + 3 y + z + w
Subject To
 r1: -5 <= x + y <= 10
 r2: x - z >= -4
 r3: 2 y + w <= 8
 r4: 3 <= x + w
Bounds
 x free
 -inf <= y <= +inf
 0 <= z <= 40
 w >= 2
Generals
 w
SOS
 s1: S1 ::
 x : 1
 y : 2
 z : 3
 s2: S2 :: 
 x : 1
 y : 2
 w : 3
End
