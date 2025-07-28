//! Expression evaluation engine for workflow DSL

use crate::error::{DslError, Result};
use chrono::Utc;
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

/// Expression evaluator for DSL expressions
#[derive(Debug, Clone)]
pub struct ExpressionEvaluator {
    built_in_functions: HashMap<String, BuiltInFunction>,
}

/// Built-in function definition
#[derive(Debug, Clone)]
pub struct BuiltInFunction {
    pub name: String,
    pub arity: Option<usize>, // None means variadic
    pub evaluator: fn(&[ExpressionValue]) -> Result<ExpressionValue>,
}

/// Value types for expression evaluation
#[derive(Debug, Clone, PartialEq)]
pub enum ExpressionValue {
    String(String),
    Number(f64),
    Boolean(bool),
    Array(Vec<ExpressionValue>),
    Object(HashMap<String, ExpressionValue>),
    Null,
}

impl ExpressionEvaluator {
    /// Create a new expression evaluator
    pub fn new() -> Self {
        let mut evaluator = Self {
            built_in_functions: HashMap::new(),
        };

        evaluator.register_built_in_functions();
        evaluator
    }

    /// Evaluate an expression string with given context
    pub fn evaluate(&self, expression: &str, context: &HashMap<String, Value>) -> Result<Value> {
        if !expression.starts_with('$') {
            // Not an expression, return as literal string
            return Ok(Value::String(expression.to_string()));
        }

        let expr_value = self.evaluate_expression(expression, context)?;
        Ok(Self::expression_value_to_json(expr_value))
    }

    /// Internal expression evaluation
    fn evaluate_expression(
        &self,
        expression: &str,
        context: &HashMap<String, Value>,
    ) -> Result<ExpressionValue> {
        let expr = &expression[1..]; // Remove '$' prefix

        if expr.contains('(') && expr.ends_with(')') {
            // Function call
            self.evaluate_function_call(expr, context)
        } else if expr.contains('.') {
            // Path expression
            self.evaluate_path_expression(expr, context)
        } else {
            // Simple variable reference
            self.evaluate_variable_reference(expr, context)
        }
    }

    /// Evaluate a function call expression
    fn evaluate_function_call(
        &self,
        expression: &str,
        context: &HashMap<String, Value>,
    ) -> Result<ExpressionValue> {
        let paren_pos = expression.find('(').unwrap();
        let function_name = &expression[..paren_pos];
        let args_str = &expression[paren_pos + 1..expression.len() - 1];

        let function = self.built_in_functions.get(function_name).ok_or_else(|| {
            DslError::InvalidExpression {
                expression: format!("Unknown function: {function_name}"),
            }
        })?;

        // Parse arguments
        let args = if args_str.trim().is_empty() {
            Vec::new()
        } else {
            self.parse_function_arguments(args_str, context)?
        };

        // Check arity
        if let Some(expected_arity) = function.arity {
            if args.len() != expected_arity {
                return Err(crate::error::AutomataError::DslParse(
                    DslError::InvalidExpression {
                        expression: format!(
                            "Function {} expects {} arguments, got {}",
                            function_name,
                            expected_arity,
                            args.len()
                        ),
                    },
                ));
            }
        }

        // Call function
        (function.evaluator)(&args)
    }

    /// Parse function arguments
    fn parse_function_arguments(
        &self,
        args_str: &str,
        context: &HashMap<String, Value>,
    ) -> Result<Vec<ExpressionValue>> {
        let mut args = Vec::new();
        let mut current_arg = String::new();
        let mut paren_count = 0;
        let mut in_quotes = false;
        let mut quote_char = '"';

        for ch in args_str.chars() {
            match ch {
                '"' | '\'' if !in_quotes => {
                    in_quotes = true;
                    quote_char = ch;
                    current_arg.push(ch);
                }
                ch if in_quotes && ch == quote_char => {
                    in_quotes = false;
                    current_arg.push(ch);
                }
                '(' if !in_quotes => {
                    paren_count += 1;
                    current_arg.push(ch);
                }
                ')' if !in_quotes => {
                    paren_count -= 1;
                    current_arg.push(ch);
                }
                ',' if !in_quotes && paren_count == 0 => {
                    let arg = self.parse_single_argument(current_arg.trim(), context)?;
                    args.push(arg);
                    current_arg.clear();
                }
                _ => {
                    current_arg.push(ch);
                }
            }
        }

        if !current_arg.trim().is_empty() {
            let arg = self.parse_single_argument(current_arg.trim(), context)?;
            args.push(arg);
        }

        Ok(args)
    }

