use dioxus::prelude::*;

/// Lucide icon paths used by the console. Keeping the path data here avoids
/// introducing a second icon dependency while retaining recognizable icons.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IconName {
    AlertCircle,
    AlertTriangle,
    ArrowRight,
    Bell,
    Building,
    Chart,
    Check,
    CheckCircle,
    ChevronDown,
    Code2,
    Copy,
    Coins,
    Cpu,
    CreditCard,
    Dollar,
    DollarSign,
    Gauge,
    Globe,
    Key,
    Layers,
    Layout,
    Menu,
    Plus,
    X,
    Search,
    Receipt,
    RotateCcw,
    Server,
    Settings,
    Shield,
    Terminal,
    Store,
    Trending,
    Users,
    Play,
    Workflow,
    Zap,
}

impl IconName {
    const fn path(self) -> &'static str {
        match self {
            Self::AlertCircle => "M22 12a10 10 0 1 1-20 0 10 10 0 0 1 20 0ZM12 8v4M12 16h.01",
            Self::AlertTriangle => "m21.73 18-8-14a2 2 0 0 0-3.46 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3ZM12 9v4M12 17h.01",
            Self::ArrowRight => "M5 12h14m-7-7 7 7-7 7",
            Self::Bell => "M10.268 21a2 2 0 0 0 3.464 0M3.262 15.326A1 1 0 0 0 4 17h16a1 1 0 0 0 .74-1.673C19.41 13.956 18 12.499 18 8A6 6 0 0 0 6 8c0 4.499-1.411 5.956-2.738 7.326",
            Self::Building => "M3 21h18M6 21V3h12v18M9 7h1M9 11h1M9 15h1M14 7h1M14 11h1M14 15h1",
            Self::Chart => "M3 3v16a2 2 0 0 0 2 2h16m-2-12-5 5-4-4-3 3",
            Self::Check => "M20 6 9 17l-5-5",
            Self::CheckCircle => "M22 11.08V12a10 10 0 1 1-5.93-9.14M9 11l3 3L22 4",
            Self::ChevronDown => "m6 9 6 6 6-6",
            Self::Code2 => "m18 16 4-4-4-4M6 8l-4 4 4 4m8.5-12-5 16",
            Self::Copy => "M8 8h12v12H8zM4 16H3a1 1 0 0 1-1-1V3a1 1 0 0 1 1-1h12a1 1 0 0 1 1 1v1",
            Self::Coins => "M8 14a6 6 0 1 0 0-12 6 6 0 0 0 0 12Zm10.09-3.63A6 6 0 1 1 10.34 18M7 6h1v4m8.71 3.88.7.71-2.82 2.82",
            Self::Cpu => "M4 4h16v16H4zM9 1v3m6-3v3m-6 16v3m6-3v3m5-14h3m-3 5h3M1 9h3m-3 5h3M9 9h6v6H9z",
            Self::CreditCard => "M4 5h16a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V7a2 2 0 0 1 2-2Zm-2 5h20",
            Self::Dollar => "M12 2v20m5-16H9.5a3.5 3.5 0 0 0 0 7h5a3.5 3.5 0 0 1 0 7H6",
            Self::DollarSign => "M12 2v20m5-16.5A5 5 0 0 0 12 4a5 5 0 0 0 0 10 5 5 0 0 1 0 10 5 5 0 0 1-5-1.5",
            Self::Gauge => "m12 14 4-4M3.34 19a10 10 0 1 1 17.32 0",
            Self::Globe => "M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20ZM2 12h20M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10",
            Self::Key => "m15.5 7.5 2.3 2.3a1 1 0 0 0 1.4 0l2.1-2.1a1 1 0 0 0 0-1.4L19 4",
            Self::Layers => "m12.83 2.18 8 4a2 2 0 0 1 0 3.58l-8 4a2 2 0 0 1-1.79 0l-8-4a2 2 0 0 1 0-3.58l8-4a2 2 0 0 1 1.79 0Zm9.17 10.32-9.17 4.59a2 2 0 0 1-1.79 0L2 12.5m20 5-9.17 4.59a2 2 0 0 1-1.79 0L2 17.5",
            Self::Layout => "M4 3h5a1 1 0 0 1 1 1v7a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1ZM15 3h5a1 1 0 0 1 1 1v3a1 1 0 0 1-1 1h-5a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1ZM15 12h5a1 1 0 0 1 1 1v7a1 1 0 0 1-1 1h-5a1 1 0 0 1-1-1v-7a1 1 0 0 1 1-1ZM4 16h5a1 1 0 0 1 1 1v3a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1v-3a1 1 0 0 1 1-1Z",
            Self::Menu => "M4 6h16M4 12h16M4 18h16",
            Self::Plus => "M12 5v14M5 12h14",
            Self::X => "M18 6 6 18M6 6l12 12",
            Self::Search => "m21 21-4.3-4.3M11 19a8 8 0 1 1 0-16 8 8 0 0 1 0 16",
            Self::Receipt => "M15 12h-5m5-4h-5m9 9V5a2 2 0 0 0-2-2H4M8 21h12a2 2 0 0 0 2-2v-1a1 1 0 0 0-1-1H11a1 1 0 0 0-1 1v1a2 2 0 1 1-4 0V5a2 2 0 1 0-4 0v2a1 1 0 0 0 1 1h3",
            Self::RotateCcw => "M3 12a9 9 0 1 0 3-6.7M3 4v5h5",
            Self::Server => "M4 2h16a2 2 0 0 1 2 2v4a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2Zm0 12h16a2 2 0 0 1 2 2v4a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2v-4a2 2 0 0 1 2-2ZM6 6h.01m0 12h.01",
            Self::Settings => "M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.38a2 2 0 0 0-.73-2.73l-.15-.09a2 2 0 0 1-1-1.74v-.51a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2zM12 9a3 3 0 1 0 0 6 3 3 0 0 0 0-6Z",
            Self::Shield => "M20 13c0 5-3.5 7.5-8 9-4.5-1.5-8-4-8-9V5l8-3 8 3zM9 12l2 2 4-4",
            Self::Terminal => "M12 19h8m-16-2 6-6-6-6",
            Self::Store => "M15 21v-5a1 1 0 0 0-1-1h-4a1 1 0 0 0-1 1v5M17.774 10.31a1.12 1.12 0 0 0-1.549 0 2.5 2.5 0 0 1-3.451 0 1.12 1.12 0 0 0-1.548 0 2.5 2.5 0 0 1-3.452 0 1.12 1.12 0 0 0-1.549 0 2.5 2.5 0 0 1-3.77-3.248l2.889-4.184A2 2 0 0 1 7 2h10a2 2 0 0 1 1.653.873l2.895 4.192a2.5 2.5 0 0 1-3.774 3.244M4 10.95V19a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-8.05",
            Self::Trending => "m22 7-8.5 8.5-5-5L2 17m14-10h6v6",
            Self::Users => "M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2m7-10a4 4 0 1 0 0-8 4 4 0 0 0 0 8Zm6 10v-2a4 4 0 0 0-3-3.87m3-11.13a4 4 0 0 1 0 7.75",
            Self::Play => "M5 5a2 2 0 0 1 3.008-1.728l11.997 6.998a2 2 0 0 1 .003 3.458l-12 7A2 2 0 0 1 5 19z",
            Self::Workflow => "M3 3h8v8H3zM13 13h8v8h-8zM7 11v3a2 2 0 0 0 2 2h4",
            Self::Zap => "M4 14a1 1 0 0 1-.78-1.63l9-11a.5.5 0 0 1 .87.45l-1.7 6.8A1 1 0 0 0 11.36 9H20a1 1 0 0 1 .78 1.63l-9 11a.5.5 0 0 1-.87-.45l1.7-6.8A1 1 0 0 0 9.94 14Z",
        }
    }
}

#[component]
pub fn Icon(name: IconName, #[props(default = 16)] size: u8) -> Element {
    rsx! {
        svg {
            width: size,
            height: size,
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "2",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            role: "presentation",
            if name == IconName::CheckCircle {
                circle { cx: "12", cy: "12", r: "10" }
                path { d: "m9 12 2 2 4-4" }
            } else if name == IconName::Cpu {
                path { d: "M12 20v2" }
                path { d: "M12 2v2" }
                path { d: "M17 20v2" }
                path { d: "M17 2v2" }
                path { d: "M2 12h2" }
                path { d: "M2 17h2" }
                path { d: "M2 7h2" }
                path { d: "M20 12h2" }
                path { d: "M20 17h2" }
                path { d: "M20 7h2" }
                path { d: "M7 20v2" }
                path { d: "M7 2v2" }
                rect { x: "4", y: "4", width: "16", height: "16", rx: "2" }
                rect { x: "8", y: "8", width: "8", height: "8", rx: "1" }
            } else if name == IconName::Key {
                path { d: name.path() }
                path { d: "m21 2-9.6 9.6" }
                circle { cx: "7.5", cy: "15.5", r: "5.5" }
            } else if name == IconName::Globe {
                circle { cx: "12", cy: "12", r: "10" }
                path { d: "M12 2a14.5 14.5 0 0 0 0 20 14.5 14.5 0 0 0 0-20" }
                path { d: "M2 12h20" }
            } else {
                path { d: name.path() }
            }
        }
    }
}

