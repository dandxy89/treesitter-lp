\* names_with_underscores *\
Minimize
OBJ: 1 flow_(1,_2) + 2 flow_(2,_3) + 3 stock_[a]
Subject To
balance_1: flow_(1,_2) - flow_(2,_3) = 0
balance_2: flow_(2,_3) + stock_[a] >= 4
Bounds
 flow_(1,_2) <= 100
 flow_(2,_3) <= 100
End
