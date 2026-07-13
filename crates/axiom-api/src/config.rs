//! P2-K1 / P2-K2 / P2-K3: 配置管理模块
//!
//! 提供多环境配置管理，支持：
//! - 环境变量读取（统一前缀 `AXIOM_`）
//! - TOML 配置文件加载
//! - dotenv `.env` 文件支持
//! - 配置验证
//!
//! 该模块是独立的配置模块，不依赖 `router` 或 `builder`。

use crate::logging::LogFormat;
use serde::{Deserialize, Serialize};
use std::env;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use thiserror::Error;

// ---------------------------------------------------------------------------
// P2-K3: 配置错误类型
// ---------------------------------------------------------------------------

/// 配置加载与验证过程中产生的错误
#[derive(Debug, Error)]
pub enum ConfigError {
    /// 环境变量值无法解析
    #[error("environment variable {0} invalid: {1}")]
    InvalidEnvVar(String, String),
    /// `AXIOM_ENVIRONMENT` 值不合法
    #[error("invalid environment value: {0}")]
    InvalidEnvironment(String),
    /// 配置文件读取失败
    #[error("config file error: {0}")]
    FileError(String),
    /// TOML 解析失败
    #[error("TOML parse error: {0}")]
    ParseError(#[from] toml::de::Error),
    /// IO 错误
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    /// 配置验证失败
    #[error("validation failed: {0}")]
    Validation(String),
    /// 认证模式缺少必需的凭据
    #[error("auth mode '{0}' requires {1} to be set")]
    MissingAuthCredential(String, String),
}

// ---------------------------------------------------------------------------
// P2-K1: 运行环境
// ---------------------------------------------------------------------------

/// 运行环境枚举
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    #[default]
    Development,
    Test,
    Production,
}

impl std::fmt::Display for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Development => write!(f, "development"),
            Self::Test => write!(f, "test"),
            Self::Production => write!(f, "production"),
        }
    }
}

impl std::str::FromStr for Environment {
    type Err = ConfigError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "development" | "dev" => Ok(Self::Development),
            "test" | "testing" => Ok(Self::Test),
            "production" | "prod" => Ok(Self::Production),
            other => Err(ConfigError::InvalidEnvironment(other.to_string())),
        }
    }
}

// ---------------------------------------------------------------------------
// LogFormat serde 适配（不修改 logging.rs）
// ---------------------------------------------------------------------------

/// 为 `LogFormat` 提供小写字符串形式的 serde 支持
mod log_format_serde {
    use crate::logging::LogFormat;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(fmt: &LogFormat, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match fmt {
            LogFormat::Json => "json".serialize(serializer),
            LogFormat::Text => "text".serialize(serializer),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<LogFormat, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.to_lowercase().as_str() {
            "json" => Ok(LogFormat::Json),
            "text" => Ok(LogFormat::Text),
            other => Err(serde::de::Error::custom(format!("unknown log format: {}", other))),
        }
    }
}

// ---------------------------------------------------------------------------
// P2-K1: AppConfig
// ---------------------------------------------------------------------------

/// 应用配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// 运行环境
    pub environment: Environment,
    /// API 监听地址
    pub api_addr: SocketAddr,
    /// 请求体大小限制（字节）
    pub body_limit: usize,
    /// 速率限制：窗口内最大请求数
    pub rate_limit_max_requests: u32,
    /// 速率限制：时间窗口大小（秒）
    pub rate_limit_window_secs: u64,
    /// CORS 允许的源列表
    pub cors_allowed_origins: Vec<String>,
    /// 日志级别（例如 `info`、`debug`）
    pub log_level: String,
    /// 日志格式
    #[serde(with = "log_format_serde")]
    pub log_format: LogFormat,
    /// 是否启用分布式追踪
    pub tracing_enabled: bool,
    /// OTLP endpoint（启用追踪时使用）
    pub otlp_endpoint: Option<String>,
    /// 认证模式：`disabled` / `api_key` / `jwt`
    pub auth_mode: String,
    /// JWT 密钥（`auth_mode = "jwt"` 时必需）
    pub jwt_secret: Option<String>,
    /// API Key（`auth_mode = "api_key"` 时必需）
    pub api_key: Option<String>,
    /// SQLite 数据库文件路径
    pub db_path: PathBuf,
    /// 备份目录
    pub backup_dir: PathBuf,
    /// 备份间隔（分钟）
    pub backup_interval_minutes: u64,
    /// 最大备份数量
    pub max_backups: usize,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self::development()
    }
}

