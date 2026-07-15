use std::sync::LazyLock;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub jwt_secret: String,
    pub jwt_expires_in_secs: u64,
    pub server_host: String,
    pub server_port: u16,

    // AI defaults
    pub default_ai_api_key: String,
    pub default_ai_base_url: String,
    pub default_ai_model: String,
    pub default_ai_max_tokens: u32,

    // AI Task limits
    pub ai_task_capacity: usize,
}

impl AppConfig {
    fn from_env() -> Self {
        Self {
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite://dev.db".to_string()),
            jwt_secret: {
                let secret = std::env::var("JWT_SECRET")
                    .unwrap_or_else(|_| "super-secret-key-change-me".to_string());
                if secret == "super-secret-key-change-me" {
                    tracing::warn!("⚠️  JWT_SECRET 未设置，使用默认不安全密钥。生产环境请设置 JWT_SECRET 环境变量！");
                }
                secret
            },
            jwt_expires_in_secs: std::env::var("JWT_EXPIRES_IN_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(3600 * 24),
            server_host: std::env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            server_port: std::env::var("SERVER_PORT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(8080),
            default_ai_api_key: std::env::var("DEEPSEEK_API_KEY")
                .unwrap_or_else(|_| "sk-66".to_string()),
            default_ai_base_url: std::env::var("DEEPSEEK_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:4000/v1".to_string()),
            default_ai_model: std::env::var("DEEPSEEK_MODEL")
                .unwrap_or_else(|_| "deepseek-v4-flash".to_string()),
            default_ai_max_tokens: std::env::var("DEEPSEEK_MAX_TOKENS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(4096),
            ai_task_capacity: 1024,
        }
    }
}

pub static CONFIG: LazyLock<AppConfig> = LazyLock::new(AppConfig::from_env);
