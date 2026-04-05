//! # Interpreter Module
//!
//! Traverses the Abstract Syntax Tree and executes the program.
//! Implements the runtime semantics for each AST node type including
//! variable access, control flow, function calls, and built-in operations.

use crate::context::Context;
use crate::error::RuntimeError;
use crate::nodes::Node;
use crate::runtime_result::RuntimeResult;
use crate::symbol_table::SymbolTable;
use crate::values::{BuiltInFunction, Function, List, Number, Value, XenithString};

/// Main interpreter that traverses and executes the AST
pub struct Interpreter {
    /// Global symbol table with built-in functions
    pub global_symbol_table: SymbolTable,
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

        Self {
            global_symbol_table: global,
        }
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
        }
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
            let elem = result.register(self.visit(elem_node, context));
            if result.should_return() {
                return result;
            }
            elements.push(elem);
        }

        result.success(Value::List(List::new(elements)))
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

        match context.symbol_table.get(var_name) {
            Some(value) => RuntimeResult::new().success(value.clone()),
            None => match self.global_symbol_table.get(var_name) {
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
            },
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

        // Try to find the variable in the current scope or parent scopes
        // If found, update it in its original scope
        // If not found, create it in the current scope
        if let Some(_) = context.symbol_table.get(var_name) {
            // Variable exists in current scope, update it
            context.symbol_table.set(var_name.clone(), value.clone());
        } else if let Some(parent) = &mut context.parent {
            // Search in parent scopes (you'd need a recursive search function)
            // For simplicity, just set in current scope for now
            context.symbol_table.set(var_name.clone(), value.clone());
        } else {
            // Variable doesn't exist, create it in current scope
            context.symbol_table.set(var_name.clone(), value.clone());
        }

        result.success(value)
    }

    fn visit_binary_op(
        &mut self,
        node: &crate::nodes::BinaryOperatorNode,
        context: &mut Context,
    ) -> RuntimeResult {
        let mut result = RuntimeResult::new();

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
            // Add logical operators
            _ if op.matches(crate::tokens::TokenType::Keyword, Some("&&")) => left.anded_by(&right),
            _ if op.matches(crate::tokens::TokenType::Keyword, Some("||")) => left.ored_by(&right),
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

        let start = result.register(self.visit(&node.start_value_node, context));
        if result.should_return() {
            return result;
        }

        let end = result.register(self.visit(&node.end_value_node, context));
        if result.should_return() {
            return result;
        }

        let step = if let Some(step_node) = &node.step_value_node {
            result.register(self.visit(step_node, context))
        } else {
            Value::Number(Number::new(1.0))
        };

        let start_val = start.as_number().unwrap().value;
        let end_val = end.as_number().unwrap().value;
        let step_val = step.as_number().unwrap().value;

        let var_name = node.variable_name_token.value.as_ref().unwrap();

        let mut i = start_val;
        while if step_val >= 0.0 {
            i < end_val
        } else {
            i > end_val
        } {
            context
                .symbol_table
                .set(var_name.clone(), Value::Number(Number::new(i)));

            let value = result.register(self.visit(&node.body_node, context));
            if result.should_return() && !result.loop_should_continue && !result.loop_should_break {
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
            .arg_name_toks
            .iter()
            .map(|t| t.value.as_ref().unwrap().clone())
            .collect();

        let func = Function::new(
            func_name.clone(),
            *node.body_node.clone(),
            arg_names,
            node.should_auto_return,
        );

        let func_value = Value::Function(Box::new(func));

        if let Some(name) = func_name {
            context.symbol_table.set(name, func_value.clone());
        }

        RuntimeResult::new().success(func_value)
    }

    fn visit_call(
        &mut self,
        node: &crate::nodes::CallNode,
        context: &mut Context,
    ) -> RuntimeResult {
        let mut result = RuntimeResult::new();

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

        // Execute the function or built-in safely
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

        // Register the result properly to propagate errors, returns, and loop controls
        let value = result.register(call_result);

        result.success(value)
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
