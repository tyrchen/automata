//! HTTP request node for making API calls

use crate::core::execution::ExecutionContext;
use crate::error::{NodeError, Result};
use crate::nodes::traits::{
    BaseNodeValidator, Node, NodeCapabilities, NodeDescription, NodeExample, NodeInput,
    NodeMetadata, NodeOutput, NodeSchema, NodeValidator, PropertySchema, ResourceRequirements,
};
use async_trait::async_trait;
use reqwest::{Client, Method, Response};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;
use tracing::{debug, warn};

/// HTTP request node for making API calls
pub struct HttpNode {
    client: Client,
}

impl HttpNode {
    /// Create a new HTTP node
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("Automata-Workflow-Engine/1.0")
            .build()
            .expect("Failed to create HTTP client");

        Self { client }
    }

    /// Build HTTP request from configuration
    async fn build_request(
        &self,
        config: &Value,
        context: &ExecutionContext,
    ) -> Result<reqwest::RequestBuilder> {
        // Get URL
        let url_template = config.get("url").and_then(|v| v.as_str()).ok_or_else(|| {
            crate::error::AutomataError::Node(NodeError::InvalidConfig {
                node_type: "http".to_string(),
                message: "Missing required field 'url'".to_string(),
            })
        })?;

        let url = context
            .evaluate_expression(url_template)?
            .as_str()
            .ok_or_else(|| {
                crate::error::AutomataError::Node(NodeError::InvalidConfig {
                    node_type: "http".to_string(),
                    message: "URL must be a string".to_string(),
                })
            })?
            .to_string();

        // Get method
        let method_str = config
            .get("method")
            .and_then(|v| v.as_str())
            .unwrap_or("GET");

        let method = Method::from_str(method_str).map_err(|_| {
            crate::error::AutomataError::Node(NodeError::InvalidConfig {
                node_type: "http".to_string(),
                message: format!("Invalid HTTP method: {method_str}"),
            })
        })?;

        let mut request = self.client.request(method, &url);

        // Add headers
        if let Some(headers) = config.get("headers").and_then(|v| v.as_object()) {
            for (key, value) in headers {
                let header_value = if value.is_string() {
                    let template = value.as_str().unwrap();
                    context.evaluate_expression(template)?
                } else {
                    value.clone()
                };

                if let Some(header_str) = header_value.as_str() {
                    request = request.header(key, header_str);
                }
            }
        }

        // Add query parameters
        if let Some(query) = config.get("query").and_then(|v| v.as_object()) {
            let mut query_params = Vec::new();
            for (key, value) in query {
                let param_value = if value.is_string() {
                    let template = value.as_str().unwrap();
                    context.evaluate_expression(template)?
                } else {
                    value.clone()
                };

                if let Some(param_str) = param_value.as_str() {
                    query_params.push((key.clone(), param_str.to_string()));
                } else {
                    query_params.push((key.clone(), param_value.to_string()));
                }
            }
            request = request.query(&query_params);
        }

        // Add body
        if let Some(body) = config.get("body") {
            let body_value = if body.is_string() {
                let template = body.as_str().unwrap();
                context.evaluate_expression(template)?
            } else {
                body.clone()
            };

            // Determine content type
            let content_type = config
                .get("content_type")
                .and_then(|v| v.as_str())
                .unwrap_or("application/json");

            match content_type {
                "application/json" => {
                    request = request.json(&body_value);
                }
                "application/x-www-form-urlencoded" => {
                    if let Some(form_data) = body_value.as_object() {
                        let mut form = reqwest::multipart::Form::new();
                        for (key, value) in form_data {
                            if let Some(text_value) = value.as_str() {
                                form = form.text(key.clone(), text_value.to_string());
                            }
                        }
                        request = request.multipart(form);
                    }
                }
                "text/plain" => {
                    if let Some(text) = body_value.as_str() {
                        request = request.body(text.to_string());
                    }
                }
                _ => {
                    // Default to JSON
                    request = request.json(&body_value);
                }
            }
        }

        // Add authentication
        if let Some(auth) = config.get("auth").and_then(|v| v.as_object()) {
            if let Some(auth_type) = auth.get("type").and_then(|v| v.as_str()) {
                match auth_type {
                    "bearer" => {
                        if let Some(token) = auth.get("token").and_then(|v| v.as_str()) {
                            let token_value = context.evaluate_expression(token)?;
                            if let Some(token_str) = token_value.as_str() {
                                request = request.bearer_auth(token_str);
                            }
                        }
                    }
                    "basic" => {
                        let username = auth.get("username").and_then(|v| v.as_str()).unwrap_or("");
                        let password = auth.get("password").and_then(|v| v.as_str()).unwrap_or("");

                        let username_value = context.evaluate_expression(username)?;
                        let password_value = context.evaluate_expression(password)?;

                        if let (Some(user_str), Some(pass_str)) =
                            (username_value.as_str(), password_value.as_str())
                        {
                            request = request.basic_auth(user_str, Some(pass_str));
                        }
                    }
                    _ => {
                        warn!(auth_type = %auth_type, "Unsupported authentication type");
                    }
                }
            }
        }

        Ok(request)
    }

    /// Process HTTP response
    async fn process_response(&self, response: Response) -> Result<(Value, NodeMetadata)> {
        let status = response.status();
        let headers: HashMap<String, String> = response
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();

        debug!(status = %status, "HTTP response received");

        // Try to parse response body
        let body_text = response.text().await.map_err(|e| {
            crate::error::AutomataError::Node(NodeError::ExternalServiceError {
                service: "http".to_string(),
                message: format!("Failed to read response body: {e}"),
            })
        })?;

        // Try to parse as JSON, fallback to text
        let body = if body_text.trim().starts_with('{') || body_text.trim().starts_with('[') {
            serde_json::from_str(&body_text).unwrap_or_else(|_| Value::String(body_text.clone()))
        } else {
            Value::String(body_text)
        };

        let response_data = json!({
            "status": status.as_u16(),
            "headers": headers,
            "body": body,
            "success": status.is_success()
        });

        let metadata = NodeMetadata::success()
            .with_http_status(status.as_u16())
            .with_custom("headers".to_string(), json!(headers));

        Ok((response_data, metadata))
    }
}

