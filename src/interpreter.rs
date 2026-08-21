//interpreter.rs
//! # Interpreter Module
//!
//! Traverses the Abstract Syntax Tree and executes the program.
//! Implements the runtime semantics for each AST node type including
//! variable access, control flow, function calls, and built-in operations.

use crate::context::Context;
use crate::error::{Error, RuntimeError};
use crate::lexer::Lexer;
use crate::modules::{Module, ModuleError, ModuleRegistry};
use crate::nodes::{
    BoolLiteralNode, DestructureNode, DestructurePattern, Node, NullLiteralNode, PanicNode,
    StructDefNode, TupleLiteralNode, TypeAliasNode,
};
use crate::parser::Parser;
use crate::position::Position;
use crate::runtime_result::RuntimeResult;
use crate::symbol_table::{AssignOutcome, SymbolTable};
use crate::types::{FunctionType, Type};
use crate::utils::{value_to_interpolated_string, value_to_string};
use crate::values::{
    BuiltInFunction, Function, List, Map, Number, Value, XenithString,
};

use std::collections::HashMap;

/// Main interpreter that traverses and executes the AST
pub struct Interpreter {
    /// Global symbol table with built-in functions
    pub global_symbol_table: SymbolTable,
    /// Module registry for caching loaded modules
    pub module_registry: Option<ModuleRegistry>,
    pub struct_names: std::collections::HashSet<String>,
    /// Declared fields of each struct, in source order, used to check literals.
    pub struct_defs: HashMap<String, Vec<(String, Type)>>,
    /// Declared variants of each enum, in source order, with the types each
    /// carries. Used to check a variant is real and its payload is the right
    /// shape, and by `match` to report what a pattern got wrong.
    pub enum_defs: HashMap<String, Vec<(String, Vec<Type>)>>,
    /// Modules part way through loading, used to detect circular imports.
    pub loading_modules: Vec<String>,
    pub type_aliases: HashMap<String, Type>,
}

impl Interpreter {
    /// Creates a new interpreter with built-in functions initialized
    pub fn new() -> Self {
        let mut global = SymbolTable::new();

        // Built-in constants and functions come from one shared list so the
        // language server cannot advertise a name the interpreter no longer has.
        for constant in crate::builtins::registry::BUILTIN_CONSTANTS {
            let value = match constant.name {
                "NULL" => Value::Null,
                "TRUE" => Value::Bool(true),
                "FALSE" => Value::Bool(false),
                "MATH_PI" => Value::Number(Number::math_pi()),
                _ => continue,
            };
            global.set(constant.name.to_string(), value);
        }

        for (index, builtin) in crate::builtins::registry::BUILTIN_FUNCTIONS
            .iter()
            .enumerate()
        {
            global.set(
                builtin.name.to_string(),
                Value::BuiltInFunction(BuiltInFunction::at(index as u16)),
            );
        }

        Self {
            global_symbol_table: global,
            module_registry: None,
            struct_names: std::collections::HashSet::new(),
            struct_defs: HashMap::new(),
            enum_defs: HashMap::new(),
            loading_modules: Vec::new(),
            type_aliases: HashMap::new(),
        }
    }

    fn load_module(
        &mut self,
        module_path: &str,
        pos: &Position,
        context: &Context,
    ) -> Result<Module, ModuleError> {
        // Refuse a cycle rather than recursing until the process runs out of
        // stack. The list lives here rather than on the registry because
        // loading a module hands the registry out and back again, so a nested
        // `grab` would otherwise start from an empty one.
        if self.loading_modules.iter().any(|name| name == module_path) {
            let mut chain = self.loading_modules.clone();
            chain.push(module_path.to_string());
            return Err(ModuleError::Circular(chain));
        }

        // Initialize module registry if needed
        if self.module_registry.is_none() {
            self.module_registry = Some(ModuleRegistry::new(&pos.file_name));
        }

        self.loading_modules.push(module_path.to_string());

        // Take ownership of the registry temporarily
        let mut registry = self.module_registry.take().unwrap();
        let result = registry.load_module(module_path, self);
        self.module_registry = Some(registry);

        self.loading_modules.pop();

        result
    }

    /// Get module registry (for testing)
    pub fn get_module_registry(&self) -> Option<&ModuleRegistry> {
        self.module_registry.as_ref()
    }

    /// Visits a node and executes it
    pub fn visit(&mut self, node: &Node, context: &mut Context) -> RuntimeResult {
        match node {
            Node::Number(n) => self.visit_number(n, context),
            Node::String(n) => self.visit_string(n, context),
            Node::List(n) => self.visit_list(n, context),
            Node::Ternary(n) => self.visit_ternary(n, context),
            Node::VarAccess(n) => self.visit_var_access(n, context),
            Node::VarAssign(n) => self.visit_var_assign(n, context),
            Node::BinaryOperator(n) => self.visit_binary_op(n, context),
            Node::UnaryOp(n) => self.visit_unary_op(n, context),
            Node::If(n) => self.visit_if(n, context),
            Node::For(n) => self.visit_for(n, context),
            Node::ForClassic(n) => self.visit_for_classic(n, context),
            Node::While(n) => self.visit_while(n, context),
            Node::FuncDef(n) => self.visit_func_def(n, context),
            Node::Call(n) => self.visit_call(n, context),
            Node::Return(n) => self.visit_return(n, context),
            Node::Continue(n) => self.visit_continue(n, context),
            Node::Break(n) => self.visit_break(n, context),
            Node::InterpolatedString(n) => self.visit_interpolated_string(n, context),
            Node::MethodAccess(n) => self.visit_method_access(n, context),
            Node::Map(n) => self.visit_map(n, context),
            Node::Panic(n) => self.visit_panic(n, context),
            Node::Grab(n) => self.visit_grab(n, context),
            Node::Export(n) => self.visit_export(n, context),
            Node::StructDef(n) => self.visit_struct_def(n, context),
            Node::EnumDef(n) => self.visit_enum_def(n, context),
            Node::EnumVariant(n) => self.visit_enum_variant(n, context),
            Node::Match(n) => self.visit_match(n, context),
            Node::TypeAlias(n) => self.visit_type_alias(n, context),
            Node::BoolLiteral(n) => self.visit_bool_literal(n, context),
            Node::NullLiteral(n) => self.visit_null_literal(n, context),
            Node::StructInstantiation(n) => self.visit_struct_instantiation(n, context),
            Node::TupleLiteral(n) => self.visit_tuple_literal(n, context),
            Node::Destructure(n) => self.visit_destructure(n, context),
            Node::DestructurePattern(_) => {
                // DestructurePattern nodes are handled within destructuring
                RuntimeResult::new().success(Value::Null)
            }
        }
    }

    fn visit_struct_def(&mut self, node: &StructDefNode, context: &mut Context) -> RuntimeResult {
        // Store struct definition in the symbol table
        let struct_name = node.name.value.as_ref().unwrap().clone();

        // The declared fields, kept so literals can be checked against them.
        let mut field_types = Vec::new();
        for field in &node.fields {
            let field_name = field.name.value.as_ref().unwrap().clone();
            field_types.push((field_name, field.field_type.clone()));
        }

        self.struct_names.insert(struct_name.clone());
        self.struct_defs.insert(struct_name.clone(), field_types);
        context.symbol_table.set(
            struct_name.clone(),
            Value::string_of(XenithString::new(format!("__struct__{}", struct_name))),
        );

        RuntimeResult::new().success(Value::Null)
    }

    fn visit_enum_def(
        &mut self,
        node: &crate::nodes::EnumDefNode,
        context: &mut Context,
    ) -> RuntimeResult {
        let enum_name = node.name.value.as_ref().unwrap().clone();

        let mut variants = Vec::new();
        for variant in &node.variants {
            let variant_name = variant.name.value.as_ref().unwrap().clone();
            variants.push((variant_name, variant.payload_types.clone()));
        }

        self.enum_defs.insert(enum_name.clone(), variants);
        // The name resolves to something, so using it where it was never
        // declared is XEN002 rather than a silent miss. Same marker trick the
        // struct definitions use.
        context.symbol_table.set(
            enum_name.clone(),
            Value::string_of(XenithString::new(format!("__enum__{}", enum_name))),
        );

        RuntimeResult::new().success(Value::Null)
    }

    /// The declared payload types of one variant, or an error naming what went
    /// wrong. Shared by construction and by pattern matching, so an unknown
    /// enum or variant reads the same either way.
    fn lookup_variant(
        &self,
        enum_name: &str,
        variant_name: &str,
        start: &Position,
        end: &Position,
        context: &Context,
    ) -> Result<Vec<Type>, Error> {
        let Some(variants) = self.enum_defs.get(enum_name) else {
            return Err(RuntimeError::new(
                start.clone(),
                end.clone(),
                &format!("no enum named '{}' is in scope", enum_name),
                Some(context.clone()),
            )
            .with_code("XEN002")
            .with_name("Undefined Variable")
            .with_help("declare it with `enum`, or import it if it lives in another module")
            .base);
        };

        match variants.iter().find(|(name, _)| name == variant_name) {
            Some((_, payload_types)) => Ok(payload_types.clone()),
            None => {
                let known: Vec<&str> = variants.iter().map(|(name, _)| name.as_str()).collect();
                Err(RuntimeError::new(
                    start.clone(),
                    end.clone(),
                    &format!("enum '{}' has no variant '{}'", enum_name, variant_name),
                    Some(context.clone()),
                )
                .with_code("XEN009")
                .with_name("Variant Not Found")
                .with_help(&format!("it has {}", known.join(", ")))
                .base)
            }
        }
    }

    fn visit_enum_variant(
        &mut self,
        node: &crate::nodes::EnumVariantNode,
        context: &mut Context,
    ) -> RuntimeResult {
        let mut result = RuntimeResult::new();

        let payload_types = match self.lookup_variant(
            &node.enum_name,
            &node.variant_name,
            &node.position_start,
            &node.position_end,
            context,
        ) {
            Ok(types) => types,
            Err(error) => return result.failure(error),
        };

        if node.arguments.len() != payload_types.len() {
            let error = if node.arguments.len() > payload_types.len() {
                Error::too_many_arguments(
                    payload_types.len(),
                    node.arguments.len(),
                    node.position_start.clone(),
                    node.position_end.clone(),
                )
            } else {
                Error::too_few_arguments(
                    payload_types.len(),
                    node.arguments.len(),
                    node.position_start.clone(),
                    node.position_end.clone(),
                )
            };
            return result.failure(error.with_note(&format!(
                "variant `{}::{}` carries {}",
                node.enum_name,
                node.variant_name,
                describe_arity(payload_types.len())
            )));
        }

        let mut payload = Vec::with_capacity(node.arguments.len());
        for (index, argument) in node.arguments.iter().enumerate() {
            let value = result.register(self.visit(argument, context));
            if result.should_return() {
                return result;
            }

            let expected = self.resolve_type_alias(&payload_types[index]);
            if !Value::value_matches_type(&value, &expected) {
                return result.failure(
                    Error::type_mismatch(
                        &expected.to_string(),
                        &Value::get_type_name(&value),
                        argument.position_start().clone(),
                        argument.position_end().clone(),
                    )
                    .with_note(&format!(
                        "in `{}::{}`",
                        node.enum_name, node.variant_name
                    )),
                );
            }

            payload.push(value);
        }

        result.success(Value::Enum(Box::new(crate::values::EnumValue::new(
            node.enum_name.clone(),
            node.variant_name.clone(),
            payload,
        ))))
    }

