//! # Interpreter Module
//!
//! Traverses the Abstract Syntax Tree and executes the program.
//! Implements the runtime semantics for each AST node type including
//! variable access, control flow, function calls, and built-in operations.

use crate::context::Context;
use crate::error::RuntimeError;
use crate::lexer::Lexer;
use crate::modules::{Module, ModuleRegistry};
use crate::nodes::{
    BoolLiteralNode, ImplNode, Node, NullLiteralNode, PanicNode, StructDefNode, TryCatchNode,
    TypeAliasNode,
};
use crate::parser::Parser;
use crate::position::Position;
use crate::runtime_result::RuntimeResult;
use crate::symbol_table::SymbolTable;
use crate::utils::{value_to_interpolated_string, value_to_string};
use crate::values::{
    BuiltInFunction, CaughtError, Function, List, Map, Number, Value, XenithString,
};
use std::collections::HashMap;

// Struct info for method lookup
#[derive(Debug, Clone)]
pub struct StructMethodInfo {
    pub name: String,
    pub methods: HashMap<String, Box<crate::nodes::FuncDefNode>>,
}

impl StructMethodInfo {
    pub fn new(name: String) -> Self {
        Self {
            name,
            methods: HashMap::new(),
        }
    }

    pub fn get_method(&self, name: &str) -> Option<&Box<crate::nodes::FuncDefNode>> {
        self.methods.get(name)
    }
}

/// Main interpreter that traverses and executes the AST
pub struct Interpreter {
    /// Global symbol table with built-in functions
    pub global_symbol_table: SymbolTable,
    /// Module registry for caching loaded modules
    pub module_registry: Option<ModuleRegistry>,
    pub struct_registry: HashMap<String, StructMethodInfo>,
}

