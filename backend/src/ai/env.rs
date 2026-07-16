use std::env;

/// AI 行为风格预设
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AiStyle {
    /// 默认 — 无特殊风格要求
    Default,
    /// 激进 — 偏好高风险、高回报的决策
    Aggressive,
    /// 保守 — 偏好安全、低风险的决策
    Conservative,
    /// 创意 — 偏好出人意料、非传统的策略
    Creative,
    /// 狡猾 — 偏好欺骗、虚张声势、隐藏真实意图
    Deceptive,
    /// 理性 — 偏好逻辑严密、证据驱动、逐步推理
    Rational,
    /// 混乱 — 不可预测、打破常规、高颠覆性
    Chaotic,
}

impl AiStyle {
    pub fn as_str(&self) -> &'static str {
        match self {
            AiStyle::Default => "default",
            AiStyle::Aggressive => "aggressive",
            AiStyle::Conservative => "conservative",
            AiStyle::Creative => "creative",
            AiStyle::Deceptive => "deceptive",
            AiStyle::Rational => "rational",
            AiStyle::Chaotic => "chaotic",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "aggressive" => AiStyle::Aggressive,
            "conservative" => AiStyle::Conservative,
            "creative" => AiStyle::Creative,
            "deceptive" => AiStyle::Deceptive,
            "rational" => AiStyle::Rational,
            "chaotic" => AiStyle::Chaotic,
            _ => AiStyle::Default,
        }
    }

    /// 风格描述（附加到 system prompt）
    pub fn instruction(&self) -> &'static str {
        match self {
            AiStyle::Default => "",
            AiStyle::Aggressive => "风格要求：采取激进策略，偏好高风险高回报的行动。不要过于保守。",
            AiStyle::Conservative => "风格要求：采取保守策略，优先保证安全，避免不必要的风险。",
            AiStyle::Creative => "风格要求：发挥创意，使用非传统策略。不要总是按常规出牌，要让对手难以预测。",
            AiStyle::Deceptive => "风格要求：采取狡猾策略。偏好虚张声势、埋设陷阱、隐藏真实意图。在适当时机误导对手，让对手做出错误判断。不要在言行中直接暴露自己的真实想法。",
            AiStyle::Rational => "风格要求：采取理性策略。偏好逻辑链推演、证据驱动决策、逐步推理。行动前先分析局势，给出合理理由。避免情绪化和冲动决策。",
            AiStyle::Chaotic => "风格要求：采取混乱策略。偏好不可预测的行动、打破常规、高颠覆性。不要遵循固定模式，让对手无法预判你的下一步。大胆尝试非常规操作。",
        }
    }
}

impl Default for AiStyle {
    fn default() -> Self { AiStyle::Default }
}

/// AI 配置：存储 API 密钥、端点、模型、风格等信息
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct AiConfig {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub max_tokens: u32,
    pub prompt: String,
    #[serde(default)]
    pub style: AiStyle,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            base_url: String::new(),
            model: String::new(),
            max_tokens: 2048,
            prompt: String::new(),
            style: AiStyle::Default,
        }
    }
}
impl AiConfig {
    /// 创建默认配置（max_tokens=2048）
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            max_tokens: 2048,
            ..Self::default()
        }
    }
    /// 从环境变量加载配置（当前 unused — AI 配置来自 DB）
    #[allow(dead_code)]
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
                    tracing::error!("max_tokens 解析失败，使用默认2048");
                    2048
                }
            },
            Err(_) => 2048 as u32,
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
            prompt: "".to_string(),
            style: AiStyle::Default,
        })
    }
    #[allow(dead_code)]
    pub fn get_env(&self, key: &str) -> Result<String, String> {
        env::var(key).map_err(|e| format!("缺少环境变量: {}", e))
    }
}

/// 构建 AI 请求消息：系统提示 + 风格指令 + 游戏快照
pub fn build_messages(config: &AiConfig, snapshot_json: String) -> String {
    let mut system_parts: Vec<String> = Vec::new();
    if !config.prompt.is_empty() {
        system_parts.push(config.prompt.clone());
    }
    let style_inst = config.style.instruction().to_string();
    if !style_inst.is_empty() {
        system_parts.push(style_inst);
    }
    let system_content = system_parts.join("\n\n");
    let messages = serde_json::json!([
        {
            "role": "system",
            "content": system_content,
        },
        {
            "role": "user",
            "content": snapshot_json,
        }
    ]);
    messages.to_string()
}