    fn visit_struct_instantiation(
        &mut self,
        node: &crate::nodes::StructInstantiationNode,
        context: &mut Context,
    ) -> RuntimeResult {
        let mut result = RuntimeResult::new();
        let mut struct_instance = crate::values::Struct::new(node.struct_name.clone());

        let declared = self.struct_defs.get(&node.struct_name).cloned();

        for (field_name, value_node) in &node.fields {
            let value = result.register(self.visit(value_node, context));
            if result.should_return() {
                return result;
            }
            let name = field_name.value.as_ref().unwrap().clone();

            // Reject unknown fields, and values whose type does not match what
            // the struct declared. Without this a literal could invent fields
            // or hold anything at all, and the mistake only surfaced much later
            // as a confusing read.
            if let Some(fields) = &declared {
                match fields.iter().find(|(declared_name, _)| *declared_name == name) {
                    Some((_, expected)) => {
                        let expected = self.resolve_type_alias(expected);
                        if !Value::value_matches_type(&value, &expected) {
                            return result.failure(
                                Error::type_mismatch(
                                    &expected.to_string(),
                                    &Value::get_type_name(&value),
                                    field_name.position_start.clone(),
                                    field_name.position_end.clone(),
                                )
                                .with_note(&format!(
                                    "field `{}` of struct `{}`",
                                    name, node.struct_name
                                )),
                            );
                        }
                    }
                    None => {
                        return result.failure(Error::field_not_found(
                            &node.struct_name,
                            &name,
                            field_name.position_start.clone(),
                            field_name.position_end.clone(),
                        ));
                    }
                }
            }

            struct_instance.set_field(name, value);
        }

        // Every declared field has to be given a value; there are no defaults.
        if let Some(fields) = &declared {
            let missing: Vec<&str> = fields
                .iter()
                .map(|(name, _)| name.as_str())
                .filter(|name| !struct_instance.fields.contains_key(*name))
                .collect();

            if !missing.is_empty() {
                return result.failure(
                    RuntimeError::new(
                        node.position_start.clone(),
                        node.position_end.clone(),
                        &format!(
                            "struct `{}` is missing {}",
                            node.struct_name,
                            missing
                                .iter()
                                .map(|name| format!("`{}`", name))
                                .collect::<Vec<_>>()
                                .join(", ")
                        ),
                        Some(context.clone()),
                    )
                    .with_code("XEN009")
                    .with_name("Missing Field")
                    .with_help("every field must be given a value; there are no defaults")
                    .base,
                );
            }
        }

        result.success(Value::Struct(Box::new(struct_instance)))
    }


    fn visit_match(
        &mut self,
        node: &crate::nodes::MatchNode,
        context: &mut Context,
    ) -> RuntimeResult {
        let mut result = RuntimeResult::new();

        let subject = result.register(self.visit(&node.subject, context));
        if result.should_return() {
            return result;
        }

        for arm in &node.arms {
            for pattern in &arm.patterns {
                let mut bindings = Vec::new();
                match self.pattern_matches(pattern, &subject, &mut bindings, context) {
                    Err(error) => return result.failure(error),
                    Ok(false) => continue,
                    Ok(true) => {}
                }

                // The arm's bindings are visible to its guard as well as its
                // body, which is the whole point of `Circle(r) when r > 10.0`.
                let mut arm_ctx = context.create_child("<match>", arm.position_start.clone());
                for (name, value) in bindings {
                    arm_ctx.symbol_table.set_local(name, value);
                }

                if let Some(guard) = &arm.guard {
                    let passed = result.register(self.visit(guard, &mut arm_ctx));
                    if result.should_return() {
                        return result;
                    }
                    if !passed.is_true() {
                        continue;
                    }
                }

                let value = result.register(self.visit_arm_body(&arm.body, &mut arm_ctx));
                if result.should_return() {
                    return result;
                }
                return result.success(value);
            }
        }

        // The checker proves this cannot happen for an enum whose variants it
        // can see. It still can for a match on an int or a string with no
        // catch-all, and for an enum the checker could not resolve.
        result.failure(
            RuntimeError::new(
                node.position_start.clone(),
                node.position_end.clone(),
                &format!("no arm matched {}", value_to_string(&subject)),
                Some(context.clone()),
            )
            .with_code("XEN023")
            .with_name("No Matching Case")
            .with_help("add an arm for it, or a `_` arm for everything else")
            .base,
        )
    }

    /// An arm's body is either one expression or a block.
    ///
    /// A block's value is its last statement's, rather than the list of every
    /// statement's value that a `when` body produces. A match is an expression,
    /// so its arms have to be worth something.
    fn visit_arm_body(&mut self, body: &Node, context: &mut Context) -> RuntimeResult {
        let Node::List(block) = body else {
            return self.visit(body, context);
        };

        let mut result = RuntimeResult::new();
        let mut last = Value::Null;
        for statement in &block.element_nodes {
            last = result.register(self.visit(statement, context));
            if result.should_return() {
                return result;
            }
        }
        result.success(last)
    }

