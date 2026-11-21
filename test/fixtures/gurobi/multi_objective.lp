\ Gurobi-style multi-objective model
Maximize multi-objectives
 profit: Priority=2 Weight=1 AbsTol=0.5 RelTol=0.01 12 x + 9 y + 11 z
 balance: Priority=1 Weight=-1 x - y
Subject To
 machine: 3 x + 2 y + 4 z <= 120
 labour: x + y + z <= 40
 demand_x: x >= 2
Bounds
 0 <= x <= 30
 0 <= y <= 30
 0 <= z <= 30
General
 z
End