#[async_trait]
impl Node for HttpNode {
    fn node_type(&self) -> &'static str {
        "http"
    }

    async fn validate_config(&self, config: &Value) -> Result<()> {
        BaseNodeValidator::validate_config(config, &self.config_schema())
    }

    async fn execute(
        &self,
        context: &mut ExecutionContext,
        input: NodeInput,
    ) -> Result<NodeOutput> {
        let start = std::time::Instant::now();

        // Build and execute request
        let request = self.build_request(&input.config, context).await?;

        let response = request.send().await.map_err(|e| {
            crate::error::AutomataError::Node(NodeError::ExternalServiceError {
                service: "http".to_string(),
                message: format!("Request failed: {e}"),
            })
        })?;

        // Process response
        let (data, mut metadata) = self.process_response(response).await?;
        metadata = metadata.with_duration(start.elapsed().as_millis() as u64);

        Ok(NodeOutput::with_metadata(data, metadata))
    }

    fn describe(&self) -> NodeDescription {
        NodeDescription {
            node_type: "http".to_string(),
            description: "Makes HTTP requests to external APIs and services".to_string(),
            inputs: self.input_schema(),
            outputs: self.output_schema(),
            config: self.config_schema(),
            examples: vec![
                NodeExample {
                    name: "Simple GET request".to_string(),
                    description: "Make a basic GET request to an API".to_string(),
                    config: json!({
                        "url": "https://api.example.com/users",
                        "method": "GET",
                        "headers": {
                            "Accept": "application/json"
                        }
                    }),
                    input: json!({}),
                    output: json!({
                        "status": 200,
                        "headers": {"content-type": "application/json"},
                        "body": {"users": []},
                        "success": true
                    }),
                },
                NodeExample {
                    name: "POST with JSON body".to_string(),
                    description: "Create a new resource with JSON payload".to_string(),
                    config: json!({
                        "url": "https://api.example.com/users",
                        "method": "POST",
                        "headers": {
                            "Content-Type": "application/json"
                        },
                        "body": {
                            "name": "$trigger.data.name",
                            "email": "$trigger.data.email"
                        }
                    }),
                    input: json!({}),
                    output: json!({
                        "status": 201,
                        "headers": {"content-type": "application/json"},
                        "body": {"id": 123, "name": "John", "email": "john@example.com"},
                        "success": true
                    }),
                },
            ],
        }
    }

    fn config_schema(&self) -> NodeSchema {
        let mut schema = NodeSchema {
            required: vec!["url".to_string()],
            ..Default::default()
        };

        let mut properties = HashMap::new();

        properties.insert(
            "url".to_string(),
            PropertySchema {
                property_type: "string".to_string(),
                description: "The URL to make the request to. Supports template expressions."
                    .to_string(),
                default: None,
                allowed_values: None,
                minimum: None,
                maximum: None,
                pattern: Some(r"^https?://.*".to_string()),
            },
        );

        properties.insert(
            "method".to_string(),
            PropertySchema {
                property_type: "string".to_string(),
                description: "HTTP method to use".to_string(),
                default: Some(json!("GET")),
                allowed_values: Some(vec![
                    json!("GET"),
                    json!("POST"),
                    json!("PUT"),
                    json!("DELETE"),
                    json!("PATCH"),
                    json!("HEAD"),
                    json!("OPTIONS"),
                ]),
                minimum: None,
                maximum: None,
                pattern: None,
            },
        );

        properties.insert(
            "headers".to_string(),
            PropertySchema {
                property_type: "object".to_string(),
                description: "HTTP headers to include in the request".to_string(),
                default: None,
                allowed_values: None,
                minimum: None,
                maximum: None,
                pattern: None,
            },
        );

        properties.insert(
            "query".to_string(),
            PropertySchema {
                property_type: "object".to_string(),
                description: "Query parameters to include in the request".to_string(),
                default: None,
                allowed_values: None,
                minimum: None,
                maximum: None,
                pattern: None,
            },
        );

        properties.insert(
            "body".to_string(),
            PropertySchema {
                property_type: "object".to_string(),
                description: "Request body data".to_string(),
                default: None,
                allowed_values: None,
                minimum: None,
                maximum: None,
                pattern: None,
            },
        );

        properties.insert(
            "auth".to_string(),
            PropertySchema {
                property_type: "object".to_string(),
                description: "Authentication configuration".to_string(),
                default: None,
                allowed_values: None,
                minimum: None,
                maximum: None,
                pattern: None,
            },
        );

        schema.properties = properties;
        schema
    }

    fn output_schema(&self) -> NodeSchema {
        let mut schema = NodeSchema::default();
        let mut properties = HashMap::new();

        properties.insert(
            "status".to_string(),
            PropertySchema {
                property_type: "number".to_string(),
                description: "HTTP status code".to_string(),
                default: None,
                allowed_values: None,
                minimum: Some(100.0),
                maximum: Some(599.0),
                pattern: None,
            },
        );

        properties.insert(
            "headers".to_string(),
            PropertySchema {
                property_type: "object".to_string(),
                description: "Response headers".to_string(),
                default: None,
                allowed_values: None,
                minimum: None,
                maximum: None,
                pattern: None,
            },
        );

        properties.insert(
            "body".to_string(),
            PropertySchema {
                property_type: "object".to_string(),
                description: "Response body (parsed as JSON if possible)".to_string(),
                default: None,
                allowed_values: None,
                minimum: None,
                maximum: None,
                pattern: None,
            },
        );

        properties.insert(
            "success".to_string(),
            PropertySchema {
                property_type: "boolean".to_string(),
                description: "Whether the request was successful (2xx status)".to_string(),
                default: None,
                allowed_values: None,
                minimum: None,
                maximum: None,
                pattern: None,
            },
        );

        schema.properties = properties;
        schema
    }

    fn capabilities(&self) -> NodeCapabilities {
        NodeCapabilities {
            supports_streaming: false,
            supports_batch: false,
            cacheable: true,
            has_side_effects: true,
            idempotent: false, // Depends on HTTP method, but generally false
            resource_requirements: ResourceRequirements {
                memory_mb: Some(10),
                cpu_percent: Some(5.0),
                network_io: true,
                disk_io: false,
                external_dependencies: vec!["internet".to_string()],
            },
        }
    }
}

