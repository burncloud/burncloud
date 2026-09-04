use crate::models::IconKind;
use leptos::prelude::*;

// Lucide icon geometry, rendered by one Rust component for consistent sizing and stroke.
#[component]
pub fn Icon(
    kind: IconKind,
    #[prop(default = 16)] size: u8,
    #[prop(default = "")] class: &'static str,
) -> impl IntoView {
    let paths = match kind {
        IconKind::AlertTriangle => {
            r#"<path d="m21.73 18-8-14a2 2 0 0 0-3.46 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3Z"/><path d="M12 9v4"/><path d="M12 17h.01"/>"#
        }
        IconKind::ArrowRight => r#"<path d="M5 12h14"/><path d="m12 5 7 7-7 7"/>"#,
        IconKind::Bell => {
            r#"<path d="M10.268 21a2 2 0 0 0 3.464 0"/><path d="M3.262 15.326A1 1 0 0 0 4 17h16a1 1 0 0 0 .74-1.673C19.41 13.956 18 12.499 18 8A6 6 0 0 0 6 8c0 4.499-1.411 5.956-2.738 7.326"/>"#
        }
        IconKind::Building => {
            r#"<path d="M3 21h18"/><path d="M6 21V3h12v18"/><path d="M9 7h1"/><path d="M9 11h1"/><path d="M9 15h1"/><path d="M14 7h1"/><path d="M14 11h1"/><path d="M14 15h1"/>"#
        }
        IconKind::Chart => r#"<path d="M3 3v18h18"/><path d="m19 9-5 5-4-4-3 3"/>"#,
        IconKind::Check => r#"<path d="M20 6 9 17l-5-5"/>"#,
        IconKind::CheckCircle => {
            r#"<path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"/><path d="m9 11 3 3L22 4"/>"#
        }
        IconKind::ChevronDown => r#"<path d="m6 9 6 6 6-6"/>"#,
        IconKind::Coins => {
            r#"<circle cx="8" cy="8" r="6"/><path d="M18.09 10.37A6 6 0 1 1 10.34 18"/><path d="M7 6h1v4"/><path d="m16.71 13.88.7.71-2.82 2.82"/>"#
        }
        IconKind::Cpu => {
            r#"<rect width="16" height="16" x="4" y="4" rx="2"/><rect width="6" height="6" x="9" y="9" rx="1"/><path d="M9 1v3"/><path d="M15 1v3"/><path d="M9 20v3"/><path d="M15 20v3"/><path d="M20 9h3"/><path d="M20 14h3"/><path d="M1 9h3"/><path d="M1 14h3"/>"#
        }
        IconKind::CreditCard => {
            r#"<rect width="20" height="14" x="2" y="5" rx="2"/><path d="M2 10h20"/>"#
        }
        IconKind::Dollar => {
            r#"<line x1="12" x2="12" y1="2" y2="22"/><path d="M17 5H9.5a3.5 3.5 0 0 0 0 7h5a3.5 3.5 0 0 1 0 7H6"/>"#
        }
        IconKind::Gauge => r#"<path d="m12 14 4-4"/><path d="M3.34 19a10 10 0 1 1 17.32 0"/>"#,
        IconKind::Globe => {
            r#"<circle cx="12" cy="12" r="10"/><path d="M2 12h20"/><path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"/>"#
        }
        IconKind::Key => {
            r#"<circle cx="7.5" cy="15.5" r="5.5"/><path d="m21 2-9.6 9.6"/><path d="m15.5 7.5 3 3L22 7l-3-3"/>"#
        }
        IconKind::Layers => {
            r#"<path d="m12.83 2.18 8 4a2 2 0 0 1 0 3.58l-8 4a2 2 0 0 1-1.79 0l-8-4a2 2 0 0 1 0-3.58l8-4a2 2 0 0 1 1.79 0Z"/><path d="m22 12.5-9.17 4.59a2 2 0 0 1-1.79 0L2 12.5"/><path d="m22 17.5-9.17 4.59a2 2 0 0 1-1.79 0L2 17.5"/>"#
        }
        IconKind::Layout => {
            r#"<rect width="7" height="9" x="3" y="3" rx="1"/><rect width="7" height="5" x="14" y="3" rx="1"/><rect width="7" height="9" x="14" y="12" rx="1"/><rect width="7" height="5" x="3" y="16" rx="1"/>"#
        }
        IconKind::Menu => r#"<path d="M4 6h16"/><path d="M4 12h16"/><path d="M4 18h16"/>"#,
        IconKind::Receipt => {
            r#"<path d="M4 2v20l2-1 2 1 2-1 2 1 2-1 2 1 2-1 2 1V2l-2 1-2-1-2 1-2-1-2 1-2-1-2 1Z"/><path d="M16 8h-6"/><path d="M16 12h-6"/><path d="M13 16h-3"/>"#
        }
        IconKind::Search => r#"<circle cx="11" cy="11" r="8"/><path d="m21 21-4.3-4.3"/>"#,
        IconKind::Server => {
            r#"<rect width="20" height="8" x="2" y="2" rx="2"/><rect width="20" height="8" x="2" y="14" rx="2"/><path d="M6 6h.01"/><path d="M6 18h.01"/>"#
        }
        IconKind::Settings => {
            r#"<path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.38a2 2 0 0 0-.73-2.73l-.15-.09a2 2 0 0 1-1-1.74v-.51a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"/><circle cx="12" cy="12" r="3"/>"#
        }
        IconKind::Shield => {
            r#"<path d="M20 13c0 5-3.5 7.5-8 9-4.5-1.5-8-4-8-9V5l8-3 8 3z"/><path d="m9 12 2 2 4-4"/>"#
        }
        IconKind::Store => {
            r#"<path d="m2 7 3-5h14l3 5"/><path d="M5 13v9h14v-9"/><path d="M5 13a3 3 0 0 1-3-3V7h20v3a3 3 0 0 1-6 0 3 3 0 0 1-6 0 3 3 0 0 1-5 3Z"/><path d="M9 22v-6h6v6"/>"#
        }
        IconKind::Terminal => {
            r#"<polyline points="4 17 10 11 4 5"/><line x1="12" x2="20" y1="19" y2="19"/>"#
        }
        IconKind::Trending => {
            r#"<polyline points="22 7 13.5 15.5 8.5 10.5 2 17"/><polyline points="16 7 22 7 22 13"/>"#
        }
        IconKind::Users => {
            r#"<path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M22 21v-2a4 4 0 0 0-3-3.87"/><path d="M16 3.13a4 4 0 0 1 0 7.75"/>"#
        }
        IconKind::Workflow => {
            r#"<rect width="8" height="8" x="3" y="3" rx="2"/><path d="M7 11v4a2 2 0 0 0 2 2h4"/><rect width="8" height="8" x="13" y="13" rx="2"/>"#
        }
        IconKind::X => r#"<path d="M18 6 6 18"/><path d="m6 6 12 12"/>"#,
        IconKind::Zap => {
            r#"<path d="M4 14a1 1 0 0 1-.78-1.63l9-11a.5.5 0 0 1 .87.45l-1.7 6.8A1 1 0 0 0 11.36 9H20a1 1 0 0 1 .78 1.63l-9 11a.5.5 0 0 1-.87-.45l1.7-6.8a1 1 0 0 0-.97-1.38z"/>"#
        }
    };

    view! {
        <svg
            class=class
            width=size
            height=size
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            aria-hidden="true"
            inner_html=paths
        />
    }
}