impl AppConfig {
    /// 开发环境预设
    pub fn development() -> Self {
        Self {
            environment: Environment::Development,
            api_addr: ([127, 0, 0, 1], 9092).into(),
            body_limit: 10 * 1024 * 1024,
            rate_limit_max_requests: 1000,
            rate_limit_window_secs: 60,
            cors_allowed_origins: vec!["*".to_string()],
            log_level: "debug".to_string(),
            log_format: LogFormat::Text,
            tracing_enabled: false,
            otlp_endpoint: None,
            auth_mode: "disabled".to_string(),
            jwt_secret: None,
            api_key: None,
            db_path: PathBuf::from("./data/axiom.db"),
            backup_dir: PathBuf::from("./backups"),
            backup_interval_minutes: 60,
            max_backups: 5,
        }
    }

    /// 测试环境预设
    pub fn test() -> Self {
        Self {
            environment: Environment::Test,
            api_addr: ([127, 0, 0, 1], 0).into(),
            body_limit: 1024 * 1024,
            rate_limit_max_requests: 10000,
            rate_limit_window_secs: 60,
            cors_allowed_origins: vec!["*".to_string()],
            log_level: "debug".to_string(),
            log_format: LogFormat::Text,
            tracing_enabled: false,
            otlp_endpoint: None,
            auth_mode: "disabled".to_string(),
            jwt_secret: None,
            api_key: None,
            db_path: PathBuf::from("./test-data/test.db"),
            backup_dir: PathBuf::from("./test-backups"),
            backup_interval_minutes: 60,
            max_backups: 3,
        }
    }

    /// 生产环境预设
    ///
    /// 注意：`jwt_secret` 与 `api_key` 默认为 `None`，需通过环境变量或配置文件提供。
    pub fn production() -> Self {
        Self {
            environment: Environment::Production,
            api_addr: ([0, 0, 0, 0], 9092).into(),
            body_limit: 10 * 1024 * 1024,
            rate_limit_max_requests: 100,
            rate_limit_window_secs: 60,
            cors_allowed_origins: vec![],
            log_level: "info".to_string(),
            log_format: LogFormat::Json,
            tracing_enabled: true,
            otlp_endpoint: Some("http://localhost:4318/v1/traces".to_string()),
            auth_mode: "jwt".to_string(),
            jwt_secret: None,
            api_key: None,
            db_path: PathBuf::from("/var/lib/axiom/axiom.db"),
            backup_dir: PathBuf::from("/var/lib/axiom/backups"),
            backup_interval_minutes: 30,
            max_backups: 10,
        }
    }

