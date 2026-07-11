use dioxus::prelude::*;

#[component]
pub fn About() -> Element {
    rsx! {
        div { class: "pg-about animate-fade-in",
            div { class: "page-header",
                h1 { "ℹ️ 关于 Turn Craft 项目" }
                p { "Turn Craft 是一个先进的、支持人机协作的回合制博弈竞技与分析平台。" }
            }

            div { class: "pg-about-grid",
                // Introduction Card
                div { class: "pg-about-card g-card",
                    h3 { "💡 项目初衷与理念" }
                    p { "在复杂的博弈对抗中，人类与 AI 往往各有优势。Turn Craft 的设计初衷是建立一个连接人类策略与 AI 智能的桥梁。" }
                    p { "通过多槽位（Actor Slots）机制，任何一局博弈对决都可以由真人和 AI 自由分配，实现 100% 灵活配置的对战或协作实验。" }
                }

                // Architecture Card
                div { class: "pg-about-card g-card",
                    h3 { "🛠️ 技术架构体系" }
                    ul { class: "pg-about-tech",
                        li {
                            strong { "前端驱动: " }
                            "Dioxus 0.7 + WebAssembly + 纯 CSS 变量与动画控制"
                        }
                        li {
                            strong { "后端核心: " }
                            "Rust Axum 高性能 Web 框架 + Sqlite 持久化存储"
                        }
                        li {
                            strong { "通讯保障: " }
                            "WebSocket 实时同步对局状态与动作广播"
                        }
                        li {
                            strong { "AI 模型层: " }
                            "标准大模型接口对接，支持每个 AI 槽位独立自定义 Prompt 和模型参数"
                        }
                    }
                }

                // Game Features Card
                div { class: "pg-about-card g-card span-two",
                    h3 { "🎲 现已支持游戏" }
                    div { class: "pg-about-games",
                        div { class: "pg-about-feature",
                            span { class: "pg-about-feature-icon", "🏛️" }
                            h4 { "林肯辩论 (Lincoln-Douglas)" }
                            p { "经典的双人对抗辩论形式。通过裁判引导，双方围绕特定议题进行有理有据的交锋与申辩。" }
                        }
                        div { class: "pg-about-feature",
                            span { class: "pg-about-feature-icon", "🃏" }
                            h4 { "德州扑克 (Texas Hold'em)" }
                            p { "支持 2-6 人的经典德扑。集数学期望、博弈论心理对抗于一体，展现策略规划与风险控制。" }
                        }
                        div { class: "pg-about-feature",
                            span { class: "pg-about-feature-icon", "🐺" }
                            h4 { "狼人杀 (Werewolf)" }
                            p { "基于自然语言交流的社交博弈（Beta 测试中）。充分体现谎言识别、团队协作与逻辑分析能力。" }
                        }
                    }
                }
            }
        }
    }
}
