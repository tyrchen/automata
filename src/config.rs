//! Application configuration module

use config::{Config, ConfigError, Environment, File};
use serde::{Deserialize, Serialize};
use std::env;
use std::time::Duration;

/// Main application configuration
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub execution: ExecutionConfig,
    pub security: SecurityConfig,
    pub api: ApiConfig,
    pub logging: LoggingConfig,
    pub nodes: NodesConfig,
    pub monitoring: MonitoringConfig,
    pub development: DevelopmentConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connect_timeout: u64,
    pub idle_timeout: u64,
    pub max_lifetime: u64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RedisConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connect_timeout: u64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ExecutionConfig {
    pub max_concurrent_executions: usize,
    pub max_concurrent_nodes: usize,
    pub default_node_timeout: u64,
    pub max_execution_timeout: u64,
    pub enable_checkpointing: bool,
    pub checkpoint_interval: u64,
    pub worker_threads: usize,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SecurityConfig {
    pub jwt_secret: String,
    pub jwt_expiry: u64,
    pub enable_auth: bool,
    pub cors_origins: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ApiConfig {
    pub base_path: String,
    pub request_timeout: u64,
    pub max_body_size: usize,
    pub enable_swagger: bool,
    pub swagger_path: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String,
    pub file_enabled: bool,
    pub file_path: String,
    pub file_max_size: String,
    pub file_max_files: u32,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct NodesConfig {
    pub http: HttpNodeConfig,
    pub database: DatabaseNodeConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct HttpNodeConfig {
    pub default_timeout: u64,
    pub max_redirects: u32,
    pub user_agent: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DatabaseNodeConfig {
    pub query_timeout: u64,
    pub log_queries: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MonitoringConfig {
    pub metrics_enabled: bool,
    pub metrics_path: String,
    pub health_check_enabled: bool,
    pub health_check_path: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DevelopmentConfig {
    pub debug: bool,
    pub log_requests: bool,
    pub log_sql: bool,
    pub mock_external: bool,
}

impl AppConfig {
    /// Load configuration from file and environment variables
    pub fn load() -> Result<Self, ConfigError> {
        let config_dir = env::current_dir()
            .map_err(|e| ConfigError::Message(format!("Failed to get current directory: {e}")))?
            .join("config");

        let builder = Config::builder()
            // Start with default configuration
            .add_source(File::from(config_dir.join("app.yml")))
            // Add environment-specific configuration
            .add_source(
                File::from(config_dir.join(format!(
                    "app.{}.yml",
                    env::var("APP_ENV").unwrap_or_else(|_| "development".to_string())
                )))
                .required(false),
            )
            // Add environment variables (with prefix AUTOMATA_)
            .add_source(
                Environment::with_prefix("AUTOMATA")
                    .prefix_separator("_")
                    .separator("__"),
            )
            // Override specific values from common environment variables
            .set_override_option("database.url", env::var("DATABASE_URL").ok())?
            .set_override_option("redis.url", env::var("REDIS_URL").ok())?
            .set_override_option("security.jwt_secret", env::var("JWT_SECRET").ok())?
            .set_override_option("server.port", env::var("API_PORT").ok())?
            .set_override_option("execution.worker_threads", env::var("WORKER_THREADS").ok())?;

        let config = builder.build()?;
        config.try_deserialize()
    }

    /// Get database URL with proper formatting
    pub fn database_url(&self) -> &str {
        &self.database.url
    }

    /// Get Redis URL if Redis is enabled
    pub fn redis_url(&self) -> Option<&str> {
        if self.redis.url.is_empty() || self.redis.url == "none" {
            None
        } else {
            Some(&self.redis.url)
        }
    }

    /// Convert execution config to ExecutionEngineConfig
    pub fn to_execution_engine_config(&self) -> crate::core::engine::ExecutionEngineConfig {
        crate::core::engine::ExecutionEngineConfig {
            max_concurrent_executions: self.execution.max_concurrent_executions,
            max_concurrent_nodes: self.execution.max_concurrent_nodes,
            default_node_timeout: Duration::from_secs(self.execution.default_node_timeout),
            max_execution_timeout: Duration::from_secs(self.execution.max_execution_timeout),
            enable_checkpointing: self.execution.enable_checkpointing,
            checkpoint_interval: Duration::from_secs(self.execution.checkpoint_interval),
        }
    }

    /// Get worker threads count (0 means use number of CPU cores)
    pub fn worker_threads(&self) -> usize {
        if self.execution.worker_threads == 0 {
            num_cpus::get()
        } else {
            self.execution.worker_threads
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8080,
            },
            database: DatabaseConfig {
                url: "postgres://postgres:password@localhost:5432/automata".to_string(),
                max_connections: 20,
                min_connections: 5,
                connect_timeout: 30,
                idle_timeout: 600,
                max_lifetime: 1800,
            },
            redis: RedisConfig {
                url: "redis://localhost:6379".to_string(),
                max_connections: 10,
                min_connections: 2,
                connect_timeout: 10,
            },
            execution: ExecutionConfig {
                max_concurrent_executions: 1000,
                max_concurrent_nodes: 100,
                default_node_timeout: 30,
                max_execution_timeout: 300,
                enable_checkpointing: true,
                checkpoint_interval: 30,
                worker_threads: 0,
            },
            security: SecurityConfig {
                jwt_secret: "change-me-in-production".to_string(),
                jwt_expiry: 86400,
                enable_auth: true,
                cors_origins: vec![
                    "http://localhost:3000".to_string(),
                    "http://localhost:5173".to_string(),
                ],
            },
            api: ApiConfig {
                base_path: "/api/v1".to_string(),
                request_timeout: 30,
                max_body_size: 10485760,
                enable_swagger: true,
                swagger_path: "/swagger-ui".to_string(),
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                format: "pretty".to_string(),
                file_enabled: false,
                file_path: "./logs/automata.log".to_string(),
                file_max_size: "100MB".to_string(),
                file_max_files: 10,
            },
            nodes: NodesConfig {
                http: HttpNodeConfig {
                    default_timeout: 30,
                    max_redirects: 5,
                    user_agent: "Automata/1.0".to_string(),
                },
                database: DatabaseNodeConfig {
                    query_timeout: 30,
                    log_queries: false,
                },
            },
            monitoring: MonitoringConfig {
                metrics_enabled: true,
                metrics_path: "/metrics".to_string(),
                health_check_enabled: true,
                health_check_path: "/health".to_string(),
            },
            development: DevelopmentConfig {
                debug: false,
                log_requests: true,
                log_sql: false,
                mock_external: false,
            },
        }
    }
}
