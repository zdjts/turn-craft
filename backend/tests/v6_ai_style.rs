use backend::ai::env::{AiConfig, AiStyle, build_messages};

#[test]
fn test_ai_style_deceptive_prompt_contains_keywords() {
    let style = AiStyle::Deceptive;
    let inst = style.instruction();
    assert!(
        inst.contains("虚张声势") || inst.contains("隐藏真实意图"),
        "Deceptive 风格应包含欺骗相关关键词"
    );
}

#[test]
fn test_ai_style_rational_prompt_contains_keywords() {
    let style = AiStyle::Rational;
    let inst = style.instruction();
    assert!(
        inst.contains("逻辑") || inst.contains("推理"),
        "Rational 风格应包含逻辑/推理相关关键词"
    );
}

#[test]
fn test_ai_style_chaotic_prompt_contains_keywords() {
    let style = AiStyle::Chaotic;
    let inst = style.instruction();
    assert!(
        inst.contains("不可预测") || inst.contains("混乱"),
        "Chaotic 风格应包含不可预测/混乱关键词"
    );
}

#[test]
fn test_ai_style_all_seven_variants_exist() {
    let styles = vec![
        AiStyle::Default,
        AiStyle::Aggressive,
        AiStyle::Conservative,
        AiStyle::Creative,
        AiStyle::Deceptive,
        AiStyle::Rational,
        AiStyle::Chaotic,
    ];
    assert_eq!(styles.len(), 7, "应有 7 种 AI 风格");

    for style in &styles {
        let _ = style.as_str();
        let _ = style.instruction();
    }

    for style in &styles {
        let config = AiConfig {
            style: style.clone(),
            ..AiConfig::default()
        };
        let result = build_messages(&config, "{}".into());
        assert!(!result.is_empty(), "build_messages 不应 panic 或返回空");
    }
}