impl Default for HttpNode {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::execution::ExecutionContext;
    use serde_json::json;
    use uuid::Uuid;

    fn create_test_context() -> ExecutionContext {
        ExecutionContext::new(Uuid::new_v4(), json!({"test": "data"}))
    }

    #[test]
    fn test_http_node_creation() {
        let node = HttpNode::new();
        assert_eq!(node.node_type(), "http");
    }

    #[tokio::test]
    async fn test_config_validation() {
        let node = HttpNode::new();

        let valid_config = json!({
            "url": "https://httpbin.org/get",
            "method": "GET"
        });

        assert!(node.validate_config(&valid_config).await.is_ok());

        let invalid_config = json!({
            "method": "GET"
            // Missing required 'url' field
        });

        assert!(node.validate_config(&invalid_config).await.is_err());
    }

    #[tokio::test]
    async fn test_http_get_request() {
        let node = HttpNode::new();
        let mut context = create_test_context();

        let input = NodeInput::new(
            json!({
                "url": "https://httpbin.org/get",
                "method": "GET",
                "headers": {
                    "User-Agent": "test"
                }
            }),
            json!({}),
        );

        let result = node.execute(&mut context, input).await;
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.metadata.success);
        assert_eq!(output.metadata.http_status, Some(200));
    }

    #[test]
    fn test_node_description() {
        let node = HttpNode::new();
        let description = node.describe();

        assert_eq!(description.node_type, "http");
        assert!(!description.description.is_empty());
        assert!(!description.examples.is_empty());
    }
}
