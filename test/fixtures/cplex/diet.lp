\Problem name: diet

Minimize
 obj: 0.18 bread + 0.23 milk + 0.05 cheese + 0.69 potato + 0.31 fish + 0.75 yogurt
Subject To
 calories: 81 bread + 215 milk + 6 cheese + 107 potato + 133 fish + 128 yogurt >= 2000
 protein: 4 bread + 9 milk + 1 cheese + 3 potato + 12 fish + 8 yogurt >= 55
 calcium: 7 bread + 70 milk + 2 cheese + 20 potato + 12 fish + 40 yogurt >= 800
 variety: bread + milk + cheese + potato + fish + yogurt <= 12
Bounds
 0 <= bread <= 10
 0 <= milk <= 10
 0 <= cheese <= 10
 0 <= potato <= 10
 0 <= fish <= 10
 0 <= yogurt <= 10
End
