use dioxus::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Language {
    En,
    Zh,
}

impl Default for Language {
    fn default() -> Self {
        Language::Zh
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct I18nContext {
    pub language: Signal<Language>,
}

pub fn use_init_i18n() -> I18nContext {
    let language = use_signal(|| Language::default());
    let ctx = I18nContext { language };
    use_context_provider(|| ctx);
    ctx
}

pub fn use_i18n() -> I18nContext {
    use_context::<I18nContext>()
}

// Simple translation maps
// In a larger app, this would be loaded from JSON or separate files.
pub fn t(lang: Language, key: &'static str) -> &'static str {
    match (lang, key) {
        // Navigation
        (Language::Zh, "nav.dashboard") => "仪表盘",
        (Language::En, "nav.dashboard") => "Dashboard",
        
        (Language::Zh, "nav.models") => "模型管理",
        (Language::En, "nav.models") => "Models",
        
        (Language::Zh, "nav.deploy") => "部署配置",
        (Language::En, "nav.deploy") => "Deployment",
        
        (Language::Zh, "nav.monitor") => "监控日志",
        (Language::En, "nav.monitor") => "Monitor",
        
        (Language::Zh, "nav.api") => "API管理",
        (Language::En, "nav.api") => "API Keys",
        
        (Language::Zh, "nav.channels") => "渠道管理",
        (Language::En, "nav.channels") => "Channels",
        
        (Language::Zh, "nav.settings") => "系统设置",
        (Language::En, "nav.settings") => "Settings",

        // Status
        (Language::Zh, "status.models_running") => "模型运行中",
        (Language::En, "status.models_running") => "models running",

        // Default to key if not found
        (_, _) => {
            // Fallback to English or return key itself
            match key {
                "nav.dashboard" => "Dashboard",
                "nav.models" => "Models",
                "nav.deploy" => "Deployment",
                "nav.monitor" => "Monitor",
                "nav.api" => "API Keys",
                "nav.channels" => "Channels",
                "nav.settings" => "Settings",
                _ => key
            }
        }
    }
}