#[component]
pub fn Logo(#[prop(default = 28)] size: u8) -> impl IntoView {
    view! {
        <svg width=size height=size viewBox="0 0 24 24" fill="none" aria-hidden="true">
            <defs>
                <linearGradient id="burn-cloud-gradient" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="0%" stop-color="#f7b52c" />
                    <stop offset="100%" stop-color="#e95513" />
                </linearGradient>
            </defs>
            <path d="M17.8 10.1q-.6-.9-1.4-1.9S14.6 6.1 14.9 3c0 0-6.9 2.7-7 8.2 0 0-1-1.6-.8-4.6 0 0-2.2 2.1-2.5 5.5-2.1.7-3.8 2.5-3.8 4.3 0 2.5 2.7 4.6 5.9 4.6-2.4-.4-4.2-2-4.2-4 0-1.4.8-2.5 2-3.3q.1 1.1.5 2.4s1.2 3.8 5.4 4.8c1.2.3 2.5.2 3.7-.3 1.3-.6 2.8-1.8 2.8-4.5 0 0 .1-2.7-1.5-4.1 0 0 2.1 5-1.8 6.5-1.3.5-2.6.5-3.9 0-1.7-.7-3.8-2.5-3.5-7.2 0 0 1 3.4 3.2 4.7 0 0-2-5.8 3.9-9.8 0 0 .5 2.1 1.9 3.3.4.4 4 3.2 3.3 8 .7-.9 1.3-3.1.7-4.8 0 0-.1-.4-.4-.9 1.5.3 2.7 1.5 2.8 4.2.1 2.3-1.6 4.2-3.8 5 3-.4 5.4-2.7 5.4-5.6 0-2.8-2.2-5.1-5.4-5.3z" fill="url(#burn-cloud-gradient)" />
        </svg>
    }
}

#[component]
pub fn ButtonLink(
    href: &'static str,
    label: &'static str,
    icon: IconKind,
    #[prop(default = false)] primary: bool,
) -> impl IntoView {
    let class = if primary {
        "button button-primary"
    } else {
        "button button-secondary"
    };
    view! {
        <a href=href rel="external" class=class>
            <Icon kind=icon size=14 />
            <span>{label}</span>
        </a>
    }
}

#[component]
pub fn Status(#[prop(default = "Healthy")] label: &'static str) -> impl IntoView {
    view! {
        <span class="status"><span class="status-dot"></span>{label}</span>
    }
}
