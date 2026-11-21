; Symbol definitions: objective, constraint and SOS-set names.
(named_objective
  name: (objective_name) @name) @definition.constant

(constraint
  name: (constraint_name) @name) @definition.constant

(general_constraint
  name: (constraint_name) @name) @definition.constant

(sos_constraint_header
  name: (sos_name) @name) @definition.constant

; Variables are declared by use (and optionally in bounds / type sections).
(bound_declaration
  (identifier) @name) @definition.variable

(generals_section
  (identifier) @name @definition.variable)

(integers_section
  (identifier) @name @definition.variable)

(binaries_section
  (identifier) @name @definition.variable)

(semi_continuous_section
  (identifier) @name @definition.variable)

; Variable references in expressions and other constraint forms.
(term
  (identifier) @name @reference.variable)

(quadratic_term
  (identifier) @name @reference.variable)

(indicator
  (identifier) @name @reference.variable)

(general_constraint
  resultant: (identifier) @name @reference.variable)

(sos_entry
  (identifier) @name @reference.variable)
