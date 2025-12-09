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

        (Language::Zh, "nav.users") => "用户管理",
        (Language::En, "nav.users") => "Users",

        (Language::Zh, "nav.settings") => "系统设置",
        (Language::En, "nav.settings") => "Settings",

        // Dashboard
        (Language::Zh, "dashboard.title") => "仪表盘",
        (Language::En, "dashboard.title") => "Dashboard",
        (Language::Zh, "dashboard.subtitle") => "BurnCloud 大模型本地部署平台概览",
        (Language::En, "dashboard.subtitle") => "BurnCloud Local LLM Platform Overview",

        (Language::Zh, "dashboard.system_status") => "系统状态",
        (Language::En, "dashboard.system_status") => "System Status",
        (Language::Zh, "dashboard.running_normal") => "运行正常",
        (Language::En, "dashboard.running_normal") => "Normal",

        (Language::Zh, "dashboard.model_status") => "模型状态",
        (Language::En, "dashboard.model_status") => "Model Status",
        (Language::Zh, "dashboard.running_count") => "运行中",
        (Language::En, "dashboard.running_count") => "Running",

        (Language::Zh, "dashboard.token_usage") => "Token 消耗",
        (Language::En, "dashboard.token_usage") => "Token Usage",

        (Language::Zh, "dashboard.storage_usage") => "存储使用",
        (Language::En, "dashboard.storage_usage") => "Storage",

        (Language::Zh, "dashboard.request_id") => "请求 ID",
        (Language::En, "dashboard.request_id") => "Request ID",
        (Language::Zh, "dashboard.status") => "状态",
        (Language::En, "dashboard.status") => "Status",
        (Language::Zh, "dashboard.path") => "路径",
        (Language::En, "dashboard.path") => "Path",
        (Language::Zh, "dashboard.latency") => "耗时",
        (Language::En, "dashboard.latency") => "Latency",
        (Language::Zh, "dashboard.user") => "用户",
        (Language::En, "dashboard.user") => "User",
        (Language::Zh, "dashboard.details") => "详情",
        (Language::En, "dashboard.details") => "Details",
        (Language::Zh, "dashboard.view") => "查看",
        (Language::En, "dashboard.view") => "View",

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
                _ => key,
            }
        }
    }
}
