//interpreter.rs
//! # Interpreter Module
//!
//! Traverses the Abstract Syntax Tree and executes the program.
//! Implements the runtime semantics for each AST node type including
//! variable access, control flow, function calls, and built-in operations.

use crate::context::Context;
use crate::error::{Error, RuntimeError};
use crate::lexer::Lexer;
use crate::modules::{Module, ModuleRegistry};
use crate::nodes::{
    BoolLiteralNode, DestructureNode, DestructurePattern, Node, NullLiteralNode, PanicNode,
    StructDefNode, TupleLiteralNode, TypeAliasNode,
};
use crate::parser::Parser;
use crate::position::Position;
use crate::runtime_result::RuntimeResult;
use crate::symbol_table::SymbolTable;
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
    pub type_aliases: HashMap<String, Type>,
}

impl Interpreter {
    /// Creates a new interpreter with built-in functions initialized
    pub fn new() -> Self {
        let mut global = SymbolTable::new();

        // Built-in constants
        global.set("NULL".to_string(), Value::Null);
        global.set("FALSE".to_string(), Value::Bool(false));
        global.set("TRUE".to_string(), Value::Bool(true));
        global.set("MATH_PI".to_string(), Value::Number(Number::math_pi()));

        // Built-in functions
        global.set(
            "echo".to_string(),
            Value::BuiltInFunction(BuiltInFunction::new("echo")),
        );
        global.set(
            "format".to_string(),
            Value::BuiltInFunction(BuiltInFunction::new("format")),
        );
        global.set(
            "ret".to_string(),
            Value::BuiltInFunction(BuiltInFunction::new("ret")),
        );
        global.set(
            "input".to_string(),
            Value::BuiltInFunction(BuiltInFunction::new("input")),
        );
        global.set(
            "input_int".to_string(),
            Value::BuiltInFunction(BuiltInFunction::new("input_int")),
        );
        global.set(
            "clear".to_string(),
            Value::BuiltInFunction(BuiltInFunction::new("clear")),
        );
        global.set(
            "is_num".to_string(),
            Value::BuiltInFunction(BuiltInFunction::new("is_num")),
        );
        global.set(
            "is_str".to_string(),
            Value::BuiltInFunction(BuiltInFunction::new("is_str")),
        );
        global.set(
            "is_list".to_string(),
            Value::BuiltInFunction(BuiltInFunction::new("is_list")),
        );
        global.set(
            "is_fun".to_string(),
            Value::BuiltInFunction(BuiltInFunction::new("is_fun")),
        );
        global.set(
            "append".to_string(),
            Value::BuiltInFunction(BuiltInFunction::new("append")),
        );
        global.set(
            "pop".to_string(),
            Value::BuiltInFunction(BuiltInFunction::new("pop")),
        );
        global.set(
            "extend".to_string(),
            Value::BuiltInFunction(BuiltInFunction::new("extend")),
        );
        global.set(
            "len".to_string(),
            Value::BuiltInFunction(BuiltInFunction::new("len")),
        );
        global.set(
            "run".to_string(),
            Value::BuiltInFunction(BuiltInFunction::new("run")),
        );

        Self {
            global_symbol_table: global,
            module_registry: None,
            struct_names: std::collections::HashSet::new(),
            type_aliases: HashMap::new(),
        }
    }

    fn load_module(
        &mut self,
        module_path: &str,
        pos: &Position,
        context: &Context,
    ) -> Result<Module, String> {
        // Initialize module registry if needed
        if self.module_registry.is_none() {
            self.module_registry = Some(ModuleRegistry::new(&pos.file_name));
        }

        // Take ownership of the registry temporarily
        let mut registry = self.module_registry.take().unwrap();
        let result = registry.load_module(module_path, self);
        self.module_registry = Some(registry);

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

        // Create a map of field names to their types for validation
        let mut field_types = Vec::new();
        for field in &node.fields {
            let field_name = field.name.value.as_ref().unwrap().clone();
            field_types.push((field_name, field.field_type.clone()));
        }

        self.struct_names.insert(struct_name.clone());
        context.symbol_table.set(
            struct_name.clone(),
            Value::String(XenithString::new(format!("__struct__{}", struct_name))),
        );

        RuntimeResult::new().success(Value::Null)
    }