impl Interpreter {
    /// Creates a new interpreter with built-in functions initialized
    pub fn new() -> Self {
        let mut global = SymbolTable::new();

        // Built-in constants
        global.set("NULL".to_string(), Value::Number(Number::null()));
        global.set("FALSE".to_string(), Value::Number(Number::false_val()));
        global.set("TRUE".to_string(), Value::Number(Number::true_val()));
        global.set("MATH_PI".to_string(), Value::Number(Number::math_pi()));

        // Built-in functions
        global.set(
            "echo".to_string(),
            Value::BuiltInFunction(BuiltInFunction::new("echo")),
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

        // File system built-ins (prefixed with __ for internal use)

        // fs
        global.set(
            "__fs_read".to_string(),
            Value::BuiltInFunction(BuiltInFunction::new("__fs_read")),
        );
        global.set(
            "__fs_write".to_string(),
            Value::BuiltInFunction(BuiltInFunction::new("__fs_write")),
        );
        global.set(
            "__fs_exists".to_string(),
            Value::BuiltInFunction(BuiltInFunction::new("__fs_exists")),
        );
        global.set(
            "__fs_append".to_string(),
            Value::BuiltInFunction(BuiltInFunction::new("__fs_append")),
        );
        global.set(
            "__fs_is_file".to_string(),
            Value::BuiltInFunction(BuiltInFunction::new("__fs_is_file")),
        );
        global.set(
            "__fs_is_dir".to_string(),
            Value::BuiltInFunction(BuiltInFunction::new("__fs_is_dir")),
        );
        global.set(
            "__fs_mkdir".to_string(),
            Value::BuiltInFunction(BuiltInFunction::new("__fs_mkdir")),
        );
        global.set(
            "__fs_mkdir_all".to_string(),
            Value::BuiltInFunction(BuiltInFunction::new("__fs_mkdir_all")),
        );
        global.set(
            "__fs_remove".to_string(),
            Value::BuiltInFunction(BuiltInFunction::new("__fs_remove")),
        );
        global.set(
            "__fs_remove_all".to_string(),
            Value::BuiltInFunction(BuiltInFunction::new("__fs_remove_all")),
        );
        global.set(
            "__fs_list_dir".to_string(),
            Value::BuiltInFunction(BuiltInFunction::new("__fs_list_dir")),
        );
        global.set(
            "__fs_copy".to_string(),
            Value::BuiltInFunction(BuiltInFunction::new("__fs_copy")),
        );

        // path
        global.set(
            "__path_join".to_string(),
            Value::BuiltInFunction(BuiltInFunction::new("__path_join")),
        );
        global.set(
            "__path_basename".to_string(),
            Value::BuiltInFunction(BuiltInFunction::new("__path_basename")),
        );
        global.set(
            "__path_dirname".to_string(),
            Value::BuiltInFunction(BuiltInFunction::new("__path_dirname")),
        );
        global.set(
            "__path_extension".to_string(),
            Value::BuiltInFunction(BuiltInFunction::new("__path_extension")),
        );
        global.set(
            "__path_stem".to_string(),
            Value::BuiltInFunction(BuiltInFunction::new("__path_stem")),
        );
        global.set(
            "__path_is_absolute".to_string(),
            Value::BuiltInFunction(BuiltInFunction::new("__path_is_absolute")),
        );
        global.set(
            "__path_is_relative".to_string(),
            Value::BuiltInFunction(BuiltInFunction::new("__path_is_relative")),
        );
        global.set(
            "__path_absolute".to_string(),
            Value::BuiltInFunction(BuiltInFunction::new("__path_absolute")),
        );
        global.set(
            "__path_normalize".to_string(),
            Value::BuiltInFunction(BuiltInFunction::new("__path_normalize")),
        );
        global.set(
            "__path_components".to_string(),
            Value::BuiltInFunction(BuiltInFunction::new("__path_components")),
        );
        global.set(
            "__path_parent".to_string(),
            Value::BuiltInFunction(BuiltInFunction::new("__path_parent")),
        );

        Self {
            global_symbol_table: global,
            module_registry: None,
            struct_registry: HashMap::new(),
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
            Node::While(n) => self.visit_while(n, context),
            Node::FuncDef(n) => self.visit_func_def(n, context),
            Node::Call(n) => self.visit_call(n, context),
            Node::Return(n) => self.visit_return(n, context),
            Node::Continue(n) => self.visit_continue(n, context),
            Node::Break(n) => self.visit_break(n, context),
            Node::InterpolatedString(n) => self.visit_interpolated_string(n, context),
            Node::MethodAccess(n) => self.visit_method_access(n, context),
            Node::Match(n) => self.visit_match(n, context),
            Node::Map(n) => self.visit_map(n, context),
            Node::TryCatch(n) => self.visit_try_catch(n, context),
            Node::Panic(n) => self.visit_panic(n, context),
            Node::Grab(n) => self.visit_grab(n, context),
            Node::Export(n) => self.visit_export(n, context),
            Node::StructDef(n) => self.visit_struct_def(n, context),
            Node::Impl(n) => self.visit_impl(n, context),
            Node::TypeAlias(n) => self.visit_type_alias(n, context),
            Node::BoolLiteral(n) => self.visit_bool_literal(n, context),
            Node::NullLiteral(n) => self.visit_null_literal(n, context),
            Node::StructInstantiation(n) => self.visit_struct_instantiation(n, context),
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

        // Store the struct definition (we'll need a separate struct registry)
        // For now, just store a marker in the symbol table
        context.symbol_table.set(
            struct_name.clone(),
            Value::String(XenithString::new(format!("__struct__{}", struct_name))),
        );

        RuntimeResult::new().success(Value::Number(Number::null()))
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

    fn visit_impl(&mut self, node: &ImplNode, context: &mut Context) -> RuntimeResult {
        let struct_name = node.struct_name.value.as_ref().unwrap().clone();

        // Get or create StructMethodInfo for this struct
        let struct_info = self
            .struct_registry
            .entry(struct_name.clone())
            .or_insert_with(|| StructMethodInfo::new(struct_name.clone()));

        for method in &node.methods {
            if let Some(name) = &method.variable_name_token {
                if let Some(method_name) = &name.value {
                    struct_info
                        .methods
                        .insert(method_name.clone(), method.clone());
                }
            }
        }

        RuntimeResult::new().success(Value::Number(Number::null()))
    }

    fn visit_type_alias(&mut self, node: &TypeAliasNode, context: &mut Context) -> RuntimeResult {
        RuntimeResult::new().success(Value::Number(Number::null()))
    }

    fn visit_null_literal(
        &mut self,
        node: &NullLiteralNode,
        _context: &mut Context,
    ) -> RuntimeResult {
        RuntimeResult::new().success(Value::Number(Number::null()))
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

        result.success(Value::Number(Number::null()))
    }

    fn visit_number(
        &mut self,
        node: &crate::nodes::NumberNode,
        _context: &mut Context,
    ) -> RuntimeResult {
        let value = node.token.value.as_ref().unwrap();
        let num = if value.contains('.') {
            Number::new(value.parse::<f64>().unwrap())
        } else {
            Number::new(value.parse::<i64>().unwrap() as f64)
        };
        RuntimeResult::new().success(Value::Number(num))
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
            if elem_result.caught_error.is_some() {
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
                    return result.failure(
                        RuntimeError::new(
                            pair.position_start.clone(),
                            pair.position_end.clone(),
                            "Map keys must be strings",
                            Some(context.clone()),
                        )
                        .base,
                    );
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

        // Use symbol table's get which already traverses parent chain
        if let Some(value) = context.symbol_table.get(var_name) {
            return RuntimeResult::new().success(value.clone());
        }

        // Check global scope
        match self.global_symbol_table.get(var_name) {
            Some(value) => RuntimeResult::new().success(value.clone()),
            None => RuntimeResult::new().failure(
                RuntimeError::new(
                    node.position_start.clone(),
                    node.position_end.clone(),
                    &format!("'{}' is not defined", var_name),
                    Some(context.clone()),
                )
                .base,
            ),
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

        // Use set_existing to update the variable in its original scope,
        // or create it in current scope if it doesn't exist
        // Note: set_existing now takes &self, so we don't need &mut
        context
            .symbol_table
            .set_existing(var_name.clone(), value.clone());

        result.success(value)
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
                        return result.failure(
                            RuntimeError::new(
                                node.position_start.clone(),
                                node.position_end.clone(),
                                &format!("Struct has no field '{}'", field_name),
                                Some(context.clone()),
                            )
                            .base,
                        );
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
            crate::tokens::TokenType::Div => left.divide(&right),
            crate::tokens::TokenType::Pow => left.power(&right),
            crate::tokens::TokenType::Ee => left.equals(&right),
            crate::tokens::TokenType::Ne => left.not_equals(&right),
            crate::tokens::TokenType::Lt => left.less_than(&right),
            crate::tokens::TokenType::Gt => left.greater_than(&right),
            crate::tokens::TokenType::Lte => left.less_than_or_equal(&right),
            crate::tokens::TokenType::Gte => left.greater_than_or_equal(&right),
            crate::tokens::TokenType::Index => match (&left, &right) {
                (Value::List(list), Value::Number(idx)) => {
                    let idx_usize = idx.value as usize;
                    if idx_usize >= list.elements.len() {
                        return RuntimeResult::new().failure(
                            RuntimeError::new(
                                node.position_start.clone(),
                                node.position_end.clone(),
                                "List index out of bounds",
                                Some(context.clone()),
                            )
                            .base,
                        );
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
                // Type conversion
                match (&left, &right) {
                    // int -> float
                    (Value::Number(n), Value::String(s)) if s.value == "float" => {
                        Ok(Value::Number(Number::new(n.value)))
                    }
                    // float -> int (truncates)
                    (Value::Number(n), Value::String(s)) if s.value == "int" => {
                        Ok(Value::Number(Number::new(n.value.trunc())))
                    }
                    // number -> string
                    (Value::Number(n), Value::String(s)) if s.value == "string" => {
                        Ok(Value::String(XenithString::new(n.value.to_string())))
                    }
                    // number -> bool (0 = false, non-zero = true)
                    (Value::Number(n), Value::String(s)) if s.value == "bool" => {
                        Ok(Value::Number(Number::new(if n.value != 0.0 {
                            1.0
                        } else {
                            0.0
                        })))
                    }
                    // string -> int
                    (Value::String(s), Value::String(target)) if target.value == "int" => {
                        match s.value.parse::<i64>() {
                            Ok(num) => Ok(Value::Number(Number::new(num as f64))),
                            Err(_) => Err(RuntimeError::new(
                                node.position_start.clone(),
                                node.position_end.clone(),
                                &format!("Cannot convert string '{}' to int", s.value),
                                Some(context.clone()),
                            )
                            .base),
                        }
                    }
                    // string -> float
                    (Value::String(s), Value::String(target)) if target.value == "float" => {
                        match s.value.parse::<f64>() {
                            Ok(num) => Ok(Value::Number(Number::new(num))),
                            Err(_) => Err(RuntimeError::new(
                                node.position_start.clone(),
                                node.position_end.clone(),
                                &format!("Cannot convert string '{}' to float", s.value),
                                Some(context.clone()),
                            )
                            .base),
                        }
                    }
                    // string -> bool
                    (Value::String(s), Value::String(target)) if target.value == "bool" => {
                        let lower = s.value.to_lowercase();
                        let result = if lower == "true" || lower == "1" {
                            1.0
                        } else if lower == "false" || lower == "0" {
                            0.0
                        } else {
                            return RuntimeResult::new().failure(
                                RuntimeError::new(
                                    node.position_start.clone(),
                                    node.position_end.clone(),
                                    &format!("Cannot convert string '{}' to bool", s.value),
                                    Some(context.clone()),
                                )
                                .base,
                            );
                        };
                        Ok(Value::Number(Number::new(result)))
                    }
                    // bool -> int
                    (Value::Number(n), Value::String(target)) if target.value == "int" => {
                        // bool is stored as Number (0 or 1)
                        Ok(Value::Number(Number::new(n.value)))
                    }
                    // bool -> string
                    (Value::Number(n), Value::String(target)) if target.value == "string" => {
                        let s = if n.value != 0.0 { "true" } else { "false" };
                        Ok(Value::String(XenithString::new(s.to_string())))
                    }
                    // bool -> float
                    (Value::Number(n), Value::String(target)) if target.value == "float" => {
                        Ok(Value::Number(Number::new(n.value)))
                    }
                    _ => Err(RuntimeError::new(
                        node.position_start.clone(),
                        node.position_end.clone(),
                        &format!("Cannot convert {:?} to {:?}", left, right),
                        Some(context.clone()),
                    )
                    .base),
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
            Err(e) => RuntimeResult::new().failure(e),
        }
    }

    fn visit_try_catch(&mut self, node: &TryCatchNode, context: &mut Context) -> RuntimeResult {
        let mut result = RuntimeResult::new();

        // Create a new context for the try block
        let mut try_context = context.create_child("<try>", node.position_start.clone());

        // Execute try block
        let try_result = self.visit(&node.try_block, &mut try_context);

        // Check if there was a caught error (from panic)
        if let Some(caught) = try_result.caught_error {
            // Execute catch block with error variable
            let mut catch_context = context.create_child("<catch>", node.position_start.clone());
            catch_context.symbol_table.set_local(
                node.catch_var.value.as_ref().unwrap().clone(),
                Value::String(XenithString::new(caught.message)),
            );

            return self.visit(&node.catch_block, &mut catch_context);
        }

        // Check if there was a runtime error
        if let Some(error) = try_result.error {
            // Create caught error value
            let error_message = format!("{}: {}", error.error_name, error.details);
            let caught_error = CaughtError {
                message: error_message,
            };

            // Execute catch block with error variable
            let mut catch_context = context.create_child("<catch>", node.position_start.clone());
            catch_context.symbol_table.set_local(
                node.catch_var.value.as_ref().unwrap().clone(),
                Value::String(XenithString::new(caught_error.message.clone())),
            );

            return self.visit(&node.catch_block, &mut catch_context);
        }

        // No error, return try block result
        try_result
    }

    fn visit_panic(&mut self, node: &PanicNode, context: &mut Context) -> RuntimeResult {
        let mut result = RuntimeResult::new();
        let message_value = result.register(self.visit(&node.message_node, context));

        if result.should_return() {
            return result;
        }

        let message = value_to_string(&message_value);
        let caught_error = CaughtError { message };

        // Return with caught error and also mark that we should return
        let mut panic_result = RuntimeResult::new();
        panic_result.caught_error = Some(caught_error);
        panic_result // Don't call success_catch, just return the error
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
                            let mut last_value = Value::Number(Number::null());
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

    fn visit_match(
        &mut self,
        node: &crate::nodes::MatchNode,
        context: &mut Context,
    ) -> RuntimeResult {
        let mut result = RuntimeResult::new();

        // Evaluate the value to match against
        let match_value = result.register(self.visit(&node.value_node, context));
        if result.should_return() {
            return result;
        }

        // Try each arm in order
        for arm in &node.arms {
            // Evaluate the pattern
            let pattern_value = result.register(self.visit(&arm.pattern_node, context));
            if result.should_return() {
                return result;
            }

            // Check if pattern matches
            let is_match = match (&match_value, &pattern_value) {
                // Underscore pattern (_) matches anything - check if it's the underscore identifier
                (_, Value::String(s)) if s.value == "_" => true,
                // Literal comparison
                (Value::Number(a), Value::Number(b)) => (a.value - b.value).abs() < 1e-10,
                (Value::String(a), Value::String(b)) => a.value == b.value,
                (Value::List(a), Value::List(b)) => {
                    if a.elements.len() != b.elements.len() {
                        false
                    } else {
                        a.elements
                            .iter()
                            .zip(b.elements.iter())
                            .all(|(x, y)| match x.equals(y) {
                                Ok(Value::Number(n)) => n.value != 0.0,
                                _ => false,
                            })
                    }
                }
                (Value::Bool(a), Value::Bool(b)) => a == b,
                _ => false,
            };

            if is_match {
                // Execute the body of the matching arm
                let value = result.register(self.visit(&arm.body_node, context));
                if result.should_return() {
                    return result;
                }
                return result.success(value);
            }
        }

        // No match found - return null
        result.success(Value::Number(Number::null()))
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

        result.success(Value::Number(Number::null()))
    }

    fn visit_for(&mut self, node: &crate::nodes::ForNode, context: &mut Context) -> RuntimeResult {
        let mut result = RuntimeResult::new();
        let mut elements = Vec::new();

        let raw_var_name = node.variable_name_token.value.as_ref().unwrap();
        let var_name = raw_var_name.trim_matches(|c| c == '(' || c == ')');

        let iterable = result.register(self.visit(&node.start_value_node, context));
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

            _ => {
                let start = iterable;
                let end = result.register(self.visit(&node.end_value_node, context));
                if result.should_return() {
                    return result;
                }

                let step = if let Some(step_node) = &node.step_value_node {
                    result.register(self.visit(step_node, context))
                } else {
                    Value::Number(Number::new(1.0))
                };

                let start_val = match start.as_number() {
                    Some(n) => n.value,
                    None => {
                        return result.failure(
                            RuntimeError::new(
                                node.position_start.clone(),
                                node.position_end.clone(),
                                "For loop start must be a number",
                                Some(context.clone()),
                            )
                            .base,
                        );
                    }
                };
                let end_val = match end.as_number() {
                    Some(n) => n.value,
                    None => {
                        return result.failure(
                            RuntimeError::new(
                                node.position_start.clone(),
                                node.position_end.clone(),
                                "For loop end must be a number",
                                Some(context.clone()),
                            )
                            .base,
                        );
                    }
                };
                let step_val = match step.as_number() {
                    Some(n) => n.value,
                    None => {
                        return result.failure(
                            RuntimeError::new(
                                node.position_start.clone(),
                                node.position_end.clone(),
                                "For loop step must be a number",
                                Some(context.clone()),
                            )
                            .base,
                        );
                    }
                };

                let mut i = start_val;
                while if step_val >= 0.0 {
                    i < end_val
                } else {
                    i > end_val
                } {
                    let mut loop_ctx = context.create_child("<for>", Self::dummy_pos());
                    loop_ctx
                        .symbol_table
                        .set(var_name.to_string(), Value::Number(Number::new(i)));

                    let value = result.register(self.visit(&node.body_node, &mut loop_ctx));
                    if result.should_return()
                        && !result.loop_should_continue
                        && !result.loop_should_break
                    {
                        return result;
                    }
                    if result.loop_should_continue {
                        result.loop_should_continue = false;
                        i += step_val;
                        continue;
                    }
                    if result.loop_should_break {
                        result.loop_should_break = false;
                        break;
                    }
                    elements.push(value);
                    i += step_val;
                }
            }
        }

        if node.should_return_null {
            result.success(Value::Number(Number::null()))
        } else {
            result.success(Value::List(List::new(elements)))
        }
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
            result.success(Value::Number(Number::null()))
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

        // Check if this is a static method call (format: "StructName::methodName")
        if let Node::VarAccess(var_node) = &*node.node_to_call {
            let call_name = var_node.variable_name_token.value.as_ref().unwrap();
            if call_name.contains("::") {
                let parts: Vec<&str> = call_name.split("::").collect();
                if parts.len() == 2 {
                    let struct_name = parts[0].to_string();
                    let method_name = parts[1].to_string();

                    // Clone the method before borrowing self again
                    let method_clone = {
                        if let Some(struct_info) = self.struct_registry.get(&struct_name) {
                            if let Some(method) = struct_info.get_method(&method_name) {
                                Some(method.clone())
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    };

                    if let Some(method) = method_clone {
                        // Evaluate arguments
                        let mut args = Vec::new();
                        for arg_node in &node.argument_nodes {
                            let arg = result.register(self.visit(arg_node, context));
                            if result.should_return() {
                                return result;
                            }
                            args.push(arg);
                        }

                        // The first argument should be the instance (self)
                        if args.is_empty() {
                            return result.failure(
                                RuntimeError::new(
                                    node.position_start.clone(),
                                    node.position_end.clone(),
                                    &format!("Method '{}' requires self argument", method_name),
                                    Some(context.clone()),
                                )
                                .base,
                            );
                        }

                        let instance = args[0].clone();
                        let mut method_context = context.create_child(
                            &format!("{}::{}", struct_name, method_name),
                            node.position_start.clone(),
                        );

                        // Bind 'self' parameter
                        method_context
                            .symbol_table
                            .set_local("self".to_string(), instance);

                        // Bind remaining parameters (skip self)
                        for (i, param_name) in method.param_names.iter().enumerate() {
                            if i == 0 {
                                continue; // Skip self, already bound
                            }
                            let param_name_str = param_name.value.as_ref().unwrap();
                            if i - 1 < args.len() - 1 {
                                method_context
                                    .symbol_table
                                    .set_local(param_name_str.clone(), args[i].clone());
                            }
                        }

                        // Execute the method body
                        let exec_result = self.visit(&method.body_node, &mut method_context);

                        if let Some(err) = exec_result.error {
                            return RuntimeResult::new().failure(err);
                        }

                        // Write mutated self back to the caller's variable
                        if let Some(arg_node) = node.argument_nodes.get(0) {
                            if let Node::VarAccess(var_node) = arg_node.as_ref() {
                                let var_name =
                                    var_node.variable_name_token.value.as_ref().unwrap().clone();
                                if let Some(updated_self) = method_context.symbol_table.get("self")
                                {
                                    context
                                        .symbol_table
                                        .set_existing(var_name, updated_self.clone()); // &self
                                }
                            }
                        }

                        if let Some(ret_val) = exec_result.func_return_value {
                            return RuntimeResult::new().success(ret_val);
                        }

                        if let Some(val) = exec_result.value {
                            return RuntimeResult::new().success(val);
                        }

                        return RuntimeResult::new().success(Value::Number(Number::null()));
                    } else {
                        return result.failure(
                            RuntimeError::new(
                                node.position_start.clone(),
                                node.position_end.clone(),
                                &format!(
                                    "Method '{}' not found for struct '{}'",
                                    method_name, struct_name
                                ),
                                Some(context.clone()),
                            )
                            .base,
                        );
                    }
                }
            }
        }

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
            Value::Function(func) => func.execute(args, context.clone(), self),
            Value::BuiltInFunction(builtin) => builtin.execute(args, self),
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
                        Value::Number(n) => Some(n.value as usize),
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
                RuntimeResult::new().success(Value::Number(Number::new(list.len() as f64)))
            }
            (Value::Map(map), "items") => RuntimeResult::new().success(Value::List(map.items())),
            (Value::Map(map), "keys") => RuntimeResult::new().success(Value::List(map.keys())),
            (Value::Map(map), "values") => RuntimeResult::new().success(Value::List(map.values())),
            (Value::Map(map), "len") => {
                RuntimeResult::new().success(Value::Number(Number::new(map.len() as f64)))
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
                let result = map.contains_key(key);
                RuntimeResult::new().success(Value::Number(Number::new(if result {
                    1.0
                } else {
                    0.0
                })))
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

    fn visit_return(
        &mut self,
        node: &crate::nodes::ReturnNode,
        context: &mut Context,
    ) -> RuntimeResult {
        let mut result = RuntimeResult::new();

        let value = if let Some(expr) = &node.node_to_return {
            result.register(self.visit(expr, context))
        } else {
            Value::Number(Number::null())
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
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}
