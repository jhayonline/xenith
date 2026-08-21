//! # Static Checker
//!
//! Walks the tree between parsing and execution and reports type errors before
//! any code runs. Two things it buys over the checks scattered through the
//! interpreter: an error in a branch that never executes is still reported, and
//! every error in the file is reported at once rather than the first one.
//!
//! ## It is deliberately conservative
//!
//! Xenith resolves names dynamically: a method body is evaluated against the
//! scope of whoever called it, so a method can legally read a variable that is
//! not visible where the method was written, and one that is declared later in
//! the file. A lexical checker cannot know those types.
//!
//! So anything the checker cannot work out becomes [`Type::Unknown`], and
//! `Unknown` is compatible with everything. The rule is that a reported error
//! must be a real one. Missing a mistake is acceptable; inventing one is not,
//! because a false positive makes the whole pass something people turn off.
//!
//! The interpreter keeps its own checks. This pass runs in front of them, it
//! does not replace them.

use std::collections::HashMap;

use crate::entry::ProgramShape;
use crate::error::Error;
use crate::nodes::*;
use crate::position::Position;
use crate::type_table::TypeTable;
use crate::types::Type;

/// What the checker knows about a name in scope.
#[derive(Debug, Clone)]
struct Binding {
    declared_type: Type,
    is_constant: bool,
}

/// A method signature, for checking calls.
#[derive(Debug, Clone)]
struct Signature {
    param_types: Vec<Type>,
    return_type: Type,
}

pub struct Checker {
    /// Innermost scope last.
    scopes: Vec<HashMap<String, Binding>>,
    structs: HashMap<String, Vec<(String, Type)>>,
    /// Variants of each enum declared in this file, with what each carries.
    /// An enum from another module is not in here, and a `match` on one is left
    /// alone rather than guessed at.
    enums: HashMap<String, Vec<(String, Vec<Type>)>>,
    aliases: HashMap<String, Type>,
    methods: HashMap<String, Signature>,
    errors: Vec<Error>,
    /// Greater than zero while inside a method body, where an unresolved name
    /// may still be supplied by the caller's scope at run time.
    method_depth: usize,
    /// Declared result type of each enclosing method, for checking `release`.
    return_types: Vec<Type>,
    /// What each expression was inferred to be. See [`crate::type_table`].
    types: TypeTable,
    /// A program's top level holds declarations only, so every name is either
    /// declared before the first statement runs or is not declared at all.
    /// That makes an unresolved name reportable, which in a script it is not.
    shape: ProgramShape,
}

/// Checks a program, returning every error found and the types inferred.
pub fn check_typed(
    ast: &Node,
    aliases: &HashMap<String, Type>,
    node_count: u32,
    shape: ProgramShape,
) -> (Vec<Error>, TypeTable) {
    let mut checker = Checker::new(aliases.clone(), node_count, shape);
    checker.declare_builtins();
    checker.visit(ast);
    (checker.errors, checker.types)
}

/// Checks a program and returns every error found, in source order.
pub fn check(ast: &Node, aliases: &HashMap<String, Type>) -> Vec<Error> {
    // `0` because a caller that does not want the table does not have to
    // thread the parser's node count through to get one. `Script` because a
    // caller that has not said otherwise gets the conservative behaviour.
    check_typed(ast, aliases, 0, ProgramShape::Script).0
}

impl Checker {
    fn new(aliases: HashMap<String, Type>, node_count: u32, shape: ProgramShape) -> Self {
        Self {
            scopes: vec![HashMap::new()],
            structs: HashMap::new(),
            enums: HashMap::new(),
            aliases,
            methods: HashMap::new(),
            errors: Vec::new(),
            method_depth: 0,
            return_types: Vec::new(),
            types: TypeTable::with_capacity(node_count),
            shape,
        }
    }

    /// The names the interpreter installs into the global scope. Their types
    /// are mostly `Unknown` because the builtins are variadic or polymorphic in
    /// ways the type system cannot describe; what matters is that they resolve.
    fn declare_builtins(&mut self) {
        for builtin in crate::builtins::registry::BUILTIN_FUNCTIONS {
            self.declare(builtin.name, Type::Unknown, false);
        }
        self.declare("TRUE", Type::Bool, true);
        self.declare("FALSE", Type::Bool, true);
        self.declare("NULL", Type::Null, true);
        self.declare("MATH_PI", Type::Float, true);
    }

