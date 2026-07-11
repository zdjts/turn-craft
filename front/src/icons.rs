use dioxus::prelude::*;

#[derive(Clone, Copy, PartialEq)]
pub enum IconSize {
    Sm,
    Md,
    Lg,
}

impl IconSize {
    pub fn dims(&self) -> (&str, &str) {
        match self {
            IconSize::Sm => ("16", "16"),
            IconSize::Md => ("20", "20"),
            IconSize::Lg => ("28", "28"),
        }
    }
}

// ═══════════════════════════════════════════════════════
//  Sidebar Navigation Icons
// ═══════════════════════════════════════════════════════

#[component]
pub fn ArenaIcon(size: IconSize) -> Element {
    let (w, h) = size.dims();
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            width: w, height: h,
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "2",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            path { d: "M12 2C8 2 4 5 4 8c0 2 1 3 2 4l-1 7h14l-1-7c1-1 2-2 2-4 0-3-4-6-8-6z" }
            path { d: "M8 15v5M16 15v5M10 8h4" }
        }
    }
}

#[component]
pub fn GlobeIcon(size: IconSize) -> Element {
    let (w, h) = size.dims();
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            width: w, height: h,
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "2",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            circle { cx: "12", cy: "12", r: "10" }
            line { x1: "2", y1: "12", x2: "22", y2: "12" }
            path { d: "M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z" }
        }
    }
}

#[component]
pub fn ScrollIcon(size: IconSize) -> Element {
    let (w, h) = size.dims();
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            width: w, height: h,
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "2",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            path { d: "M4 19.5A2.5 2.5 0 0 1 6.5 17H20" }
            path { d: "M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z" }
            line { x1: "8", y1: "7", x2: "16", y2: "7" }
            line { x1: "8", y1: "11", x2: "14", y2: "11" }
        }
    }
}

#[component]
pub fn UserIcon(size: IconSize) -> Element {
    let (w, h) = size.dims();
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            width: w, height: h,
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "2",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            path { d: "M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2" }
            circle { cx: "12", cy: "7", r: "4" }
        }
    }
}

#[component]
pub fn InfoIcon(size: IconSize) -> Element {
    let (w, h) = size.dims();
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            width: w, height: h,
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "2",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            circle { cx: "12", cy: "12", r: "10" }
            line { x1: "12", y1: "16", x2: "12", y2: "12" }
            line { x1: "12", y1: "8", x2: "12.01", y2: "8" }
        }
    }
}

#[component]
pub fn GameIcon(size: IconSize) -> Element {
    let (w, h) = size.dims();
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            width: w, height: h,
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "2",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            path { d: "M6 11V5l4-3 4 3v6M6 11l-4 3v5l4 3M18 11l4 3v5l-4 3M12 15l-6 4v3l6-3 6 3v-3l-6-4z" }
        }
    }
}

#[component]
pub fn LogoutIcon(size: IconSize) -> Element {
    let (w, h) = size.dims();
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            width: w, height: h,
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "2",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            path { d: "M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4" }
            polyline { points: "16,17 21,12 16,7" }
            line { x1: "21", y1: "12", x2: "9", y2: "12" }
        }
    }
}

#[component]
pub fn SunIcon(size: IconSize) -> Element {
    let (w, h) = size.dims();
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            width: w, height: h,
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "2",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            circle { cx: "12", cy: "12", r: "5" }
            line { x1: "12", y1: "1", x2: "12", y2: "3" }
            line { x1: "12", y1: "21", x2: "12", y2: "23" }
            line { x1: "4.22", y1: "4.22", x2: "5.64", y2: "5.64" }
            line { x1: "18.36", y1: "18.36", x2: "19.78", y2: "19.78" }
            line { x1: "1", y1: "12", x2: "3", y2: "12" }
            line { x1: "21", y1: "12", x2: "23", y2: "12" }
            line { x1: "4.22", y1: "19.78", x2: "5.64", y2: "18.36" }
            line { x1: "18.36", y1: "5.64", x2: "19.78", y2: "4.22" }
        }
    }
}

#[component]
pub fn MoonIcon(size: IconSize) -> Element {
    let (w, h) = size.dims();
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            width: w, height: h,
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "2",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            path { d: "M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" }
        }
    }
}

#[component]
pub fn SparklesIcon(size: IconSize) -> Element {
    let (w, h) = size.dims();
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            width: w, height: h,
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "2",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            path { d: "M9.937 15.5A2 2 0 0 0 8.5 14.063l-6.135-1.582a.5.5 0 0 1 0-.962L8.5 9.936A2 2 0 0 0 9.937 8.5l1.582-6.135a.5.5 0 0 1 .963 0L14.063 8.5A2 2 0 0 0 15.5 9.937l6.135 1.581a.5.5 0 0 1 0 .964L15.5 14.063a2 2 0 0 0-1.437 1.437l-1.582 6.135a.5.5 0 0 1-.963 0z" }
            path { d: "M20 3v4M22 5h-4M4 17v2M5 18H3" }
        }
    }
}