#[component]
pub fn Logo(#[props(default = 28)] size: u8) -> Element {
    rsx! {
        svg { width: size, height: size, view_box: "0 0 24 24", fill: "none",
            defs { linearGradient { id: "burnCloudGrad", x1: "0", y1: "0", x2: "0", y2: "1", stop { offset: "0%", stop_color: "#f7b52c" } stop { offset: "100%", stop_color: "#e95513" } } }
            path { d: "M17.8 10.1q-.6-.9-1.4-1.9S14.6 6.1 14.9 3c0 0-6.9 2.7-7 8.2 0 0-1-1.6-.8-4.6 0 0-2.2 2.1-2.5 5.5-2.1.7-3.8 2.5-3.8 4.3 0 2.5 2.7 4.6 5.9 4.6-2.4-.4-4.2-2-4.2-4 0-1.4.8-2.5 2-3.3q.1 1.1.5 2.4s1.2 3.8 5.4 4.8c1.2.3 2.5.2 3.7-.3 1.3-.6 2.8-1.8 2.8-4.5 0 0 .1-2.7-1.5-4.1 0 0 2.1 5-1.8 6.5-1.3.5-2.6.5-3.9 0-1.7-.7-3.8-2.5-3.5-7.2 0 0 1 3.4 3.2 4.7 0 0-2-5.8 3.9-9.8 0 0 .5 2.1 1.9 3.3.4.4 4 3.2 3.3 8 .7-.9 1.3-3.1.7-4.8 0 0-.1-.4-.4-.9 1.5.3 2.7 1.5 2.8 4.2.1 2.3-1.6 4.2-3.8 5 3-.4 5.4-2.7 5.4-5.6 0-2.8-2.2-5.1-5.4-5.3z", fill: "url(#burnCloudGrad)" }
        }
    }
}

#[component]
pub fn Button(
    label: String,
    href: Option<String>,
    #[props(default)] secondary: bool,
    #[props(default)] warning: bool,
    #[props(default)] icon: Option<IconName>,
) -> Element {
    let class = if warning {
        "button button-warning"
    } else if secondary {
        "button button-secondary"
    } else {
        "button button-primary"
    };
    rsx! { Link { role: "button", class: class, to: href.unwrap_or_else(|| "#".to_string()), if let Some(name) = icon { Icon { name, size: 14 } } span { {label} } } }
}

#[component]
pub fn Badge(label: String, #[props(default)] tone: String) -> Element {
    let class = match tone.as_str() {
        "healthy" | "success" => "badge-success",
        "warning" => "badge-warning",
        "error" | "critical" => "badge-error",
        "brand" => "badge-brand",
        "marketplace-accent" => "badge-accent",
        "accent" => "badge-neutral",
        _ => "tier",
    };
    rsx! { span { class: class, {label} } }
}

#[component]
pub fn MetricCard(
    label: String,
    value: String,
    detail: String,
    #[props(default)] unit: Option<String>,
    #[props(default)] trend: Option<String>,
    #[props(default)] trend_positive: bool,
    #[props(default)] status: Option<String>,
    #[props(default)] status_tone: String,
    #[props(default)] badge: Option<String>,
) -> Element {
    let trend_class = if trend_positive {
        "trend trend-positive"
    } else {
        "trend"
    };
    let badge_class = match status_tone.as_str() {
        "warning" => "badge-warning",
        "critical" | "error" => "badge-error",
        "neutral" => "badge-neutral",
        _ => "badge-success",
    };
    rsx! {
        article { class: "metric-card",
            div { class: "metric-label-row",
                span { class: "metric-label", {label} }
                if let Some(badge) = badge { span { class: badge_class, {badge} } }
                if let Some(status) = status {
                    if status_tone == "neutral" {
                        span { class: "status status-neutral", span { class: "status-label", {status} } }
                    } else {
                        span { class: "status", span { class: "status-dot" } span { class: "status-label", {status} } }
                    }
                }
            }
            div { class: "metric-value-row", strong { {value} } if let Some(unit) = unit { span { class: "metric-unit", {unit} } } }
            div { class: "metric-meta",
                if let Some(trend) = trend { span { class: trend_class, {trend} } }
                span { class: "metric-subtitle", {detail} }
            }
        }
    }
}

#[component]
pub fn Card(title: String, children: Element) -> Element {
    rsx! {
        section { class: "panel",
            div { class: "section-header", h2 { {title} } }
            {children}
        }
    }
}

#[component]
pub fn GlobalStyle() -> Element {
    rsx! { style { {STYLE} } }
}

