//! Differential test against `lp_parser_rs`, the parser this grammar mirrors.
//!
//! Random LP files (mostly valid, with adversarial names, numbers, spacing and
//! comments) are parsed by both. Whenever `lp_parser_rs` accepts a file, the
//! tree-sitter tree must be free of errors and describe the same model: the
//! same objectives, constraints, coefficients, bounds and variable lists.
//! Both sides are reduced to the same canonical text and compared.
//!
//! Files `lp_parser_rs` rejects are not compared: the grammar leaves semantic
//! checks (an exponent other than 2, a range whose operators disagree, ...)
//! to it.
//!
//! Run more cases, or another seed, with
//! `LP_DIFF_CASES=1000000 LP_DIFF_SEED=7 cargo test differential`.

// Test code: numeric casts are small and exact, and a wrong index is a failure.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::too_many_lines,
    clippy::indexing_slicing
)]

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::fmt::Write as _;
    use std::time::{Duration, Instant};

    use lp_parser_rs::lexer::{
        Lexer, ParseResult, RawCoefficient, RawConstraint, RawQuadraticTerm, Token,
    };
    use lp_parser_rs::lp::LpProblemParser;
    use lp_parser_rs::model::{ComparisonOp, GeneralFunction, VariableType};
    use tree_sitter::{Node, Parser};

    type R<T> = Result<T, String>;

    /// `writeln!` into a `String`, which cannot fail.
    macro_rules! put {
    ($dst:expr, $($arg:tt)*) => {
        writeln!($dst, $($arg)*).expect("writing to a String cannot fail")
    };
}

    fn parser() -> Parser {
        let mut parser = Parser::new();
        parser
            .set_language(&crate::LANGUAGE.into())
            .expect("Error loading LP parser");
        parser
    }

    fn linear(coefficients: &[RawCoefficient<'_>]) -> String {
        coefficients
            .iter()
            .map(|c| format!("({:?} {})", c.value, c.name))
            .collect::<Vec<_>>()
            .concat()
    }

    fn quadratic(terms: &[RawQuadraticTerm<'_>]) -> String {
        terms
            .iter()
            .map(|q| format!("({:?} {} {})", q.coefficient, q.var1, q.var2))
            .collect::<Vec<_>>()
            .concat()
    }

    fn dump_constraint(label: &str, constraint: &RawConstraint<'_>, s: &mut String) {
        match constraint {
            RawConstraint::Standard {
                name,
                coefficients,
                operator,
                rhs,
                ..
            } => put!(
                s,
                "{label} std {name} | {} | {operator:?} {rhs:?}",
                linear(coefficients)
            ),
            RawConstraint::Quadratic {
                name,
                coefficients,
                quadratic: terms,
                operator,
                rhs,
                ..
            } => put!(
                s,
                "{label} quad {name} | {} | {} | {operator:?} {rhs:?}",
                linear(coefficients),
                quadratic(terms)
            ),
            RawConstraint::Indicator {
                name,
                variable,
                active_value,
                coefficients,
                operator,
                rhs,
                ..
            } => put!(
                s,
                "{label} ind {name} {variable}={active_value} | {} | {operator:?} {rhs:?}",
                linear(coefficients)
            ),
            RawConstraint::General {
                name,
                resultant,
                function,
                ..
            } => {
                let body = match function {
                    GeneralFunction::Max {
                        variables,
                        constant,
                    } => format!("MAX {variables:?} {constant:?}"),
                    GeneralFunction::Min {
                        variables,
                        constant,
                    } => format!("MIN {variables:?} {constant:?}"),
                    GeneralFunction::Abs { variable } => format!("ABS {variable}"),
                    GeneralFunction::And { variables } => format!("AND {variables:?}"),
                    GeneralFunction::Or { variables } => format!("OR {variables:?}"),
                };
                put!(s, "{label} gen {name} {resultant} = {body}");
            }
            RawConstraint::SOS {
                name,
                sos_type,
                weights,
                ..
            } => put!(s, "{label} {name} {sos_type:?} | {}", linear(weights)),
        }
    }

    fn dump_upstream(result: &ParseResult<'_>) -> String {
        let mut s = String::new();
        put!(s, "sense {:?}", result.sense);
        for objective in &result.objectives {
            let a = &objective.attributes;
            put!(
                s,
                "obj {} | {} | {} | const {:?} | {:?} {:?} {:?} {:?}",
                objective.name,
                linear(&objective.coefficients),
                quadratic(&objective.quadratic),
                objective.constant,
                a.priority.map(|p| p as f64),
                a.weight,
                a.abs_tol,
                a.rel_tol
            );
        }
        for (label, constraints) in [
            ("con", &result.constraints),
            ("lazy", &result.lazy_constraints),
            ("cut", &result.user_cuts),
        ] {
            for constraint in constraints {
                dump_constraint(label, constraint, &mut s);
            }
        }
        for (name, bound) in &result.bounds {
            put!(s, "bound {name} {bound:?}");
        }
        put!(s, "generals {:?}", result.generals);
        put!(s, "integers {:?}", result.integers);
        put!(s, "binaries {:?}", result.binaries);
        put!(s, "semis {:?}", result.semi_continuous);
        for set in &result.sos {
            dump_constraint("sos", set, &mut s);
        }
        s
    }

    /// Reads the model back out of a syntax tree, normalising it the way
    /// `lp_parser_rs::assemble` does (folded constants, flipped and expanded
    /// ranged constraints, halved objective quadratics).
    struct Tree<'a> {
        src: &'a str,
    }

    /// The children of a node, comments aside.
    fn kids(node: Node<'_>) -> Vec<Node<'_>> {
        let mut cursor = node.walk();
        node.children(&mut cursor)
            .filter(|k| !k.is_extra())
            .collect()
    }

    fn at<'t>(nodes: &[Node<'t>], i: usize) -> R<Node<'t>> {
        nodes
            .get(i)
            .copied()
            .ok_or_else(|| format!("missing child {i}"))
    }

    #[derive(Default)]
    struct Segment {
        linear: Vec<(f64, String)>,
        quadratic: Vec<(f64, String, String)>,
        constant: f64,
    }

    fn linear_ts(coefficients: &[(f64, String)]) -> String {
        coefficients
            .iter()
            .map(|(value, name)| format!("({value:?} {name})"))
            .collect::<Vec<_>>()
            .concat()
    }

    fn quadratic_ts(terms: &[(f64, String, String)]) -> String {
        terms
            .iter()
            .map(|(c, a, b)| format!("({c:?} {a} {b})"))
            .collect::<Vec<_>>()
            .concat()
    }

    fn operator(text: &str) -> R<ComparisonOp> {
        Ok(match text {
            "<=" | "=<" => ComparisonOp::LTE,
            ">=" | "=>" => ComparisonOp::GTE,
            "<" => ComparisonOp::LT,
            ">" => ComparisonOp::GT,
            "=" => ComparisonOp::EQ,
            other => return Err(format!("bad operator {other:?}")),
        })
    }

    const fn flip(op: ComparisonOp) -> ComparisonOp {
        match op {
            ComparisonOp::LT => ComparisonOp::GT,
            ComparisonOp::LTE => ComparisonOp::GTE,
            ComparisonOp::GT => ComparisonOp::LT,
            ComparisonOp::GTE => ComparisonOp::LTE,
            ComparisonOp::EQ => ComparisonOp::EQ,
        }
    }

    const fn is_le(op: ComparisonOp) -> bool {
        matches!(op, ComparisonOp::LT | ComparisonOp::LTE)
    }

    const fn is_ge(op: ComparisonOp) -> bool {
        matches!(op, ComparisonOp::GT | ComparisonOp::GTE)
    }

    impl<'a> Tree<'a> {
        fn text(&self, node: Node<'_>) -> &'a str {
            &self.src[node.byte_range()]
        }

        fn number(&self, node: Node<'_>) -> R<f64> {
            match node.kind() {
                "number" => self
                    .text(node)
                    .parse::<f64>()
                    .map_err(|e| format!("number {:?}: {e}", self.text(node))),
                "infinity" => Ok(f64::INFINITY),
                kind => Err(format!(
                    "expected a constant, found {kind} {:?}",
                    self.text(node)
                )),
            }
        }

        /// `[sign] constant` at `nodes[i]`: the value and the index after it.
        fn signed(&self, nodes: &[Node<'_>], mut i: usize) -> R<(f64, usize)> {
            let mut sign = 1.0;
            match nodes.get(i).map(Node::kind) {
                Some("+") => i += 1,
                Some("-") => {
                    sign = -1.0;
                    i += 1;
                }
                _ => {}
            }
            Ok((sign * self.number(at(nodes, i)?)?, i + 1))
        }

        fn quadratic_block(
            &self,
            node: Node<'_>,
            block_sign: f64,
            objective: bool,
            out: &mut Vec<(f64, String, String)>,
        ) -> R<()> {
            let mut sign = 1.0;
            let mut terms = Vec::new();
            for k in kids(node) {
                match k.kind() {
                    "+" => sign = 1.0,
                    "-" => sign = -1.0,
                    "quadratic_term" => {
                        let parts = kids(k);
                        let (coefficient, rest) = if at(&parts, 0)?.kind() == "number" {
                            (self.number(parts[0])?, &parts[1..])
                        } else {
                            (1.0, &parts[..])
                        };
                        let [var1, op, var2] = rest else {
                            return Err(format!("quadratic term shape {:?}", self.text(k)));
                        };
                        let var1 = self.text(*var1).to_string();
                        let var2 = if op.kind() == "^" {
                            var1.clone()
                        } else {
                            self.text(*var2).to_string()
                        };
                        terms.push((block_sign * sign * coefficient, var1, var2));
                        sign = 1.0;
                    }
                    // The brackets and the objective's `/ 2`.
                    "[" | "]" | "/" | "number" => {}
                    other => return Err(format!("unexpected {other} in a quadratic block")),
                }
            }
            if objective {
                for term in &mut terms {
                    term.0 /= 2.0;
                }
            }
            out.extend(terms);
            Ok(())
        }

        fn expression(&self, node: Node<'_>, objective: bool) -> R<Segment> {
            if node.kind() != "linear_expression" {
                return Err(format!(
                    "expected linear_expression, found {} {:?}",
                    node.kind(),
                    self.text(node)
                ));
            }
            let mut segment = Segment::default();
            let mut sign = 1.0;
            for k in kids(node) {
                match k.kind() {
                    "+" => sign = 1.0,
                    "-" => sign = -1.0,
                    "term" => {
                        let parts = kids(k);
                        match parts.as_slice() {
                            [v] if v.kind() == "identifier" => {
                                segment.linear.push((sign, self.text(*v).to_string()));
                            }
                            [c] => {
                                segment.constant =
                                    f64::mul_add(sign, self.number(*c)?, segment.constant);
                            }
                            [c, v] if v.kind() == "identifier" => segment
                                .linear
                                .push((sign * self.number(*c)?, self.text(*v).to_string())),
                            _ => return Err(format!("term shape {:?}", self.text(k))),
                        }
                        sign = 1.0;
                    }
                    "quadratic_block" => {
                        self.quadratic_block(k, sign, objective, &mut segment.quadratic)?;
                        sign = 1.0;
                    }
                    other => return Err(format!("unexpected {other} in an expression")),
                }
            }
            Ok(segment)
        }

        fn constraint(
            &self,
            node: Node<'_>,
            label: &str,
            taken: &HashSet<&str>,
            s: &mut String,
        ) -> R<()> {
            let nodes = kids(node);
            let mut i = 0;
            let name = if at(&nodes, 0)?.kind() == "constraint_name" {
                i = 2;
                Some(self.text(nodes[0]))
            } else {
                None
            };
            let shown = name.unwrap_or("__c__");
            let indicator = if at(&nodes, i)?.kind() == "indicator" {
                let parts = kids(nodes[i]);
                let value = self.number(at(&parts, 2)?)?;
                if value != 0.0 && value != 1.0 {
                    return Err("indicator value is neither 0 nor 1".into());
                }
                i += 1;
                Some((self.text(parts[0]), value == 1.0))
            } else {
                None
            };

            let mut rows: Vec<(String, Segment, ComparisonOp, f64)> = Vec::new();
            if at(&nodes, i)?.kind() == "linear_expression" {
                // Standard: `expr op rhs`.
                let lhs = self.expression(nodes[i], false)?;
                if lhs.linear.is_empty() && lhs.quadratic.is_empty() {
                    return Err("standard constraint without a variable".into());
                }
                let op = operator(self.text(at(&nodes, i + 1)?))?;
                let (rhs, next) = self.signed(&nodes, i + 2)?;
                if next != nodes.len() {
                    return Err("nodes after a standard constraint".into());
                }
                let rhs = rhs - lhs.constant;
                rows.push((shown.to_string(), lhs, op, rhs));
            } else {
                // Flipped `lo op expr` or ranged `lo op expr op hi`.
                let mut constant = 0.0;
                let mut sign = 1.0;
                while at(&nodes, i)?.kind() != "comparison_operator" {
                    match nodes[i].kind() {
                        "+" => sign = 1.0,
                        "-" => sign = -1.0,
                        _ => {
                            constant = f64::mul_add(sign, self.number(nodes[i])?, constant);
                            sign = 1.0;
                        }
                    }
                    i += 1;
                }
                let op1 = operator(self.text(nodes[i]))?;
                let mid = self.expression(at(&nodes, i + 1)?, false)?;
                i += 2;
                if i < nodes.len() {
                    let op2 = operator(self.text(nodes[i]))?;
                    let (rhs, next) = self.signed(&nodes, i + 1)?;
                    if next != nodes.len() {
                        return Err("nodes after a ranged constraint".into());
                    }
                    // `lp_parser_rs::assemble::range_upper_name`.
                    let upper = name.map_or_else(
                        || "__c__".to_string(),
                        |base| {
                            let mut candidate = format!("{base}_rng");
                            let mut suffix = 1u32;
                            while taken.contains(candidate.as_str()) {
                                suffix += 1;
                                candidate = format!("{base}_rng{suffix}");
                            }
                            candidate
                        },
                    );
                    let lower = Segment {
                        linear: mid.linear.clone(),
                        ..Segment::default()
                    };
                    rows.push((shown.to_string(), lower, flip(op1), constant - mid.constant));
                    let rhs = rhs - mid.constant;
                    rows.push((upper, mid, op2, rhs));
                } else {
                    let rhs = constant - mid.constant;
                    rows.push((shown.to_string(), mid, flip(op1), rhs));
                }
            }

            for (name, segment, op, rhs) in rows {
                let lin = linear_ts(&segment.linear);
                if let Some((variable, active)) = indicator {
                    put!(
                        s,
                        "{label} ind {name} {variable}={active} | {lin} | {op:?} {rhs:?}"
                    );
                } else if segment.quadratic.is_empty() {
                    put!(s, "{label} std {name} | {lin} | {op:?} {rhs:?}");
                } else {
                    put!(
                        s,
                        "{label} quad {name} | {lin} | {} | {op:?} {rhs:?}",
                        quadratic_ts(&segment.quadratic)
                    );
                }
            }
            Ok(())
        }

        fn general_constraint(&self, node: Node<'_>, s: &mut String) -> R<()> {
            let nodes = kids(node);
            let mut i = 0;
            let name = if at(&nodes, 0)?.kind() == "constraint_name" {
                i = 2;
                self.text(nodes[0])
            } else {
                "__c__"
            };
            let resultant = self.text(at(&nodes, i)?);
            let function = self.text(at(&nodes, i + 2)?).to_ascii_uppercase();
            i += 4;
            let mut variables: Vec<&str> = Vec::new();
            let mut constants: Vec<f64> = Vec::new();
            loop {
                if at(&nodes, i)?.kind() == "identifier" {
                    variables.push(self.text(nodes[i]));
                    i += 1;
                } else {
                    let (value, next) = self.signed(&nodes, i)?;
                    constants.push(value);
                    i = next;
                }
                match at(&nodes, i)?.kind() {
                    "," => i += 1,
                    ")" => break,
                    other => return Err(format!("general argument separator {other}")),
                }
            }
            let body = match function.as_str() {
                "MAX" => format!(
                    "MAX {variables:?} {:?}",
                    constants.iter().copied().reduce(f64::max)
                ),
                "MIN" => format!(
                    "MIN {variables:?} {:?}",
                    constants.iter().copied().reduce(f64::min)
                ),
                "ABS" => format!("ABS {}", variables.first().copied().unwrap_or("?")),
                "AND" | "OR" => format!("{function} {variables:?}"),
                other => return Err(format!("function {other}")),
            };
            put!(s, "con gen {name} {resultant} = {body}");
            Ok(())
        }

        fn bound(&self, node: Node<'_>, s: &mut String) -> R<()> {
            let nodes = kids(node);
            let single = |op: ComparisonOp, bound: f64, variable_first: bool| {
                if op == ComparisonOp::EQ {
                    VariableType::DoubleBound(bound, bound)
                } else if is_ge(op) == variable_first {
                    VariableType::LowerBound(bound)
                } else {
                    VariableType::UpperBound(bound)
                }
            };
            let (name, bound) = if at(&nodes, 0)?.kind() == "identifier" {
                let name = self.text(nodes[0]);
                if at(&nodes, 1)?.kind() == "free_keyword" {
                    (name, VariableType::Free)
                } else {
                    let op = operator(self.text(nodes[1]))?;
                    let (value, _) = self.signed(&nodes, 2)?;
                    (name, single(op, value, true))
                }
            } else {
                let (first, i) = self.signed(&nodes, 0)?;
                let op1 = operator(self.text(at(&nodes, i)?))?;
                let name = self.text(at(&nodes, i + 1)?);
                if i + 2 < nodes.len() {
                    let op2 = operator(self.text(nodes[i + 2]))?;
                    let (second, _) = self.signed(&nodes, i + 3)?;
                    if is_le(op1) && is_le(op2) {
                        (name, VariableType::DoubleBound(first, second))
                    } else if is_ge(op1) && is_ge(op2) {
                        (name, VariableType::DoubleBound(second, first))
                    } else {
                        return Err("double bound with mixed operators".into());
                    }
                } else {
                    (name, single(op1, first, false))
                }
            };
            put!(s, "bound {name} {bound:?}");
            Ok(())
        }

        fn objectives(&self, section: Node<'_>, s: &mut String) -> R<()> {
            for k in kids(section) {
                let mut attributes = [None; 4];
                let mut segment = Segment::default();
                let name = if k.kind() == "linear_expression" {
                    segment = self.expression(k, true)?;
                    "__obj__"
                } else {
                    let parts = kids(k);
                    for part in parts.iter().skip(2) {
                        if part.kind() == "objective_attribute" {
                            let fields = kids(*part);
                            let (value, _) = self.signed(&fields, 2)?;
                            let slot =
                                match self.text(at(&fields, 0)?).to_ascii_lowercase().as_str() {
                                    "priority" => 0,
                                    "weight" => 1,
                                    "abstol" => 2,
                                    "reltol" => 3,
                                    other => return Err(format!("attribute {other}")),
                                };
                            // lp_parser_rs stores the priority as an integer.
                            attributes[slot] = Some(if slot == 0 {
                                value as i64 as f64
                            } else {
                                value
                            });
                        } else {
                            segment = self.expression(*part, true)?;
                        }
                    }
                    self.text(at(&parts, 0)?)
                };
                put!(
                    s,
                    "obj {name} | {} | {} | const {:?} | {:?} {:?} {:?} {:?}",
                    linear_ts(&segment.linear),
                    quadratic_ts(&segment.quadratic),
                    segment.constant,
                    attributes[0],
                    attributes[1],
                    attributes[2],
                    attributes[3]
                );
            }
            Ok(())
        }

        fn sos(&self, section: &[Node<'_>], s: &mut String) -> R<()> {
            let mut open = false;
            for entry in section.iter().skip(1) {
                let parts = kids(*entry);
                if entry.kind() == "sos_constraint_header" {
                    let (name, set_type) = if parts.len() == 4 {
                        (self.text(parts[0]), parts[2])
                    } else {
                        ("__c__", at(&parts, 0)?)
                    };
                    if open {
                        s.push('\n');
                    }
                    write!(
                        s,
                        "sos {name} {} | ",
                        self.text(set_type).to_ascii_uppercase()
                    )
                    .expect("writing to a String cannot fail");
                    open = true;
                } else {
                    let (weight, _) = self.signed(&parts, 2)?;
                    write!(s, "({weight:?} {})", self.text(at(&parts, 0)?))
                        .expect("writing to a String cannot fail");
                }
            }
            if open {
                s.push('\n');
            }
            Ok(())
        }

        fn dump(&self, root: Node<'_>) -> R<String> {
            let top = kids(root);
            let mut s = String::new();
            let sense = self.text(at(&top, 0)?).to_ascii_lowercase();
            let sense = if sense.starts_with("min") {
                "Minimize"
            } else {
                "Maximize"
            };
            put!(s, "sense {sense}");

            // The explicit names a generated range name must avoid.
            let mut taken: HashSet<&str> = HashSet::new();
            for section in &top {
                if matches!(
                    section.kind(),
                    "constraints_section" | "lazy_constraints_section" | "user_cuts_section"
                ) {
                    for constraint in kids(*section) {
                        if let Some(name) = constraint.child_by_field_name("name") {
                            taken.insert(self.text(name));
                        }
                    }
                }
            }

            let mut constraints = String::new();
            let mut generals = String::new();
            let mut lazy = String::new();
            let mut cuts = String::new();
            let mut bounds = String::new();
            let mut sos = String::new();
            let mut lists: [Vec<&str>; 4] = Default::default();

            for section in &top {
                let nodes = kids(*section);
                match section.kind() {
                    "sense" | "multi_objectives_keyword" | "end_marker" => {}
                    "objectives_section" => self.objectives(*section, &mut s)?,
                    "constraints_section" | "lazy_constraints_section" | "user_cuts_section" => {
                        let (label, out) = match section.kind() {
                            "constraints_section" => ("con", &mut constraints),
                            "lazy_constraints_section" => ("lazy", &mut lazy),
                            _ => ("cut", &mut cuts),
                        };
                        for constraint in nodes.iter().skip(1) {
                            self.constraint(*constraint, label, &taken, out)?;
                        }
                    }
                    "general_constraints_section" => {
                        for constraint in nodes.iter().skip(1) {
                            self.general_constraint(*constraint, &mut generals)?;
                        }
                    }
                    "bounds_section" => {
                        for bound in nodes.iter().skip(1) {
                            self.bound(*bound, &mut bounds)?;
                        }
                    }
                    "generals_section"
                    | "integers_section"
                    | "binaries_section"
                    | "semi_continuous_section" => {
                        let slot = match section.kind() {
                            "generals_section" => 0,
                            "integers_section" => 1,
                            "binaries_section" => 2,
                            _ => 3,
                        };
                        for variable in nodes.iter().skip(1) {
                            if variable.kind() != "identifier" {
                                return Err(format!("{} in a variable list", variable.kind()));
                            }
                            lists[slot].push(self.text(*variable));
                        }
                    }
                    "sos_section" => self.sos(&nodes, &mut sos)?,
                    other => return Err(format!("unexpected top-level {other}")),
                }
            }
            s.push_str(&constraints);
            s.push_str(&generals);
            s.push_str(&lazy);
            s.push_str(&cuts);
            s.push_str(&bounds);
            put!(s, "generals {:?}", lists[0]);
            put!(s, "integers {:?}", lists[1]);
            put!(s, "binaries {:?}", lists[2]);
            put!(s, "semis {:?}", lists[3]);
            s.push_str(&sos);
            Ok(s)
        }
    }

    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    enum Verdict {
        /// Both accept and describe the same model.
        Agree,
        /// `lp_parser_rs` rejects the file: nothing to compare.
        Rejected,
        /// `lp_parser_rs` accepts, the tree has an `ERROR` or `MISSING` node.
        TreeError,
        /// Both accept but describe different models.
        Differ,
    }

    fn compare(parser: &mut Parser, input: &str) -> (Verdict, String) {
        let Ok(result) = LpProblemParser::new().parse(Lexer::new(input)) else {
            return (Verdict::Rejected, String::new());
        };
        let tree = parser
            .parse(input, None)
            .expect("tree-sitter returned no tree");
        let root = tree.root_node();
        if root.has_error() {
            return (Verdict::TreeError, root.to_sexp());
        }
        let expected = dump_upstream(&result);
        match (Tree { src: input }).dump(root) {
            Err(e) => (
                Verdict::Differ,
                format!(
                    "cannot read the tree: {e}\n--- lp_parser_rs\n{expected}--- tree\n{}",
                    root.to_sexp()
                ),
            ),
            Ok(actual) if actual == expected => (Verdict::Agree, String::new()),
            Ok(actual) => (
                Verdict::Differ,
                format!("--- lp_parser_rs\n{expected}--- tree-sitter\n{actual}"),
            ),
        }
    }

    /// Inputs on which the grammar knowingly departs from `lp_parser_rs`:
    ///
    /// - `lp_parser_rs` reads a line-start keyword as a name when the previous
    ///   token is `:` or `::`, so in `obj:` / `st` / `c1: x >= 1` it takes `st`
    ///   for a variable of the objective (and then rejects the file). The parse
    ///   table cannot see the previous token; the grammar reads the header.
    /// - A block comment between the `Subject To` header and its optional `:`.
    fn known_divergence(input: &str) -> bool {
        const KEYWORDS: &[&str] = &[
            "st",
            "s.t.",
            "bound",
            "bounds",
            "gen",
            "general",
            "generals",
            "integer",
            "integers",
            "bin",
            "binary",
            "binaries",
            "semi",
            "semis",
            "semi-continuous",
            "sos",
            "end",
            "genconstr",
            "genconstrs",
        ];
        let mut after_colon = false;
        for item in Lexer::new(input) {
            let Ok((start, token, end)) = item else {
                return false;
            };
            match &token {
                Token::Identifier(name) if after_colon => {
                    let line = &input[input[..start].rfind('\n').map_or(0, |nl| nl + 1)..start];
                    let at_line_start = line.trim().is_empty() || line.contains("*\\");
                    if at_line_start && KEYWORDS.contains(&name.to_ascii_lowercase().as_str()) {
                        return true;
                    }
                }
                Token::SubjectTo
                    if input[end..]
                        .trim_start_matches([' ', '\t'])
                        .starts_with("\\*") =>
                {
                    return true;
                }
                _ => {}
            }
            after_colon = matches!(token, Token::Colon | Token::DoubleColon);
        }
        false
    }

    /// Shrink a failing input: drop lines, then words, then characters, for as
    /// long as it still fails the same way.
    fn reduce(parser: &mut Parser, input: &str, want: Verdict) -> String {
        let mut input = input.to_string();
        loop {
            let before = input.len();
            for unit in [Some('\n'), Some(' '), None] {
                let mut i = 0;
                loop {
                    let pieces: Vec<&str> = unit.map_or_else(
                        || input.split_inclusive(|_| true).collect(),
                        |unit| input.split_inclusive(unit).collect(),
                    );
                    if i >= pieces.len() {
                        break;
                    }
                    let candidate: String = pieces
                        .iter()
                        .enumerate()
                        .filter(|(j, _)| *j != i)
                        .map(|(_, piece)| *piece)
                        .collect();
                    if compare(parser, &candidate).0 == want && !known_divergence(&candidate) {
                        input = candidate;
                    } else {
                        i += 1;
                    }
                }
            }
            if input.len() == before {
                return input;
            }
        }
    }

    fn env_number(name: &str, default: u64) -> u64 {
        std::env::var(name).map_or(default, |value| {
            value
                .parse()
                .unwrap_or_else(|e| panic!("{name}={value:?} is not a number: {e}"))
        })
    }

    #[test]
    fn agrees_with_lp_parser_rs_on_generated_files() {
        let cases = env_number("LP_DIFF_CASES", 50_000);
        let seed = env_number("LP_DIFF_SEED", 1);
        let mut parser = parser();
        let mut agreed = 0u64;
        for n in 0..cases {
            let input = generate::lp_file(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(n));
            let (verdict, _) = compare(&mut parser, &input);
            match verdict {
                Verdict::Agree => agreed += 1,
                Verdict::Rejected => {}
                Verdict::TreeError | Verdict::Differ => {
                    if known_divergence(&input) {
                        continue;
                    }
                    let reduced = reduce(&mut parser, &input, verdict);
                    let (_, detail) = compare(&mut parser, &reduced);
                    panic!(
                    "{verdict:?} on case {n} of seed {seed}; reduced input:\n{reduced}\n{detail}"
                );
                }
            }
        }
        // The generator must keep producing files worth comparing.
        assert!(
            agreed * 5 > cases,
            "only {agreed} of {cases} generated files were accepted by lp_parser_rs"
        );
    }

    /// The scanner must not rescan the line for every token (`get_column`), nor
    /// keep parallel parse stacks alive, on a single very long line.
    #[test]
    fn long_lines_parse_in_linear_time() {
        const TERMS: usize = 60_000;
        let terms =
            |term: &dyn Fn(usize) -> String| (0..TERMS).map(term).collect::<Vec<_>>().join(" + ");
        let inputs = [
            (
                "block comments on one line",
                format!(
                    "min\n obj: x\nst\n c: \\* a\n *\\ {} <= 1\nend\n",
                    terms(&|i| format!("\\* c *\\ x{i}"))
                ),
            ),
            (
                // `bin` is no keyword after a sign, so the scanner lexes no token
                // there and its line-start state outlives the comment.
                "terms after a line-start comment",
                format!(
                    "min\n obj: x + \\* a\n *\\ bin + {}\nst\n x >= 1\nend\n",
                    terms(&|i| format!("{i} x{i}"))
                ),
            ),
            (
                "constants before the variable",
                format!(
                    "min\n obj: x\nst\n c: {} + x <= 1\nend\n",
                    terms(&|i| i.to_string())
                ),
            ),
        ];
        let mut parser = parser();
        for (what, input) in &inputs {
            let start = Instant::now();
            let tree = parser.parse(input, None).expect("parse returned no tree");
            let elapsed = start.elapsed();
            assert!(!tree.root_node().has_error(), "{what}: parse error");
            // Linear time is tens of milliseconds; the quadratic scanner took
            // over a minute on the first input.
            assert!(
                elapsed < Duration::from_secs(10),
                "{what}: {} bytes took {elapsed:?}",
                input.len()
            );
        }
    }

    /// Random LP text: mostly valid, with adversarial names, numbers and spacing.
    mod generate {
        /// xorshift64*.
        struct Rng(u64);

        impl Rng {
            fn next(&mut self) -> u64 {
                self.0 ^= self.0 >> 12;
                self.0 ^= self.0 << 25;
                self.0 ^= self.0 >> 27;
                self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
            }

            fn below(&mut self, n: u64) -> u64 {
                self.next() % n
            }

            fn chance(&mut self, percent: u64) -> bool {
                self.below(100) < percent
            }

            fn pick<'a>(&mut self, items: &[&'a str]) -> &'a str {
                items[self.below(items.len() as u64) as usize]
            }
        }

        const PLAIN: &[&str] = &[
            "x", "y", "z", "x1", "x2", "y12", "cost", "a_b", "flow", "w", "t3", "b", "r",
        ];

        /// Keywords used as names, names made of special characters, and names
        /// that begin or end like a keyword or a number.
        const TRICKY: &[&str] = &[
            "bin",
            "end",
            "st",
            "s.t.",
            "free",
            "inflow",
            "infinity2",
            "S1",
            "s2",
            "min",
            "max",
            "bounds",
            "bound",
            "general",
            "generals",
            "gen",
            "semi",
            "semis",
            "sos",
            "integer",
            "binary",
            "e",
            "e5",
            "E",
            "x>y",
            "a-b",
            "x.1",
            "x[1]",
            "x(1)",
            "_",
            "a|b",
            "x-1",
            "multi-objectives",
            "subject",
            "to",
            "lazy",
            "cuts",
            "user",
            "constraints",
            "genconstrs",
            "genconstr",
            "semi-continuous",
            "x,y",
            "{a}",
            "x'",
            "#1",
            "$v",
            "~t",
            "a%>%[0]",
            "x>",
            "Priority",
            "Weight",
            "MAX",
            "ABS",
            "st1",
            "ends",
            "binx",
            "sosx",
            "x..",
            "End",
            "BIN",
            "ST",
            "Free",
            "minimize",
            "inf_x",
            "infx",
            "s1x",
            "such",
            "that",
            "x_e5",
            "c1",
            "obj",
            "n1e5",
            "e-5",
            "x-e",
            "a.b-c>d",
            "!x",
            "?",
            "@v",
            "x;",
            "S3",
            "s",
            "t",
            "gen1",
            "bounds2",
            "x>e",
            "x>1a",
            "end>x",
            "bin>y",
        ];

        /// The first `FINITE` are well-formed and finite.
        const NUMBERS: &[&str] = &[
            "1", "2", "3", "10", "0", "3.5", "3.", ".5", "1e5", "2E-3", "1.5e+2", "0.0", "12", "7",
            "100", "4", "1e0", "0.25", "inf", "infinity", "Inf", "INFINITY", "1e", "2e+", "5",
        ];
        const FINITE: u64 = 18;

        const OPERATORS: &[&str] = &["<=", ">=", "=", "<", ">", "=<", "=>", "<=", ">=", "="];
        const LE: &[&str] = &["<=", "<", "=<"];
        const GE: &[&str] = &[">=", ">", "=>"];

        struct Writer {
            rng: Rng,
            out: String,
            /// Plain spacing only: a third of the files have no comments, glued
            /// tokens or stray line breaks.
            plain: bool,
        }

        impl Writer {
            fn separator(&mut self) {
                if self.plain {
                    self.out.push(' ');
                    return;
                }
                let separator = match self.rng.below(100) {
                    0..=64 => " ",
                    65..=76 => "\n",
                    77..=80 => "  \t ",
                    81..=83 => " \\ note: x >= 1\n",
                    84..=86 => " \\* c *\\ ",
                    87..=88 => "\n\\* a\n b: *\\ ",
                    89..=90 => "\n\n  ",
                    91..=92 => "\r\n",
                    93..=94 => "",
                    95..=96 => " \\* x ** *\\\n",
                    97 => "\\* c *\\",
                    98 => "\n\\\n",
                    _ => " \\* a *\\ \\* b\n *\\ ",
                };
                self.out.push_str(separator);
            }

            fn token(&mut self, text: &str) {
                self.separator();
                self.out.push_str(text);
            }

            /// A token that normally starts a line.
            fn line(&mut self, text: &str) {
                if !self.plain && self.rng.chance(8) {
                    self.separator();
                } else if !self.plain && self.rng.chance(10) {
                    let lead = self.rng.pick(&[
                        "\n\\* c *\\ ",
                        "\n \\ c\n",
                        "\n\\* a\n*\\ ",
                        " \\* a\n*\\ ",
                        "\n\\* a *\\ \\* b *\\ ",
                        "\n\t",
                    ]);
                    self.out.push_str(lead);
                } else {
                    self.out.push_str("\n ");
                }
                self.out.push_str(text);
            }

            fn cased(&mut self, text: &str) -> String {
                match self.rng.below(3) {
                    0 => text.to_string(),
                    1 => text.to_ascii_uppercase(),
                    _ => {
                        let mut chars = text.chars();
                        chars.next().map_or_else(String::new, |first| {
                            first.to_ascii_uppercase().to_string() + chars.as_str()
                        })
                    }
                }
            }

            fn keyword(&mut self, spellings: &[&str]) {
                let spelling = self.rng.pick(spellings);
                let spelling = self.cased(spelling);
                self.line(&spelling);
            }

            fn name(&mut self) -> &'static str {
                if self.rng.chance(30) {
                    self.rng.pick(TRICKY)
                } else {
                    self.rng.pick(PLAIN)
                }
            }

            fn number(&mut self) -> &'static str {
                if self.rng.chance(85) {
                    NUMBERS[self.rng.below(FINITE) as usize]
                } else {
                    self.rng.pick(NUMBERS)
                }
            }

            fn pick_token(&mut self, items: &[&str]) {
                let item = self.rng.pick(items);
                self.token(item);
            }

            fn signed_number(&mut self) {
                if self.rng.chance(30) {
                    self.pick_token(&["+", "-", "-"]);
                }
                let number = self.number();
                self.token(number);
            }

            fn quadratic_block(&mut self, objective: bool) {
                self.token("[");
                for i in 0..=self.rng.below(3) {
                    if i > 0 || self.rng.chance(20) {
                        self.pick_token(&["+", "-"]);
                    }
                    if self.rng.chance(60) {
                        let number = self.number();
                        self.token(number);
                    }
                    let variable = self.name();
                    self.token(variable);
                    if self.rng.chance(50) {
                        self.token("^");
                        let exponent = if self.rng.chance(95) { "2" } else { "3" };
                        self.token(exponent);
                    } else {
                        self.token("*");
                        let variable = self.name();
                        self.token(variable);
                    }
                }
                self.token("]");
                if self.rng.chance(if objective { 95 } else { 3 }) {
                    self.token("/");
                    self.token("2");
                }
            }

            fn expression(&mut self, objective: bool) {
                for i in 0..=self.rng.below(4) {
                    if (i > 0 && self.rng.chance(98)) || (i == 0 && self.rng.chance(25)) {
                        self.pick_token(&["+", "-"]);
                    }
                    match self.rng.below(100) {
                        0..=54 => {
                            let number = self.number();
                            self.token(number);
                            let variable = self.name();
                            self.token(variable);
                        }
                        55..=79 => {
                            let variable = self.name();
                            self.token(variable);
                        }
                        80..=92 => {
                            let number = self.number();
                            self.token(number);
                        }
                        _ => self.quadratic_block(objective),
                    }
                }
            }

            fn constraint(&mut self) {
                // The entry normally starts on a new line.
                if self.plain || !self.rng.chance(8) {
                    self.out.push_str("\n ");
                }
                let entry = self.out.len();
                if self.rng.chance(65) {
                    let name = self.name();
                    self.token(name);
                    let colon = if self.rng.chance(92) { ":" } else { "::" };
                    self.token(colon);
                }
                if self.rng.chance(8) {
                    let variable = self.name();
                    self.token(variable);
                    self.token("=");
                    self.pick_token(&["0", "1", "1", "2"]);
                    self.token("->");
                }
                match self.rng.below(10) {
                    0..=5 => {
                        self.expression(false);
                        self.pick_token(OPERATORS);
                        self.signed_number();
                    }
                    6..=7 => {
                        self.signed_number();
                        if self.rng.chance(10) {
                            self.token("+");
                            let number = self.number();
                            self.token(number);
                        }
                        self.pick_token(OPERATORS);
                        self.expression(false);
                    }
                    _ => {
                        let number = self.number();
                        self.token(number);
                        let (first, second) = if self.rng.chance(90) {
                            let direction = if self.rng.chance(50) { LE } else { GE };
                            (self.rng.pick(direction), self.rng.pick(direction))
                        } else {
                            (self.rng.pick(OPERATORS), self.rng.pick(OPERATORS))
                        };
                        self.token(first);
                        self.expression(false);
                        self.token(second);
                        self.signed_number();
                    }
                }
                // Keep the entry's first token at the start of its line in plain files.
                if self.plain {
                    let body = self.out.split_off(entry);
                    self.out.push_str(body.trim_start());
                }
            }

            fn objectives(&mut self, multi: bool) {
                if self.rng.chance(12) {
                    return;
                }
                let named = self.rng.chance(60);
                if !named || self.rng.chance(15) {
                    self.expression(true);
                }
                if !named {
                    return;
                }
                for _ in 0..=self.rng.below(3) {
                    let name = self.name();
                    self.line(name);
                    self.token(":");
                    if multi || self.rng.chance(3) {
                        for _ in 0..self.rng.below(4) {
                            self.pick_token(&[
                                "Priority", "Weight", "AbsTol", "RelTol", "priority", "WEIGHT",
                                "Foo",
                            ]);
                            self.token("=");
                            self.signed_number();
                        }
                    }
                    if self.rng.chance(90) {
                        self.expression(true);
                    }
                }
            }

            fn variable_list(&mut self) {
                for _ in 0..self.rng.below(5) {
                    let variable = self.name();
                    if self.rng.chance(30) {
                        self.line(variable);
                    } else {
                        self.token(variable);
                    }
                }
            }

            fn bounds(&mut self) {
                for _ in 0..self.rng.below(5) {
                    let variable = self.name();
                    match self.rng.below(6) {
                        0 => {
                            self.line(variable);
                            let free = self.cased("free");
                            self.token(&free);
                        }
                        1 => {
                            self.line(variable);
                            self.pick_token(OPERATORS);
                            self.signed_number();
                        }
                        2 => {
                            let number = self.number();
                            self.line(number);
                            self.pick_token(OPERATORS);
                            self.token(variable);
                        }
                        3 => {
                            self.line("-");
                            let number = self.number();
                            self.token(number);
                            self.pick_token(LE);
                            self.token(variable);
                            self.pick_token(LE);
                            self.signed_number();
                        }
                        4 => {
                            let number = self.number();
                            self.line(number);
                            self.pick_token(GE);
                            self.token(variable);
                            self.pick_token(&[">=", ">", "=>", "<="]);
                            self.signed_number();
                        }
                        _ => {
                            let number = self.number();
                            self.line(number);
                            self.pick_token(OPERATORS);
                            self.token(variable);
                            self.pick_token(OPERATORS);
                            self.signed_number();
                        }
                    }
                }
            }

            fn sos(&mut self) {
                for _ in 0..self.rng.below(3) {
                    if self.rng.chance(75) {
                        let name = self.name();
                        self.line(name);
                        self.token(":");
                        self.pick_token(&["S1", "S2", "s1", "s2"]);
                    } else {
                        let set_type = self.rng.pick(&["S1", "S2", "s1", "S3"]);
                        self.line(set_type);
                    }
                    self.token("::");
                    for _ in 0..self.rng.below(4) + u64::from(self.rng.chance(95)) {
                        let variable = self.name();
                        self.token(variable);
                        self.token(":");
                        self.signed_number();
                    }
                }
            }

            fn general_constraints(&mut self) {
                for _ in 0..self.rng.below(3) {
                    if self.rng.chance(60) {
                        let name = self.name();
                        self.line(name);
                        self.token(":");
                        let resultant = self.name();
                        self.token(resultant);
                    } else {
                        let resultant = self.name();
                        self.line(resultant);
                    }
                    self.token("=");
                    let function = self
                        .rng
                        .pick(&["MAX", "MIN", "ABS", "AND", "OR", "max", "Min", "and"]);
                    self.token(function);
                    self.token("(");
                    let abs = function.eq_ignore_ascii_case("abs");
                    let arguments = if abs { 1 } else { 1 + self.rng.below(3) };
                    for i in 0..arguments {
                        if i > 0 {
                            self.token(",");
                        }
                        if abs || self.rng.chance(85) {
                            self.pick_token(PLAIN);
                        } else {
                            self.signed_number();
                        }
                    }
                    self.token(")");
                }
            }

            fn section(&mut self) {
                match self.rng.below(9) {
                    0 => {
                        self.keyword(&["bounds", "bound"]);
                        self.bounds();
                    }
                    1 => {
                        self.keyword(&["generals", "general", "gen"]);
                        self.variable_list();
                    }
                    2 => {
                        self.keyword(&["integers", "integer"]);
                        self.variable_list();
                    }
                    3 => {
                        self.keyword(&["binaries", "binary", "bin"]);
                        self.variable_list();
                    }
                    4 => {
                        self.keyword(&["semi-continuous", "semis", "semi"]);
                        self.variable_list();
                    }
                    5 => {
                        self.keyword(&["sos"]);
                        self.sos();
                    }
                    6 | 7 => {
                        self.keyword(&["lazy constraints", "user cuts"]);
                        for _ in 0..self.rng.below(3) {
                            self.constraint();
                        }
                    }
                    _ => {
                        self.keyword(&[
                            "general constraints",
                            "general constraint",
                            "general constrs",
                            "gen cons",
                            "genconstrs",
                            "genconstr",
                            "General  Constr",
                        ]);
                        self.general_constraints();
                    }
                }
            }
        }

        /// The LP file for `seed`.
        pub fn lp_file(seed: u64) -> String {
            // splitmix64, so neighbouring seeds give unrelated files.
            let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            let mut rng = Rng((z ^ (z >> 31)) | 1);
            let plain = rng.below(3) == 0;
            let mut w = Writer {
                rng,
                out: String::new(),
                plain,
            };

            if !w.plain && w.rng.chance(15) {
                w.out
                    .push_str("\\ header comment\n\\* block\n comment *\\\n");
            }
            let sense = w.rng.pick(&[
                "minimize", "maximize", "min", "max", "minimise", "maximise", "minimum", "maximum",
            ]);
            let sense = w.cased(sense);
            w.out.push_str(&sense);
            let multi = w.rng.chance(15);
            if multi {
                let marker = w.rng.pick(&[
                    " multi-objectives",
                    " Multi-Objectives",
                    " multi-objective",
                    "\nmulti-objectives",
                ]);
                w.out.push_str(marker);
            }
            w.objectives(multi);

            let header = w.rng.pick(&[
                "subject to",
                "such that",
                "st",
                "s.t.",
                "Subject To",
                "ST",
                "S.T.",
                "subject  to",
                "Subject To:",
                "st:",
                "s.t. :",
            ]);
            w.line(header);
            for _ in 0..w.rng.below(6) {
                w.constraint();
            }
            for _ in 0..w.rng.below(5) {
                w.section();
            }
            if w.rng.chance(85) {
                w.keyword(&["end"]);
            }
            w.out.push('\n');

            // Occasionally damage the text by dropping a few bytes.
            if w.rng.chance(12) && w.out.len() > 8 {
                let at = w.rng.below(w.out.len() as u64 - 4) as usize;
                let len = 1 + w.rng.below(3) as usize;
                if w.out.is_char_boundary(at) && w.out.is_char_boundary(at + len) {
                    w.out.replace_range(at..at + len, "");
                }
            }
            w.out
        }
    }
}