    /// Parse a single function argument
    fn parse_single_argument(
        &self,
        arg_str: &str,
        context: &HashMap<String, Value>,
    ) -> Result<ExpressionValue> {
        if arg_str.starts_with('$') {
            // Expression argument
            self.evaluate_expression(arg_str, context)
        } else if arg_str.starts_with('"') && arg_str.ends_with('"') {
            // String literal
            Ok(ExpressionValue::String(
                arg_str[1..arg_str.len() - 1].to_string(),
            ))
        } else if arg_str.starts_with('\'') && arg_str.ends_with('\'') {
            // String literal (single quotes)
            Ok(ExpressionValue::String(
                arg_str[1..arg_str.len() - 1].to_string(),
            ))
        } else if let Ok(num) = arg_str.parse::<f64>() {
            // Number literal
            Ok(ExpressionValue::Number(num))
        } else if arg_str == "true" {
            Ok(ExpressionValue::Boolean(true))
        } else if arg_str == "false" {
            Ok(ExpressionValue::Boolean(false))
        } else if arg_str == "null" {
            Ok(ExpressionValue::Null)
        } else {
            // Treat as string literal
            Ok(ExpressionValue::String(arg_str.to_string()))
        }
    }

    /// Evaluate a path expression (e.g., trigger.body.user.id)
    fn evaluate_path_expression(
        &self,
        expression: &str,
        context: &HashMap<String, Value>,
    ) -> Result<ExpressionValue> {
        let parts: Vec<&str> = expression.split('.').collect();
        if parts.is_empty() {
            return Ok(ExpressionValue::Null);
        }

        let root_var = parts[0];
        let mut current_value = context.get(root_var).unwrap_or(&Value::Null);

        for part in parts.iter().skip(1) {
            match current_value {
                Value::Object(obj) => {
                    current_value = obj.get(*part).unwrap_or(&Value::Null);
                }
                Value::Array(arr) => {
                    if let Ok(index) = part.parse::<usize>() {
                        current_value = arr.get(index).unwrap_or(&Value::Null);
                    } else {
                        current_value = &Value::Null;
                    }
                }
                _ => {
                    current_value = &Value::Null;
                    break;
                }
            }
        }

        Ok(Self::json_value_to_expression_value(current_value.clone()))
    }

    /// Evaluate a simple variable reference
    fn evaluate_variable_reference(
        &self,
        variable: &str,
        context: &HashMap<String, Value>,
    ) -> Result<ExpressionValue> {
        let value = context.get(variable).unwrap_or(&Value::Null);
        Ok(Self::json_value_to_expression_value(value.clone()))
    }

    /// Register built-in functions
    fn register_built_in_functions(&mut self) {
        // now() - current timestamp
        self.built_in_functions.insert(
            "now".to_string(),
            BuiltInFunction {
                name: "now".to_string(),
                arity: Some(0),
                evaluator: |_args| Ok(ExpressionValue::String(Utc::now().to_rfc3339())),
            },
        );

        // uuid() - generate UUID
        self.built_in_functions.insert(
            "uuid".to_string(),
            BuiltInFunction {
                name: "uuid".to_string(),
                arity: Some(0),
                evaluator: |_args| Ok(ExpressionValue::String(Uuid::new_v4().to_string())),
            },
        );

        // hash(value) - hash a string
        self.built_in_functions.insert(
            "hash".to_string(),
            BuiltInFunction {
                name: "hash".to_string(),
                arity: Some(1),
                evaluator: |args| {
                    let input = match &args[0] {
                        ExpressionValue::String(s) => s.clone(),
                        other => format!("{other:?}"),
                    };

                    // Simple hash implementation - in production, use a proper hash function
                    let hash = crate::utils::hash_password(&input).map_err(|_| {
                        DslError::InvalidExpression {
                            expression: "Failed to hash value".to_string(),
                        }
                    })?;

                    Ok(ExpressionValue::String(hash))
                },
            },
        );

        // length(value) - get length of string or array
        self.built_in_functions.insert(
            "length".to_string(),
            BuiltInFunction {
                name: "length".to_string(),
                arity: Some(1),
                evaluator: |args| {
                    let length = match &args[0] {
                        ExpressionValue::String(s) => s.len(),
                        ExpressionValue::Array(arr) => arr.len(),
                        _ => 0,
                    };
                    Ok(ExpressionValue::Number(length as f64))
                },
            },
        );

        // empty(value) - check if value is empty
        self.built_in_functions.insert(
            "empty".to_string(),
            BuiltInFunction {
                name: "empty".to_string(),
                arity: Some(1),
                evaluator: |args| {
                    let is_empty = match &args[0] {
                        ExpressionValue::String(s) => s.is_empty(),
                        ExpressionValue::Array(arr) => arr.is_empty(),
                        ExpressionValue::Object(obj) => obj.is_empty(),
                        ExpressionValue::Null => true,
                        _ => false,
                    };
                    Ok(ExpressionValue::Boolean(is_empty))
                },
            },
        );

        // not(value) - logical NOT
        self.built_in_functions.insert(
            "not".to_string(),
            BuiltInFunction {
                name: "not".to_string(),
                arity: Some(1),
                evaluator: |args| {
                    let is_truthy = match &args[0] {
                        ExpressionValue::Boolean(b) => *b,
                        ExpressionValue::String(s) => !s.is_empty(),
                        ExpressionValue::Number(n) => *n != 0.0,
                        ExpressionValue::Array(arr) => !arr.is_empty(),
                        ExpressionValue::Object(obj) => !obj.is_empty(),
                        ExpressionValue::Null => false,
                    };
                    Ok(ExpressionValue::Boolean(!is_truthy))
                },
            },
        );
    }