    /// 从环境变量读取配置（前缀 `AXIOM_`）
    ///
    /// 未设置的环境变量将使用 `AXIOM_ENVIRONMENT` 指定环境的预设值；
    /// 若 `AXIOM_ENVIRONMENT` 也未设置，则使用开发环境预设。
    pub fn from_env() -> Result<Self, ConfigError> {
        // 先读取环境，决定使用哪个预设作为基础
        let mut config = match env::var("AXIOM_ENVIRONMENT") {
            Ok(s) => {
                let env: Environment = s.parse()?;
                match env {
                    Environment::Development => Self::development(),
                    Environment::Test => Self::test(),
                    Environment::Production => Self::production(),
                }
            }
            Err(_) => Self::development(),
        };

        if let Ok(s) = env::var("AXIOM_API_ADDR") {
            config.api_addr = s.parse().map_err(|e| {
                ConfigError::InvalidEnvVar("AXIOM_API_ADDR".to_string(), format!("{}", e))
            })?;
        }
        if let Ok(s) = env::var("AXIOM_BODY_LIMIT") {
            config.body_limit = s.parse().map_err(|e| {
                ConfigError::InvalidEnvVar("AXIOM_BODY_LIMIT".to_string(), format!("{}", e))
            })?;
        }
        if let Ok(s) = env::var("AXIOM_RATE_LIMIT_MAX_REQUESTS") {
            config.rate_limit_max_requests = s.parse().map_err(|e| {
                ConfigError::InvalidEnvVar(
                    "AXIOM_RATE_LIMIT_MAX_REQUESTS".to_string(),
                    format!("{}", e),
                )
            })?;
        }
        if let Ok(s) = env::var("AXIOM_RATE_LIMIT_WINDOW_SECS") {
            config.rate_limit_window_secs = s.parse().map_err(|e| {
                ConfigError::InvalidEnvVar(
                    "AXIOM_RATE_LIMIT_WINDOW_SECS".to_string(),
                    format!("{}", e),
                )
            })?;
        }
        if let Ok(s) = env::var("AXIOM_CORS_ALLOWED_ORIGINS") {
            config.cors_allowed_origins =
                s.split(',').map(|o| o.trim().to_string()).filter(|o| !o.is_empty()).collect();
        }
        if let Ok(s) = env::var("AXIOM_LOG_LEVEL") {
            config.log_level = s;
        }
        if let Ok(s) = env::var("AXIOM_LOG_FORMAT") {
            config.log_format = match s.to_lowercase().as_str() {
                "json" => LogFormat::Json,
                "text" => LogFormat::Text,
                other => {
                    return Err(ConfigError::InvalidEnvVar(
                        "AXIOM_LOG_FORMAT".to_string(),
                        format!("unknown log format: {}", other),
                    ));
                }
            };
        }
        if let Ok(s) = env::var("AXIOM_TRACING_ENABLED") {
            config.tracing_enabled = match s.to_lowercase().as_str() {
                "true" | "1" | "yes" => true,
                "false" | "0" | "no" => false,
                other => {
                    return Err(ConfigError::InvalidEnvVar(
                        "AXIOM_TRACING_ENABLED".to_string(),
                        format!("invalid boolean: {}", other),
                    ));
                }
            };
        }
        if let Ok(s) = env::var("AXIOM_OTLP_ENDPOINT") {
            config.otlp_endpoint = if s.is_empty() { None } else { Some(s) };
        }
        if let Ok(s) = env::var("AXIOM_AUTH_MODE") {
            config.auth_mode = s;
        }
        if let Ok(s) = env::var("AXIOM_JWT_SECRET") {
            config.jwt_secret = if s.is_empty() { None } else { Some(s) };
        }
        if let Ok(s) = env::var("AXIOM_API_KEY") {
            config.api_key = if s.is_empty() { None } else { Some(s) };
        }
        if let Ok(s) = env::var("AXIOM_DB_PATH") {
            config.db_path = PathBuf::from(s);
        }
        if let Ok(s) = env::var("AXIOM_BACKUP_DIR") {
            config.backup_dir = PathBuf::from(s);
        }
        if let Ok(s) = env::var("AXIOM_BACKUP_INTERVAL_MINUTES") {
            config.backup_interval_minutes = s.parse().map_err(|e| {
                ConfigError::InvalidEnvVar(
                    "AXIOM_BACKUP_INTERVAL_MINUTES".to_string(),
                    format!("{}", e),
                )
            })?;
        }
        if let Ok(s) = env::var("AXIOM_MAX_BACKUPS") {
            config.max_backups = s.parse().map_err(|e| {
                ConfigError::InvalidEnvVar("AXIOM_MAX_BACKUPS".to_string(), format!("{}", e))
            })?;
        }

        Ok(config)
    }

    /// 从 TOML 配置文件加载
    pub fn load_from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self, ConfigError> {
        let path_ref = path.as_ref();
        let content = std::fs::read_to_string(path_ref)
            .map_err(|e| ConfigError::FileError(format!("{}: {}", path_ref.display(), e)))?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    /// 加载配置：
    ///
    /// 1. 先调用 `dotenvy::dotenv()` 加载 `.env` 文件（若存在）
    /// 2. 若环境变量 `AXIOM_CONFIG_FILE` 指定了配置文件路径，则从该文件加载
    /// 3. 否则从环境变量读取
    pub fn load() -> Result<Self, ConfigError> {
        // 加载 .env 文件（若存在）；忽略文件不存在错误
        let _ = dotenvy::dotenv();

        if let Ok(path) = env::var("AXIOM_CONFIG_FILE") {
            Self::load_from_file(path)
        } else {
            Self::from_env()
        }
    }

    /// 验证配置
    ///
    /// - `auth_mode = "jwt"` 时必须有 `jwt_secret`
    /// - `auth_mode = "api_key"` 时必须有 `api_key`
    /// - `backup_dir` 的父目录必须存在或可创建
    /// - `db_path` 的父目录必须存在或可创建
    /// - `rate_limit_max_requests` 必须 > 0
    /// - `body_limit` 必须 > 0
    pub fn validate(&self) -> Result<(), ConfigError> {
        // 认证凭据检查
        if self.auth_mode == "jwt" && self.jwt_secret.is_none() {
            return Err(ConfigError::MissingAuthCredential(
                self.auth_mode.clone(),
                "jwt_secret".to_string(),
            ));
        }
        if self.auth_mode == "api_key" && self.api_key.is_none() {
            return Err(ConfigError::MissingAuthCredential(
                self.auth_mode.clone(),
                "api_key".to_string(),
            ));
        }

        // backup_dir 父目录存在或可创建
        ensure_parent_dir(&self.backup_dir, "backup_dir")?;

        // db_path 父目录存在或可创建
        ensure_parent_dir(&self.db_path, "db_path")?;

        // rate_limit_max_requests > 0
        if self.rate_limit_max_requests == 0 {
            return Err(ConfigError::Validation("rate_limit_max_requests must be > 0".to_string()));
        }

        // body_limit > 0
        if self.body_limit == 0 {
            return Err(ConfigError::Validation("body_limit must be > 0".to_string()));
        }

        Ok(())
    }
}

/// 确保给定路径的父目录存在或可创建
fn ensure_parent_dir(path: &Path, field: &str) -> Result<(), ConfigError> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            std::fs::create_dir_all(parent).map_err(|e| {
                ConfigError::Validation(format!(
                    "failed to create parent directory for {} '{}': {}",
                    field,
                    parent.display(),
                    e
                ))
            })?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn development_preset_is_valid() {
        let config = AppConfig::development();
        config.validate().expect("development preset should be valid");
        assert_eq!(config.environment, Environment::Development);
        assert_eq!(config.auth_mode, "disabled");
    }