    /// Does `value` match `pattern`? Collects the names it binds along the way.
    ///
    /// Bindings are collected rather than written straight into a scope because
    /// a pattern that fails half way through must not leave the names it did
    /// match behind for the next arm to see.
    fn pattern_matches(
        &mut self,
        pattern: &crate::nodes::Pattern,
        value: &Value,
        bindings: &mut Vec<(String, Value)>,
        context: &mut Context,
    ) -> Result<bool, Error> {
        use crate::nodes::PatternKind;

        match &pattern.kind {
            PatternKind::Wildcard => Ok(true),

            PatternKind::Binding(token) => {
                if let Some(name) = token.value.clone() {
                    bindings.push((name, value.clone()));
                }
                Ok(true)
            }

            PatternKind::Literal(literal) => {
                let expected = {
                    let outcome = self.visit(literal, context);
                    if let Some(error) = outcome.error {
                        return Err(*error);
                    }
                    outcome.value.unwrap_or(Value::Null)
                };
                match value.equals(&expected) {
                    Ok(Value::Bool(same)) => Ok(same),
                    // Comparing values of different types is not an error here,
                    // it is simply a pattern that does not match.
                    _ => Ok(false),
                }
            }

            PatternKind::Tuple(elements) => {
                let Value::Tuple(values) = value else {
                    return Ok(false);
                };
                if values.len() != elements.len() {
                    return Ok(false);
                }
                for (element, value) in elements.iter().zip(values.iter()) {
                    if !self.pattern_matches(element, value, bindings, context)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }

            PatternKind::Variant {
                enum_name,
                variant_name,
                sub_patterns,
                has_parens,
            } => {
                // A pattern naming an enum or variant that does not exist is a
                // mistake in the pattern, not a value that failed to match, so
                // it is reported even when the subject is a different variant.
                let payload_types = self.lookup_variant(
                    enum_name,
                    variant_name,
                    &pattern.position_start,
                    &pattern.position_end,
                    context,
                )?;

                if *has_parens && sub_patterns.len() != payload_types.len() {
                    return Err(RuntimeError::new(
                        pattern.position_start.clone(),
                        pattern.position_end.clone(),
                        &format!(
                            "pattern binds {} for `{}::{}`, which carries {}",
                            describe_arity(sub_patterns.len()),
                            enum_name,
                            variant_name,
                            describe_arity(payload_types.len())
                        ),
                        Some(context.clone()),
                    )
                    .with_code("XEN020")
                    .with_name("Destructuring Mismatch")
                    .base);
                }

                let Value::Enum(actual) = value else {
                    return Ok(false);
                };
                if &actual.enum_name != enum_name || &actual.variant != variant_name {
                    return Ok(false);
                }

                // `Empty` without parentheses matches whatever it carries; that
                // is only reachable for a variant with no payload anyway.
                if !has_parens {
                    return Ok(true);
                }

                for (sub, value) in sub_patterns.iter().zip(actual.payload.clone().iter()) {
                    if !self.pattern_matches(sub, value, bindings, context)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
        }
    }

    fn visit_type_alias(&mut self, node: &TypeAliasNode, context: &mut Context) -> RuntimeResult {
        let alias_name = node.name.value.as_ref().unwrap().clone();
        let alias_type = node.alias_type.clone();

        // Resolve the alias type immediately and store the resolved type
        let resolved_type = self.resolve_type_alias(&alias_type);
        self.type_aliases.insert(alias_name, resolved_type);

        RuntimeResult::new().success(Value::Null)
    }

    /// Perform type checking without executing
    pub fn type_check(&mut self, node: &Node, context: &mut Context) -> Result<Type, Error> {
        match node {
            Node::Number(_) => Ok(Type::Float), // or Int, but Float is safer for unions
            Node::String(_) => Ok(Type::String),
            Node::BoolLiteral(_) => Ok(Type::Bool),
            Node::NullLiteral(_) => Ok(Type::Null),

            Node::List(list_node) => {
                // ... keep your existing list logic
                if let Some(first) = list_node.element_nodes.first() {
                    let elem_type = self.type_check(first, context)?;
                    Ok(Type::List(Box::new(elem_type)))
                } else {
                    Ok(Type::List(Box::new(Type::Unknown)))
                }
            }

            Node::VarAccess(node) => {
                let var_name = node.variable_name_token.value.as_ref().unwrap();
                if let Some(typ) = context.symbol_table.get_declared_type(var_name) {
                    Ok(typ)
                } else if let Some(typ) = self.global_symbol_table.get_declared_type(var_name) {
                    Ok(typ)
                } else {
                    Err(Error::undefined_variable(
                        var_name,
                        node.position_start.clone(),
                        node.position_end.clone(),
                    ))
                }
            }

            Node::FuncDef(node) => Ok(Type::Function(FunctionType {
                param_types: node.param_types.clone(),
                return_type: Box::new(node.return_type.clone()),
            })),

            _ => Ok(Type::Unknown),
        }
    }

    fn visit_null_literal(
        &mut self,
        node: &NullLiteralNode,
        _context: &mut Context,
    ) -> RuntimeResult {
        RuntimeResult::new().success(Value::Null)
    }

    fn visit_bool_literal(
        &mut self,
        node: &BoolLiteralNode,
        _context: &mut Context,
    ) -> RuntimeResult {
        RuntimeResult::new().success(Value::Bool(node.value))
    }

    fn visit_export(
        &mut self,
        node: &crate::nodes::ExportNode,
        context: &mut Context,
    ) -> RuntimeResult {
        // Execute the inner node
        let inner_result = self.visit(&node.node, context);
        if inner_result.error.is_some() {
            return inner_result;
        }

        // A struct is a type, not a value, so there is nothing useful in
        // `inner_result` to export. What an importer needs is the field list,
        // which running the definition has just put in `struct_defs`.
        if matches!(&*node.node, Node::StructDef(_)) {
            if let Some(fields) = self.struct_defs.get(&node.exported_name) {
                context.add_struct_export(node.exported_name.clone(), fields.clone());
            }
            return inner_result;
        }

        // Same for an enum, and for the same reason.
        if matches!(&*node.node, Node::EnumDef(_)) {
            if let Some(variants) = self.enum_defs.get(&node.exported_name) {
                context.add_enum_export(node.exported_name.clone(), variants.clone());
            }
            return inner_result;
        }

        // Mark the value as exported in the current context's module exports
        if let Some(value) = &inner_result.value {
            // Store in a special "exports" table in the context
            // We'll need to add an exports field to Context
            context.add_export(node.exported_name.clone(), value.clone());
        }

        inner_result
    }

    /// Turns a module load failure into the error the user sees.
    ///
    /// Each kind gets its own code. A failure *inside* a module is reported as
    /// itself, with its own position in the module's file, rather than being
    /// flattened into "module not found": a type error in an imported file is
    /// not a missing file, and saying so sends people looking in the wrong
    /// place.
    fn module_failure(
        failure: ModuleError,
        node: &crate::nodes::GrabNode,
        context: &Context,
    ) -> Error {
        match failure {
            ModuleError::NotFound(module) => RuntimeError::new(
                node.position_start.clone(),
                node.position_end.clone(),
                &format!("Module '{}' not found", module),
                Some(context.clone()),
            )
            .with_code("XEN012")
            .with_name("Module Not Found")
            .with_help("check the path is relative to this file, and that the file exists")
            .base,

            ModuleError::Unreadable(module, reason) => RuntimeError::new(
                node.position_start.clone(),
                node.position_end.clone(),
                &format!("could not read module '{}': {}", module, reason),
                Some(context.clone()),
            )
            .with_code("XEN012")
            .with_name("Module Not Found")
            .base,

            ModuleError::Circular(chain) => RuntimeError::new(
                node.position_start.clone(),
                node.position_end.clone(),
                &format!(
                    "circular import: {}",
                    chain
                        .iter()
                        .map(|name| format!("'{}'", name))
                        .collect::<Vec<_>>()
                        .join(" imports ")
                ),
                Some(context.clone()),
            )
            .with_code("XEN021")
            .with_name("Circular Import")
            .with_help("break the cycle by moving the shared definitions into a third module")
            .base,

            // The module's own errors already carry its file and line. Only the
            // first is returned, because a `RuntimeResult` holds one, but the
            // count is worth saying so nobody fixes one and assumes it is done.
            ModuleError::Failed { module, mut errors } => {
                let count = errors.len();
                let mut first = errors.remove(0);
                let note = if count == 1 {
                    format!("from module '{}'", module)
                } else {
                    format!("from module '{}', which has {} errors", module, count)
                };
                first.note = Some(match first.note.take() {
                    Some(existing) => format!("{}; {}", note, existing),
                    None => note,
                });
                first
            }
        }
    }

    fn visit_grab(
        &mut self,
        node: &crate::nodes::GrabNode,
        context: &mut Context,
    ) -> RuntimeResult {
        let mut result = RuntimeResult::new();

        // Create module registry if not exists (store in interpreter)
        // We'll add a field to Interpreter for this
        let module_path = node.from_module.clone();

        // Load the module
        let module = match self.load_module(&module_path, &node.position_start, context) {
            Ok(m) => m,
            Err(failure) => return result.failure(Self::module_failure(failure, node, context)),
        };

        if node.is_namespace_import {
            // Import * as namespace
            if let Some(alias) = &node.namespace_alias {
                let mut namespace_map = Map::new();
                for (name, value) in &module.exports {
                    namespace_map.set(name.clone(), value.clone());
                }
                context
                    .symbol_table
                    .set(alias.clone(), Value::Map(Box::new(namespace_map)));
            }
        } else {
            // Import specific items
            for spec in &node.imports {
                let original_name = &spec.original_name;
                let target_name = spec.alias.as_ref().unwrap_or(original_name);

                if let Some(value) = module.exports.get(original_name) {
                    context.symbol_table.set(target_name.clone(), value.clone());
                } else if let Some(fields) = module.struct_exports.get(original_name) {
                    // A struct is identified by its name -- that is what makes
                    // one `User` the same type as another. Renaming it on the
                    // way in would produce a value the exporting module's own
                    // methods reject, so it is refused rather than half done.
                    if target_name != original_name {
                        return result.failure(
                            RuntimeError::new(
                                spec.position_start.clone(),
                                spec.position_end.clone(),
                                &format!(
                                    "struct '{}' cannot be renamed on import",
                                    original_name
                                ),
                                Some(context.clone()),
                            )
                            .with_code("XEN012")
                            .with_name("Module Not Found")
                            .with_help("import it under its own name; a struct is identified by that name, so a renamed one would not match the methods that take it")
                            .base,
                        );
                    }

                    self.struct_names.insert(original_name.clone());
                    self.struct_defs.insert(original_name.clone(), fields.clone());
                    context.symbol_table.set(
                        original_name.clone(),
                        Value::string_of(XenithString::new(format!("__struct__{}", original_name))),
                    );
                } else if let Some(variants) = module.enum_exports.get(original_name) {
                    // An enum is named the same way a struct is, so renaming it
                    // on import breaks in the same way and is refused likewise.
                    if target_name != original_name {
                        return result.failure(
                            RuntimeError::new(
                                spec.position_start.clone(),
                                spec.position_end.clone(),
                                &format!("enum '{}' cannot be renamed on import", original_name),
                                Some(context.clone()),
                            )
                            .with_code("XEN012")
                            .with_name("Module Not Found")
                            .with_help("import it under its own name; an enum is identified by that name, so a renamed one would not match the methods that take it")
                            .base,
                        );
                    }

                    self.enum_defs.insert(original_name.clone(), variants.clone());
                    context.symbol_table.set(
                        original_name.clone(),
                        Value::string_of(XenithString::new(format!("__enum__{}", original_name))),
                    );
                } else {
                    return result.failure(
                        RuntimeError::new(
                            spec.position_start.clone(),
                            spec.position_end.clone(),
                            &format!(
                                "'{}' is not exported from module '{}'",
                                original_name, module_path
                            ),
                            Some(context.clone()),
                        )
                        .with_code("XEN012")
                        .with_name("Module Not Found")
                        .with_help("mark the definition with `export` in the module")
                        .base,
                    );
                }
            }
        }

        result.success(Value::Null)
    }

    fn visit_number(
        &mut self,
        node: &crate::nodes::NumberNode,
        _context: &mut Context,
    ) -> RuntimeResult {
        let text = node.token.value.as_ref().unwrap();
        if text.contains('.') || text.contains('e') || text.contains('E') {
            match text.parse::<f64>() {
                Ok(f) => RuntimeResult::new().success(Value::float(f)),
                Err(_) => RuntimeResult::new().failure(
                    RuntimeError::new(
                        node.position_start.clone(),
                        node.position_end.clone(),
                        &format!("`{}` is not a valid float", text),
                        None,
                    )
                    .base,
                ),
            }
        } else {
            match text.parse::<i64>() {
                Ok(i) => RuntimeResult::new().success(Value::int(i)),
                Err(_) => RuntimeResult::new().failure(
                    RuntimeError::new(
                        node.position_start.clone(),
                        node.position_end.clone(),
                        &format!(
                            "`{}` does not fit in an int (range {} to {})",
                            text,
                            i64::MIN,
                            i64::MAX
                        ),
                        None,
                    )
                    .with_code("XEN017")
                    .with_name("Integer Overflow")
                    .with_help("use a float if the value needs to be this large")
                    .base,
                ),
            }
        }
    }

    fn visit_string(
        &mut self,
        node: &crate::nodes::StringNode,
        _context: &mut Context,
    ) -> RuntimeResult {
        let value = node.token.value.as_ref().unwrap();
        RuntimeResult::new().success(Value::string_of(XenithString::new(value.clone())))
    }

    fn visit_list(
        &mut self,
        node: &crate::nodes::ListNode,
        context: &mut Context,
    ) -> RuntimeResult {
        let mut result = RuntimeResult::new();
        let mut elements = Vec::new();

        for elem_node in &node.element_nodes {
            let elem_result = self.visit(elem_node, context);

            // Check if this result has a caught error (panic)
            if false {
                return elem_result;
            }

            let elem = result.register(elem_result);
            if result.should_return() {
                return result;
            }
            elements.push(elem);
        }

        // Return the list, not the last value!
        result.success(Value::List(List::new(elements)))
    }

    fn visit_map(&mut self, node: &crate::nodes::MapNode, context: &mut Context) -> RuntimeResult {
        let mut result = RuntimeResult::new();
        let mut map = Map::new();

        for pair in &node.pairs {
            let key_node = &pair.key_node;
            let value_node = &pair.value_node;

            // Evaluate key (should be a string)
            let key_value = result.register(self.visit(key_node, context));
            if result.should_return() {
                return result;
            }

            let key_str = match &key_value {
                Value::String(s) => s.value.clone(),
                _ => {
                    return result.failure(Error::type_mismatch(
                        "string",
                        "non-string",
                        pair.position_start.clone(),
                        pair.position_end.clone(),
                    ));
                }
            };

            // Evaluate value
            let value = result.register(self.visit(value_node, context));
            if result.should_return() {
                return result;
            }

            map.set(key_str, value);
        }

        result.success(Value::Map(Box::new(map)))
    }

    fn visit_ternary(
        &mut self,
        node: &crate::nodes::TernaryNode,
        context: &mut Context,
    ) -> RuntimeResult {
        let mut result = RuntimeResult::new();

        let condition = result.register(self.visit(&node.condition, context));
        if result.should_return() {
            return result;
        }

        let value = if condition.is_true() {
            result.register(self.visit(&node.true_expression, context))
        } else {
            result.register(self.visit(&node.false_expression, context))
        };

        if result.should_return() {
            return result;
        }

        result.success(value)
    }

    fn visit_var_access(
        &mut self,
        node: &crate::nodes::VarAccessNode,
        context: &mut Context,
    ) -> RuntimeResult {
        let var_name = node.variable_name_token.value.as_ref().unwrap();

        // Go straight to where this name was last found. The name at that slot
        // is checked, so a stale position misses and falls through to a proper
        // lookup rather than reading the wrong variable.
        if let Some((hops, slot)) = node.cache.get().get() {
            if let Some(value) = context.symbol_table.get_slot(hops, slot, var_name) {
                return RuntimeResult::new().success(value);
            }
        }

        if let Some((hops, slot, value)) = context.symbol_table.locate(var_name) {
            node.cache.set(crate::nodes::SlotCache::set(hops, slot));
            return RuntimeResult::new().success(value);
        }

        match self.global_symbol_table.get(var_name) {
            Some(value) => RuntimeResult::new().success(value),
            None => RuntimeResult::new().failure(Error::undefined_variable(
                var_name,
                node.position_start.clone(),
                node.position_end.clone(),
            )),
        }
    }

    fn visit_var_assign(
        &mut self,
        node: &crate::nodes::VarAssignNode,
        context: &mut Context,
    ) -> RuntimeResult {
        let mut result = RuntimeResult::new();
        let var_name = node.variable_name_token.value.as_ref().unwrap();

        let value = result.register(self.visit(&node.value_node, context));
        if result.should_return() {
            return result;
        }

        if node.is_declaration {
            // `let x: T = v` / `let x = v` -- always binds in the current scope
            let declared_type = match &node.var_type {
                Some(t) => {
                    let resolved = self.resolve_type_alias(t);
                    if !self.value_matches_type(&value, &resolved) {
                        return RuntimeResult::new().failure(Error::type_mismatch(
                            &t.to_string(),
                            &Self::get_type_name(&value),
                            node.position_start.clone(),
                            node.position_end.clone(),
                        ));
                    }
                    resolved
                }
                // No annotation: infer from the value
                None => Self::infer_type(&value),
            };

            context
                .symbol_table
                .set_with_type(var_name.clone(), value.clone(), declared_type);
            if node.is_constant {
                context.symbol_table.mark_constant(var_name.clone());
            }
            return result.success(value);
        }

        // `x = v` -- updates an existing binding rather than shadowing it.
        //
        // Find, check and store in one walk of the scope chain. This is the
        // hottest statement in most programs, so nothing here clones what it
        // does not have to: the value is moved into the table, and the declared
        // type is cloned only to build an error message.
        // `get_type_name` is only wanted for an error message, so it is taken
        // from the outcome rather than computed on every assignment.
        let matches = |value: &Value, declared: &Type| self.value_matches_type(value, declared);

        // Straight to the remembered slot, exactly as reads do. The name there
        // is verified, so a stale position falls through to a full walk.
        let mut value = value;
        if let Some((hops, slot)) = node.cache.get().get() {
            match context
                .symbol_table
                .assign_slot(hops, slot, var_name, value, &matches)
            {
                Ok(outcome) => return Self::assignment_result(outcome, node, var_name, context),
                // Stale position; the value comes back untouched.
                Err(returned) => value = returned,
            }
        }

        let outcome = context.symbol_table.assign_checked(var_name, value, &matches);
        if matches!(outcome, AssignOutcome::Stored) {
            if let Some((hops, slot)) = context.symbol_table.locate_binding(var_name) {
                node.cache.set(crate::nodes::SlotCache::set(hops, slot));
            }
        }
        return Self::assignment_result(outcome, node, var_name, context);
    }

    /// Turns an assignment outcome into the result the interpreter returns.
    fn assignment_result(
        outcome: AssignOutcome,
        node: &crate::nodes::VarAssignNode,
        var_name: &str,
        context: &Context,
    ) -> RuntimeResult {
        let result = RuntimeResult::new();

        match outcome {
            AssignOutcome::Stored => result.success(Value::Null),

            AssignOutcome::NotDeclared => RuntimeResult::new().failure(
                RuntimeError::new(
                    node.position_start.clone(),
                    node.position_end.clone(),
                    &format!("`{}` is not declared", var_name),
                    Some(context.clone()),
                )
                .with_code("XEN002")
                .with_name("Undefined Variable")
                .with_help(&format!("declare it first: `let {} = ...`", var_name))
                .base,
            ),

            AssignOutcome::Constant => RuntimeResult::new().failure(
                RuntimeError::new(
                    node.position_start.clone(),
                    node.position_end.clone(),
                    &format!("cannot reassign constant `{}`", var_name),
                    Some(context.clone()),
                )
                .with_code("XEN018")
                .with_name("Constant Reassignment")
                .with_help("declare it with `let` instead of `const let` if it needs to change")
                .base,
            ),

            AssignOutcome::TypeMismatch { expected, found } => {
                RuntimeResult::new().failure(Error::type_mismatch(
                    &expected.to_string(),
                    &found,
                    node.position_start.clone(),
                    node.position_end.clone(),
                ))
            }
        }
    }


    /// Infer a value's type
    fn infer_type(value: &Value) -> Type {
        match value {
            Value::Number(Number::Int(_)) => Type::Int,
            Value::Number(Number::Float(_)) => Type::Float,
            Value::String(_) => Type::String,
            Value::Bool(_) => Type::Bool,
            Value::Null => Type::Null,
            Value::List(l) => Type::List(Box::new(
                l.elements
                    .first()
                    .map(Self::infer_type)
                    .unwrap_or(Type::Unknown),
            )),
            Value::Map(m) => Type::Map(
                Box::new(Type::String),
                Box::new(
                    m.pairs
                        .values()
                        .next()
                        .map(Self::infer_type)
                        .unwrap_or(Type::Unknown),
                ),
            ),
            Value::Tuple(t) => Type::Tuple(t.iter().map(Self::infer_type).collect()),
            Value::Struct(st) => Type::Struct(st.name.clone(), Vec::new()),
            _ => Type::Unknown,
        }
    }

    /// Get a string name for a value's type
    fn get_type_name(value: &Value) -> String {
        Value::get_type_name(value)
    }

    /// Check if a value matches an expected type
    pub fn value_matches_type(&self, value: &Value, expected_type: &Type) -> bool {
        // `resolve_type_alias` rebuilds the type, which allocates for anything
        // compound. Most annotations contain no alias at all, and this runs on
        // every assignment, so the common case avoids it entirely.
        if !Self::contains_alias(expected_type) {
            return Value::value_matches_type(value, expected_type);
        }
        let resolved_type = self.resolve_type_alias(expected_type);
        Value::value_matches_type(value, &resolved_type)
    }

    /// Is there an alias anywhere inside this type that needs resolving?
    fn contains_alias(typ: &Type) -> bool {
        match typ {
            Type::Alias(_, _) => true,
            Type::List(inner) => Self::contains_alias(inner),
            Type::Map(k, v) => Self::contains_alias(k) || Self::contains_alias(v),
            Type::Tuple(types) => types.iter().any(Self::contains_alias),
            Type::Function(f) => {
                f.param_types.iter().any(Self::contains_alias)
                    || Self::contains_alias(&f.return_type)
            }
            Type::Struct(_, fields) => {
                fields.iter().any(|field| Self::contains_alias(&field.field_type))
            }
            _ => false,
        }
    }

    // Helper function to resolve type aliases
    fn resolve_type_alias(&self, typ: &Type) -> Type {
        match typ {
            Type::Alias(name, _) => {
                if let Some(resolved) = self.type_aliases.get(name) {
                    self.resolve_type_alias(resolved)
                } else {
                    typ.clone()
                }
            }
            Type::List(inner) => Type::List(Box::new(self.resolve_type_alias(inner))),
            Type::Map(k, v) => Type::Map(
                Box::new(self.resolve_type_alias(k)),
                Box::new(self.resolve_type_alias(v)),
            ),
            Type::Tuple(types) => {
                let resolved: Vec<Type> =
                    types.iter().map(|t| self.resolve_type_alias(t)).collect();
                Type::Tuple(resolved)
            }
            Type::Function(f) => Type::Function(FunctionType {
                param_types: f
                    .param_types
                    .iter()
                    .map(|t| self.resolve_type_alias(t))
                    .collect(),
                return_type: Box::new(self.resolve_type_alias(&f.return_type)),
            }),
            _ => typ.clone(),
        }
    }


    /// Can this expression be assigned to? True for a variable, a field access
    /// or an index, and false for anything that produces a temporary.
    fn is_assignable(node: &Node) -> bool {
        match node {
            Node::VarAccess(_) => true,
            Node::MethodAccess(field) => Self::is_assignable(&field.object),
            Node::BinaryOperator(bin_op) => matches!(
                bin_op.operator_token.kind,
                crate::tokens::TokenType::Index | crate::tokens::TokenType::Dot
            ),
            _ => false,
        }
    }

    /// Sets one field on the value `object` evaluates to, then stores the
    /// updated object back where it came from.
    fn assign_field(
        &mut self,
        object: &Node,
        field_name: &str,
        value: Value,
        position_start: &Position,
        position_end: &Position,
        context: &mut Context,
    ) -> Option<Error> {
        let mut result = RuntimeResult::new();
        let current = result.register(self.visit(object, context));
        if let Some(error) = result.error {
            return Some(*error);
        }

        let updated = match current {
            Value::Struct(mut s) => {
                if !s.fields.contains_key(field_name) {
                    return Some(Error::field_not_found(
                        &s.name,
                        field_name,
                        position_start.clone(),
                        position_end.clone(),
                    ));
                }
                s.set_field(field_name.to_string(), value);
                Value::Struct(s)
            }
            Value::Map(mut m) => {
                m.set(field_name.to_string(), value);
                Value::Map(m)
            }
            other => {
                return Some(
                    RuntimeError::new(
                        position_start.clone(),
                        position_end.clone(),
                        &format!(
                            "cannot set field `{}` on {}",
                            field_name,
                            Value::get_type_name(&other)
                        ),
                        Some(context.clone()),
                    )
                    .with_code("XEN009")
                    .with_name("Field Not Found")
                    .base,
                );
            }
        };

        // Push the updated object back to wherever it came from, which may
        // itself be a field or an index.
        self.assign_into(object, updated, context)
    }

    /// Stores `value` at the location `target` names, and writes the change
    /// back through every container it is nested inside.
    ///
    /// Values are held by value, so `grid[1][2] = 9` cannot mutate in place:
    /// the inner list is updated, then stored back into the outer list, which
    /// is stored back into the variable. Recursing over the target expression
    /// makes that unwinding fall out for free at any depth.
    ///
    /// Returns `Some(error)` on failure, `None` on success.
    fn assign_into(
        &mut self,
        target: &Node,
        value: Value,
        context: &mut Context,
    ) -> Option<Error> {
        match target {
            // Base case: a plain variable.
            Node::VarAccess(var_node) => {
                let name = var_node.variable_name_token.value.as_ref()?;

                if context.symbol_table.is_constant(name) {
                    return Some(
                        RuntimeError::new(
                            var_node.position_start.clone(),
                            var_node.position_end.clone(),
                            &format!("cannot reassign constant `{}`", name),
                            Some(context.clone()),
                        )
                        .with_code("XEN018")
                        .with_name("Constant Reassignment")
                        .base,
                    );
                }

                if !context.symbol_table.assign_existing(name, value) {
                    return Some(Error::undefined_variable(
                        name,
                        var_node.position_start.clone(),
                        var_node.position_end.clone(),
                    ));
                }
                None
            }

            // `container[index] = value`
            Node::BinaryOperator(bin_op)
                if bin_op.operator_token.kind == crate::tokens::TokenType::Index =>
            {
                let mut result = RuntimeResult::new();

                // The container is lifted out of its variable rather than
                // copied out of it, for the reason given in `visit_call`: it
                // leaves this the only reference, so the copy-on-write below
                // has nothing to copy and `m[key] = value` costs the same
                // whatever the size of `m`. It goes back at the end of this
                // branch, and every path out in between is an error, which ends
                // the program.
                //
                // The index is evaluated first, because it may name the same
                // variable -- `xs[xs.len() - 1] = v` has to see `xs` rather
                // than the `Null` left in its place.
                let lifted_from = match &*bin_op.left_node {
                    Node::VarAccess(var) => var.variable_name_token.value.as_deref(),
                    _ => None,
                };

                let container = if lifted_from.is_none() {
                    let value = result.register(self.visit(&bin_op.left_node, context));
                    if let Some(error) = result.error {
                        return Some(*error);
                    }
                    Some(value)
                } else {
                    None
                };

                let index = result.register(self.visit(&bin_op.right_node, context));
                if let Some(error) = result.error {
                    return Some(*error);
                }

                let container = match container {
                    Some(value) => value,
                    None => match lifted_from.and_then(|name| context.symbol_table.take(name)) {
                        Some(value) => value,
                        None => {
                            let value = result.register(self.visit(&bin_op.left_node, context));
                            if let Some(error) = result.error {
                                return Some(*error);
                            }
                            value
                        }
                    },
                };

                let position_start = bin_op.position_start.clone();
                let position_end = bin_op.position_end.clone();
                let fail = |detail: &str| {
                    Some(
                        RuntimeError::new(
                            position_start.clone(),
                            position_end.clone(),
                            detail,
                            Some(context.clone()),
                        )
                        .with_code("XEN004")
                        .base,
                    )
                };

                let updated = match (container, index) {
                    (Value::List(mut list), Value::Number(n)) => {
                        let Some(slot) = n.as_index() else {
                            return fail("list index must be a non-negative int");
                        };
                        if !list.set(slot, value) {
                            return Some(Error::index_out_of_bounds(
                                slot,
                                list.elements.len(),
                                bin_op.position_start.clone(),
                                bin_op.position_end.clone(),
                            ));
                        }
                        Value::List(list)
                    }
                    // Assigning to a key that is absent inserts it, which is
                    // how a map gets built up after its literal.
                    (Value::Map(mut map), Value::String(key)) => {
                        map.set(key.value.clone(), value);
                        Value::Map(map)
                    }
                    (Value::List(_), other) => {
                        return fail(&format!(
                            "list index must be an int, found {}",
                            Value::get_type_name(&other)
                        ));
                    }
                    (Value::Map(_), other) => {
                        return fail(&format!(
                            "map key must be a string, found {}",
                            Value::get_type_name(&other)
                        ));
                    }
                    (other, _) => {
                        return fail(&format!(
                            "cannot index-assign into {}",
                            Value::get_type_name(&other)
                        ));
                    }
                };

                // Push the rebuilt container back to wherever it came from.
                self.assign_into(&bin_op.left_node, updated, context)
            }

            // `record.field = value`, at any depth
            Node::MethodAccess(field_node) => {
                let field_name = field_node.method_name.value.clone()?;
                self.assign_field(
                    &field_node.object,
                    &field_name,
                    value,
                    &field_node.position_start,
                    &field_node.position_end,
                    context,
                )
            }

            // The same thing written as a binary `.`, which is the other shape
            // the parser produces for field access.
            Node::BinaryOperator(bin_op)
                if bin_op.operator_token.kind == crate::tokens::TokenType::Dot =>
            {
                let Node::VarAccess(field) = &*bin_op.right_node else {
                    return Some(
                        RuntimeError::new(
                            bin_op.position_start.clone(),
                            bin_op.position_end.clone(),
                            "expected a field name after `.`",
                            Some(context.clone()),
                        )
                        .base,
                    );
                };
                let field_name = field.variable_name_token.value.clone()?;
                self.assign_field(
                    &bin_op.left_node,
                    &field_name,
                    value,
                    &bin_op.position_start,
                    &bin_op.position_end,
                    context,
                )
            }

            other => Some(
                RuntimeError::new(
                    other.position_start().clone(),
                    other.position_end().clone(),
                    "left side of `=` is not something that can be assigned to",
                    Some(context.clone()),
                )
                .base,
            ),
        }
    }

    fn visit_binary_op(
        &mut self,
        node: &crate::nodes::BinaryOperatorNode,
        context: &mut Context,
    ) -> RuntimeResult {
        let mut result = RuntimeResult::new();

        // Handle assignment separately before evaluating both sides
        if node.operator_token.kind == crate::tokens::TokenType::Eq {
            // Indexed assignment: `xs[0] = v`, `m["key"] = v`, `grid[1][2] = v`
            if let Node::BinaryOperator(bin_op) = &*node.left_node {
                if bin_op.operator_token.kind == crate::tokens::TokenType::Index {
                    let value = result.register(self.visit(&node.right_node, context));
                    if result.should_return() {
                        return result;
                    }

                    if let Some(error) = self.assign_into(&node.left_node, value.clone(), context) {
                        return RuntimeResult::new().failure(error);
                    }
                    return result.success(value);
                }
            }

            // The parser has two shapes for field access. This is the older
            // one, a binary `.` operator; `MethodAccess` below is the other.
            // Both go through `assign_into` so nesting works either way.
            if let Node::BinaryOperator(bin_op) = &*node.left_node {
                if bin_op.operator_token.kind == crate::tokens::TokenType::Dot {
                    let value = result.register(self.visit(&node.right_node, context));
                    if result.should_return() {
                        return result;
                    }

                    if let Some(error) = self.assign_into(&node.left_node, value.clone(), context) {
                        return RuntimeResult::new().failure(error);
                    }
                    return result.success(value);
                }
            }


            // `object.field = value`. `assign_into` walks the target
            // expression, so this works however deeply the field is nested:
            // `q.p.x`, `items[0].name`, `lookup["k"].count`. The version before
            // it only wrote back when the object was a bare variable, so
            // anything deeper was evaluated, updated, and then thrown away
            // without a word.
            if let Node::MethodAccess(_) = &*node.left_node {
                let right = result.register(self.visit(&node.right_node, context));
                if result.should_return() {
                    return result;
                }

                if let Some(error) = self.assign_into(&node.left_node, right.clone(), context) {
                    return RuntimeResult::new().failure(error);
                }
                return result.success(right);
            } else {
                // Plain variable assignment
                let right = result.register(self.visit(&node.right_node, context));
                if result.should_return() {
                    return result;
                }
                let var_name = if let Node::VarAccess(var_node) = &*node.left_node {
                    var_node.variable_name_token.value.as_ref().unwrap().clone()
                } else {
                    return result.failure(
                        RuntimeError::new(
                            node.position_start.clone(),
                            node.position_end.clone(),
                            "Invalid left-hand side in assignment",
                            Some(context.clone()),
                        )
                        .base,
                    );
                };
                context.symbol_table.set_existing(&var_name, right.clone());
                return result.success(right);
            }
        }

        // Handle struct field access before other operators
        if node.operator_token.kind == crate::tokens::TokenType::Dot {
            let left = result.register(self.visit(&node.left_node, context));
            if result.should_return() {
                return result;
            }

            // The right node should be an identifier (field name)
            let field_name = if let Node::VarAccess(var_node) = &*node.right_node {
                var_node.variable_name_token.value.as_ref().unwrap().clone()
            } else {
                return result.failure(
                    RuntimeError::new(
                        node.position_start.clone(),
                        node.position_end.clone(),
                        "Expected field name after '.'",
                        Some(context.clone()),
                    )
                    .base,
                );
            };

            // Access the field
            match left {
                Value::Struct(s) => {
                    if let Some(field_value) = s.get_field(&field_name) {
                        return result.success(field_value.clone());
                    } else {
                        return result.failure(Error::field_not_found(
                            &s.name,
                            &field_name,
                            node.position_start.clone(),
                            node.position_end.clone(),
                        ));
                    }
                }
                Value::Map(m) => {
                    if let Some(field_value) = m.get(&field_name) {
                        return result.success(field_value.clone());
                    } else {
                        return result.failure(
                            RuntimeError::new(
                                node.position_start.clone(),
                                node.position_end.clone(),
                                &format!("Map has no key '{}'", field_name),
                                Some(context.clone()),
                            )
                            .base,
                        );
                    }
                }
                _ => {
                    return result.failure(
                        RuntimeError::new(
                            node.position_start.clone(),
                            node.position_end.clone(),
                            "Cannot access field on non-struct/non-map value",
                            Some(context.clone()),
                        )
                        .base,
                    );
                }
            }
        }

        let left = result.register(self.visit(&node.left_node, context));
        if result.should_return() {
            return result;
        }

        // `&&` and `||` settle on the left where they can, and must not touch
        // the right side otherwise. Guards depend on it:
        //
        //     while i > 0 && is_space(text[i - 1]) { ... }
        //
        // Evaluating both sides first made that index out of bounds the moment
        // `i` reached 0, which is the shape of every loop that walks backwards
        // through a string.
        let is_and = node
            .operator_token
            .matches(crate::tokens::TokenType::Keyword, Some("&&"));
        let is_or = node
            .operator_token
            .matches(crate::tokens::TokenType::Keyword, Some("||"));

        if is_and || is_or {
            let decided_by_left = if is_and { !left.is_true() } else { left.is_true() };
            if decided_by_left {
                return result.success(Value::Bool(is_or));
            }

            let right = result.register(self.visit(&node.right_node, context));
            if result.should_return() {
                return result;
            }
            return result.success(Value::Bool(right.is_true()));
        }

        // Everything else needs both sides.
        let right = result.register(self.visit(&node.right_node, context));
        if result.should_return() {
            return result;
        }

        let op = &node.operator_token;

        let result_value = match op.kind {
            crate::tokens::TokenType::Plus => left.add(&right),
            crate::tokens::TokenType::Minus => left.subtract(&right),
            crate::tokens::TokenType::Mul => left.multiply(&right),
            crate::tokens::TokenType::Mod => left.modulo(&right),
            crate::tokens::TokenType::Div => {
                if let (Value::Number(_a), Value::Number(b)) = (&left, &right) {
                    if b.is_zero() {
                        return RuntimeResult::new().failure(Error::division_by_zero(
                            node.position_start.clone(),
                            node.position_end.clone(),
                        ));
                    }
                }
                left.divide(&right)
            }
            crate::tokens::TokenType::Pow => left.power(&right),
            crate::tokens::TokenType::Ee => left.equals(&right),
            crate::tokens::TokenType::Ne => left.not_equals(&right),
            crate::tokens::TokenType::Lt => left.less_than(&right),
            crate::tokens::TokenType::Gt => left.greater_than(&right),
            crate::tokens::TokenType::Lte => left.less_than_or_equal(&right),
            crate::tokens::TokenType::Gte => left.greater_than_or_equal(&right),
            crate::tokens::TokenType::Index => match (&left, &right) {
                (Value::List(list), Value::Number(idx)) => {
                    let Some(idx_usize) = idx.as_index() else {
                        return RuntimeResult::new().failure(
                            RuntimeError::new(
                                node.position_start.clone(),
                                node.position_end.clone(),
                                "list index must be a non-negative int",
                                None,
                            )
                            .with_code("XEN004")
                            .base,
                        );
                    };
                    if idx_usize >= list.elements.len() {
                        return RuntimeResult::new().failure(Error::index_out_of_bounds(
                            idx_usize,
                            list.elements.len(),
                            node.position_start.clone(),
                            node.position_end.clone(),
                        ));
                    }
                    Ok(list.elements[idx_usize].clone())
                }
                (Value::Map(map), Value::String(key)) => {
                    if let Some(value) = map.get(&key.value) {
                        Ok(value.clone())
                    } else {
                        Err(RuntimeError::new(
                            node.position_start.clone(),
                            node.position_end.clone(),
                            &format!("Key '{}' not found in map", key.value),
                            Some(context.clone()),
                        )
                        .base)
                    }
                }

                // `raw[i]` gives one byte, as an int in 0..=255.
                (Value::Bytes(raw), Value::Number(idx)) => {
                    let Some(position) = idx.as_index() else {
                        return RuntimeResult::new().failure(
                            RuntimeError::new(
                                node.position_start.clone(),
                                node.position_end.clone(),
                                "a bytes index must be a non-negative int",
                                None,
                            )
                            .with_code("XEN004")
                            .with_name("Index Out of Bounds")
                            .base,
                        );
                    };
                    match raw.data.get(position) {
                        Some(byte) => Ok(Value::int(*byte as i64)),
                        None => Err(Error::index_out_of_bounds(
                            position,
                            raw.len(),
                            node.position_start.clone(),
                            node.position_end.clone(),
                        )),
                    }
                }

                // `text[i]` gives the character at a position, as a one
                // character string. Counted in characters rather than bytes, to
                // agree with `.len()`, so indexing text with an accent in it
                // lands where a reader expects.
                //
                // Without this there is no way to reach into a string at all,
                // and every string function has to be written in Rust.
                (Value::String(text), Value::Number(idx)) => {
                    let Some(position) = idx.as_index() else {
                        return RuntimeResult::new().failure(
                            RuntimeError::new(
                                node.position_start.clone(),
                                node.position_end.clone(),
                                "a string index must be a non-negative int",
                                None,
                            )
                            .with_code("XEN004")
                            .with_name("Index Out of Bounds")
                            .base,
                        );
                    };
                    match text.char_at(position) {
                        Some(character) => Ok(Value::string_of(XenithString::new(character.to_string()))),
                        None => Err(Error::index_out_of_bounds(
                            position,
                            text.char_len(),
                            node.position_start.clone(),
                            node.position_end.clone(),
                        )),
                    }
                }

                _ => Err(RuntimeError::new(
                    node.position_start.clone(),
                    node.position_end.clone(),
                    &format!(
                        "cannot index {} with {}",
                        Value::get_type_name(&left),
                        Value::get_type_name(&right)
                    ),
                    Some(context.clone()),
                )
                .with_code("XEN004")
                .with_name("Index Out of Bounds")
                .with_help("a list takes an int, a map takes a string, and a string takes an int")
                .base),
            },
            _ if op.matches(crate::tokens::TokenType::Keyword, Some("&&")) => left.anded_by(&right),
            _ if op.matches(crate::tokens::TokenType::Keyword, Some("||")) => left.ored_by(&right),
            _ if op.matches(crate::tokens::TokenType::Keyword, Some("as")) => {
                let target = match &right {
                    Value::String(s) => s.value.clone(),
                    _ => String::new(),
                };
                let convert_err = |detail: String| {
                    RuntimeError::new(
                        node.position_start.clone(),
                        node.position_end.clone(),
                        &detail,
                        Some(context.clone()),
                    )
                    .with_code("XEN011")
                    .with_name("Invalid Type Conversion")
                    .base
                };

                match (&left, target.as_str()) {
                    // ---- numeric conversions ----
                    (Value::Number(n), "float") => Ok(Value::float(n.to_f64())),
                    (Value::Number(n), "int") => match n {
                        Number::Int(i) => Ok(Value::int(*i)),
                        Number::Float(_) => n.to_i64().map(Value::int).ok_or_else(|| {
                            convert_err(format!("float {} does not fit in an int", n))
                        }),
                    },
                    (Value::Number(n), "string") => {
                        Ok(Value::string_of(XenithString::new(n.to_string())))
                    }
                    (Value::Number(n), "bool") => Ok(Value::Bool(!n.is_zero())),

                    // ---- string conversions ----
                    (Value::String(s), "int") => s
                        .value
                        .trim()
                        .parse::<i64>()
                        .map(Value::int)
                        .map_err(|_| {
                            convert_err(format!("cannot convert string \"{}\" to int", s.value))
                        }),
                    (Value::String(s), "float") => s
                        .value
                        .trim()
                        .parse::<f64>()
                        .map(Value::float)
                        .map_err(|_| {
                            convert_err(format!("cannot convert string \"{}\" to float", s.value))
                        }),
                    (Value::String(s), "bool") => match s.value.trim() {
                        "true" => Ok(Value::Bool(true)),
                        "false" => Ok(Value::Bool(false)),
                        other => Err(convert_err(format!(
                            "cannot convert string \"{}\" to bool -- expected \"true\" or \"false\"",
                            other
                        ))),
                    },
                    (Value::String(s), "string") => Ok(Value::String(s.clone())),
                    // A string is already valid UTF-8, so this direction never
                    // fails. The other one does.
                    (Value::String(s), "bytes") => {
                        Ok(Value::bytes(s.value.clone().into_bytes()))
                    }

                    // ---- bytes conversions ----
                    //
                    // Bytes that are not valid UTF-8 have no string form, and
                    // quietly substituting replacement characters would lose
                    // data that a caller may well have been about to write back
                    // out. `bytes_to_string` is the form that hands back the
                    // failure instead of stopping.
                    (Value::Bytes(b), "string") => {
                        String::from_utf8(b.data.clone())
                            .map(|text| Value::string_of(XenithString::new(text)))
                            .map_err(|e| {
                                convert_err(format!(
                                    "bytes are not valid UTF-8 (at byte {})",
                                    e.utf8_error().valid_up_to()
                                ))
                            })
                    }
                    (Value::Bytes(b), "bytes") => Ok(Value::Bytes(b.clone())),

                    // ---- bool conversions ----
                    (Value::Bool(b), "int") => Ok(Value::int(if *b { 1 } else { 0 })),
                    (Value::Bool(b), "float") => Ok(Value::float(if *b { 1.0 } else { 0.0 })),
                    (Value::Bool(b), "string") => Ok(Value::string_of(XenithString::new(
                        if *b { "true" } else { "false" }.to_string(),
                    ))),
                    (Value::Bool(b), "bool") => Ok(Value::Bool(*b)),

                    _ => Err(convert_err(format!(
                        "cannot convert {} to {}",
                        Value::get_type_name(&left),
                        if target.is_empty() { "that type" } else { &target }
                    ))),
                }
            }
            _ => {
                return RuntimeResult::new().failure(
                    RuntimeError::new(
                        node.position_start.clone(),
                        node.position_end.clone(),
                        "Unknown binary operator",
                        Some(context.clone()),
                    )
                    .base,
                );
            }
        };

        match result_value {
            Ok(v) => result.success(v),
            Err(mut e) => {
                // Value-level ops build errors without positions; attach the
                // operator's real span so the diagnostic points at the source.
                if e.position_start.index == 0 && e.position_end.index == 0 {
                    e.position_start = node.position_start.clone();
                    e.position_end = node.position_end.clone();
                }
                RuntimeResult::new().failure(e)
            }
        }
    }

    fn visit_panic(&mut self, node: &PanicNode, context: &mut Context) -> RuntimeResult {
        let mut result = RuntimeResult::new();
        let message_value = result.register(self.visit(&node.message_node, context));

        if result.should_return() {
            return result;
        }

        let message = value_to_string(&message_value);

        result.failure(
            RuntimeError::new(
                node.position_start.clone(),
                node.position_end.clone(),
                &message,
                Some(context.clone()),
            )
            .with_code("XEN300")
            .with_name("Panic")
            .base,
        )
    }

    fn visit_interpolated_string(
        &mut self,
        node: &crate::nodes::InterpolatedStringNode,
        context: &mut Context,
    ) -> RuntimeResult {
        let mut result = RuntimeResult::new();
        let mut final_string = String::new();

        for part in &node.parts {
            if part.is_expression {
                // Parsed once, at parse time. Re-lexing and re-parsing here
                // meant a loop printing an interpolated string paid for a parse
                // on every iteration.
                if let Some(expression) = &part.parsed {
                    let value = result.register(self.visit(expression, context));
                    if result.should_return() {
                        return result;
                    }
                    final_string.push_str(&value_to_interpolated_string(&value));
                    continue;
                }

                // Did not parse at parse time; fall through so the error is
                // raised here, the way it always was.
                let mut lexer = Lexer::new("<interpolated>".to_string(), part.content.clone());
                let tokens = match lexer.make_tokens() {
                    Ok(t) => t,
                    Err(e) => {
                        return RuntimeResult::new().failure(e.base);
                    }
                };

                let mut parser = Parser::new(tokens);
                let parse_result = parser.parse_expression();

                if let Some(error) = parse_result.error {
                    return RuntimeResult::new().failure(error);
                }

                match parse_result.node {
                    Some(Node::List(list_node)) => {
                        // If there's only one element, evaluate it directly
                        if list_node.element_nodes.len() == 1 {
                            let value =
                                result.register(self.visit(&list_node.element_nodes[0], context));
                            if result.should_return() {
                                return result;
                            }
                            final_string.push_str(&value_to_interpolated_string(&value));
                        } else {
                            // For multiple statements, evaluate each and use the last value
                            let mut last_value = Value::Null;
                            for stmt_node in list_node.element_nodes {
                                let value = result.register(self.visit(&stmt_node, context));
                                if result.should_return() {
                                    return result;
                                }
                                last_value = value;
                            }
                            final_string.push_str(&value_to_interpolated_string(&last_value));
                        }
                    }
                    Some(node) => {
                        // Single expression node
                        let value = result.register(self.visit(&node, context));
                        if result.should_return() {
                            return result;
                        }
                        final_string.push_str(&value_to_interpolated_string(&value));
                    }
                    None => {
                        return RuntimeResult::new().failure(
                            RuntimeError::new(
                                node.position_start.clone(),
                                node.position_end.clone(),
                                "Invalid interpolation expression",
                                Some(context.clone()),
                            )
                            .base,
                        );
                    }
                }
            } else {
                final_string.push_str(&part.content);
            }
        }

        result.success(Value::string_of(XenithString::new(final_string)))
    }


    fn visit_unary_op(
        &mut self,
        node: &crate::nodes::UnaryOpNode,
        context: &mut Context,
    ) -> RuntimeResult {
        let mut result = RuntimeResult::new();

        let value = result.register(self.visit(&node.node, context));
        if result.should_return() {
            return result;
        }

        let op = &node.operator_token;

        let result_value = match op.kind {
            crate::tokens::TokenType::Minus => value.negative(),
            _ if op.matches(crate::tokens::TokenType::Keyword, Some("!")) => value.logical_not(),
            _ => {
                return RuntimeResult::new().failure(
                    RuntimeError::new(
                        node.position_start.clone(),
                        node.position_end.clone(),
                        "Unknown unary operator",
                        Some(context.clone()),
                    )
                    .base,
                );
            }
        };

        match result_value {
            Ok(v) => result.success(v),
            Err(e) => RuntimeResult::new().failure(e),
        }
    }

    fn visit_if(&mut self, node: &crate::nodes::IfNode, context: &mut Context) -> RuntimeResult {
        let mut result = RuntimeResult::new();

        // The branch body runs in a child scope, so a `let` inside it is local
        // to the branch. Loop and method bodies already worked this way;
        // `when` and `while` did not, which made block scoping depend on which
        // keyword you happened to be standing in.
        for (condition, expr) in &node.cases {
            let condition_value = result.register(self.visit(condition, context));
            if result.should_return() {
                return result;
            }

            if condition_value.is_true() {
                let mut branch_ctx =
                    context.create_child("<when>", node.position_start.clone());
                let value = result.register(self.visit(expr, &mut branch_ctx));
                if result.should_return() {
                    return result;
                }
                return result.success(value);
            }
        }

        if let Some((expr, _)) = &node.else_case {
            let mut branch_ctx =
                context.create_child("<otherwise>", node.position_start.clone());
            let value = result.register(self.visit(expr, &mut branch_ctx));
            if result.should_return() {
                return result;
            }
            return result.success(value);
        }

        result.success(Value::Null)
    }

    fn visit_for(&mut self, node: &crate::nodes::ForNode, context: &mut Context) -> RuntimeResult {
        let mut result = RuntimeResult::new();
        let mut elements = Vec::new();

        let raw_var_name = node.variable_name_token.value.as_ref().unwrap();
        let var_name = raw_var_name.trim_matches(|c| c == '(' || c == ')');

        let iterable = result.register(self.visit(&node.iterable_node, context));
        if result.should_return() {
            return result;
        }

        match &iterable {
            Value::List(list) => {
                let is_pair_list = var_name.contains(',');

                if is_pair_list {
                    let parts: Vec<String> =
                        var_name.split(',').map(|s| s.trim().to_string()).collect();

                    for item in list.elements.iter() {
                        if let Value::List(pair) = item {
                            if pair.elements.len() == 2 && parts.len() == 2 {
                                let mut loop_ctx = context.create_child("<for>", Self::dummy_pos());
                                loop_ctx
                                    .symbol_table
                                    .set(parts[0].clone(), pair.elements[0].clone());
                                loop_ctx
                                    .symbol_table
                                    .set(parts[1].clone(), pair.elements[1].clone());

                                let loop_value =
                                    result.register(self.visit(&node.body_node, &mut loop_ctx));
                                if result.should_return()
                                    && !result.loop_should_continue
                                    && !result.loop_should_break
                                {
                                    return result;
                                }
                                if result.loop_should_continue {
                                    result.loop_should_continue = false;
                                    continue;
                                }
                                if result.loop_should_break {
                                    result.loop_should_break = false;
                                    break;
                                }
                                elements.push(loop_value);
                            }
                        }
                    }
                } else {
                    for item in list.elements.iter() {
                        let mut loop_ctx = context.create_child("<for>", Self::dummy_pos());
                        loop_ctx
                            .symbol_table
                            .set(var_name.to_string(), item.clone());

                        let value = result.register(self.visit(&node.body_node, &mut loop_ctx));
                        if result.should_return()
                            && !result.loop_should_continue
                            && !result.loop_should_break
                        {
                            return result;
                        }
                        if result.loop_should_continue {
                            result.loop_should_continue = false;
                            continue;
                        }
                        if result.loop_should_break {
                            result.loop_should_break = false;
                            break;
                        }
                        // Same as the other loops: the collected values are
                        // discarded unless the loop's own value is wanted.
                        if !node.should_return_null {
                            elements.push(value);
                        }
                    }
                }
            }

            Value::Map(map) => {
                let parts: Option<Vec<String>> = if var_name.contains(',') {
                    let p: Vec<String> =
                        var_name.split(',').map(|s| s.trim().to_string()).collect();
                    if p.len() == 2 { Some(p) } else { None }
                } else {
                    None
                };

                // Iterate in sorted key order, the same order `.keys()`,
                // `.values()` and `.items()` produce. Walking the underlying
                // HashMap directly gave a different order on every run, so the
                // same program printed its output shuffled each time.
                let mut ordered: Vec<(&String, &Value)> = map.pairs.iter().collect();
                ordered.sort_by(|a, b| a.0.cmp(b.0));

                for (key_str, val) in ordered {
                    let mut loop_ctx = context.create_child("<for>", Self::dummy_pos());

                    if let Some(ref p) = parts {
                        loop_ctx.symbol_table.set(
                            p[0].clone(),
                            Value::string_of(XenithString::new(key_str.clone())),
                        );
                        loop_ctx.symbol_table.set(p[1].clone(), val.clone());
                    } else {
                        loop_ctx.symbol_table.set(
                            var_name.to_string(),
                            Value::string_of(XenithString::new(key_str.clone())),
                        );
                    }

                    let loop_value = result.register(self.visit(&node.body_node, &mut loop_ctx));
                    if result.should_return()
                        && !result.loop_should_continue
                        && !result.loop_should_break
                    {
                        return result;
                    }
                    if result.loop_should_continue {
                        result.loop_should_continue = false;
                        continue;
                    }
                    if result.loop_should_break {
                        result.loop_should_break = false;
                        break;
                    }
                    elements.push(loop_value);
                }
            }

            other => {
                return result.failure(
                    RuntimeError::new(
                        node.position_start.clone(),
                        node.position_end.clone(),
                        &format!(
                            "cannot iterate over {}",
                            Value::get_type_name(other)
                        ),
                        Some(context.clone()),
                    )
                    .with_code("XEN001")
                    .with_name("Type Mismatch")
                    .with_help("`for x in ...` iterates a list or map; to count, use `for (let i: int = 0; i < n; i++)`")
                    .base,
                );
            }
        }

        if node.should_return_null {
            result.success(Value::Null)
        } else {
            result.success(Value::List(List::new(elements)))
        }
    }

    /// `for (init; condition; step) { ... }`
    ///
    /// The three clauses share one scope so the counter stays visible to all of
    /// them; the body runs in a nested scope so its `let`s do not leak between
    /// iterations.
    fn visit_for_classic(
        &mut self,
        node: &crate::nodes::ForClassicNode,
        context: &mut Context,
    ) -> RuntimeResult {
        let mut result = RuntimeResult::new();
        let mut loop_ctx = context.create_child("<for>", node.position_start.clone());

        if let Some(init) = &node.init_node {
            result.register(self.visit(init, &mut loop_ctx));
            if result.should_return() {
                return result;
            }
        }

        // One scope reused across iterations: cleared each time so `let`s in
        // the body cannot leak between iterations, without reallocating.
        let mut body_ctx = loop_ctx.create_child("<for body>", node.position_start.clone());

        loop {
            // An absent condition means loop forever, as in C.
            if let Some(cond) = &node.condition_node {
                let cond_value = result.register(self.visit(cond, &mut loop_ctx));
                if result.should_return() {
                    return result;
                }
                if !cond_value.is_true() {
                    break;
                }
            }

            body_ctx.symbol_table.clear_local();
            result.register(self.visit(&node.body_node, &mut body_ctx));

            if result.loop_should_break {
                result.loop_should_break = false;
                break;
            }
            if result.loop_should_continue {
                result.loop_should_continue = false;
            } else if result.should_return() {
                return result;
            }

            if let Some(step) = &node.step_node {
                result.register(self.visit(step, &mut loop_ctx));
                if result.should_return() {
                    return result;
                }
            }
        }

        result.success(Value::Null)
    }

    fn visit_while(
        &mut self,
        node: &crate::nodes::WhileNode,
        context: &mut Context,
    ) -> RuntimeResult {
        let mut result = RuntimeResult::new();
        let mut elements = Vec::new();

        // One scope reused across iterations, cleared each time, so a `let` in
        // the body is local to the iteration rather than leaking outward.
        let mut body_ctx = context.create_child("<while>", node.position_start.clone());

        loop {
            let condition = result.register(self.visit(&node.condition_node, context));
            if result.should_return() {
                return result;
            }

            if !condition.is_true() {
                break;
            }

            body_ctx.symbol_table.clear_local();
            let value = result.register(self.visit(&node.body_node, &mut body_ctx));
            if result.should_return() && !result.loop_should_continue && !result.loop_should_break {
                return result;
            }

            if result.loop_should_continue {
                result.loop_should_continue = false;
                continue;
            }

            if result.loop_should_break {
                result.loop_should_break = false;
                break;
            }

            // Only worth keeping if the loop's value is actually wanted. A
            // three million iteration loop was collecting three million values
            // into a Vec and then throwing the whole thing away.
            if !node.should_return_null {
                elements.push(value);
            }
        }

        if node.should_return_null {
            result.success(Value::Null)
        } else {
            result.success(Value::List(List::new(elements)))
        }
    }

    fn visit_func_def(
        &mut self,
        node: &crate::nodes::FuncDefNode,
        context: &mut Context,
    ) -> RuntimeResult {
        let func_name = node
            .variable_name_token
            .as_ref()
            .map(|t| t.value.as_ref().unwrap().clone());

        let arg_names: Vec<String> = node
            .param_names
            .iter()
            .map(|t| t.value.as_ref().unwrap().clone())
            .collect();

        // Capture the scope this definition sits in. The clone is O(1) and
        // shares the underlying tables, so the capture stays live: a name
        // declared after this line is still visible, and a named method can
        // see itself.
        let func = Function::new(
            func_name.clone(),
            *node.body_node.clone(),
            arg_names,
            node.param_types.clone(),
            node.is_arrow,
            std::rc::Rc::new(context.clone()),
        );

        let func_value = Value::Function(Box::new(func));

        if let Some(name) = func_name {
            context.symbol_table.set(name, func_value.clone());
        }

        RuntimeResult::new().success(func_value)
    }

    fn visit_method_access(
        &mut self,
        node: &crate::nodes::MethodAccessNode,
        context: &mut Context,
    ) -> RuntimeResult {
        let mut result = RuntimeResult::new();

        // Evaluate the object
        let object = result.register(self.visit(&node.object, context));
        if result.should_return() {
            return result;
        }

        // For now, method access returns a special value that will be called
        // We'll handle this in visit_call
        let method_name = node.method_name.value.as_ref().unwrap().clone();

        // Return a special wrapper that represents a method to be called
        result.success(Value::string_of(XenithString::new(format!(
            "__METHOD__:{}",
            method_name
        ))))
    }

    fn visit_call(
        &mut self,
        node: &crate::nodes::CallNode,
        context: &mut Context,
    ) -> RuntimeResult {
        let mut result = RuntimeResult::new();

// Check if this is a method call (node_to_call is a MethodAccess)
        if let Node::MethodAccess(method_node) = &*node.node_to_call {
            // A method that rewrites its receiver is given the receiver
            // outright rather than a copy of it: the value is lifted out of the
            // variable, leaving `Null`, so that this is the only reference to
            // it. That is what lets the copy-on-write in `List` and `Map` skip
            // the copy. Without it `xs.append(x)` duplicates the whole list on
            // every call and filling one is quadratic.
            //
            // Only a plain variable is worth doing this for. Anything deeper,
            // like `record.items.append(v)`, still goes the copying way; it is
            // correct either way, and this keeps the fast path easy to follow.
            //
            // All three of these either hand the receiver back or fail, and a
            // failure ends the program, so the emptied variable is never
            // visible to Xenith code.
            let lifted_from = match (&*method_node.object, method_node.method_name.value.as_deref())
            {
                (Node::VarAccess(var), Some("append" | "pop" | "remove")) => {
                    var.variable_name_token.value.as_deref()
                }
                _ => None,
            };

            // The receiver is lifted *after* the arguments are evaluated, since
            // an argument may name the same variable -- `xs.append(xs.len())`
            // has to see `xs`, not the `Null` left in its place. Reading a
            // variable has no side effects, so deferring it past the arguments
            // changes nothing else. Anything not being lifted keeps the
            // original order and is evaluated first.
            let object = if lifted_from.is_none() {
                let value = result.register(self.visit(&method_node.object, context));
                if result.should_return() {
                    return result;
                }
                Some(value)
            } else {
                None
            };

            // Evaluate arguments
            let mut args = Vec::new();
            for arg_node in &node.argument_nodes {
                let arg = result.register(self.visit(arg_node, context));
                if result.should_return() {
                    return result;
                }
                args.push(arg);
            }

            let object = match object {
                Some(value) => value,
                None => match lifted_from.and_then(|name| context.symbol_table.take(name)) {
                    Some(value) => value,
                    // Not a variable that exists. Evaluating it gives the right
                    // error.
                    None => {
                        let value = result.register(self.visit(&method_node.object, context));
                        if result.should_return() {
                            return result;
                        }
                        value
                    }
                },
            };

            // Call the method on the object
            let method_name = method_node.method_name.value.as_ref().unwrap();
            let (mut call_result, mutated) =
                self.call_method(object, method_name, args, context);

            // `call_method` builds its errors without positions, the way the
            // value-level operators do. Attach the call's real span so a
            // failure points at the source rather than at line 1.
            if let Some(error) = &mut call_result.error {
                if error.position_start.index == 0 && error.position_end.index == 0 {
                    error.position_start = node.position_start.clone();
                    error.position_end = node.position_end.clone();
                }
            }

            // Register the result
            let value = result.register(call_result);
            if result.should_return() {
                return result;
            }

            // A method like `append` mutates its receiver, so the new state has
            // to be stored back where the receiver came from. `assign_into`
            // handles that at any depth, so `record.items.append(v)` works and
            // not just `items.append(v)`.
            //
            // A receiver that is not a place, such as `[1, 2].append(3)`, has
            // nowhere to write back to and is left alone rather than erroring.
            if let Some(updated) = mutated {
                if Self::is_assignable(&method_node.object) {
                    if let Some(error) = self.assign_into(&method_node.object, updated, context) {
                        return RuntimeResult::new().failure(error);
                    }
                }
            }

            return result.success(value);
        }

        // Regular function call
        let callee = result.register(self.visit(&node.node_to_call, context));
        if result.should_return() {
            return result;
        }

        let mut args = Vec::new();
        for arg_node in &node.argument_nodes {
            let arg = result.register(self.visit(arg_node, context));
            if result.should_return() {
                return result;
            }
            args.push(arg);
        }

        let call_result = match callee {
            Value::Function(func) => {
                // for (i, (arg, param_type)) in args.iter().zip(func.param_types.iter()).enumerate() {
                //     let matches = Self::value_matches_type(arg, param_type, &self.type_aliases);
                //     println!(
                //         "DEBUG: Arg {}: {:?} matches {:?} ? {}",
                //         i, arg, param_type, matches
                //     );
                //
                //     if !matches {
                //         return result.failure(Error::type_mismatch(
                //             &param_type.to_string(),
                //             &Self::get_type_name(arg),
                //             node.position_start.clone(),
                //             node.position_end.clone(),
                //         ));
                //     }
                // }

                let exec_result =
                    func.execute(args, context.clone(), self, node.position_start.clone());

                exec_result
                // func.execute(args, context.clone(), self, node.position_start.clone())
            }
            Value::BuiltInFunction(builtin) => {
                builtin.execute(args, self, node.position_start.clone(), context)
            }
            _ => {
                return result.failure(
                    RuntimeError::new(
                        node.position_start.clone(),
                        node.position_end.clone(),
                        "Cannot call non-function value",
                        Some(context.clone()),
                    )
                    .base,
                );
            }
        };

        let value = result.register(call_result);
        result.success(value)
    }

    /// Calls a built-in method on a value.
    ///
    /// Returns the call's result and, for the methods that mutate their
    /// receiver, the receiver's new state so the caller can store it back.
    fn call_method(
        &mut self,
        object: Value,
        method_name: &str,
        args: Vec<Value>,
        context: &mut Context,
    ) -> (RuntimeResult, Option<Value>) {
        let fail = |detail: &str| {
            (
                RuntimeResult::new().failure(
                    RuntimeError::new(
                        Self::dummy_pos(),
                        Self::dummy_pos(),
                        detail,
                        Some(context.clone()),
                    )
                    .base,
                ),
                None,
            )
        };

        match (object, method_name) {
            (Value::List(mut list), "append") => {
                if args.len() != 1 {
                    return fail("append expects 1 argument");
                }
                list.append(args[0].clone());
                let updated = Value::List(list);
                (
                    RuntimeResult::new().success(updated.clone()),
                    Some(updated),
                )
            }
            (Value::List(mut list), "pop") => {
                let index = if !args.is_empty() {
                    match &args[0] {
                        Value::Number(n) => n.as_index(),
                        _ => None,
                    }
                } else {
                    None
                };
                match list.pop(index) {
                    // The popped element is the call's value; the shortened
                    // list is what the variable should now hold.
                    Some(popped) => (
                        RuntimeResult::new().success(popped),
                        Some(Value::List(list)),
                    ),
                    None => fail("pop index out of bounds"),
                }
            }
            (Value::List(list), "len") => (
                RuntimeResult::new().success(Value::int(list.len() as i64)),
                None,
            ),
            (Value::Map(map), "items") => (
                RuntimeResult::new().success(Value::List(map.items())),
                None,
            ),
            (Value::Map(map), "keys") => (
                RuntimeResult::new().success(Value::List(map.keys())),
                None,
            ),
            (Value::Map(map), "values") => (
                RuntimeResult::new().success(Value::List(map.values())),
                None,
            ),
            (Value::Map(map), "len") => (
                RuntimeResult::new().success(Value::int(map.len() as i64)),
                None,
            ),
            // Deleting a key. It errors on a key that is not there rather than
            // doing nothing quietly, which is the same choice `pop` and
            // `map[key]` make: `has_key` is how you ask first.
            (Value::Map(mut map), "remove") => {
                if args.len() != 1 {
                    return fail("remove expects 1 argument");
                }
                let Value::String(key) = &args[0] else {
                    return fail("remove expects a string key");
                };
                match map.remove(&key.value) {
                    // The removed value is the call's result; the shortened map
                    // is what the variable should now hold.
                    Some(removed) => (
                        RuntimeResult::new().success(removed),
                        Some(Value::Map(map)),
                    ),
                    None => (
                        RuntimeResult::new().failure(
                            RuntimeError::new(
                                Self::dummy_pos(),
                                Self::dummy_pos(),
                                &format!("Key '{}' not found in map", key.value),
                                Some(context.clone()),
                            )
                            .with_help("check with `has_key` before removing")
                            .base,
                        ),
                        None,
                    ),
                }
            }
            (Value::Map(map), "has_key") => {
                if args.len() != 1 {
                    return fail("has_key expects 1 argument");
                }
                let Value::String(key) = &args[0] else {
                    return fail("has_key expects a string key");
                };
                (
                    RuntimeResult::new().success(Value::Bool(map.contains_key(&key.value))),
                    None,
                )
            }
            (Value::String(s), "len") => (
                RuntimeResult::new().success(Value::int(s.char_len() as i64)),
                None,
            ),
            (_, name) => fail(&format!("Method '{}' not found on object", name)),
        }
    }

    fn dummy_pos() -> Position {
        Position::new(0, 0, 0, "", "")
    }

    fn visit_tuple_literal(
        &mut self,
        node: &TupleLiteralNode,
        context: &mut Context,
    ) -> RuntimeResult {
        let mut result = RuntimeResult::new();
        let mut elements = Vec::new();

        for elem_node in &node.elements {
            let value = result.register(self.visit(elem_node, context));
            if result.should_return() {
                return result;
            }
            elements.push(value);
        }

        result.success(Value::tuple(elements))
    }

    fn visit_destructure(
        &mut self,
        node: &DestructureNode,
        context: &mut Context,
    ) -> RuntimeResult {
        let mut result = RuntimeResult::new();
        let value = result.register(self.visit(&node.value_node, context));
        if result.should_return() {
            return result;
        }

        for pattern in &node.patterns {
            result.register(self.destructure_value(pattern, &value, context));
            if result.should_return() {
                return result;
            }
        }

        result.success(Value::Null)
    }

    fn destructure_value(
        &mut self,
        pattern: &DestructurePattern,
        value: &Value,
        context: &mut Context,
    ) -> RuntimeResult {
        let mut result = RuntimeResult::new();

        match pattern {
            DestructurePattern::Variable(token) => {
                let name = token.value.as_ref().unwrap().clone();
                context.symbol_table.set_local(name, value.clone());
            }
            DestructurePattern::Ignore => {
                // Do nothing
            }
            DestructurePattern::Tuple(patterns) => {
                let tuple_values = match value.as_tuple() {
                    Some(v) => v,
                    None => {
                        return result.failure(Error::type_mismatch(
                            "tuple",
                            &crate::interpreter::Interpreter::get_type_name(value),
                            pattern.position_start(),
                            pattern.position_end(),
                        ));
                    }
                };

                if patterns.len() != tuple_values.len() {
                    return result.failure(
                        Error::new(
                            pattern.position_start(),
                            pattern.position_end(),
                            "Destructuring Mismatch",
                            &format!(
                                "Expected {} elements, got {}",
                                patterns.len(),
                                tuple_values.len()
                            ),
                        )
                        .with_code("XEN020"),
                    );
                }

                for (i, sub_pattern) in patterns.iter().enumerate() {
                    result.register(self.destructure_value(sub_pattern, &tuple_values[i], context));
                    if result.should_return() {
                        return result;
                    }
                }
            }
        }

        result.success(Value::Null)
    }

    fn visit_return(
        &mut self,
        node: &crate::nodes::ReturnNode,
        context: &mut Context,
    ) -> RuntimeResult {
        let mut result = RuntimeResult::new();

        let value = if let Some(expr) = &node.node_to_return {
            result.register(self.visit(expr, context))
        } else {
            Value::Null
        };

        if result.should_return() {
            return result;
        }

        result.success_return(value)
    }

    fn visit_continue(
        &mut self,
        _node: &crate::nodes::ContinueNode,
        _context: &mut Context,
    ) -> RuntimeResult {
        RuntimeResult::new().success_continue()
    }

    fn visit_break(
        &mut self,
        _node: &crate::nodes::BreakNode,
        _context: &mut Context,
    ) -> RuntimeResult {
        RuntimeResult::new().success_break()
    }

    /// Emit warnings for potential issues
    pub fn emit_warnings(&self, node: &Node, _context: &Context) -> Vec<String> {
        let mut warnings = Vec::new();

        match node {
            Node::If(node) if node.cases.len() > 5 => {
                warnings.push(format!(
                "⚠️  Too many `when` branches ({}). Consider using `match` for better readability",
                node.cases.len()
            ));
            }
            Node::BinaryOperator(node) => {
                // Check for division by zero constant
                if let (_, Node::Number(num)) = (&*node.left_node, &*node.right_node) {
                    if node.operator_token.kind == crate::tokens::TokenType::Div {
                        if let Ok(val) = num.token.value.as_ref().unwrap().parse::<f64>() {
                            if val == 0.0 {
                                warnings.push(format!(
                                    "⚠️  Division by zero constant at {}:{}",
                                    node.position_start.line + 1,
                                    node.position_start.column + 1
                                ));
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        warnings
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

/// "no payload", "1 value", "3 values" -- for arity messages about variants.
pub fn describe_arity(count: usize) -> String {
    match count {
        0 => "no payload".to_string(),
        1 => "1 value".to_string(),
        n => format!("{} values", n),
    }
}

