\* Transport *\
Minimize
OBJ: 2 Route_A_X + 4 Route_A_Y + 5 Route_B_X + 3 Route_B_Y + 0 __dummy
Subject To
Supply_A: Route_A_X + Route_A_Y <= 30
Supply_B: Route_B_X + Route_B_Y <= 40
Demand_X: Route_A_X + Route_B_X >= 25
Demand_Y: Route_A_Y + Route_B_Y >= 35
_C1: __dummy = 0
Bounds
 0 <= Route_A_X <= 1e+30
 Route_A_Y >= 0
 Route_B_X >= 0
 Route_B_Y >= 0
 __dummy = 0
End