    // -- scopes ------------------------------------------------------------

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn declare(&mut self, name: &str, declared_type: Type, is_constant: bool) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(
                name.to_string(),
                Binding {
                    declared_type,
                    is_constant,
                },
            );
        }
    }

    fn lookup(&self, name: &str) -> Option<&Binding> {
        self.scopes.iter().rev().find_map(|scope| scope.get(name))
    }

    // -- types -------------------------------------------------------------

    fn resolve(&self, t: &Type) -> Type {
        match t {
            Type::Alias(name, inner) => match self.aliases.get(name) {
                Some(resolved) => self.resolve(resolved),
                None => self.resolve(inner),
            },
            other => other.clone(),
        }
    }

    /// Could a value of type `actual` be used where `expected` is wanted?
    ///
    /// `Unknown` on either side means the checker does not know enough, and the
    /// answer is yes. That is what keeps it from inventing errors.
    fn compatible(&self, expected: &Type, actual: &Type) -> bool {
        let expected = self.resolve(expected);
        let actual = self.resolve(actual);

        if matches!(expected, Type::Unknown) || matches!(actual, Type::Unknown) {
            return true;
        }

        match (&expected, &actual) {
            (Type::List(a), Type::List(b)) => self.compatible(a, b),
            (Type::Map(ak, av), Type::Map(bk, bv)) => {
                self.compatible(ak, bk) && self.compatible(av, bv)
            }
            (Type::Tuple(a), Type::Tuple(b)) => {
                a.len() == b.len()
                    && a.iter().zip(b.iter()).all(|(x, y)| self.compatible(x, y))
            }
            (Type::Struct(a, _), Type::Struct(b, _)) => a == b,
            (Type::Function(_), Type::Function(_)) => true,
            _ => expected == actual,
        }
    }

    fn error(&mut self, error: Error) {
        self.errors.push(error);
    }

    fn type_error(&mut self, expected: &Type, actual: &Type, start: &Position, end: &Position) {
        let error = Error::type_mismatch(
            &self.resolve(expected).to_string(),
            &self.resolve(actual).to_string(),
            start.clone(),
            end.clone(),
        );
        self.error(error);
    }

    // -- statements --------------------------------------------------------

    fn visit(&mut self, node: &Node) {
        match node {
            Node::List(n) => {
                for element in &n.element_nodes {
                    self.visit(element);
                }
            }

            Node::VarAssign(n) => self.check_var_assign(n),
            Node::FuncDef(n) => self.check_func_def(n),
            Node::StructDef(n) => self.check_struct_def(n),
            Node::EnumDef(n) => self.check_enum_def(n),

            Node::TypeAlias(n) => {
                if let Some(name) = n.name.value.clone() {
                    let resolved = self.resolve(&n.alias_type);
                    self.aliases.insert(name, resolved);
                }
            }

            Node::If(n) => {
                for (condition, body) in &n.cases {
                    self.infer(condition);
                    self.push_scope();
                    self.visit(body);
                    self.pop_scope();
                }
                if let Some((body, _)) = &n.else_case {
                    self.push_scope();
                    self.visit(body);
                    self.pop_scope();
                }
            }

            Node::While(n) => {
                self.infer(&n.condition_node);
                self.push_scope();
                self.visit(&n.body_node);
                self.pop_scope();
            }

            Node::ForClassic(n) => {
                self.push_scope();
                if let Some(init) = &n.init_node {
                    self.visit(init);
                }
                if let Some(condition) = &n.condition_node {
                    self.infer(condition);
                }
                if let Some(step) = &n.step_node {
                    self.visit(step);
                }
                self.push_scope();
                self.visit(&n.body_node);
                self.pop_scope();
                self.pop_scope();
            }

            Node::For(n) => {
                let iterable = self.infer(&n.iterable_node);
                self.push_scope();

                // `for k, v in map` packs both names into one token as "(k,v)".
                let raw = n.variable_name_token.value.clone().unwrap_or_default();
                let names: Vec<String> = raw
                    .trim_matches(['(', ')'])
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();

                match (names.len(), self.resolve(&iterable)) {
                    (1, Type::List(element)) => self.declare(&names[0], *element, false),
                    (1, Type::Map(key, _)) => self.declare(&names[0], *key, false),
                    (2, Type::Map(key, value)) => {
                        self.declare(&names[0], *key, false);
                        self.declare(&names[1], *value, false);
                    }
                    _ => {
                        for name in &names {
                            self.declare(name, Type::Unknown, false);
                        }
                    }
                }

                self.visit(&n.body_node);
                self.pop_scope();
            }

            Node::Return(n) => self.check_return(n),

            Node::Destructure(n) => {
                self.infer(&n.value_node);
                // Element types of a tuple pattern are not tracked yet, so the
                // names are introduced as Unknown rather than guessed at.
                self.declare_pattern(&n.patterns);
            }

            Node::Export(n) => self.visit(&n.node),
            Node::Panic(n) => {
                self.infer(&n.message_node);
            }

            // Anything else is an expression used as a statement.
            other => {
                self.infer(other);
            }
        }
    }

    fn declare_pattern(&mut self, patterns: &[DestructurePattern]) {
        for pattern in patterns {
            match pattern {
                DestructurePattern::Variable(token) => {
                    if let Some(name) = token.value.clone() {
                        self.declare(&name, Type::Unknown, false);
                    }
                }
                DestructurePattern::Tuple(nested) => self.declare_pattern(nested),
                DestructurePattern::Ignore => {}
            }
        }
    }

    fn check_var_assign(&mut self, node: &VarAssignNode) {
        let Some(name) = node.variable_name_token.value.clone() else {
            return;
        };
        let value_type = self.infer(&node.value_node);

        if node.is_declaration {
            let declared = match &node.var_type {
                Some(annotation) => {
                    let annotation = self.resolve(annotation);
                    if !self.compatible(&annotation, &value_type) {
                        self.type_error(
                            &annotation,
                            &value_type,
                            &node.position_start,
                            &node.position_end,
                        );
                    }
                    annotation
                }
                // No annotation, so the initialiser decides.
                None => value_type,
            };
            self.declare(&name, declared, node.is_constant);
            return;
        }

        // Reassignment. A name the checker cannot see may still be supplied by
        // the caller's scope at run time, so an unknown one is left alone.
        let Some(binding) = self.lookup(&name).cloned() else {
            return;
        };

        if binding.is_constant {
            self.error(
                crate::error::RuntimeError::new(
                    node.position_start.clone(),
                    node.position_end.clone(),
                    &format!("cannot reassign constant `{}`", name),
                    None,
                )
                .with_code("XEN018")
                .with_name("Constant Reassignment")
                .with_help("declare it with `let` instead of `const let` if it needs to change")
                .base,
            );
            return;
        }

        if !self.compatible(&binding.declared_type, &value_type) {
            self.type_error(
                &binding.declared_type,
                &value_type,
                &node.position_start,
                &node.position_end,
            );
        }
    }

    fn check_struct_def(&mut self, node: &StructDefNode) {
        let Some(name) = node.name.value.clone() else {
            return;
        };
        let fields = node
            .fields
            .iter()
            .filter_map(|field| {
                let field_name = field.name.value.clone()?;
                Some((field_name, self.resolve(&field.field_type)))
            })
            .collect();
        self.structs.insert(name, fields);
    }

    fn check_enum_def(&mut self, node: &EnumDefNode) {
        let Some(name) = node.name.value.clone() else {
            return;
        };
        let variants = node
            .variants
            .iter()
            .filter_map(|variant| {
                let variant_name = variant.name.value.clone()?;
                let payload = variant
                    .payload_types
                    .iter()
                    .map(|t| self.resolve(t))
                    .collect();
                Some((variant_name, payload))
            })
            .collect();
        self.enums.insert(name, variants);
    }

    /// The declared payload of one variant, or `None` when the checker cannot
    /// see the enum -- which is the normal case for an imported one.
    fn variant_payload(&self, enum_name: &str, variant_name: &str) -> Option<Vec<Type>> {
        self.enums
            .get(enum_name)?
            .iter()
            .find(|(name, _)| name == variant_name)
            .map(|(_, payload)| payload.clone())
    }

    fn unknown_variant_error(&mut self, enum_name: &str, variant_name: &str, start: &Position, end: &Position) {
        let known: Vec<&str> = self
            .enums
            .get(enum_name)
            .map(|variants| variants.iter().map(|(name, _)| name.as_str()).collect())
            .unwrap_or_default();

        self.error(
            crate::error::RuntimeError::new(
                start.clone(),
                end.clone(),
                &format!("enum `{}` has no variant `{}`", enum_name, variant_name),
                None,
            )
            .with_code("XEN009")
            .with_name("Variant Not Found")
            .with_help(&format!("it has {}", known.join(", ")))
            .base,
        );
    }

    fn arity_error(
        &mut self,
        enum_name: &str,
        variant_name: &str,
        expected: usize,
        got: usize,
        start: &Position,
        end: &Position,
    ) {
        let error = if got > expected {
            Error::too_many_arguments(expected, got, start.clone(), end.clone())
        } else {
            Error::too_few_arguments(expected, got, start.clone(), end.clone())
        };
        self.error(error.with_note(&format!(
            "variant `{}::{}` carries {}",
            enum_name,
            variant_name,
            crate::interpreter::describe_arity(expected)
        )));
    }

    fn infer_enum_variant(&mut self, node: &EnumVariantNode) -> Type {
        let Some(payload_types) = self.variant_payload(&node.enum_name, &node.variant_name) else {
            for argument in &node.arguments {
                self.infer(argument);
            }
            // A known enum with an unknown variant is a real mistake; an enum
            // the checker has never seen is left to the interpreter.
            if self.enums.contains_key(&node.enum_name) {
                self.unknown_variant_error(
                    &node.enum_name,
                    &node.variant_name,
                    &node.position_start,
                    &node.position_end,
                );
                return Type::Struct(node.enum_name.clone(), Vec::new());
            }
            return Type::Unknown;
        };

        if node.arguments.len() != payload_types.len() {
            for argument in &node.arguments {
                self.infer(argument);
            }
            self.arity_error(
                &node.enum_name,
                &node.variant_name,
                payload_types.len(),
                node.arguments.len(),
                &node.position_start,
                &node.position_end,
            );
            return Type::Struct(node.enum_name.clone(), Vec::new());
        }

        for (argument, expected) in node.arguments.iter().zip(&payload_types) {
            let actual = self.infer(argument);
            if !self.compatible(expected, &actual) {
                self.type_error(
                    expected,
                    &actual,
                    argument.position_start(),
                    argument.position_end(),
                );
            }
        }

        Type::Struct(node.enum_name.clone(), Vec::new())
    }

    // -- match ---------------------------------------------------------------

    fn infer_match(&mut self, node: &MatchNode) -> Type {
        let subject = self.infer(&node.subject);
        let subject = self.resolve(&subject);

        let mut result: Option<Type> = None;

        for arm in &node.arms {
            self.push_scope();

            for pattern in &arm.patterns {
                self.check_pattern(pattern, &subject);
            }

            if let Some(guard) = &arm.guard {
                self.infer(guard);
            }

            let body = self.infer_arm_body(&arm.body);
            self.pop_scope();

            // Every arm of an expression has to agree about what it produces.
            // Two arms with concrete, incompatible types is provably wrong, so
            // it is reported; anything the checker could not work out is not.
            match &result {
                None if !matches!(body, Type::Unknown) => result = Some(body),
                Some(expected) if !self.compatible(expected, &body) => {
                    let expected = expected.clone();
                    self.type_error(
                        &expected,
                        &body,
                        &arm.position_start,
                        &arm.position_end,
                    );
                }
                _ => {}
            }
        }

        self.check_exhaustive(node, &subject);

        result.unwrap_or(Type::Unknown)
    }

    /// An arm's body is one expression, or a block whose value is its last
    /// statement. `infer` on a block would call `infer_list` and report it as a
    /// list, which is what a block node means everywhere else.
    fn infer_arm_body(&mut self, body: &Node) -> Type {
        let Node::List(block) = body else {
            return self.infer(body);
        };

        let mut last = Type::Null;
        for (index, statement) in block.element_nodes.iter().enumerate() {
            if index + 1 == block.element_nodes.len() {
                last = self.infer(statement);
            } else {
                self.visit(statement);
            }
        }
        last
    }

    /// Checks a pattern against the type it will be matched on, and declares
    /// whatever it binds into the current scope.
    fn check_pattern(&mut self, pattern: &Pattern, expected: &Type) {
        match &pattern.kind {
            PatternKind::Wildcard => {}

            PatternKind::Binding(token) => {
                if let Some(name) = token.value.clone() {
                    self.declare(&name, expected.clone(), false);
                }
            }

            PatternKind::Literal(literal) => {
                let literal_type = self.infer(literal);
                // A pattern whose type cannot occur here can never match, which
                // is a mistake worth reporting rather than dead code to ignore.
                if !self.compatible(expected, &literal_type) {
                    self.type_error(
                        expected,
                        &literal_type,
                        &pattern.position_start,
                        &pattern.position_end,
                    );
                }
            }

            PatternKind::Tuple(elements) => {
                let element_types: Vec<Type> = match self.resolve(expected) {
                    Type::Tuple(types) if types.len() == elements.len() => types,
                    Type::Tuple(types) => {
                        self.error(
                            crate::error::RuntimeError::new(
                                pattern.position_start.clone(),
                                pattern.position_end.clone(),
                                &format!(
                                    "pattern has {} elements, the tuple has {}",
                                    elements.len(),
                                    types.len()
                                ),
                                None,
                            )
                            .with_code("XEN020")
                            .with_name("Destructuring Mismatch")
                            .base,
                        );
                        vec![Type::Unknown; elements.len()]
                    }
                    _ => vec![Type::Unknown; elements.len()],
                };

                for (element, element_type) in elements.iter().zip(element_types) {
                    self.check_pattern(element, &element_type);
                }
            }

            PatternKind::Variant {
                enum_name,
                variant_name,
                sub_patterns,
                has_parens,
            } => {
                // Matching one enum's variant against another enum's value can
                // never succeed. Only reported when both names are known.
                if let Type::Struct(subject_name, _) = expected {
                    if subject_name != enum_name
                        && self.enums.contains_key(subject_name.as_str())
                        && self.enums.contains_key(enum_name.as_str())
                    {
                        self.type_error(
                            expected,
                            &Type::Struct(enum_name.clone(), Vec::new()),
                            &pattern.position_start,
                            &pattern.position_end,
                        );
                        return;
                    }
                }

                let Some(payload) = self.variant_payload(enum_name, variant_name) else {
                    if self.enums.contains_key(enum_name.as_str()) {
                        self.unknown_variant_error(
                            enum_name,
                            variant_name,
                            &pattern.position_start,
                            &pattern.position_end,
                        );
                    }
                    // Unknown enum: still declare the bindings, as Unknown, so
                    // the arm body does not report them undefined.
                    let mut bound = Vec::new();
                    pattern.bindings(&mut bound);
                    for token in bound {
                        if let Some(name) = token.value.clone() {
                            self.declare(&name, Type::Unknown, false);
                        }
                    }
                    return;
                };

                if *has_parens && sub_patterns.len() != payload.len() {
                    self.arity_error(
                        enum_name,
                        variant_name,
                        payload.len(),
                        sub_patterns.len(),
                        &pattern.position_start,
                        &pattern.position_end,
                    );
                    for sub in sub_patterns {
                        self.check_pattern(sub, &Type::Unknown);
                    }
                    return;
                }

                for (sub, sub_type) in sub_patterns.iter().zip(payload) {
                    self.check_pattern(sub, &sub_type);
                }
            }
        }
    }

    /// Reports a match that could fall off the end.
    ///
    /// This is the reason enums are worth having: add a variant, and every
    /// match that does not handle it stops compiling. It only fires where the
    /// full set of cases is known, so an unresolved type is never complained
    /// about.
    fn check_exhaustive(&mut self, node: &MatchNode, subject: &Type) {
        // An arm with a guard proves nothing: whether it matches cannot be
        // decided by looking at it.
        let unguarded = || node.arms.iter().filter(|arm| arm.guard.is_none());

        let has_catch_all = unguarded()
            .any(|arm| arm.patterns.iter().any(|p| p.is_irrefutable()));
        if has_catch_all {
            return;
        }

        let missing: Vec<String> = match subject {
            Type::Struct(name, _) => {
                // Not an enum this pass can see. A struct needs a catch-all;
                // anything else is left alone, because an imported enum would
                // otherwise be wrongly accused of a hole it does not have.
                let variants = match self.enums.get(name).cloned() {
                    Some(variants) => variants,
                    None if self.structs.contains_key(name) => {
                        vec![("every case".to_string(), Vec::new())]
                    }
                    None => return,
                };

                let mut covered: Vec<&str> = Vec::new();
                for arm in unguarded() {
                    for pattern in &arm.patterns {
                        if let PatternKind::Variant {
                            enum_name,
                            variant_name,
                            sub_patterns,
                            ..
                        } = &pattern.kind
                        {
                            // `Circle(0.0)` does not cover `Circle`.
                            if enum_name == name
                                && sub_patterns.iter().all(|p| p.is_irrefutable())
                            {
                                covered.push(variant_name);
                            }
                        }
                    }
                }

                variants
                    .iter()
                    .map(|(variant, _)| variant.clone())
                    .filter(|variant| !covered.iter().any(|c| c == variant))
                    .collect()
            }

            // Two values, so both literals are a complete set.
            Type::Bool => {
                let mut seen_true = false;
                let mut seen_false = false;
                for arm in unguarded() {
                    for pattern in &arm.patterns {
                        if let PatternKind::Literal(literal) = &pattern.kind {
                            if let Node::BoolLiteral(b) = &**literal {
                                seen_true |= b.value;
                                seen_false |= !b.value;
                            }
                        }
                    }
                }
                match (seen_true, seen_false) {
                    (true, true) => Vec::new(),
                    (true, false) => vec!["false".to_string()],
                    (false, true) => vec!["true".to_string()],
                    (false, false) => vec!["true".to_string(), "false".to_string()],
                }
            }

            // An open set: there is no finite list of ints or strings to cover,
            // so a match on one always needs a catch-all.
            Type::Int | Type::Float | Type::String | Type::Bytes | Type::Null => {
                vec!["every other value".to_string()]
            }

            // Anything the checker could not pin down.
            _ => return,
        };

        if missing.is_empty() {
            return;
        }

        let detail = if missing.len() == 1 && missing[0].contains(' ') {
            format!("this match does not cover {}", missing[0])
        } else {
            format!(
                "this match does not cover {}",
                missing
                    .iter()
                    .map(|name| format!("`{}`", name))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };

        self.error(
            crate::error::RuntimeError::new(
                node.position_start.clone(),
                node.position_end.clone(),
                &detail,
                None,
            )
            .with_code("XEN022")
            .with_name("Match Not Exhaustive")
            .with_help("add the missing arms, or a `_` arm for everything else")
            .base,
        );
    }

    fn check_func_def(&mut self, node: &FuncDefNode) {
        let return_type = self.resolve(&node.return_type);
        let param_types: Vec<Type> = node.param_types.iter().map(|t| self.resolve(t)).collect();

        if let Some(token) = &node.variable_name_token {
            if let Some(name) = token.value.clone() {
                self.methods.insert(
                    name.clone(),
                    Signature {
                        param_types: param_types.clone(),
                        return_type: return_type.clone(),
                    },
                );
                self.declare(
                    &name,
                    Type::Function(crate::types::FunctionType {
                        param_types: param_types.clone(),
                        return_type: Box::new(return_type.clone()),
                    }),
                    false,
                );
            }
        }

        self.push_scope();
        self.method_depth += 1;
        self.return_types.push(return_type.clone());

        for (index, param) in node.param_names.iter().enumerate() {
            if let Some(param_name) = param.value.clone() {
                let param_type = param_types.get(index).cloned().unwrap_or(Type::Unknown);
                self.declare(&param_name, param_type, false);
            }
        }

        self.visit(&node.body_node);

        // An arrow method's body is its result, so it is checked here rather
        // than at a `release`.
        if node.is_arrow {
            let body_type = self.infer(&node.body_node);
            if !self.compatible(&return_type, &body_type) {
                self.type_error(
                    &return_type,
                    &body_type,
                    &node.position_start,
                    &node.position_end,
                );
            }
        }

        self.return_types.pop();
        self.method_depth -= 1;
        self.pop_scope();
    }

    fn check_return(&mut self, node: &ReturnNode) {
        let released = match &node.node_to_return {
            Some(value) => self.infer(value),
            None => Type::Null,
        };

        let Some(expected) = self.return_types.last().cloned() else {
            return;
        };

        if !self.compatible(&expected, &released) {
            self.type_error(
                &expected,
                &released,
                &node.position_start,
                &node.position_end,
            );
        }
    }

    // -- expressions -------------------------------------------------------

    /// Infers a node's type and records it.
    ///
    /// Everything that used to call `infer` still does; the recording is the
    /// only addition, which is what keeps this pass a single source of truth
    /// rather than logic a compiler would have to duplicate.
    fn infer(&mut self, node: &Node) -> Type {
        let inferred = self.infer_uncached(node);
        self.types.record(node.id(), inferred.clone());
        inferred
    }

    fn infer_uncached(&mut self, node: &Node) -> Type {
        match node {
            Node::Number(n) => match n.token.kind {
                crate::tokens::TokenType::Float => Type::Float,
                _ => Type::Int,
            },
            Node::String(_) => Type::String,

            // Each `{}` holds a real expression now that they are parsed at
            // parse time, so they are checked like any other.
            Node::InterpolatedString(n) => {
                for part in &n.parts {
                    if let Some(expression) = &part.parsed {
                        self.infer(expression);
                    }
                }
                Type::String
            }
            Node::BoolLiteral(_) => Type::Bool,
            Node::NullLiteral(_) => Type::Null,

            Node::VarAccess(n) => {
                let Some(name) = n.variable_name_token.value.as_ref() else {
                    return Type::Unknown;
                };
                match self.lookup(name) {
                    Some(binding) => binding.declared_type.clone(),
                    None => {
                        // In a program this is provably wrong: the top level
                        // holds declarations only, so nothing can appear in
                        // scope later than it is read. In a script it is not --
                        // a method body is evaluated against the caller's
                        // scope, which may supply the name -- so the checker
                        // stays quiet there and the interpreter reports it.
                        if self.shape == ProgramShape::Program {
                            let error = Error::undefined_variable(
                                name,
                                n.position_start.clone(),
                                n.position_end.clone(),
                            );
                            self.error(error);
                        }
                        // Unknown, not an error type: an unresolved name must
                        // not cascade into a second complaint about whatever
                        // expression contains it.
                        Type::Unknown
                    }
                }
            }

            Node::List(n) => self.infer_list(n),
            Node::Map(n) => self.infer_map(n),

            Node::TupleLiteral(n) => {
                let elements = n.elements.iter().map(|e| self.infer(e)).collect();
                Type::Tuple(elements)
            }

            Node::BinaryOperator(n) => self.infer_binary(n),

            Node::UnaryOp(n) => {
                let inner = self.infer(&n.node);
                if n.operator_token.matches(crate::tokens::TokenType::Keyword, Some("!")) {
                    Type::Bool
                } else {
                    inner
                }
            }

            Node::Ternary(n) => {
                self.infer(&n.condition);
                let a = self.infer(&n.true_expression);
                let b = self.infer(&n.false_expression);
                // Only commit to a type when both arms agree.
                if self.compatible(&a, &b) { a } else { Type::Unknown }
            }

            Node::Call(n) => self.infer_call(n),
            Node::EnumVariant(n) => self.infer_enum_variant(n),
            Node::Match(n) => self.infer_match(n),
            Node::StructInstantiation(n) => self.infer_struct_literal(n),
            Node::MethodAccess(n) => self.infer_field(n),

            Node::FuncDef(n) => {
                self.check_func_def(n);
                Type::Function(crate::types::FunctionType {
                    param_types: n.param_types.iter().map(|t| self.resolve(t)).collect(),
                    return_type: Box::new(self.resolve(&n.return_type)),
                })
            }

            // Statement shaped nodes that can appear in expression position.
            other => {
                self.visit_nested(other);
                Type::Unknown
            }
        }
    }

    /// Visits a node for its side effects without claiming to know its type.
    fn visit_nested(&mut self, node: &Node) {
        match node {
            Node::VarAssign(_)
            | Node::If(_)
            | Node::While(_)
            | Node::For(_)
            | Node::ForClassic(_)
            | Node::Return(_)
            | Node::StructDef(_)
            | Node::EnumDef(_)
            | Node::TypeAlias(_)
            | Node::Destructure(_)
            | Node::Export(_)
            | Node::Panic(_) => self.visit(node),
            _ => {}
        }
    }

    fn infer_list(&mut self, node: &ListNode) -> Type {
        let mut element_type: Option<Type> = None;

        for element in &node.element_nodes {
            let found = self.infer(element);
            element_type = Some(match element_type {
                None => found,
                Some(existing) if self.compatible(&existing, &found) => existing,
                // Mixed elements. The interpreter reports this against the
                // declared type, which gives a better message than anything
                // available here, so only the type is widened.
                Some(_) => return Type::List(Box::new(Type::Unknown)),
            });
        }

        Type::List(Box::new(element_type.unwrap_or(Type::Unknown)))
    }

    fn infer_map(&mut self, node: &MapNode) -> Type {
        let mut value_type: Option<Type> = None;

        for pair in &node.pairs {
            self.infer(&pair.key_node);
            let found = self.infer(&pair.value_node);
            value_type = Some(match value_type {
                None => found,
                Some(existing) if self.compatible(&existing, &found) => existing,
                Some(_) => return Type::Map(Box::new(Type::String), Box::new(Type::Unknown)),
            });
        }

        Type::Map(
            Box::new(Type::String),
            Box::new(value_type.unwrap_or(Type::Unknown)),
        )
    }

    fn infer_binary(&mut self, node: &BinaryOperatorNode) -> Type {
        use crate::tokens::TokenType;

        // Assignment is handled as a statement; here it only needs walking.
        if node.operator_token.kind == TokenType::Eq {
            self.infer(&node.right_node);
            return Type::Unknown;
        }

        // `value.field`, the binary shape. `call()` produces this one and
        // `try_parse_field_access` produces `MethodAccess`; both mean the same
        // thing and both need checking.
        if node.operator_token.kind == TokenType::Dot {
            let Node::VarAccess(field) = &*node.right_node else {
                return Type::Unknown;
            };
            let Some(field_name) = field.variable_name_token.value.clone() else {
                return Type::Unknown;
            };
            return self.check_field_access(
                &node.left_node,
                &field_name,
                &node.position_start,
                &node.position_end,
            );
        }

        if node.operator_token.kind == TokenType::Index {
            return self.infer_index(node);
        }

        // `as` carries its target type as a string on the right.
        if node.operator_token.matches(TokenType::Keyword, Some("as")) {
            self.infer(&node.left_node);
            if let Node::String(target) = &*node.right_node {
                return match target.token.value.as_deref() {
                    Some("int") => Type::Int,
                    Some("float") => Type::Float,
                    Some("string") => Type::String,
                    Some("bytes") => Type::Bytes,
                    Some("bool") => Type::Bool,
                    _ => Type::Unknown,
                };
            }
            return Type::Unknown;
        }

        let left = self.infer(&node.left_node);
        let right = self.infer(&node.right_node);

        let is_comparison = matches!(
            node.operator_token.kind,
            TokenType::Ee | TokenType::Ne | TokenType::Lt | TokenType::Gt | TokenType::Lte | TokenType::Gte
        );
        let is_logical = node.operator_token.matches(TokenType::Keyword, Some("&&"))
            || node.operator_token.matches(TokenType::Keyword, Some("||"));

        if is_logical {
            return Type::Bool;
        }

        let left = self.resolve(&left);
        let right = self.resolve(&right);

        if matches!(left, Type::Unknown) || matches!(right, Type::Unknown) {
            return if is_comparison { Type::Bool } else { Type::Unknown };
        }

        if is_comparison {
            if !self.compatible(&left, &right) {
                self.error(
                    crate::error::RuntimeError::new(
                        node.position_start.clone(),
                        node.position_end.clone(),
                        &format!(
                            "cannot compare {} with {}",
                            left.to_string(),
                            right.to_string()
                        ),
                        None,
                    )
                    .with_code("XEN001")
                    .with_name("Type Mismatch")
                    .with_help("both sides of a comparison must be the same type")
                    .base,
                );
            }
            return Type::Bool;
        }

        self.infer_arithmetic(node, &left, &right)
    }

    fn infer_arithmetic(
        &mut self,
        node: &BinaryOperatorNode,
        left: &Type,
        right: &Type,
    ) -> Type {
        use crate::tokens::TokenType;

        let operator = match node.operator_token.kind {
            TokenType::Plus => "add",
            TokenType::Minus => "subtract",
            TokenType::Mul => "multiply",
            TokenType::Div => "divide",
            TokenType::Mod => "take the remainder of",
            TokenType::Pow => "raise",
            _ => return Type::Unknown,
        };

        // These arms must stay a subset of what `Value`'s arithmetic accepts in
        // `src/values.rs`. Rejecting something the interpreter would have run is
        // the one failure this pass must not have, so when adding an operand
        // pair there, add it here too.
        match (left, right) {
            (Type::Int, Type::Int) => Type::Int,
            (Type::Float, Type::Float) => Type::Float,

            // `+` also joins strings, bytes and lists.
            (Type::String, Type::String) if node.operator_token.kind == TokenType::Plus => {
                Type::String
            }
            (Type::Bytes, Type::Bytes) if node.operator_token.kind == TokenType::Plus => {
                Type::Bytes
            }
            (Type::List(a), Type::List(b)) if node.operator_token.kind == TokenType::Plus => {
                if self.compatible(a, b) {
                    Type::List(a.clone())
                } else {
                    Type::List(Box::new(Type::Unknown))
                }
            }

            // `"ab" * 3` repeats a string.
            (Type::String, Type::Int) if node.operator_token.kind == TokenType::Mul => Type::String,

            _ => {
                let detail = if left.is_numeric() && right.is_numeric() {
                    format!("cannot {} int and float", operator)
                } else {
                    format!(
                        "cannot {} {} and {}",
                        operator,
                        left.to_string(),
                        right.to_string()
                    )
                };
                self.error(
                    crate::error::RuntimeError::new(
                        node.position_start.clone(),
                        node.position_end.clone(),
                        &detail,
                        None,
                    )
                    .with_code("XEN001")
                    .with_name("Type Mismatch")
                    .with_help("convert explicitly, e.g. `x as float`")
                    .base,
                );
                Type::Unknown
            }
        }
    }

    fn infer_index(&mut self, node: &BinaryOperatorNode) -> Type {
        let container = self.infer(&node.left_node);
        let container = self.resolve(&container);
        let index = self.infer(&node.right_node);
        let index = self.resolve(&index);

        match (&container, &index) {
            (Type::List(element), Type::Int) => (**element).clone(),
            (Type::List(_), Type::Unknown) | (Type::Unknown, _) => Type::Unknown,
            (Type::List(_), other) => {
                self.error(
                    crate::error::RuntimeError::new(
                        node.position_start.clone(),
                        node.position_end.clone(),
                        &format!("a list index must be an int, found {}", other.to_string()),
                        None,
                    )
                    .with_code("XEN004")
                    .with_name("Index Out of Bounds")
                    .base,
                );
                Type::Unknown
            }
            // One byte, as an int.
            (Type::Bytes, Type::Int) => Type::Int,
            (Type::Bytes, Type::Unknown) => Type::Unknown,
            (Type::Map(_, value), Type::String) => (**value).clone(),
            (Type::Map(_, _), Type::Unknown) => Type::Unknown,
            (Type::Map(_, _), other) => {
                self.error(
                    crate::error::RuntimeError::new(
                        node.position_start.clone(),
                        node.position_end.clone(),
                        &format!("a map key must be a string, found {}", other.to_string()),
                        None,
                    )
                    .with_code("XEN001")
                    .with_name("Type Mismatch")
                    .base,
                );
                Type::Unknown
            }
            _ => Type::Unknown,
        }
    }

    fn infer_call(&mut self, node: &CallNode) -> Type {
        for argument in &node.argument_nodes {
            self.infer(argument);
        }

        // A method called on a value, such as `xs.append(1)`. The receiver is
        // walked but the result type is not modelled.
        if let Node::MethodAccess(access) = &*node.node_to_call {
            self.infer(&access.object);
            return Type::Unknown;
        }

        let Node::VarAccess(callee) = &*node.node_to_call else {
            self.infer(&node.node_to_call);
            return Type::Unknown;
        };
        let Some(name) = callee.variable_name_token.value.clone() else {
            return Type::Unknown;
        };

        let Some(signature) = self.methods.get(&name).cloned() else {
            // Not a method the checker has seen: a builtin, a method value in a
            // variable, or something the caller's scope supplies.
            return Type::Unknown;
        };

        let expected = signature.param_types.len();
        let got = node.argument_nodes.len();

        if got != expected {
            let error = if got > expected {
                Error::too_many_arguments(
                    expected,
                    got,
                    node.position_start.clone(),
                    node.position_end.clone(),
                )
            } else {
                Error::too_few_arguments(
                    expected,
                    got,
                    node.position_start.clone(),
                    node.position_end.clone(),
                )
            };
            self.error(error);
            return signature.return_type;
        }

        for (argument, expected_type) in node.argument_nodes.iter().zip(&signature.param_types) {
            let actual = self.infer(argument);
            if !self.compatible(expected_type, &actual) {
                self.type_error(
                    expected_type,
                    &actual,
                    argument.position_start(),
                    argument.position_end(),
                );
            }
        }

        signature.return_type
    }

    fn infer_struct_literal(&mut self, node: &StructInstantiationNode) -> Type {
        let Some(declared) = self.structs.get(&node.struct_name).cloned() else {
            for (_, value) in &node.fields {
                self.infer(value);
            }
            return Type::Unknown;
        };

        let mut given: Vec<String> = Vec::new();

        for (field_token, value) in &node.fields {
            let actual = self.infer(value);
            let Some(field_name) = field_token.value.clone() else {
                continue;
            };
            given.push(field_name.clone());

            match declared.iter().find(|(name, _)| *name == field_name) {
                Some((_, expected)) => {
                    if !self.compatible(expected, &actual) {
                        self.type_error(
                            expected,
                            &actual,
                            &field_token.position_start,
                            &field_token.position_end,
                        );
                    }
                }
                None => {
                    let error = Error::field_not_found(
                        &node.struct_name,
                        &field_name,
                        field_token.position_start.clone(),
                        field_token.position_end.clone(),
                    );
                    self.error(error);
                }
            }
        }

        let missing: Vec<&str> = declared
            .iter()
            .map(|(name, _)| name.as_str())
            .filter(|name| !given.iter().any(|g| g == name))
            .collect();

        if !missing.is_empty() {
            let detail = format!(
                "struct `{}` is missing {}",
                node.struct_name,
                missing
                    .iter()
                    .map(|name| format!("`{}`", name))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            self.error(
                crate::error::RuntimeError::new(
                    node.position_start.clone(),
                    node.position_end.clone(),
                    &detail,
                    None,
                )
                .with_code("XEN009")
                .with_name("Missing Field")
                .with_help("every field must be given a value; there are no defaults")
                .base,
            );
        }

        Type::Struct(node.struct_name.clone(), Vec::new())
    }

    fn infer_field(&mut self, node: &MethodAccessNode) -> Type {
        let Some(field_name) = node.method_name.value.clone() else {
            return Type::Unknown;
        };
        self.check_field_access(
            &node.object,
            &field_name,
            &node.position_start,
            &node.position_end,
        )
    }

    /// Types `object.field`, reporting a field the struct does not declare.
    fn check_field_access(
        &mut self,
        object: &Node,
        field_name: &str,
        start: &Position,
        end: &Position,
    ) -> Type {
        let object_type = self.infer(object);
        let object_type = self.resolve(&object_type);

        let Type::Struct(struct_name, _) = &object_type else {
            // Maps, and anything the checker could not type, are left alone.
            return Type::Unknown;
        };

        let Some(declared) = self.structs.get(struct_name) else {
            return Type::Unknown;
        };

        match declared.iter().find(|(name, _)| name == field_name) {
            Some((_, field_type)) => field_type.clone(),
            None => {
                // A method call such as `value.len()` arrives here as a field
                // access, so only names that cannot be methods are reported.
                if !is_builtin_method(field_name) {
                    let error = Error::field_not_found(
                        struct_name,
                        field_name,
                        start.clone(),
                        end.clone(),
                    );
                    self.error(error);
                }
                Type::Unknown
            }
        }
    }
}

/// Names that are methods on a built in type rather than struct fields.
fn is_builtin_method(name: &str) -> bool {
    matches!(
        name,
        "len" | "append" | "pop" | "remove" | "keys" | "values" | "items" | "has_key"
    )
}
