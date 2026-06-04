use std::env;
pub struct AiConfig {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub max_tokens: u32,
}
impl AiConfig {
    pub fn from_env(path: Option<&str>) -> Result<Self, String> {
        match path {
            Some(p) => {
                dotenv::from_path(p).ok();
            }
            None => {
                dotenv::dotenv().ok();
            }
        };
        let max_tokens: u32 = match env::var("DEBAT_MAX_TOKENS") {
            Ok(s) => match s.parse::<u32>() {
                Ok(n) => n,
                Err(_) => {
                    tracing::error!("max_tokens 解析失败，使用默认200");
                    200
                }
            },
            Err(_) => 200 as u32,
        };
        let api_key =
            env::var("DEBAT_API_KEY").map_err(|_| "缺少环境变量 OPENAI_API_KEY".to_string())?;

        let base_url = env::var("DEBAT_API_BASE")
            .unwrap_or_else(|_| "https://api.openai.com/v1".to_string())
            .trim_end_matches('/')
            .to_string();

        let model = env::var("DEBAT_MODEL").unwrap_or_else(|_| "gpt-4o-mini".to_string());
        print!(
            "DEBAT_API_BASE = {} \n 
                DEBAT_MODEL = {} \n
            ",
            base_url, model
        );
        Ok(Self {
            api_key,
            base_url,
            model,
            max_tokens,
        })
    }
    pub fn get_env(&self, key: &str) -> Result<String, String> {
        env::var(key).map_err(|e| format!("缺少环境变量: {}", e))
    }
}
pub fn build_messages(
    config: &AiConfig,
    env_name: &str,
    snapshot_json: String,
) -> Result<String, String> {
    let promat = config.get_env(env_name)?;
    let messages = serde_json::json!([
        {
            "role": "system",
            "content": promat,
        },
        {
            "role": "user",
            "content": snapshot_json,
        }
    ]);
    Ok(messages.to_string())
}
