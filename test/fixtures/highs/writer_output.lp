\ Problem written in the style of HiGHS' LP writer.
Minimize
 obj: +3 x0 +2 x1 +4 x2 +1 x3
Subject To
 c0: +1 x0 +1 x1 +1 x2 +1 x3 >= 10
 c1: +2 x0 -1 x1 <= 15
 c2: +1 x1 -1 x2 = 0
 c3: -4 <= +1 x0 -1 x3 <= 4
Bounds
 0 <= x0 <= 8
 0 <= x1 <= inf
 x2 = 3
 -inf <= x3 <= 12
Generals
 x0
End