    #[test]
    fn test_preset_is_valid() {
        let config = AppConfig::test();
        config.validate().expect("test preset should be valid");
        assert_eq!(config.environment, Environment::Test);
    }

    #[test]
    fn production_preset_fails_validation_without_secret() {
        let config = AppConfig::production();
        let err = config.validate().expect_err("production preset without secret should fail");
        matches!(err, ConfigError::MissingAuthCredential(_, _));
    }

    #[test]
    fn production_preset_validates_with_secret() {
        let config =
            AppConfig { jwt_secret: Some("test-secret".to_string()), ..AppConfig::production() };
        // 注：生产预设的 db_path / backup_dir 指向 /var/lib/axiom，测试环境可能无权创建
        // 这里仅验证认证部分，目录检查可能在 CI 上失败，因此使用本地路径覆盖
        let config = AppConfig {
            jwt_secret: Some("test-secret".to_string()),
            db_path: PathBuf::from("./test-data/prod.db"),
            backup_dir: PathBuf::from("./test-data/prod-backups"),
            ..config
        };
        config.validate().expect("production config with secret should be valid");
    }

    #[test]
    fn environment_parses_aliases() {
        assert_eq!("dev".parse::<Environment>().unwrap(), Environment::Development);
        assert_eq!("test".parse::<Environment>().unwrap(), Environment::Test);
        assert_eq!("prod".parse::<Environment>().unwrap(), Environment::Production);
        assert_eq!("development".parse::<Environment>().unwrap(), Environment::Development);
        assert_eq!("production".parse::<Environment>().unwrap(), Environment::Production);
    }

    #[test]
    fn environment_rejects_invalid() {
        assert!("staging".parse::<Environment>().is_err());
    }

    #[test]
    fn api_key_mode_requires_api_key() {
        let config = AppConfig {
            auth_mode: "api_key".to_string(),
            api_key: None,
            ..AppConfig::development()
        };
        let err = config.validate().expect_err("api_key mode without key should fail");
        assert!(matches!(err, ConfigError::MissingAuthCredential(_, _)));
    }

    #[test]
    fn zero_rate_limit_fails_validation() {
        let config = AppConfig { rate_limit_max_requests: 0, ..AppConfig::development() };
        let err = config.validate().expect_err("zero rate limit should fail");
        assert!(matches!(err, ConfigError::Validation(_)));
    }

    #[test]
    fn zero_body_limit_fails_validation() {
        let config = AppConfig { body_limit: 0, ..AppConfig::development() };
        let err = config.validate().expect_err("zero body limit should fail");
        assert!(matches!(err, ConfigError::Validation(_)));
    }

    #[test]
    fn toml_roundtrip() {
        let config = AppConfig::development();
        let toml_str = toml::to_string(&config).expect("serialize to toml");
        let parsed: AppConfig = toml::from_str(&toml_str).expect("deserialize from toml");
        assert_eq!(parsed.environment, config.environment);
        assert_eq!(parsed.api_addr, config.api_addr);
        assert_eq!(parsed.body_limit, config.body_limit);
        assert_eq!(parsed.log_level, config.log_level);
        assert_eq!(parsed.auth_mode, config.auth_mode);
        assert_eq!(parsed.db_path, config.db_path);
    }
}