    fn visit_struct_instantiation(
        &mut self,
        node: &crate::nodes::StructInstantiationNode,
        context: &mut Context,
    ) -> RuntimeResult {
        let mut result = RuntimeResult::new();
        let mut struct_instance = crate::values::Struct::new(node.struct_name.clone());

        for (field_name, value_node) in &node.fields {
            let value = result.register(self.visit(value_node, context));
            if result.should_return() {
                return result;
            }
            let name = field_name.value.as_ref().unwrap().clone();
            struct_instance.set_field(name, value);
        }

        result.success(Value::Struct(struct_instance))
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

        // Mark the value as exported in the current context's module exports
        if let Some(value) = &inner_result.value {
            // Store in a special "exports" table in the context
            // We'll need to add an exports field to Context
            context.add_export(node.exported_name.clone(), value.clone());
        }

        inner_result
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
            Err(err) => {
                return result.failure(
                    RuntimeError::new(
                        node.position_start.clone(),
                        node.position_end.clone(),
                        &err,
                        Some(context.clone()),
                    )
                    .base,
                );
            }
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
                    .set(alias.clone(), Value::Map(namespace_map));
            }
        } else {
            // Import specific items
            for spec in &node.imports {
                let original_name = &spec.original_name;
                let target_name = spec.alias.as_ref().unwrap_or(original_name);

                if let Some(value) = module.exports.get(original_name) {
                    context.symbol_table.set(target_name.clone(), value.clone());
                } else if self.struct_names.contains(original_name.as_str()) {
                    // It's a struct — just register the marker in the symbol table
                    context.symbol_table.set(
                        target_name.clone(),
                        Value::String(XenithString::new(format!("__struct__{}", original_name))),
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
        RuntimeResult::new().success(Value::String(XenithString::new(value.clone())))
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

        result.success(Value::Map(map))
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

        if let Some(value) = context.symbol_table.get(var_name) {
            return RuntimeResult::new().success(value.clone());
        }

        match self.global_symbol_table.get(var_name) {
            Some(value) => RuntimeResult::new().success(value.clone()),
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
        // One scope-chain walk answers declared/constant/declared-type.
        let Some(binding) = context.symbol_table.resolve_for_assign(var_name) else {
            return RuntimeResult::new().failure(
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
            );
        };

        if binding.is_constant {
            return RuntimeResult::new().failure(
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
            );
        }

        // Reassignment must respect the type the variable was declared with
        if let Some(declared) = &binding.declared_type {
            if *declared != Type::Unknown && !self.value_matches_type(&value, declared) {
                return RuntimeResult::new().failure(Error::type_mismatch(
                    &declared.to_string(),
                    &Self::get_type_name(&value),
                    node.position_start.clone(),
                    node.position_end.clone(),
                ));
            }
        }

        context.symbol_table.assign_existing(var_name, value.clone());
        result.success(value)
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
        let resolved_type = self.resolve_type_alias(expected_type);
        Value::value_matches_type(value, &resolved_type)
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


    fn visit_binary_op(
        &mut self,
        node: &crate::nodes::BinaryOperatorNode,
        context: &mut Context,
    ) -> RuntimeResult {
        let mut result = RuntimeResult::new();

        // Handle assignment separately before evaluating both sides
        if node.operator_token.kind == crate::tokens::TokenType::Eq {
            // Check if this is a struct field assignment (left side has a dot)
            if let Node::BinaryOperator(bin_op) = &*node.left_node {
                if bin_op.operator_token.kind == crate::tokens::TokenType::Dot {
                    // This is struct.field = value
                    let struct_value = result.register(self.visit(&bin_op.left_node, context));
                    if result.should_return() {
                        return result;
                    }

                    let field_name = if let Node::VarAccess(var_node) = &*bin_op.right_node {
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

                    let value = result.register(self.visit(&node.right_node, context));
                    if result.should_return() {
                        return result;
                    }

                    // Get the variable name to write back
                    let var_name = if let Node::VarAccess(var_node) = &*bin_op.left_node {
                        Some(var_node.variable_name_token.value.as_ref().unwrap().clone())
                    } else {
                        None
                    };

                    // Update the struct field
                    match struct_value {
                        Value::Struct(mut s) => {
                            s.set_field(field_name.clone(), value.clone());
                            // Write the mutated struct back to the variable
                            if let Some(name) = var_name {
                                context
                                    .symbol_table
                                    .set_existing(name, Value::Struct(s.clone())); // &self
                            }
                            return result.success(value);
                        }
                        Value::Map(mut m) => {
                            m.set(field_name.clone(), value.clone());
                            if let Some(name) = var_name {
                                context.symbol_table.set_existing(name, Value::Map(m));
                            }
                            return result.success(value);
                        }
                        _ => {
                            return result.failure(
                                RuntimeError::new(
                                    node.position_start.clone(),
                                    node.position_end.clone(),
                                    &format!(
                                        "Cannot set field '{}' on non-struct/non-map value",
                                        field_name
                                    ),
                                    Some(context.clone()),
                                )
                                .base,
                            );
                        }
                    }
                }
            }

            // Check for MethodAccess pattern (object.field = value)
            if let Node::MethodAccess(field_node) = &*node.left_node {
                let object_value = result.register(self.visit(&field_node.object, context));
                if result.should_return() {
                    return result;
                }
                let right = result.register(self.visit(&node.right_node, context));
                if result.should_return() {
                    return result;
                }

                let field_name = field_node.method_name.value.as_ref().unwrap();

                // Get the variable name to write back
                let var_name = if let Node::VarAccess(var_node) = &*field_node.object {
                    Some(var_node.variable_name_token.value.as_ref().unwrap().clone())
                } else {
                    None
                };

                let updated = match object_value {
                    Value::Struct(mut s) => {
                        s.set_field(field_name.clone(), right.clone());
                        Ok(Value::Struct(s))
                    }
                    Value::Map(mut m) => {
                        m.set(field_name.clone(), right.clone());
                        Ok(Value::Map(m))
                    }
                    _ => Err(RuntimeError::new(
                        node.position_start.clone(),
                        node.position_end.clone(),
                        &format!(
                            "Cannot set field '{}' on non-struct/non-map value",
                            field_name
                        ),
                        Some(context.clone()),
                    )
                    .base),
                };

                return match updated {
                    Ok(new_obj) => {
                        // Write the mutated object back to the variable
                        if let Some(name) = var_name {
                            context.symbol_table.set_existing(name, new_obj.clone());
                        }
                        result.success(new_obj)
                    }
                    Err(e) => RuntimeResult::new().failure(e),
                };
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
                context.symbol_table.set_existing(var_name, right.clone());
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

        // All non-assignment operators: evaluate both sides first
        let left = result.register(self.visit(&node.left_node, context));
        if result.should_return() {
            return result;
        }
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
                _ => Err(RuntimeError::new(
                    node.position_start.clone(),
                    node.position_end.clone(),
                    "Cannot index non-list/non-map with non-number/non-string",
                    Some(context.clone()),
                )
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
                        Ok(Value::String(XenithString::new(n.to_string())))
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

                    // ---- bool conversions ----
                    (Value::Bool(b), "int") => Ok(Value::int(if *b { 1 } else { 0 })),
                    (Value::Bool(b), "float") => Ok(Value::float(if *b { 1.0 } else { 0.0 })),
                    (Value::Bool(b), "string") => Ok(Value::String(XenithString::new(
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
                // Parse and evaluate the expression
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

        result.success(Value::String(XenithString::new(final_string)))
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

        for (condition, expr) in &node.cases {
            let condition_value = result.register(self.visit(condition, context));
            if result.should_return() {
                return result;
            }

            if condition_value.is_true() {
                let value = result.register(self.visit(expr, context));
                if result.should_return() {
                    return result;
                }
                return result.success(value);
            }
        }

        if let Some((expr, _)) = &node.else_case {
            let value = result.register(self.visit(expr, context));
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

                    for item in &list.elements {
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
                    for item in &list.elements {
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
                        elements.push(value);
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

                for (key_str, val) in &map.pairs {
                    let mut loop_ctx = context.create_child("<for>", Self::dummy_pos());

                    if let Some(ref p) = parts {
                        loop_ctx.symbol_table.set(
                            p[0].clone(),
                            Value::String(XenithString::new(key_str.clone())),
                        );
                        loop_ctx.symbol_table.set(p[1].clone(), val.clone());
                    } else {
                        loop_ctx.symbol_table.set(
                            var_name.to_string(),
                            Value::String(XenithString::new(key_str.clone())),
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

        loop {
            let condition = result.register(self.visit(&node.condition_node, context));
            if result.should_return() {
                return result;
            }

            if !condition.is_true() {
                break;
            }

            let value = result.register(self.visit(&node.body_node, context));
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

            elements.push(value);
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

        let func = Function::new(
            func_name.clone(),
            *node.body_node.clone(),
            arg_names,
            node.param_types.clone(),
            node.is_arrow,
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
        result.success(Value::String(XenithString::new(format!(
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
            // Check if the object is a variable access (so we can update it)
            let var_name = if let Node::VarAccess(var_node) = &*method_node.object {
                Some(var_node.variable_name_token.value.as_ref().unwrap().clone())
            } else {
                None
            };

            // Evaluate the object
            let object = result.register(self.visit(&method_node.object, context));
            if result.should_return() {
                return result;
            }

            // Evaluate arguments
            let mut args = Vec::new();
            for arg_node in &node.argument_nodes {
                let arg = result.register(self.visit(arg_node, context));
                if result.should_return() {
                    return result;
                }
                args.push(arg);
            }

            // Call the method on the object
            let method_name = method_node.method_name.value.as_ref().unwrap();
            let call_result = self.call_method(object.clone(), method_name, args, context);

            // Register the result
            let value = result.register(call_result);
            if result.should_return() {
                return result;
            }

            // If this is a method that modifies the object (like append),
            // update the variable in the context
            if let Some(name) = var_name {
                if method_name == "append" {
                    context.symbol_table.set(name, value.clone());
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
                builtin.execute(args, self, node.position_start.clone())
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

    fn call_method(
        &mut self,
        object: Value,
        method_name: &str,
        args: Vec<Value>,
        context: &mut Context,
    ) -> RuntimeResult {
        match (object, method_name) {
            (Value::List(mut list), "append") => {
                if args.len() != 1 {
                    return RuntimeResult::new().failure(
                        RuntimeError::new(
                            Self::dummy_pos(),
                            Self::dummy_pos(),
                            "append expects 1 argument",
                            Some(context.clone()),
                        )
                        .base,
                    );
                }
                list.append(args[0].clone());
                // Return the modified list (or could return NULL)
                RuntimeResult::new().success(Value::List(list))
            }
            (Value::List(mut list), "pop") => {
                let index = if args.len() >= 1 {
                    match &args[0] {
                        Value::Number(n) => n.as_index(),
                        _ => None,
                    }
                } else {
                    None
                };
                if let Some(popped) = list.pop(index) {
                    // Need to also return the modified list if you want chainable methods
                    // For now, return popped value
                    RuntimeResult::new().success(popped)
                } else {
                    RuntimeResult::new().failure(
                        RuntimeError::new(
                            Self::dummy_pos(),
                            Self::dummy_pos(),
                            "pop index out of bounds",
                            Some(context.clone()),
                        )
                        .base,
                    )
                }
            }
            (Value::List(list), "len") => {
                RuntimeResult::new().success(Value::int(list.len() as i64))
            }
            (Value::Map(map), "items") => RuntimeResult::new().success(Value::List(map.items())),
            (Value::Map(map), "keys") => RuntimeResult::new().success(Value::List(map.keys())),
            (Value::Map(map), "values") => RuntimeResult::new().success(Value::List(map.values())),
            (Value::Map(map), "len") => {
                RuntimeResult::new().success(Value::int(map.len() as i64))
            }
            (Value::Map(map), "has_key") => {
                if args.len() != 1 {
                    return RuntimeResult::new().failure(
                        RuntimeError::new(
                            Self::dummy_pos(),
                            Self::dummy_pos(),
                            "has_key expects 1 argument",
                            Some(context.clone()),
                        )
                        .base,
                    );
                }
                let key = match &args[0] {
                    Value::String(s) => &s.value,
                    _ => {
                        return RuntimeResult::new().failure(
                            RuntimeError::new(
                                Self::dummy_pos(),
                                Self::dummy_pos(),
                                "has_key expects a string key",
                                Some(context.clone()),
                            )
                            .base,
                        );
                    }
                };
                RuntimeResult::new().success(Value::Bool(map.contains_key(key)))
            }
_ => RuntimeResult::new().failure(
                RuntimeError::new(
                    Self::dummy_pos(),
                    Self::dummy_pos(),
                    &format!("Method '{}' not found on object", method_name),
                    Some(context.clone()),
                )
                .base,
            ),
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

        result.success(Value::Tuple(elements))
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
                        .with_code("XEN100"),
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

