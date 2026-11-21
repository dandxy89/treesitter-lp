minimize multi-objectives
 profit: Priority=2 12 x + 9 w
\^ definition.constant
 loss: Priority=1 x
\^ definition.constant
subject to
 ind1: b = 1 -> x + w <= 3
\^ definition.constant
\      ^ reference.variable
general constraints
 gmax: m = MAX ( x , w , 3 )
\^ definition.constant
end
