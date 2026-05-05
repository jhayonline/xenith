//! # Linting Module
//!
//! Provides static analysis to catch common bugs and style issues.

use crate::nodes::Node;
use crate::position::Position;

#[derive(Debug, Clone)]
pub struct LintWarning {
    pub code: String,
    pub message: String,
    pub position: Position,
    pub suggestion: Option<String>,
}

pub struct Linter;

impl Linter {
    pub fn lint(node: &Node) -> Vec<LintWarning> {
        let mut warnings = Vec::new();

        match node {
            Node::BinaryOperator(node) => {
                // Check for division by zero constant
                if let (_, Node::Number(num)) = (&*node.left_node, &*node.right_node) {
                    if node.operator_token.kind == crate::tokens::TokenType::Div {
                        if let Ok(val) = num.token.value.as_ref().unwrap().parse::<f64>() {
                            if val == 0.0 {
                                warnings.push(LintWarning {
                                    code: "XENL001".to_string(),
                                    message: "Division by zero constant".to_string(),
                                    position: node.position_start.clone(),
                                    suggestion: Some(
                                        "Check if denominator can be zero".to_string(),
                                    ),
                                });
                            }
                        }
                    }
                }
            }
            Node::VarAssign(node) if node.is_constant => {
                // Check if constant is reassigned elsewhere (would need data flow)
            }
            Node::If(node) if node.cases.len() > 5 => {
                warnings.push(LintWarning {
                    code: "XENL002".to_string(),
                    message: "Too many `when` branches".to_string(),
                    position: node.position_start.clone(),
                    suggestion: Some("Consider using `match` for many branches".to_string()),
                });
            }
            _ => {}
        }

        warnings
    }
}
