//! Utility functions and helpers

use chrono::{DateTime, Utc};
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

/// Generate a unique ID
pub fn generate_id() -> String {
    Uuid::new_v4().to_string()
}

/// Get current timestamp
pub fn now() -> DateTime<Utc> {
    Utc::now()
}

/// Evaluate expressions in workflow context
pub fn evaluate_expression(expr: &str, context: &HashMap<String, Value>) -> crate::Result<Value> {
    // Simple expression evaluation - can be extended with a proper expression engine
    if let Some(path) = expr.strip_prefix('$') {
        evaluate_path(path, context)
    } else {
        Ok(Value::String(expr.to_string()))
    }
}

/// Evaluate a dot-notation path in the context
fn evaluate_path(path: &str, context: &HashMap<String, Value>) -> crate::Result<Value> {
    let parts: Vec<&str> = path.split('.').collect();
    if parts.is_empty() {
        return Ok(Value::Null);
    }

    let mut current = context.get(parts[0]).unwrap_or(&Value::Null);

    for part in parts.iter().skip(1) {
        match current {
            Value::Object(obj) => {
                current = obj.get(*part).unwrap_or(&Value::Null);
            }
            Value::Array(arr) => {
                if let Ok(index) = part.parse::<usize>() {
                    current = arr.get(index).unwrap_or(&Value::Null);
                } else {
                    current = &Value::Null;
                }
            }
            _ => current = &Value::Null,
        }
    }

    Ok(current.clone())
}

/// Hash a password using argon2
pub fn hash_password(password: &str) -> crate::Result<String> {
    use argon2::password_hash::{rand_core::OsRng, SaltString};
    use argon2::{Argon2, PasswordHasher};

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|e| crate::error::AutomataError::Internal(format!("Failed to hash password: {e}")))
}

/// Verify a password against a hash
pub fn verify_password(password: &str, hash: &str) -> crate::Result<bool> {
    use argon2::password_hash::PasswordHash;
    use argon2::{Argon2, PasswordVerifier};

    let parsed_hash = PasswordHash::new(hash).map_err(|e| {
        crate::error::AutomataError::Internal(format!("Failed to parse password hash: {e}"))
    })?;

    let argon2 = Argon2::default();

    match argon2.verify_password(password.as_bytes(), &parsed_hash) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

/// Convert a serde_json::Value to a specific type
pub fn from_value<T>(value: Value) -> crate::Result<T>
where
    T: serde::de::DeserializeOwned,
{
    serde_json::from_value(value).map_err(Into::into)
}

/// Convert a type to serde_json::Value
pub fn to_value<T>(value: &T) -> crate::Result<Value>
where
    T: serde::Serialize,
{
    serde_json::to_value(value).map_err(Into::into)
}

/// Merge two JSON values
pub fn merge_json(a: &mut Value, b: Value) {
    match (a, b) {
        (Value::Object(ref mut a), Value::Object(b)) => {
            for (k, v) in b {
                merge_json(a.entry(k).or_insert(Value::Null), v);
            }
        }
        (a, b) => *a = b,
    }
}

/// Performance measurement utilities
pub mod perf {
    use std::time::{Duration, Instant};

    /// Timer for measuring execution time
    pub struct Timer {
        start: Instant,
        name: String,
    }

    impl Timer {
        pub fn new(name: impl Into<String>) -> Self {
            Self {
                start: Instant::now(),
                name: name.into(),
            }
        }

        pub fn elapsed(&self) -> Duration {
            self.start.elapsed()
        }

        pub fn elapsed_ms(&self) -> u64 {
            self.elapsed().as_millis() as u64
        }
    }

    impl Drop for Timer {
        fn drop(&mut self) {
            tracing::debug!(
                target: "automata::perf",
                timer = %self.name,
                elapsed_ms = self.elapsed_ms(),
                "Timer completed"
            );
        }
    }

    /// Macro for easy timing
    #[macro_export]
    macro_rules! time {
        ($name:expr, $code:block) => {{
            let _timer = $crate::utils::perf::Timer::new($name);
            $code
        }};
    }
}

/// Configuration utilities
pub mod config {
    use std::env;

    /// Get environment variable with default value
    pub fn env_var_or_default(key: &str, default: &str) -> String {
        env::var(key).unwrap_or_else(|_| default.to_string())
    }

    /// Get environment variable as integer with default
    pub fn env_var_or_default_int(key: &str, default: i32) -> i32 {
        env::var(key)
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(default)
    }

    /// Get environment variable as boolean with default
    pub fn env_var_or_default_bool(key: &str, default: bool) -> bool {
        env::var(key)
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(default)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_generate_id() {
        let id1 = generate_id();
        let id2 = generate_id();
        assert_ne!(id1, id2);
        assert!(Uuid::parse_str(&id1).is_ok());
    }

    #[test]
    fn test_evaluate_expression() {
        let mut context = HashMap::new();
        context.insert("user".to_string(), json!({"name": "test", "age": 25}));

        let result = evaluate_expression("$user.name", &context).unwrap();
        assert_eq!(result, json!("test"));

        let result = evaluate_expression("literal", &context).unwrap();
        assert_eq!(result, json!("literal"));
    }

    #[test]
    fn test_merge_json() {
        let mut a = json!({"x": 1, "y": {"z": 2}});
        let b = json!({"y": {"w": 3}, "q": 4});

        merge_json(&mut a, b);
        assert_eq!(a, json!({"x": 1, "y": {"z": 2, "w": 3}, "q": 4}));
    }
}