// ═══════════════════════════════════════════════════════
//  AI Provider Icon - model name based detection
// ═══════════════════════════════════════════════════════

#[derive(Clone, Copy, PartialEq)]
pub enum AiProvider {
    OpenAI,
    Anthropic,
    DeepSeek,
    Google,
    Meta,
    Qwen,
    Cohere,
    Generic,
}

impl AiProvider {
    pub fn from_model(model: &str) -> Self {
        let m = model.to_lowercase();
        if m.contains("gpt") || m.contains("o1") || m.contains("o3") || m.contains("davinci") {
            AiProvider::OpenAI
        } else if m.contains("claude") {
            AiProvider::Anthropic
        } else if m.contains("deepseek") {
            AiProvider::DeepSeek
        } else if m.contains("gemini") || m.contains("palm") || m.contains("bard") {
            AiProvider::Google
        } else if m.contains("llama") || m.contains("mistral") {
            AiProvider::Meta
        } else if m.contains("qwen") || m.contains("tongyi") {
            AiProvider::Qwen
        } else if m.contains("command") || m.contains("cohere") {
            AiProvider::Cohere
        } else {
            AiProvider::Generic
        }
    }
}

#[component]
pub fn AiProviderIcon(model: Option<String>) -> Element {
    let provider = model
        .as_deref()
        .map(AiProvider::from_model)
        .unwrap_or(AiProvider::Generic);
    rsx! {
        span { class: "ai-provider-icon",
            match provider {
                AiProvider::OpenAI => rsx! { OpenAiSvg {} },
                AiProvider::Anthropic => rsx! { AnthropicSvg {} },
                AiProvider::DeepSeek => rsx! { DeepSeekSvg {} },
                AiProvider::Google => rsx! { GoogleSvg {} },
                AiProvider::Meta => rsx! { MetaSvg {} },
                AiProvider::Qwen => rsx! { QwenSvg {} },
                AiProvider::Cohere => rsx! { CohereSvg {} },
                AiProvider::Generic => rsx! { GenericAiSvg {} },
            }
        }
    }
}

// ── Provider SVG components ───────────────────────────

#[component]
fn OpenAiSvg() -> Element {
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            width: "16", height: "16",
            view_box: "0 0 24 24",
            fill: "currentColor",
            path { d: "M21.5 10.4c-.4-1.4-1.5-2.5-2.9-2.8-.2-1.8-1.3-3.3-2.9-3.9-1-.3-2.1 0-2.9.8-.9-1.1-2.3-1.5-3.6-1.1-1.7.5-3 1.8-3.3 3.6-.2.9 0 1.8.5 2.5-1.4.5-2.4 1.8-2.5 3.3-.1 1.3.7 2.6 1.9 3.1.9.4 2 .3 2.8-.2.6.1 1.2.2 1.8.2h6.5c1.3 0 2.6-.5 3.5-1.4 1.2-1.2 1.7-2.9 1.3-4.6zm-2.1 3.5c-.5.5-1.2.8-2 .8h-6.5c-.5 0-1-.1-1.4-.2l-.4-.1-.3.3c-.4.3-.9.4-1.4.2-.7-.3-1.1-1-1.1-1.7 0-.8.3-1.6.8-2.1l.5-.5-.5-.5c-.6-.5-1-1.3-1-2.1 0-1.2.7-2.3 1.8-2.8.7-.3 1.5-.1 2 .4l.4.4.1-.6c.2-1.2 1-2.2 2.2-2.5 1.1-.3 2.2.1 2.8 1 .1.1.2.3.3.4l.3.3.4-.2c.5-.3 1.1-.3 1.7-.1 1.1.4 1.9 1.5 2 2.8v.4l.4.1c1.1.4 1.9 1.4 2 2.5.1.9-.4 1.7-1.1 2.3z" }
        }
    }
}

#[component]
fn AnthropicSvg() -> Element {
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            width: "16", height: "16",
            view_box: "0 0 24 24",
            fill: "currentColor",
            path { d: "M17.5 4h-11l-5 8 5 8h11l5-8-5-8zm-1.2 2.5l3.8 6.1-3.8 6.1h-9.6l-3.8-6.1 3.8-6.1h9.6z" }
            path { d: "M14.5 11c-.3 1.5-1.6 2.7-3.2 2.7-1.9 0-3.3-1.5-3.3-3.3 0-.8.3-1.5.8-2h-3l-1 1.6 1 1.6h2.5c.4 2.4 2.5 4.2 5 4.2 2.9 0 5.2-2.4 5.2-5.2 0-1.5-.6-2.8-1.6-3.8l-.8-.8h-3.2l.6.5c.3.2.6.4.9.7.5.5.8 1.1.8 1.8 0 1-.8 1.9-1.9 1.9-.7 0-1.3-.3-1.6-1h-1.3z" }
        }
    }
}