const STYLE: &str = r#"
@import url('https://fonts.googleapis.com/css2?family=Plus+Jakarta+Sans:wght@400;500;600;700;800&family=JetBrains+Mono:wght@400;500;600;700&display=swap');
:root { --sans: "Plus Jakarta Sans", -apple-system, BlinkMacSystemFont, "SF Pro Text", "Segoe UI", Roboto, sans-serif; --mono: "JetBrains Mono", "SF Mono", Menlo, Monaco, Consolas, monospace; font-family: var(--sans); color: #111827; background: #f8f9fa; }
* { box-sizing: border-box; border-color: rgba(229,231,235,.8); -webkit-font-smoothing: antialiased; -moz-osx-font-smoothing: grayscale; }
html, body, #main { min-height: 100%; }
html { line-height: 1.5; }
body { margin: 0; overflow-x: hidden; background: #f8f9fa; color: #111827; font-family: var(--sans); line-height: inherit; font-feature-settings: "cv02", "cv03", "cv04", "cv11"; }
a { color: inherit; text-decoration: none; }
button, input, select, textarea { font: inherit; }
button:focus-visible, a:focus-visible, input:focus-visible { outline: 2px solid #111827; outline-offset: 2px; }
::selection { color: #fff; background: #111827; }
::-webkit-scrollbar { width: 6px; height: 6px; }
::-webkit-scrollbar-track { background: transparent; }
::-webkit-scrollbar-thumb { border-radius: 9999px; background: rgba(156,163,175,.35); }
::-webkit-scrollbar-thumb:hover { background: rgba(107,114,128,.6); }
.app-shell { display: flex; width: 100%; height: 100vh; overflow: hidden; background: #f9fafb; color: #111827; font-family: var(--sans); user-select: none; }
.sidebar { position: relative; z-index: 30; display: flex; width: 240px; flex: 0 0 240px; flex-direction: column; border-right: 1px solid #e5e7eb; background: rgba(249,250,251,.95); backdrop-filter: blur(8px); }
.sidebar-brand-area { position: relative; padding: 14px; border-bottom: 1px solid rgba(229,231,235,.8); }
.role-switcher { position: relative; }
.brand-button { display: flex; width: 100%; align-items: center; justify-content: space-between; padding: 10px; border: 1px solid rgba(229,231,235,.9); border-radius: 16px; background: #fff; box-shadow: 0 1px 2px rgba(0,0,0,.02); color: #111827; cursor: pointer; text-align: left; }
.brand-identity { display: flex; min-width: 0; align-items: center; gap: 10px; }
.brand-copy { display: flex; min-width: 0; flex-direction: column; }
.brand-name-row { display: flex; align-items: center; gap: 6px; }
.brand-copy strong { display: block; font-size: 13px; font-weight: 800; line-height: 1.5; letter-spacing: -.025em; }
.brand-role-row { display: flex; max-width: 150px; min-width: 0; align-items: center; gap: 4px; overflow: hidden; color: #6b7280; font: 600 11px/1.5 var(--mono); white-space: nowrap; }
.brand-role-row > span { min-width: 0; overflow: hidden; text-overflow: ellipsis; }
.brand-role-row small { flex: 0 0 auto; color: #9ca3af; font: 600 9px/1.5 var(--mono); }
.role-color-dot { display: inline-block; width: 6px; height: 6px; border-radius: 50%; background: #10b981; box-shadow: 0 0 0 2px #fff; }
.role-color-dot.supplier { background: #6366f1; }
.role-color-dot.admin { background: #f59e0b; }
.workflow-strip { margin: 0; padding: 8px 16px; border-bottom: 1px solid rgba(229,231,235,.6); background: rgba(243,244,246,.5); color: #6b7280; font: 600 10px/15px var(--mono); letter-spacing: .05em; text-transform: uppercase; }
.side-nav { flex: 1; overflow-y: auto; padding: 12px; }
.nav-list { display: block; }
.nav-link { display: flex; min-width: 0; align-items: center; justify-content: space-between; padding: 8px 12px; margin: 0 0 4px; border: 0 solid rgba(229,231,235,.8); border-radius: 12px; color: #4b5563; font-size: 12px; font-weight: 600; line-height: 16px; letter-spacing: -.025em; }
.nav-link:hover { color: #030712; background: rgba(229,231,235,.5); }
.nav-link.active { border: 1px solid rgba(229,231,235,.9); background: #fff; box-shadow: 0 1px 3px rgba(0,0,0,.06), 0 1px 1px rgba(0,0,0,.04); color: #030712; font-weight: 700; }
.nav-label { display: flex; min-width: 0; align-items: center; gap: 10px; overflow: hidden; }
.nav-label > svg { flex: 0 0 auto; color: #9ca3af; }
.nav-link.active .nav-label > svg { color: #030712; }
.nav-badge { flex: 0 0 auto; padding: 2px 6px; border-radius: 99px; color: #6b7280; background: #f3f4f6; font: 700 9px/1 var(--mono); letter-spacing: .05em; }
.nav-link.active .nav-badge { color: #fff; background: #18181b; }
.sidebar-footer { display: flex; flex-direction: column; gap: 10px; padding: 14px; border-top: 1px solid rgba(229,231,235,.8); background: rgba(255,255,255,.7); }
.role-metric { display: flex; flex-direction: column; padding: 12px; border: 1px solid rgba(229,231,235,.8); border-radius: 16px; background: rgba(249,250,251,.9); box-shadow: 0 1px 2px rgba(0,0,0,.01); }
.role-metric > span { margin-bottom: 6px; color: #9ca3af; font: 700 10px/15px var(--mono); letter-spacing: .05em; text-transform: uppercase; }
.role-metric > div { display: flex; align-items: baseline; justify-content: space-between; }
.sidebar-meta { display: flex; align-items: center; justify-content: space-between; }
.role-metric strong { color: #09090b; font: 800 16px/24px var(--mono); }
.top-up-link { color: #18181b; font: 700 11px/1.5 var(--mono); }
.top-up-link:hover, .table-link:hover { text-decoration: underline; }
.sidebar-meta { padding: 0 4px; color: #6b7280; font-size: 12px; font-weight: 400; line-height: 16px; }
.sidebar-meta a { display: flex; align-items: center; gap: 4px; color: #6b7280; font-size: 11px; font-weight: 500; line-height: 1.3333; }
.sla { display: flex; align-items: center; gap: 6px; color: #059669; font: 600 10px/1.3333 var(--mono); }
.sla i { width: 6px; height: 6px; border-radius: 50%; background: #10b981; }
.app-main { display: flex; min-width: 0; flex: 1; flex-direction: column; background: #f8f9fa; }
.topbar { position: relative; z-index: 20; display: flex; height: 56px; flex: 0 0 56px; align-items: center; gap: 0; padding: 0 32px; border-bottom: 1px solid rgba(229,231,235,.8); background: rgba(255,255,255,.92); backdrop-filter: blur(12px); }
.global-search { position: relative; display: flex; width: 448px; flex: 0 0 448px; align-items: center; }
.global-search > svg { position: absolute; left: 14px; color: #9ca3af; pointer-events: none; }
.global-search input { width: 100%; height: 34px; padding: 0 14px 0 36px; border: 1px solid rgba(229,231,235,.8); border-radius: 12px; outline: 0; background: rgba(249,250,251,.9); color: #111827; font-size: 12px; font-weight: 500; line-height: 16px; box-shadow: 0 1px 2px rgba(0,0,0,.01); }
.global-search input::placeholder { color: #9ca3af; }
.global-search input:focus { border-color: #111827; background: #fff; box-shadow: 0 0 0 2px rgba(17,24,39,.08); }
.topbar-actions { display: flex; flex: 0 0 auto; align-items: center; gap: 12px; margin-left: auto; }
.autopilot { display: flex; align-items: center; gap: 8px; padding: 4px 12px; border: 1px solid rgba(229,231,235,.8); border-radius: 99px; background: rgba(249,250,251,.9); color: #4b5563; font: 12px/16px var(--mono); letter-spacing: normal; white-space: nowrap; }
.language-button { display: flex; height: 32px; align-items: center; gap: 8px; padding: 0 10px; border: 1px solid rgba(229,231,235,.8); border-radius: 12px; background: rgba(249,250,251,.9); color: #4b5563; font: 500 12px/16px var(--sans); letter-spacing: normal; white-space: nowrap; }
.autopilot > span:first-child { width: 8px; height: 8px; flex: 0 0 8px; border-radius: 50%; background: #10b981; box-shadow: 0 0 0 2px rgba(16,185,129,.2); }
.autopilot-label { color: #374151; font: 700 11px/14.6667px var(--mono); letter-spacing: normal; }
.language-switcher { position: relative; }
.language-copy { display: flex; align-items: center; gap: 6px; }
.language-button { padding-right: 10px; padding-left: 10px; cursor: pointer; border-radius: 12px; font-family: inherit; font-weight: 500; }
.language-button:hover { background: #f3f4f6; }
.language-flag { font-size: 14px; }
.icon-button { display: inline-flex; width: 32px; height: 32px; align-items: center; justify-content: center; padding: 0; border: 0; border-radius: 11px; background: transparent; color: #6b7280; cursor: pointer; }
.icon-button:hover { color: #030712; background: #f3f4f6; }
.notification { position: relative; padding: 8px; border-radius: 12px; }
.notification-dot { position: absolute; top: 6px; right: 6px; width: 8px; height: 8px; border: 0; border-radius: 50%; background: #f59e0b; box-shadow: 0 0 0 2px #fff; }
.topbar-divider { width: 1px; height: 16px; margin: 0 2px; background: #e5e7eb; }
.profile { display: flex; align-items: center; gap: 10px; padding-left: 4px; }
.avatar { display: inline-flex; width: 30px; height: 30px; align-items: center; justify-content: center; border-radius: 50%; background: linear-gradient(45deg,#111827,#374151); color: #fff; font: 700 11px/1 var(--mono); }
.profile-copy { display: flex; flex-direction: column; }
.profile-copy strong { color: #030712; font-size: 12px; line-height: 1; }
.profile-copy small { margin-top: 2px; color: #9ca3af; font: 600 10px/1.25 var(--mono); }
.page-viewport { flex: 1; min-width: 0; padding: 40px; overflow-y: auto; }
.content-width { width: 100%; max-width: 1280px; margin: 0 auto; }
.overview-stack { display: flex; flex-direction: column; gap: 28px; padding-bottom: 4px; animation: page-in 300ms ease-out both; }
@keyframes page-in { from { opacity: 0; transform: translateY(8px); } to { opacity: 1; transform: translateY(0); } }
.page-header { display: flex; align-items: center; justify-content: space-between; gap: 24px; margin-bottom: -12px; }
.page-heading-copy { min-width: 0; }
.page-heading-copy h1 { margin: 0; color: #030712; font-size: 28px; font-weight: 800; line-height: 1.333333; letter-spacing: -.025em; }
.page-heading-copy p { margin: 4px 0 0; color: #6b7280; font-size: 14px; font-weight: 500; line-height: 20px; }
.section-header p { margin: 4px 0 0; color: #6b7280; font-size: 12px; font-weight: 500; line-height: 16px; }
.page-actions { display: flex; flex: 0 0 auto; align-items: center; gap: 10px; }
.button { display: inline-flex; height: 34px; min-height: 34px; align-items: center; justify-content: center; gap: 6px; padding: 0 12px; border: 1px solid rgba(229,231,235,.9); border-radius: 12px; font-size: 12px; font-weight: 500; line-height: 16px; letter-spacing: -.025em; white-space: nowrap; transition: background 150ms,border-color 150ms,transform 100ms; }
.button:active { transform: scale(.98); }
.button-secondary { color: #111827; background: #fff; box-shadow: 0 1px 2px rgba(0,0,0,.03); }
.button-secondary:hover { border-color: #d1d5db; background: #f9fafb; }
.button-primary { border-color: #27272a; color: #fff; background: #09090b; box-shadow: 0 1px 2px rgba(0,0,0,.12),inset 0 1px rgba(255,255,255,.12); }
.button-primary:hover { background: #27272a; }
.button-warning { border-color: #451a03; color: #fff; background: #451a03; box-shadow: 0 1px 2px rgba(69,26,3,.18),inset 0 1px rgba(255,255,255,.1); }
.button-warning:hover { border-color: #78350f; background: #78350f; }
.conclusion { display: flex; align-items: center; gap: 12px; padding: 10px 16px; border: 1px solid; border-radius: 12px; font-size: 12px; font-weight: 500; line-height: 16px; box-shadow: 0 1px 2px rgba(0,0,0,.01); }
.conclusion-text { line-height: 1.375; }
.conclusion svg { flex: 0 0 auto; }
.conclusion-healthy svg { color: oklch(59.6% .145 163.225); }
.conclusion-healthy { border-color: oklch(90.5% .093 164.15 / .8); color: oklch(37.8% .077 168.94); background: oklch(97.9% .021 166.113 / .7); }
.conclusion-warning { border-color: #fde68a; color: #78350f; background: rgba(255,251,235,.9); }
.metric-grid { display: grid; grid-template-columns: repeat(4,minmax(0,1fr)); gap: 20px; }
.metric-card, .panel { border: 1px solid rgba(229,231,235,.8); border-radius: 16px; background: #fff; box-shadow: 0 1px 3px 0 rgba(0,0,0,.02), 0 1px 2px -1px rgba(0,0,0,.02); }
.metric-card { min-width: 0; padding: 24px; transition: border-color 150ms; }
.metric-card > :not(:last-child) { margin-bottom: 10px; }
.metric-card:hover { border-color: #d1d5db; }
.metric-label-row { display: flex; min-width: 0; align-items: center; justify-content: space-between; gap: 8px; }
.metric-label { color: #9ca3af; font: 700 11px/1.5 var(--mono); letter-spacing: .05em; text-transform: uppercase; }
.metric-value-row { display: flex; min-width: 0; align-items: baseline; gap: 8px; padding-top: 2px; }
.metric-value-row strong { overflow-wrap: anywhere; color: #030712; font: 800 30px/1.2 var(--mono); letter-spacing: -.025em; }
.metric-unit { color: #9ca3af; font-size: 12px; font-weight: 600; letter-spacing: normal; }
.metric-meta { display: flex; min-width: 0; align-items: center; gap: 8px; padding-top: 2px; line-height: 16px; }
.trend { flex: 0 1 auto; padding: 2px 8px; border: 1px solid rgba(229,231,235,.7); border-radius: 6px; color: #4b5563; background: #f3f4f6; font: 600 11px/1.3333 var(--mono); }
.trend-positive { border-color: rgba(167,243,208,.7); color: #047857; background: #ecfdf5; }
.metric-subtitle { min-width: 0; overflow: hidden; color: #6b7280; font-size: 11px; font-weight: 500; line-height: 1.3333; text-overflow: ellipsis; white-space: nowrap; }
.badge-success, .badge-warning, .badge-error, .badge-neutral, .badge-brand, .badge-accent, .tier { display: inline-flex; flex: 0 0 auto; padding: 2px 8px; border: 1px solid rgba(167,243,208,.8); border-radius: 99px; color: #065f46; background: #ecfdf5; font: 600 10px/1 var(--mono); letter-spacing: .05em; text-transform: uppercase; }
.badge-warning { border-color: #fde68a; color: #92400e; background: #fffbeb; }
.badge-error { border-color: #fecaca; color: #9f1239; background: #fff1f2; }
.badge-neutral, .tier { border-color: rgba(229,231,235,.8); color: #374151; background: rgba(243,244,246,.8); }
.badge-brand { border-color: #27272a; color: #f4f4f5; background: #18181b; }
.badge-accent { border-color: rgba(228,228,231,.9); color: #27272a; background: #f4f4f5; }
.status { display: inline-flex; flex: 0 0 auto; align-items: center; gap: 6px; color: #111827; font-size: 12px; font-weight: 500; line-height: 16px; letter-spacing: -.025em; white-space: nowrap; }
.status-label { color: #111827; font-weight: 600; }
.status-dot { width: 8px; height: 8px; border-radius: 50%; background: #10b981; box-shadow: 0 0 0 4px rgba(16,185,129,.1); }
.status-warning { color: #92400e; }
.status-warning .status-dot { background: #f59e0b; box-shadow: 0 0 0 4px rgba(245,158,11,.15); }
.status-neutral { color: #6b7280; }
.attention { display: flex; align-items: center; justify-content: space-between; gap: 20px; padding: 20px; border: 1px solid #fde68a; border-radius: 16px; color: #78350f; background: #fffbeb; box-shadow: 0 1px 2px rgba(0,0,0,.02); }
.attention-copy { display: flex; align-items: flex-start; gap: 14px; }
.attention-copy svg { flex: 0 0 auto; margin-top: 2px; color: #d97706; }
.attention h4, .attention p { margin: 0; }
.attention h4 { font-size: 14px; }
.attention p { margin-top: 4px; font-size: 12px; line-height: 1.5; }
.panel { padding: 28px; overflow: hidden; }
.section-header { display: flex; align-items: center; justify-content: space-between; gap: 20px; margin-bottom: 20px; }
.section-header h2, .section-header h3 { margin: 0; color: #030712; font-size: 16px; font-weight: 700; }
.section-header p { margin-top: 2px; }
.ghost-link { display: inline-flex; flex: 0 0 auto; height: 34px; min-height: 34px; align-items: center; gap: 6px; padding: 0 12px; border-radius: 12px; color: #4b5563; font-size: 12px; font-weight: 600; line-height: 16px; letter-spacing: -.025em; }
.ghost-link:hover { color: #030712; background: #f3f4f6; }
.table-scroll { width: 100%; overflow-x: auto; overscroll-behavior-x: contain; }
table { width: 100%; border-collapse: collapse; text-align: left; font-size: 12px; line-height: 16px; }
thead tr { border-bottom: 1px solid #f3f4f6; }
th { padding: 0 0 14px; color: #9ca3af; font: 600 10px/13.3333px var(--mono); letter-spacing: .05em; text-transform: uppercase; }
td { padding: 16px 0; border-bottom: 0; color: #374151; white-space: normal; }
tbody tr { border-bottom: 1px solid #f3f4f6; }
tbody tr:last-child { border-bottom: 0; }
tbody tr:hover { background: rgba(249,250,251,.82); }
.align-right { text-align: right; }
.model-cell { display: flex; align-items: center; gap: 10px; }
.model-mark, .activity-icon { display: inline-flex; flex: 0 0 auto; align-items: center; justify-content: center; border: 1px solid rgba(229,231,235,.9); background: #f9fafb; }
.model-mark { width: 26px; height: 26px; border-radius: 12px; color: #374151; background: #f3f4f6; font: 700 10px/1.3333 var(--mono); }
.model-cell > span:last-child { display: flex; flex-direction: column; }
.model-cell strong { color: #030712; font-size: 12px; }
.model-cell small { margin-top: 0; color: #9ca3af; font: 600 10px/1.3333 var(--mono); }
.tier { letter-spacing: .05em; }
.mono { font-family: var(--mono); font-variant-numeric: tabular-nums; }
.strong { color: #111827; font-weight: 700; }
.table-link { color: #18181b; font: 700 12px/16px var(--mono); }
.activity-list { display: flex; flex-direction: column; gap: 12px; }
.activity-item { display: flex; align-items: flex-start; justify-content: space-between; gap: 16px; padding: 16px; border: 1px solid #f3f4f6; border-radius: 16px; background: rgba(249,250,251,.82); }
.activity-main { display: flex; min-width: 0; align-items: flex-start; gap: 14px; }
.activity-icon { width: 30px; height: 30px; margin-top: 2px; border-radius: 12px; color: #4b5563; background: #fff; }
.activity-item:nth-child(1) .activity-icon { color: #059669; }
.activity-item:nth-child(2) .activity-icon { color: #d97706; }
.activity-copy { min-width: 0; }
.activity-copy strong { display: block; color: #030712; font-size: 12px; line-height: 16px; }
.activity-copy p { margin: 2px 0 0; color: #6b7280; font-size: 11px; font-weight: 500; line-height: 16.5px; }
.activity-item time { color: #9ca3af; font: 500 10px/15px var(--mono); white-space: nowrap; }
.placeholder-panel { display: flex; min-height: calc(100vh - 136px); align-items: center; justify-content: center; flex-direction: column; padding: 48px 24px; color: #6b7280; text-align: center; }
.placeholder-icon { display: inline-flex; width: 48px; height: 48px; align-items: center; justify-content: center; margin-bottom: 18px; border: 1px solid #e5e7eb; border-radius: 14px; background: #fff; color: #4b5563; }
.placeholder-panel h1 { margin: 0; color: #030712; font-size: 24px; }
.placeholder-panel p { max-width: 460px; margin: 8px 0 22px; font-size: 13px; line-height: 1.55; }
.dropdown-scrim { position: fixed; z-index: 15; inset: 0; padding: 0; border: 0; background: transparent; }
.dropdown-menu { position: absolute; z-index: 50; padding: 6px; border: 1px solid rgba(229,231,235,.9); border-radius: 14px; background: #fff; box-shadow: 0 10px 25px -5px rgba(0,0,0,.1); }
.role-menu { top: calc(100% + 6px); right: 0; left: 0; border-radius: 16px; }
.role-menu .dropdown-title { margin: 0 0 4px; padding: 4px 10px; border-bottom: 0; }
.dropdown-title { margin: 0 4px 4px; padding: 5px 6px 7px; border-bottom: 1px solid #f3f4f6; color: #9ca3af; font: 700 10px/15px var(--mono); text-transform: uppercase; }
.role-option, .language-option { display: flex; width: 100%; min-width: 0; align-items: center; gap: 10px; padding: 10px 12px; border: 0; border-radius: 12px; background: #fff; cursor: pointer; text-align: left; }
.role-option { justify-content: space-between; gap: normal; font: 600 12px/16px var(--sans); }
.role-option + .role-option { margin-top: 4px; }
.role-option:hover, .language-option:hover { background: #f3f4f6; }
.role-option.selected, .language-option.selected { color: #fff; background: #030712; }
.role-option.selected { box-shadow: 0 1px 2px rgba(0,0,0,.05); }
.role-option-main { display: flex; min-width: 0; align-items: center; gap: 10px; }
.role-option-dot { flex: 0 1 8px; width: 8px; height: 8px; border-radius: 50%; background: #10b981; box-shadow: 0 0 0 2px transparent; }
.role-option.selected .role-option-dot { box-shadow: 0 0 0 2px rgba(255,255,255,.3); }
.role-option-dot.supplier { background: #6366f1; }
.role-option-dot.admin { background: #f59e0b; }
.role-option-copy { display: flex; min-width: 0; flex-direction: column; }
.role-option-copy strong { display: block; color: inherit; font: 700 12px/16px var(--sans); letter-spacing: -.025em; }
.role-option-copy small { display: block; margin-top: 0; color: #9ca3af; font: 500 10px/1.3333 var(--sans); }
.role-option.selected .role-option-copy small { color: #d1d5db; }
.active-pill { margin-left: 0; padding: 2px 6px; border-radius: 4px; color: #fff; background: rgba(255,255,255,.2); font: 700 10px/1.3333 var(--mono); text-transform: uppercase; }
.language-menu { top: calc(100% + 6px); right: 0; width: 176px; border-radius: 12px; }
.language-option { justify-content: space-between; font-size: 12px; }
.language-option { padding: 6px 10px; border-radius: 8px; }
.language-option-flag { flex: 0 0 auto; font-size: 16px; }
.language-option-copy { display: flex; min-width: 0; flex: 1; flex-direction: column; }
.language-option-copy strong { overflow: hidden; font-size: 12px; text-overflow: ellipsis; white-space: nowrap; }
.language-option-copy small { margin-top: 2px; overflow: hidden; color: #9ca3af; font-size: 10px; text-overflow: ellipsis; white-space: nowrap; }
.selected .language-option-copy small { color: #d1d5db; }
.language-option > svg { flex: 0 0 auto; }
.mobile-menu-button, .sidebar-close, .mobile-scrim { display: none; }
@media (max-width: 1023px) { .topbar { padding: 0 24px; } .page-viewport { padding: 32px; } .metric-grid { grid-template-columns: repeat(2,minmax(0,1fr)); } }
@media (max-width: 1100px) { .global-search { width: auto; flex: 1 1 auto; min-width: 0; } }
@media (max-width: 800px) { .sidebar { position: fixed; inset: 0 auto 0 0; width: 240px; flex-basis: auto; box-shadow: 20px 0 45px rgba(0,0,0,.12); transform: translateX(-105%); transition: transform 220ms ease; } .sidebar.open { transform: translateX(0); } .sidebar-close { position: absolute; z-index: 2; top: 6px; right: 6px; display: inline-flex; border: 1px solid #e5e7eb; background: #fff; } .mobile-scrim { position: fixed; z-index: 25; inset: 0; border: 0; background: rgba(17,24,39,.28); backdrop-filter: blur(2px); } .mobile-menu-button { display: inline-flex; flex: 0 0 auto; } .global-search { max-width: none; } }
@media (max-width: 639px) { .topbar { gap: 10px; } .autopilot { display: none; } .page-header { align-items: flex-start; flex-direction: column; } .page-viewport { padding: 24px; } .page-heading-copy h1 { font-size: 24px; line-height: 1.333333; } .page-heading-copy p { font-size: 14px; } .metric-grid { grid-template-columns: 1fr; gap: 14px; } .metric-card { min-height: 142px; padding: 20px; } .metric-value-row strong { font-size: 24px; } .attention { align-items: stretch; flex-direction: column; } .panel { padding: 24px; } }
@media (max-width: 600px) { .global-search { width: min(100%, 448px); flex: 1 1 auto; min-width: 0; } .topbar-actions { gap: 4px; } .language-button { width: 34px; padding: 0; justify-content: center; } .language-button > svg, .language-name { display: none; } .topbar-divider, .profile-copy { display: none; } .language-menu { position: fixed; top: 60px; right: 12px; } }
.playground-stack { display: flex; flex-direction: column; gap: 0; padding-bottom: 4px; animation: page-in 300ms ease-out both; }
.playground-header h1 { margin: 0; color: #030712; font-size: 28px; font-weight: 800; line-height: 1.333333; letter-spacing: -.025em; }
.playground-header p { margin: 4px 0 0; color: #6b7280; font-size: 14px; font-weight: 500; line-height: 20px; }
.playground-header { margin-bottom: 16px; }
.playground-conclusion { margin: 0 0 28px; }
.playground-grid { display: grid; grid-template-columns: 1fr; align-items: stretch; gap: 24px; }
.playground-controls { display: flex; min-width: 0; flex-direction: column; gap: 16px; }
.playground-settings { display: flex; flex-direction: column; gap: 16px; padding: 20px; border-color: oklch(92.8% .006 264.531 / .8); }
.playground-panel-title { display: flex; align-items: center; gap: 6px; margin: 0; color: #111827; font: 700 12px/16px var(--mono); letter-spacing: .05em; text-transform: uppercase; }
.playground-panel-title svg { color: oklch(37.3% .034 259.733); }
.playground-field { margin-top: 0; }
.playground-field label, .playground-label, .playground-prompt-field label { color: oklch(37.3% .034 259.733); font-size: 12px; font-weight: 600; line-height: 16px; }
.playground-field select { display: block; width: 100%; height: 36px; margin-top: 0; padding: 0 12px; border: 1px solid oklch(92.8% .006 264.531); border-radius: 12px; outline: 0; background: oklch(98.5% .002 247.839); color: oklch(21% .034 264.665); font-size: 12px; font-weight: 500; line-height: 16px; cursor: pointer; }
.playground-field select:focus, .playground-prompt-field textarea:focus { border-color: oklch(21% .034 264.665); background: #fff; box-shadow: 0 0 0 2px oklch(21% .034 264.665 / .1); }
.tier-selector { display: grid; grid-template-columns: repeat(3,minmax(0,1fr)); gap: 6px; margin-top: 0; padding: 4px; border-radius: 12px; background: oklch(96.7% .003 264.542); }
.tier-option { min-width: 0; padding: 6px 0; border: 0; border-radius: 8px; background: transparent; color: oklch(44.6% .03 256.802); font-size: 12px; font-weight: 500; line-height: 16px; cursor: pointer; }
.tier-option:hover { color: oklch(21% .034 264.665); }
.tier-option.active { background: #fff; box-shadow: 0 1px 2px rgba(0,0,0,.05); color: oklch(13% .028 261.692); font-weight: 500; }
.tier-description { min-height: 15px; margin: 6px 0 0; color: #6b7280; font-size: 10px; font-weight: 400; line-height: 15px; }
.playground-parameters { display: flex; flex-direction: column; gap: 12px; margin-top: 0; padding-top: 12px; padding-bottom: 6px; border-top: 1px solid #f3f4f6; }
.range-label { display: flex; align-items: center; justify-content: space-between; gap: 12px; color: #374151; font-size: 12px; font-weight: 600; line-height: 16px; }
.range-label span { flex: 0 0 auto; color: #111827; font: 700 12px/16px var(--mono); }
.playground-parameters input[type="range"] { width: 100%; height: 20px; margin: 0; appearance: none; accent-color: #111827; cursor: pointer; background: transparent; }
.playground-parameters input[type="range"]::-webkit-slider-runnable-track { height: 4px; border-radius: 9999px; background: #e5e7eb; }
.playground-parameters input[type="range"]::-webkit-slider-thumb { width: 16px; height: 16px; margin-top: -6px; appearance: none; border: 0; border-radius: 9999px; background: #111827; box-shadow: 0 1px 3px rgba(0,0,0,.2); }
.playground-parameters input[type="range"]::-moz-range-track { height: 4px; border-radius: 9999px; background: #e5e7eb; }
.playground-parameters input[type="range"]::-moz-range-thumb { width: 16px; height: 16px; border: 0; border-radius: 9999px; background: #111827; box-shadow: 0 1px 3px rgba(0,0,0,.2); }
.playground-parameters input[type="range"] + .range-label { padding-top: 4px; }
.playground-code-panel { display: flex; flex-direction: column; gap: 12px; padding: 16px; border-color: oklch(92.8% .006 264.531 / .8); }
.playground-code-heading { display: flex; align-items: center; justify-content: space-between; gap: 0; }
.playground-code-panel .playground-panel-title { gap: 6px; letter-spacing: normal; text-transform: none; }
.code-tabs { display: flex; flex: 0 1 auto; align-items: center; gap: 4px; padding: 2px; border-radius: 8px; background: oklch(96.7% .003 264.542); }
.code-tab { height: 19px; padding: 2px 8px; border: 0; border-radius: 4px; background: transparent; color: oklch(55.1% .027 264.364); font: 400 10px/15px var(--mono); cursor: pointer; }
.code-tab.active { background: #fff; box-shadow: 0 1px 2px rgba(0,0,0,.05); color: oklch(21% .034 264.665); font-weight: 700; }
.code-block { min-height: 0; max-height: 192px; margin: 0; padding: 14px; overflow: auto; border: 1px solid oklch(27.8% .033 256.848 / .8); border-radius: 16px; background: oklch(21% .034 264.665 / .95); box-shadow: inset 0 2px 4px rgba(0,0,0,.05); color: oklch(96.7% .003 264.542); font: 400 10px/1.625 var(--mono); white-space: pre; }
.code-block code { font: inherit; }
.copy-code-button, .clear-output-button, .run-inference-button { display: inline-flex; align-items: center; justify-content: center; gap: 6px; border: 1px solid transparent; cursor: pointer; font-size: 12px; font-weight: 600; line-height: 16px; transition: background 150ms,border-color 150ms,transform 100ms; }
.copy-code-button:active, .clear-output-button:active, .run-inference-button:active { transform: scale(.98); }
.copy-code-button { width: 100%; height: 28px; padding: 0 10px; border-color: oklch(92.8% .006 264.531 / .9); border-radius: 8px; background: #fff; box-shadow: 0 1px 2px rgba(0,0,0,.03); color: oklch(21% .034 264.665); font-size: 11px; font-weight: 500; line-height: 16.5px; letter-spacing: -.025em; }
.copy-code-button:hover { border-color: #d1d5db; background: #f9fafb; color: #111827; }
.playground-workspace-column { display: flex; min-width: 0; flex-direction: column; gap: 16px; }
.playground-workspace { display: flex; min-height: 500px; flex-direction: column; justify-content: space-between; gap: 0; padding: 24px; border-color: oklch(92.8% .006 264.531 / .8); }
.playground-workspace > :not(:last-child) { margin-bottom: 16px; }
.playground-prompt-field { display: block; }
.playground-prompt-field > :not(:last-child) { margin-bottom: 6px; }
.prompt-label-row { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
.prompt-label-row > span { color: oklch(70.7% .022 261.325); font: 400 10px/15px var(--mono); }
.playground-prompt-field textarea { display: inline-block; width: 100%; min-height: 0; padding: 12px; border: 1px solid oklch(92.8% .006 264.531 / .8); border-radius: 12px; outline: 0; background: oklch(98.5% .002 247.839); color: oklch(21% .034 264.665); font-family: var(--sans); font-size: 12px; font-weight: 400; line-height: 16px; resize: vertical; transition: all 150ms cubic-bezier(.4,0,.2,1); }
.playground-prompt-field:first-child textarea { resize: none; font-family: var(--mono); }
.playground-runbar { display: flex; align-items: center; justify-content: space-between; gap: 0; padding-top: 8px; border-top: 1px solid oklch(96.7% .003 264.542); }
.attestation-copy { display: flex; min-width: 0; align-items: center; gap: 8px; color: oklch(55.1% .027 264.364); font: 400 12px/16px var(--mono); }
.attestation-copy svg { color: oklch(59.6% .145 163.225); }
.run-actions { display: flex; flex: 0 0 auto; align-items: center; gap: 8px; }
.clear-output-button { height: 34px; padding: 0 12px; border: 0; border-radius: 12px; background: transparent; color: oklch(44.6% .03 256.802); font-weight: 500; }
.clear-output-button:hover { background: rgba(243,244,246,.8); color: #030712; }
.run-inference-button { height: 40px; gap: 8px; padding: 0 16px; border-color: oklch(27.4% .006 286.033); border-radius: 12px; background: oklch(14.1% .005 285.823); box-shadow: 0 1px 2px rgba(0,0,0,.12),inset 0 1px rgba(255,255,255,.12); color: #fff; font-size: 13px; font-weight: 600; line-height: 19.5px; letter-spacing: -.025em; }
.run-inference-button svg { fill: currentColor; }
.run-inference-button:hover:not(:disabled) { background: #27272a; }
.run-inference-button:disabled { cursor: wait; opacity: .7; }
.playground-output { display: flex; min-height: 203.725px; flex: 1; flex-direction: column; justify-content: space-between; gap: 12px; margin-top: 16px; padding: 16px; border: 1px solid rgba(229,231,235,.7); border-radius: 12px; background: #f9fafb; }
.output-content { display: flex; flex-direction: column; gap: 8px; }
.output-heading { display: flex; align-items: center; justify-content: space-between; gap: 10px; padding-bottom: 8px; border-bottom: 1px solid rgba(229,231,235,.6); color: #374151; font: 600 10px/13.3333px var(--mono); letter-spacing: normal; text-transform: uppercase; }
.output-running { display: inline-flex; align-items: center; gap: 6px; color: #059669; font-size: 10px; letter-spacing: 0; text-transform: none; }
.output-running i { width: 7px; height: 7px; border-radius: 50%; background: #10b981; animation: output-pulse 1s ease-in-out infinite; }
@keyframes output-pulse { 50% { opacity: .35; transform: scale(.75); } }
.output-text { min-height: 140px; color: #111827; font: 400 12px/1.625 var(--mono); white-space: pre-wrap; overflow-wrap: anywhere; }
.empty-output { color: #9ca3af; font-style: italic; }
.inference-stats { display: grid; grid-template-columns: repeat(4,minmax(0,1fr)); gap: 8px; padding-top: 12px; border-top: 1px solid rgba(229,231,235,.8); color: #4b5563; font: 400 11px/16px var(--mono); }
.inference-stat { min-width: 0; padding: 8px; border: 1px solid rgba(229,231,235,.6); border-radius: 8px; background: #fff; }
.inference-stat span { display: block; overflow: hidden; color: #9ca3af; font: 400 9px/13px var(--mono); text-overflow: ellipsis; white-space: nowrap; }
.inference-stat strong { display: block; margin-top: 2px; overflow: hidden; color: #111827; font: 700 11px/15px var(--mono); text-overflow: ellipsis; white-space: nowrap; }
.inference-stat.cost strong { color: #047857; }
.marketplace-stack { display: flex; flex-direction: column; gap: 24px; padding-bottom: 4px; animation: page-in 300ms ease-out both; }
.marketplace-page-header { margin-bottom: -8px; }
.marketplace-conclusion { margin: 0 0 4px; }
.marketplace-toolbar { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
.marketplace-categories { display: flex; min-width: 0; align-items: center; gap: 6px; overflow-x: auto; scrollbar-width: none; }
.marketplace-categories::-webkit-scrollbar { display: none; }
.marketplace-category { flex: 0 0 auto; padding: 6px 12px; border: 1px solid rgba(229,231,235,.8); border-radius: 12px; background: #fff; color: oklch(44.6% .03 256.802); font-family: var(--sans); font-size: 12px; font-weight: 500; line-height: 16px; white-space: nowrap; cursor: pointer; transition: background 150ms,border-color 150ms,color 150ms,transform 100ms; }
.marketplace-category:hover { border-color: #d1d5db; background: #f3f4f6; color: #111827; }
.marketplace-category:active { transform: scale(.98); }
.marketplace-category.active { border-width: 0; background: oklch(21% .034 264.665); box-shadow: 0 1px 2px rgba(0,0,0,.05); color: #fff; font-weight: 500; }
.marketplace-search { position: relative; display: block; width: 288px; height: 38px; flex: 0 1 288px; min-width: 0; color: #9ca3af; }
.marketplace-search > svg { position: absolute; z-index: 1; top: 50%; left: 14px; pointer-events: none; transform: translateY(-50%); }
.marketplace-search input { width: 100%; height: 38px; min-width: 0; padding: 0 14px 0 38px; border: 1px solid rgba(229,231,235,.8); border-radius: 12px; outline: 0; background: rgba(249,250,251,.9); color: oklch(21% .034 264.665); font-family: var(--sans); font-size: 12px; font-weight: 400; line-height: 16px; transition: background 150ms,border-color 150ms,box-shadow 150ms; }
.marketplace-search input:hover { background: rgba(243,244,246,.7); }
.marketplace-search input:focus { border-color: oklch(21% .034 264.665); background: #fff; box-shadow: 0 0 0 2px rgba(17,24,39,.1); }
.marketplace-search input::placeholder { color: #9ca3af; }
.marketplace-search input::-webkit-search-cancel-button { cursor: pointer; }
.marketplace-grid { display: grid; grid-template-columns: repeat(3,minmax(0,1fr)); gap: 20px; align-items: stretch; }
.marketplace-card { display: flex; min-width: 0; min-height: 310px; flex-direction: column; justify-content: space-between; gap: 16px; padding: 20px; border-color: rgba(229,231,235,.8); transition: border-color 160ms,box-shadow 160ms,transform 160ms; }
.marketplace-card:hover { border-color: #d1d5db; box-shadow: 0 6px 20px rgba(0,0,0,.04); transform: translateY(-2px); }
.marketplace-card-main { display: flex; min-width: 0; flex-direction: column; gap: 12px; }
.marketplace-card-meta { display: flex; min-width: 0; min-height: 16.5px; align-items: center; justify-content: space-between; gap: 10px; }
.marketplace-family { overflow: hidden; color: #6b7280; font: 600 11px/16px var(--mono); letter-spacing: .05em; text-overflow: ellipsis; text-transform: uppercase; white-space: nowrap; }
.marketplace-card-meta .status { min-width: 0; flex: 0 1 auto; }
.marketplace-card-meta .status-label { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.marketplace-card-heading h2 { width: 103.1%; margin: 0; overflow-wrap: anywhere; color: #030712; font-size: 16px; font-weight: 700; line-height: 24px; letter-spacing: 0; transform: scaleX(.97); transform-origin: left center; }
.marketplace-card-heading p { display: -webkit-box; min-height: 39px; margin: 4px 0 0; overflow: hidden; -webkit-box-orient: vertical; -webkit-line-clamp: 2; color: #4b5563; font-size: 12px; line-height: 19px; }
.marketplace-price-box { display: flex; min-width: 0; align-items: center; justify-content: space-between; padding: 14px; border: 1px solid rgba(243,244,246,.9); border-radius: 16px; background: rgba(249,250,251,.9); box-shadow: 0 1px 2px rgba(0,0,0,.01); font: 400 12px/16px var(--mono); }
.marketplace-price-copy, .marketplace-context-copy { display: block; }
.marketplace-context-copy { flex: 0 1 auto; align-items: flex-end; padding-left: 12px; border-left: 1px solid rgba(229,231,235,.7); text-align: right; }
.marketplace-eyebrow { display: block; color: #9ca3af; font: 700 10px/1.3333 var(--mono); letter-spacing: .05em; text-transform: uppercase; }
.marketplace-price-values { display: block; padding-top: 2px; white-space: nowrap; }
.marketplace-price-values strong { display: inline; color: #030712; font: 800 14px/20px var(--mono); white-space: nowrap; }
.marketplace-context-copy > strong { display: block; padding-top: 2px; color: #030712; font: 800 14px/20px var(--mono); }
.marketplace-price-values span { padding: 0 4px; color: #9ca3af; font-weight: 400; }
.marketplace-price-copy small, .marketplace-context-copy small { display: block; margin-top: 2px; color: #9ca3af; font: 500 9px/12px var(--sans); }
.marketplace-context-copy small { color: #059669; font-family: var(--mono); font-weight: 700; white-space: normal; }
.marketplace-latency-value { white-space: nowrap; }
.marketplace-tier-row { display: flex; min-height: 19.6px; min-width: 0; flex-wrap: nowrap; align-items: center; gap: 6px; overflow: visible; padding-top: 4px; }
.marketplace-tier-row .marketplace-eyebrow { margin-right: 0; font-weight: 400; letter-spacing: 0; }
.marketplace-tier-row .tier, .marketplace-tier-row .badge-success, .marketplace-tier-row .badge-neutral, .marketplace-tier-row .badge-brand, .marketplace-tier-row .badge-accent { font-size: 10px; line-height: 10px; }
.marketplace-card-actions { display: flex; align-items: center; justify-content: space-between; gap: 8px; padding-top: 12px; border-top: 1px solid #f3f4f6; }
.marketplace-details-button { min-width: 0; padding: 5px 0; border: 0; background: transparent; color: #4b5563; font-size: 12px; font-weight: 500; line-height: 16px; cursor: pointer; text-align: left; }
.marketplace-details-button:hover { color: #030712; text-decoration: underline; }
.marketplace-playground-link { height: 34px; min-height: 34px; flex: 0 0 auto; padding: 0 12px; font-size: 12px; line-height: 16px; }
.marketplace-empty { display: flex; min-height: 260px; align-items: center; justify-content: center; flex-direction: column; gap: 10px; border: 1px dashed #d1d5db; border-radius: 16px; background: rgba(255,255,255,.7); color: #9ca3af; text-align: center; }
.marketplace-empty strong { color: #4b5563; font-size: 13px; font-weight: 600; }
.marketplace-drawer-backdrop { position: fixed; z-index: 60; inset: 0; background: rgba(3,7,18,.25); backdrop-filter: blur(2px); animation: marketplace-fade-in 180ms ease-out both; }
.marketplace-drawer { position: fixed; z-index: 65; top: 0; right: 0; bottom: 0; display: flex; width: min(576px,100vw); flex-direction: column; border-left: 1px solid rgba(229,231,235,.9); background: #fff; box-shadow: -20px 0 50px rgba(0,0,0,.1); animation: marketplace-slide-in 220ms ease-out both; }
.marketplace-drawer-header { display: flex; flex: 0 0 auto; min-width: 0; align-items: center; justify-content: space-between; gap: 16px; padding: 18px 24px; border-bottom: 1px solid #f3f4f6; }
.marketplace-drawer-header > div { min-width: 0; }
.marketplace-drawer-header h2 { margin: 0; overflow-wrap: anywhere; color: #030712; font-size: 16px; line-height: 22px; }
.marketplace-drawer-header p { margin: 2px 0 0; color: #6b7280; font-size: 12px; font-weight: 500; line-height: 17px; }
.marketplace-drawer-close { flex: 0 0 auto; background: #fff; color: #9ca3af; }
.marketplace-drawer-body { display: flex; min-height: 0; flex: 1 1 auto; flex-direction: column; gap: 24px; padding: 24px; overflow-y: auto; overscroll-behavior: contain; }
.marketplace-drawer-section h3 { margin: 0; color: #111827; font: 700 12px/16px var(--mono); letter-spacing: .05em; text-transform: uppercase; }
.marketplace-drawer-section > p { margin: 8px 0 0; color: #374151; font-size: 12px; line-height: 19px; }
.marketplace-direct-rate { padding: 16px; border: 1px solid rgba(229,231,235,.75); border-radius: 12px; background: #f9fafb; }
.marketplace-direct-rate-header { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
.marketplace-direct-rate-header > strong { color: #111827; font: 700 12px/16px var(--mono); }
.marketplace-direct-rate-grid { display: grid; grid-template-columns: repeat(2,minmax(0,1fr)); gap: 12px; padding-top: 10px; }
.marketplace-direct-rate-grid > div { display: flex; min-width: 0; flex-direction: column; }
.marketplace-direct-rate-grid span { color: #6b7280; font: 500 11px/16px var(--mono); }
.marketplace-direct-rate-grid strong { margin-top: 1px; color: #030712; font: 700 16px/22px var(--mono); }
.marketplace-direct-rate-grid small { color: #9ca3af; font: 500 10px/14px var(--mono); }
.marketplace-benchmarks { display: grid; grid-template-columns: repeat(3,minmax(0,1fr)); gap: 8px; margin-top: 8px; }
.marketplace-benchmark { min-width: 0; padding: 12px 8px; border: 1px solid #f3f4f6; border-radius: 12px; background: #f9fafb; text-align: center; }
.marketplace-benchmark span { display: block; overflow-wrap: anywhere; color: #6b7280; font: 500 10px/14px var(--mono); }
.marketplace-benchmark strong { display: block; margin-top: 4px; overflow-wrap: anywhere; color: #111827; font: 700 13px/18px var(--mono); }
.marketplace-recommendation { padding: 14px; border: 1px solid #e5e7eb; border-radius: 12px; background: #fafafa; color: #3f3f46 !important; font-weight: 500; }
.marketplace-slo-section { padding-top: 12px; border-top: 1px solid #e5e7eb; }
.marketplace-slo-toggle { display: flex; width: 100%; min-height: 34px; align-items: center; justify-content: space-between; gap: 12px; padding: 7px 0; border: 0; background: transparent; color: #1f2937; cursor: pointer; }
.marketplace-slo-label { display: flex; min-width: 0; align-items: center; gap: 6px; font: 700 12px/16px var(--mono); text-align: left; }
.marketplace-slo-label svg { flex: 0 0 auto; color: #6b7280; }
.marketplace-slo-toggle[aria-expanded="true"] > svg { transform: rotate(180deg); }
.marketplace-slo-data { display: flex; flex-direction: column; gap: 0; margin-top: 10px; padding: 16px; border-radius: 12px; background: #f9fafb; font: 500 11px/16px var(--mono); }
.marketplace-slo-data > div { display: flex; align-items: flex-start; justify-content: space-between; gap: 18px; padding: 9px 0; border-bottom: 1px solid rgba(229,231,235,.7); }
.marketplace-slo-data > div:first-child { padding-top: 0; }
.marketplace-slo-data > div:last-child { padding-bottom: 0; border-bottom: 0; }
.marketplace-slo-data span { color: #6b7280; }
.marketplace-slo-data strong { color: #111827; text-align: right; overflow-wrap: anywhere; }
.marketplace-slo-data > div:nth-child(3) strong { color: #047857; }
.marketplace-drawer-footer { margin-top: auto; padding-top: 16px; border-top: 1px solid #f3f4f6; }
.marketplace-drawer-cta { width: 100%; min-height: 40px; justify-content: center; }
@keyframes marketplace-fade-in { from { opacity: 0; } }
@keyframes marketplace-slide-in { from { transform: translateX(100%); } }
@media (max-width: 1023px) { .marketplace-grid { grid-template-columns: repeat(2,minmax(0,1fr)); } }
@media (max-width: 767px) { .marketplace-grid { grid-template-columns: 1fr; gap: 14px; } .marketplace-card { min-height: 0; } }
@media (max-width: 639px) { .marketplace-stack { gap: 18px; } .marketplace-page-header { margin-bottom: 0; } .marketplace-conclusion { margin-bottom: 0; } .marketplace-toolbar { align-items: center; flex-direction: column; } .marketplace-categories { width: 100%; } .marketplace-search { width: 100%; flex-basis: auto; } .marketplace-card { padding: 18px; } .marketplace-card-actions { align-items: stretch; flex-direction: column; } .marketplace-details-button { min-height: 32px; text-align: center; } .marketplace-playground-link { width: 100%; justify-content: center; } .marketplace-drawer-header, .marketplace-drawer-body { padding-right: 18px; padding-left: 18px; } }
@media (max-width: 420px) { .marketplace-price-box { flex-direction: column; } .marketplace-context-copy { align-items: flex-start; padding-top: 10px; padding-left: 0; border-top: 1px solid rgba(229,231,235,.7); border-left: 0; text-align: left; } .marketplace-benchmarks { grid-template-columns: 1fr; } .marketplace-drawer-header { align-items: flex-start; } .marketplace-slo-data > div { flex-direction: column; gap: 3px; } .marketplace-slo-data strong { text-align: left; } }
@media (min-width: 1024px) { .playground-grid { grid-template-columns: repeat(12,minmax(0,1fr)); } .playground-controls { grid-column: span 4 / span 4; } .playground-workspace-column { grid-column: span 8 / span 8; } }
@media (max-width: 800px) { .playground-workspace-column { grid-column: auto; } .playground-workspace { min-height: 0; } }
@media (max-width: 639px) { .playground-stack { gap: 18px; } .playground-header h1 { font-size: 24px; } .playground-controls { display: flex; } .playground-settings, .playground-code-panel, .playground-workspace { padding: 18px; } .playground-runbar { align-items: flex-start; flex-direction: column; } .run-actions { width: 100%; justify-content: flex-end; } .attestation-copy { font-size: 10px; } .inference-stats { grid-template-columns: repeat(2,minmax(0,1fr)); } }
@media (max-width: 420px) { .run-actions { justify-content: space-between; } .clear-output-button { padding: 0 4px; } .run-inference-button { flex: 1; padding: 0 9px; } }
.api-keys-stack { display: flex; min-width: 0; flex-direction: column; gap: 24px; }
.api-keys-header-block { display: flex; min-width: 0; flex-direction: column; gap: 16px; margin-bottom: 4px; }
.api-keys-page-header { gap: 16px; margin-bottom: 0; }
.api-keys-page-header h1 { color: oklch(13% .028 261.692); }
.api-keys-page-header .page-heading-copy p { color: oklch(55.1% .027 264.364); }
.api-keys-conclusion { margin: 0; }
.api-keys-create-button { display: inline-flex; height: 34px; min-height: 34px; align-items: center; justify-content: center; gap: 6px; padding: 0 12px; border: 1px solid #27272a; border-radius: 12px; background: #09090b; box-shadow: 0 1px 2px rgba(0,0,0,.12),inset 0 1px rgba(255,255,255,.12); color: #fff; cursor: pointer; font-size: 12px; font-weight: 500; line-height: 16px; letter-spacing: -.025em; white-space: nowrap; transition: all 150ms; }
.api-keys-create-button:hover { background: #09090b; }
.api-keys-create-button:active { background: #000; transform: scale(.98); }
.api-keys-panel { padding: 24px; overflow: hidden; border: 1px solid oklch(92.8% .006 264.531 / .8); border-radius: 16px; background: #fff; box-shadow: 0 1px 3px 0 rgba(0,0,0,.02),0 1px 2px -1px rgba(0,0,0,.02); transition: all 200ms; }
.api-keys-panel .section-header { gap: 0; margin-bottom: 16px; }
.api-keys-panel .section-header h2 { color: oklch(13% .028 261.692); }
.api-keys-panel .section-header p { margin-top: 0; color: oklch(55.1% .027 264.364); font-weight: 400; }
.api-keys-count { flex: 0 0 auto; color: oklch(70.7% .022 261.325); font: 400 12px/16px var(--mono); white-space: nowrap; }
.api-keys-count strong { color: inherit; font-weight: inherit; }
.api-keys-table-scroll { width: 100%; overflow-x: auto; overscroll-behavior-x: contain; }
.api-keys-table { width: 100%; border-collapse: collapse; text-align: left; font-size: 12px; line-height: 1.333333; }
.api-keys-table thead tr, .api-keys-table tbody tr { border-bottom: 1px solid oklch(96.7% .003 264.542); }
.api-keys-table tbody tr:last-child { border-bottom: 0; }
.api-keys-table th { padding: 0 0 12px; color: oklch(70.7% .022 261.325); font: 600 10px/1.333333 var(--mono); letter-spacing: 0; text-transform: uppercase; }
.api-keys-table td { padding: 14px 0; }
.api-keys-table tbody tr { transition: background 150ms; }
.api-keys-table tbody tr:hover { background: oklch(98.5% .002 247.839 / .7); }
.api-key-name { line-height: 16px; }
.api-key-name-label { display: block; color: oklch(21% .034 264.665); font-size: 12px; font-weight: 600; line-height: 16px; }
.api-key-name-created { display: block; color: oklch(70.7% .022 261.325); font: 500 10px/1.333333 var(--mono); }
.api-key-secret { color: oklch(44.6% .03 256.802); font: 400 12px/16px var(--mono); }
.api-key-rate { color: oklch(37.3% .034 259.733); font: 400 12px/16px var(--mono); }
.api-key-last-used { color: oklch(55.1% .027 264.364); font: 400 12px/16px var(--mono); }
.api-key-spend strong { display: block; color: oklch(21% .034 264.665); font: 600 12px/16px var(--mono); }
.api-key-progress { display: block; width: 96px; height: 6px; margin-top: 4px; overflow: hidden; border-radius: 9999px; background: oklch(96.7% .003 264.542); }
.api-key-progress span { display: block; height: 100%; border-radius: inherit; background: oklch(21% .034 264.665); }
.api-keys-panel .badge-success { border-color: oklch(90.5% .093 164.15 / .8); color: oklch(43.2% .095 166.913); background: oklch(97.9% .021 166.113); }
.api-keys-panel .badge-error { border-color: oklch(89.2% .058 10.001 / .8); color: oklch(45.5% .188 13.697); background: oklch(96.9% .015 12.422); }
.api-key-revoke { padding: 0; border: 0; background: transparent; color: oklch(58.6% .253 17.585); cursor: pointer; font: 500 12px/16px var(--mono); }
.api-key-revoke:hover { color: oklch(45.5% .188 13.697); text-decoration: underline; }
.api-keys-empty { display: flex; min-height: 250px; align-items: center; justify-content: center; flex-direction: column; gap: 8px; padding: 32px; border: 1px dashed #d1d5db; border-radius: 12px; background: #fafafa; text-align: center; }
.api-keys-empty-icon { display: inline-flex; width: 42px; height: 42px; align-items: center; justify-content: center; margin-bottom: 4px; border: 1px solid #e5e7eb; border-radius: 12px; background: #fff; color: #6b7280; }
.api-keys-empty h3 { margin: 0; color: #111827; font-size: 14px; line-height: 20px; }
.api-keys-empty p { margin: 0 0 8px; color: #6b7280; font-size: 12px; line-height: 18px; }
.api-key-modal-layer { position: fixed; z-index: 80; inset: 0; display: flex; align-items: center; justify-content: center; padding: 16px; overflow-y: auto; }
.api-key-modal-backdrop { position: fixed; z-index: 0; inset: 0; padding: 0; border: 0; outline: 0; background: rgba(3,7,18,.3); backdrop-filter: blur(12px); cursor: default; animation: api-key-fade-in 150ms ease-out both; }
.api-key-modal-backdrop:focus-visible { outline: 0; }
.api-key-modal { position: relative; z-index: 1; width: min(512px,100%); margin: 32px auto; overflow: hidden; border: 1px solid #e5e7eb; border-radius: 16px; background: #fff; box-shadow: 0 25px 50px -12px rgba(0,0,0,.15); animation: api-key-modal-in 180ms cubic-bezier(.2,.8,.2,1) both; }
.api-key-modal-header { display: flex; min-width: 0; align-items: center; justify-content: space-between; gap: 20px; padding: 18px 24px; border-bottom: 1px solid #f3f4f6; }
.api-key-modal-header > div { min-width: 0; }
.api-key-modal-header h2 { margin: 0; color: #030712; font-size: 16px; font-weight: 700; line-height: 24px; }
.api-key-modal-header p { margin: 2px 0 0; color: #6b7280; font-size: 12px; font-weight: 500; line-height: 16px; }
.api-key-modal-close { width: 28px; height: 28px; flex: 0 0 auto; padding: 6px; border-radius: 8px; color: #9ca3af; }
.api-key-modal-close:hover { color: #1f2937; background: #f3f4f6; }
.api-key-modal-body { display: flex; flex-direction: column; gap: 16px; padding: 24px; }
.api-key-field { display: flex; min-width: 0; flex-direction: column; gap: 6px; }
.api-key-field label { color: #374151; font-size: 12px; font-weight: 600; line-height: 16px; }
.api-key-field input { width: 100%; height: 38px; padding: 0 14px; border: 1px solid rgba(229,231,235,.9); border-radius: 12px; outline: 0; background: #fff; box-shadow: 0 1px 2px rgba(0,0,0,.02); color: #111827; font-size: 12px; transition: border-color 150ms,box-shadow 150ms; }
.api-key-field input::placeholder { color: #9ca3af; }
.api-key-field input:focus { border-color: #111827; box-shadow: 0 0 0 2px rgba(17,24,39,.1); }
.api-key-field-error { min-height: 16px; margin: 0; color: #be123c; font-size: 11px; font-weight: 500; line-height: 16px; }
.api-key-form-grid { display: grid; grid-template-columns: repeat(2,minmax(0,1fr)); gap: 12px; }
.api-key-modal-actions { display: flex; align-items: center; justify-content: flex-end; gap: 8px; padding-top: 12px; border-top: 1px solid #f3f4f6; }
.api-key-action-button { display: inline-flex; height: 34px; align-items: center; justify-content: center; gap: 6px; padding: 0 12px; border: 1px solid rgba(229,231,235,.9); border-radius: 12px; cursor: pointer; font-size: 12px; font-weight: 500; line-height: 16px; letter-spacing: -.025em; white-space: nowrap; transition: all 150ms; }
.api-key-action-button.secondary { background: #fff; box-shadow: 0 1px 2px rgba(0,0,0,.03); color: #111827; }
.api-key-action-button.secondary:hover { border-color: #d1d5db; background: #f9fafb; }
.api-key-action-button.primary { border-color: #27272a; background: #09090b; box-shadow: 0 1px 2px rgba(0,0,0,.12),inset 0 1px rgba(255,255,255,.12); color: #fff; }
.api-key-action-button:active { transform: scale(.98); }
.api-key-action-button.primary:hover { background: #09090b; }
.api-key-action-button.primary:active { background: #000; }
.api-key-warning { display: flex; align-items: flex-start; gap: 8px; padding: 12px; border: 1px solid #fde68a; border-radius: 12px; background: #fffbeb; color: #78350f; font-size: 12px; line-height: 16px; }
.api-key-warning svg { flex: 0 0 auto; margin-top: 2px; color: #d97706; }
.api-key-warning p { margin: 0; }
.api-key-secret-result { display: flex; min-width: 0; align-items: center; justify-content: space-between; padding: 12px; border-radius: 12px; background: #030712; color: #34d399; font-family: var(--mono); }
.api-key-secret-result code { min-width: 0; padding-right: 8px; overflow: hidden; font: 400 12px/16px var(--mono); text-overflow: ellipsis; white-space: nowrap; }
.api-key-copy-button { display: inline-flex; height: 28px; flex: 0 0 auto; align-items: center; justify-content: center; gap: 6px; padding: 0 10px; border: 1px solid rgba(229,231,235,.9); border-radius: 8px; background: #fff; box-shadow: 0 1px 2px rgba(0,0,0,.03); color: #111827; cursor: pointer; font-size: 12px; font-weight: 500; line-height: 16px; letter-spacing: -.025em; transition: all 150ms; }
.api-key-copy-button:hover { border-color: #d1d5db; background: #f9fafb; }
.api-key-copy-button:active { background: #f3f4f6; transform: scale(.98); }
.api-key-copy-button.copied svg { color: #059669; }
.api-key-result-actions { display: flex; justify-content: flex-end; padding-top: 8px; }
.api-keys-create-button:focus-visible, .api-key-action-button.primary:focus-visible { outline: 0; box-shadow: 0 0 0 1px #fff, 0 0 0 3px rgba(9,9,11,.2), 0 1px 2px rgba(0,0,0,.12), inset 0 1px rgba(255,255,255,.12); }
.api-key-action-button.secondary:focus-visible, .api-key-copy-button:focus-visible { outline: 0; box-shadow: 0 0 0 1px #fff, 0 0 0 3px rgba(9,9,11,.2), 0 1px 2px rgba(0,0,0,.03); }
@keyframes api-key-fade-in { from { opacity: 0; } }
@keyframes api-key-modal-in { from { opacity: 0; transform: translateY(10px) scale(.95); } }
@media (max-width: 639px) { .api-keys-stack { gap: 24px; } .api-keys-page-header { align-items: stretch; } .api-key-modal-header, .api-key-modal-body { padding-right: 18px; padding-left: 18px; } }
@media (max-width: 420px) { .api-key-form-grid { grid-template-columns: 1fr; } .api-key-modal-actions { align-items: stretch; flex-direction: column-reverse; } .api-key-action-button { width: 100%; } .api-key-secret-result { align-items: stretch; flex-direction: column; gap: 12px; } .api-key-secret-result code { overflow-wrap: anywhere; text-overflow: initial; white-space: normal; } .api-key-copy-button { width: 100%; } }
@media (prefers-reduced-motion: reduce) { *, *::before, *::after { animation-duration: .01ms !important; transition-duration: .01ms !important; } }
"#;
