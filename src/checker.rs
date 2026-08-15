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

use crate::error::Error;
use crate::nodes::*;
use crate::position::Position;
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
    aliases: HashMap<String, Type>,
    methods: HashMap<String, Signature>,
    errors: Vec<Error>,
    /// Greater than zero while inside a method body, where an unresolved name
    /// may still be supplied by the caller's scope at run time.
    method_depth: usize,
    /// Declared result type of each enclosing method, for checking `release`.
    return_types: Vec<Type>,
}

/// Checks a program and returns every error found, in source order.
pub fn check(ast: &Node, aliases: &HashMap<String, Type>) -> Vec<Error> {
    let mut checker = Checker::new(aliases.clone());
    checker.declare_builtins();
    checker.visit(ast);
    checker.errors
}

impl Checker {
    fn new(aliases: HashMap<String, Type>) -> Self {
        Self {
            scopes: vec![HashMap::new()],
            structs: HashMap::new(),
            aliases,
            methods: HashMap::new(),
            errors: Vec::new(),
            method_depth: 0,
            return_types: Vec::new(),
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

    fn infer(&mut self, node: &Node) -> Type {
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
                    // Not visible here. Inside a method that is legal, because
                    // the caller's scope may supply it; outside one the
                    // interpreter reports it, so either way the checker stays
                    // quiet and gives up on the type.
                    None => Type::Unknown,
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