#[component]
fn DeepSeekSvg() -> Element {
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            width: "16", height: "16",
            view_box: "0 0 24 24",
            fill: "currentColor",
            path { d: "M12 2a10 10 0 1 0 10 10A10 10 0 0 0 12 2zm0 18a8 8 0 1 1 8-8 8 8 0 0 1-8 8z" }
            path { d: "M8 9.5a1.5 1.5 0 1 0 0 3 1.5 1.5 0 0 0 0-3zM16 9.5a1.5 1.5 0 1 0 0 3 1.5 1.5 0 0 0 0-3z" }
            path { d: "M12 14.5a2.5 2.5 0 0 0-2.2 1.4h4.4a2.5 2.5 0 0 0-2.2-1.4z" }
        }
    }
}

#[component]
fn GoogleSvg() -> Element {
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            width: "16", height: "16",
            view_box: "0 0 24 24",
            fill: "currentColor",
            path { d: "M12 2a10 10 0 1 0 10 10A10 10 0 0 0 12 2zm0 18a8 8 0 1 1 8-8 8 8 0 0 1-8 8z" }
            path { d: "M16 12h-1.5V8h-1v4H12v1h1.5v4h1v-4H16v-1zM8.5 10.5a1.5 1.5 0 1 0 0 3 1.5 1.5 0 0 0 0-3z" }
        }
    }
}

#[component]
fn MetaSvg() -> Element {
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            width: "16", height: "16",
            view_box: "0 0 24 24",
            fill: "currentColor",
            path { d: "M12 2C6.5 2 2 6.5 2 12s4.5 10 10 10 10-4.5 10-10S17.5 2 12 2zm0 18c-4.4 0-8-3.6-8-8s3.6-8 8-8 8 3.6 8 8-3.6 8-8 8z" }
            path { d: "M9 9.5c.6-.3 1.3-.5 2-.5.3 0 .6 0 .9.1l.8-.8c-.5-.2-1.1-.3-1.7-.3-1 0-1.9.3-2.7.8L9 9.5zM12.2 14.4c-.1.3-.4.6-.7.6-.1 0-.1 0-.2 0l1.2 1.2c.7-.2 1.2-.8 1.4-1.5l-1.7-.3zM16.5 11c-.1-.6-.4-1.1-.9-1.5l-.9.6c.2.2.4.5.5.9h1.3z" }
            path { d: "M12 17c-1.2 0-2.2-.4-3-1l-.8.8c1 .8 2.3 1.2 3.8 1.2 1.2 0 2.3-.3 3.2-.8l-.6-1.1c-.6.4-1.3.7-2.1.8V17h-.5zm-5-7c0 1 .5 1.9 1.3 2.5l.6-.9c-.5-.3-.9-.9-.9-1.6H7zm3.5-2.5c.8 0 1.5.3 2 .8l.8-.8c-.7-.6-1.7-1-2.8-1-1.9 0-3.5 1.3-4 3h1.1c.4-1 1.4-1.7 2.9-1.7z" }
        }
    }
}

#[component]
fn QwenSvg() -> Element {
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            width: "16", height: "16",
            view_box: "0 0 24 24",
            fill: "currentColor",
            path { d: "M12 2a10 10 0 1 0 10 10A10 10 0 0 0 12 2zm0 18a8 8 0 1 1 8-8 8 8 0 0 1-8 8z" }
            path { d: "M8 8h3v5H8V8zM13 8h3v8h-3V8z" }
        }
    }
}

#[component]
fn CohereSvg() -> Element {
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            width: "16", height: "16",
            view_box: "0 0 24 24",
            fill: "currentColor",
            path { d: "M12 2a10 10 0 1 0 10 10A10 10 0 0 0 12 2zm0 18a8 8 0 1 1 8-8 8 8 0 0 1-8 8z" }
            path { d: "M8 11c.6-1.1 1.8-1.9 3.2-1.9.3 0 .7 0 1 .1l.9-.9c-.6-.1-1.2-.2-1.9-.2-1.9 0-3.5 1.1-4.2 2.8V11h1zm2 2c-.6 1.1-1.8 1.9-3.2 1.9-.3 0-.7 0-1-.1l-.9.9c.6.1 1.2.2 1.9.2 1.9 0 3.5-1.1 4.2-2.8L10 13zm4-3h3v3h-3v-3zm0 4h3v3h-3v-3z" }
        }
    }
}

#[component]
fn GenericAiSvg() -> Element {
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            width: "16", height: "16",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "2",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            path { d: "M12 2a4 4 0 0 1 4 4v2a4 4 0 0 1-8 0V6a4 4 0 0 1 4-4z" }
            path { d: "M4 12a8 8 0 0 1 16 0v2a4 4 0 0 1-8 0 4 4 0 0 0-8 0v-2z" }
            circle { cx: "12", cy: "18", r: "2" }
        }
    }
}