    /// Convert JSON value to expression value
    fn json_value_to_expression_value(value: Value) -> ExpressionValue {
        match value {
            Value::String(s) => ExpressionValue::String(s),
            Value::Number(n) => ExpressionValue::Number(n.as_f64().unwrap_or(0.0)),
            Value::Bool(b) => ExpressionValue::Boolean(b),
            Value::Array(arr) => {
                let expr_arr = arr
                    .into_iter()
                    .map(Self::json_value_to_expression_value)
                    .collect();
                ExpressionValue::Array(expr_arr)
            }
            Value::Object(obj) => {
                let expr_obj = obj
                    .into_iter()
                    .map(|(k, v)| (k, Self::json_value_to_expression_value(v)))
                    .collect();
                ExpressionValue::Object(expr_obj)
            }
            Value::Null => ExpressionValue::Null,
        }
    }

    /// Convert expression value to JSON value
    fn expression_value_to_json(value: ExpressionValue) -> Value {
        match value {
            ExpressionValue::String(s) => Value::String(s),
            ExpressionValue::Number(n) => Value::Number(
                serde_json::Number::from_f64(n).unwrap_or_else(|| serde_json::Number::from(0)),
            ),
            ExpressionValue::Boolean(b) => Value::Bool(b),
            ExpressionValue::Array(arr) => {
                let json_arr = arr
                    .into_iter()
                    .map(Self::expression_value_to_json)
                    .collect();
                Value::Array(json_arr)
            }
            ExpressionValue::Object(obj) => {
                let json_obj = obj
                    .into_iter()
                    .map(|(k, v)| (k, Self::expression_value_to_json(v)))
                    .collect();
                Value::Object(json_obj)
            }
            ExpressionValue::Null => Value::Null,
        }
    }
}

impl Default for ExpressionEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_expression_evaluator() {
        let evaluator = ExpressionEvaluator::new();
        let mut context = HashMap::new();
        context.insert("user".to_string(), json!({"name": "John", "age": 30}));

        // Test path expression
        let result = evaluator.evaluate("$user.name", &context).unwrap();
        assert_eq!(result, json!("John"));

        // Test function call
        let result = evaluator.evaluate("$uuid()", &context).unwrap();
        assert!(!result.as_str().unwrap().is_empty());

        // Test built-in function
        let result = evaluator.evaluate("$length(\"hello\")", &context).unwrap();
        assert_eq!(result, json!(5.0));
    }

    #[test]
    fn test_function_arguments() {
        let evaluator = ExpressionEvaluator::new();
        let context = HashMap::new();

        let result = evaluator.evaluate("$empty(\"\")", &context).unwrap();
        assert_eq!(result, json!(true));

        let result = evaluator.evaluate("$not(true)", &context).unwrap();
        assert_eq!(result, json!(false));
    }

    #[test]
    fn test_literal_expressions() {
        let evaluator = ExpressionEvaluator::new();
        let context = HashMap::new();

        let result = evaluator.evaluate("literal string", &context).unwrap();
        assert_eq!(result, json!("literal string"));
    }
}
