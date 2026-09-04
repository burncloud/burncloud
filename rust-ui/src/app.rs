use crate::components::{Icon, Logo};
use crate::i18n::{Locale, initial_locale, persist_locale};
use crate::models::{BALANCE, IconKind, NavKey, Role, nav_items};
use crate::pages::{OverviewPage, PlaceholderPage};
use leptos::prelude::*;
use leptos_router::components::Router;
use leptos_router::hooks::use_location;

#[derive(Clone, Copy)]
pub struct AppState {
    pub locale: RwSignal<Locale>,
    pub role: RwSignal<Role>,
    pub role_menu_open: RwSignal<bool>,
    pub language_menu_open: RwSignal<bool>,
    pub sidebar_open: RwSignal<bool>,
    pub search: RwSignal<String>,
}

#[component]
pub fn App() -> impl IntoView {
    let locale = initial_locale();
    persist_locale(locale);
    let state = AppState {
        locale: RwSignal::new(locale),
        role: RwSignal::new(Role::Buyer),
        role_menu_open: RwSignal::new(false),
        language_menu_open: RwSignal::new(false),
        sidebar_open: RwSignal::new(false),
        search: RwSignal::new(String::new()),
    };
    provide_context(state);

    view! { <Router><AppShell /></Router> }
}

#[component]
fn AppShell() -> impl IntoView {
    let state = expect_context::<AppState>();
    let location = use_location();
    Effect::new(move |_| {
        let path = location.pathname.get();
        if !is_known_path(&path) {
            if let Some(window) = web_sys::window() {
                let _ = window.location().replace("/");
            }
            return;
        }
        state.role.set(Role::from_path(&path));
        state.sidebar_open.set(false);
        state.role_menu_open.set(false);
        state.language_menu_open.set(false);
    });

    view! {
        <div class="app-shell">
            <button
                class="mobile-scrim"
                class:visible=move || state.sidebar_open.get()
                aria-label=move || state.locale.get().common().close_menu
                on:click=move |_| state.sidebar_open.set(false)
            ></button>
            <Sidebar />
            <div class="app-main">
                <Topbar />
                <main class="page-viewport">
                    <div class="content-width">
                        {move || {
                            let path = location.pathname.get();
                            if path == "/" || path == "/buyer/overview" {
                                view! { <OverviewPage /> }.into_any()
                            } else {
                                view! { <PlaceholderPage /> }.into_any()
                            }
                        }}
                    </div>
                </main>
            </div>
        </div>
    }
}

#[component]
fn Sidebar() -> impl IntoView {
    let state = expect_context::<AppState>();
    let location = use_location();

    view! {
        <aside class="sidebar" class:open=move || state.sidebar_open.get()>
            <div class="sidebar-brand-area">
                <button
                    class="icon-button sidebar-close"
                    aria-label=move || state.locale.get().common().close_menu
                    on:click=move |_| state.sidebar_open.set(false)
                ><Icon kind=IconKind::X size=18 /></button>
                <div class="role-switcher">
                    <button
                        class="brand-button"
                        aria-haspopup="true"
                        aria-expanded=move || state.role_menu_open.get().to_string()
                        on:click=move |_| {
                            state.language_menu_open.set(false);
                            state.role_menu_open.update(|open| *open = !*open);
                        }
                    >
                        <span class="brand-identity"><Logo size=27 /><span class="brand-copy">
                            <strong>"BurnCloud"<span class="role-color-dot" class:supplier=move || state.role.get() == Role::Supplier class:admin=move || state.role.get() == Role::Admin></span></strong>
                            <span>{move || state.locale.get().role(state.role.get()).title}<small>" • Pro"</small></span>
                        </span></span>
                        <Icon kind=IconKind::ChevronDown size=16 />
                    </button>

                    <Show when=move || state.role_menu_open.get()>
                        <button class="dropdown-scrim" aria-label="Close" on:click=move |_| state.role_menu_open.set(false)></button>
                        <div class="role-menu dropdown-menu">
                            <p class="dropdown-title">{move || state.locale.get().common().switch_role}</p>
                            {Role::ALL.into_iter().map(|candidate| {
                                view! {
                                    <a
                                        href=candidate.overview_path()
                                        rel="external"
                                        class="role-option"
                                        class:selected=move || state.role.get() == candidate
                                        on:click=move |_| {
                                            state.role.set(candidate);
                                            state.role_menu_open.set(false);
                                            state.sidebar_open.set(false);
                                        }
                                    >
                                        <span class=format!("role-option-dot {}", candidate.slug())></span>
                                        <span class="role-option-copy"><strong>{move || state.locale.get().role(candidate).title}</strong><small>{move || state.locale.get().role(candidate).subtext}</small></span>
                                        <Show when=move || state.role.get() == candidate><span class="active-pill">{move || state.locale.get().common().active}</span></Show>
                                    </a>
                                }
                            }).collect_view()}
                        </div>
                    </Show>
                </div>
            </div>

            <div class="workflow-strip">{move || state.locale.get().role(state.role.get()).flow}</div>

            <nav class="side-nav" aria-label="Primary navigation">
                {move || {
                    let locale = state.locale.get();
                    let role = state.role.get();
                    let path = location.pathname.get();
                    nav_items(role).iter().copied().map(|item| {
                        let is_root_overview = path == "/" && role == Role::Buyer && item.key == NavKey::Overview;
                        let active = is_root_overview || path.starts_with(item.path);
                        view! {
                            <a href=item.path rel="external" class=if active { "nav-link active" } else { "nav-link" }>
                                <span class="nav-label"><Icon kind=item.icon size=16 /><span>{locale.nav(item.key)}</span></span>
                                {item.badge.map(|badge| view! { <span class="nav-badge">{if badge == "LIVE" { locale.common().live } else { badge }}</span> })}
                            </a>
                        }
                    }).collect_view()
                }}
            </nav>

            <div class="sidebar-footer">
                <div class="role-metric">
                    <span>{move || role_metric_label(state.locale.get(), state.role.get())}</span>
                    <div><strong>{move || role_metric_value(state.role.get())}</strong>
                        <Show when=move || state.role.get() == Role::Buyer>
                            <a href="/buyer/billing" rel="external" class="top-up-link">"+ "{move || state.locale.get().common().top_up}</a>
                        </Show>
                    </div>
                </div>
                <div class="sidebar-meta">
                    <a href="/home" rel="external"><Icon kind=IconKind::Globe size=14 /><span>{move || state.locale.get().common().public_portal}</span></a>
                    <span class="sla"><i></i>{move || state.locale.get().common().sla}</span>
                </div>
            </div>
        </aside>
    }
}

#[component]
fn Topbar() -> impl IntoView {
    let state = expect_context::<AppState>();

    view! {
        <header class="topbar">
            <button
                class="icon-button mobile-menu-button"
                aria-label=move || state.locale.get().common().open_menu
                on:click=move |_| state.sidebar_open.set(true)
            ><Icon kind=IconKind::Menu size=18 /></button>

            <div class="global-search">
                <Icon kind=IconKind::Search size=14 />
                <input
                    type="search"
                    aria-label="Global search"
                    prop:value=move || state.search.get()
                    placeholder=move || match state.role.get() {
                        Role::Buyer => state.locale.get().common().search_buyer,
                        Role::Supplier => state.locale.get().common().search_supplier,
                        Role::Admin => state.locale.get().common().search_admin,
                    }
                    on:input=move |event| state.search.set(event_target_value(&event))
                />
            </div>

            <div class="topbar-actions">
                <div class="autopilot"><span></span>{move || state.locale.get().common().autopilot}</div>
                <LanguageMenu />
                <a href=move || format!("{}#attention", state.role.get().overview_path()) rel="external" class="icon-button notification" title=move || state.locale.get().common().attention>
                    <Icon kind=IconKind::Bell size=17 /><span class="notification-dot"></span>
                </a>
                <span class="topbar-divider"></span>
                <div class="profile"><span class="avatar">{move || match state.role.get() { Role::Buyer => "BY", Role::Supplier => "SP", Role::Admin => "AD" }}</span>
                    <span class="profile-copy"><strong>"burncloud.com"</strong><small>{move || state.locale.get().role(state.role.get()).title}</small></span>
                </div>
            </div>
        </header>
    }
}

#[component]
fn LanguageMenu() -> impl IntoView {
    let state = expect_context::<AppState>();

    view! {
        <div class="language-switcher">
            <button
                class="language-button"
                aria-haspopup="true"
                aria-expanded=move || state.language_menu_open.get().to_string()
                title=move || state.locale.get().common().select_language
                on:click=move |_| {
                    state.role_menu_open.set(false);
                    state.language_menu_open.update(|open| *open = !*open);
                }
            >
                <Icon kind=IconKind::Globe size=14 /><span class="language-flag">{move || state.locale.get().flag()}</span>
                <span class="language-name">{move || state.locale.get().native_name()}</span><Icon kind=IconKind::ChevronDown size=14 />
            </button>
            <Show when=move || state.language_menu_open.get()>
                <button class="dropdown-scrim" aria-label="Close" on:click=move |_| state.language_menu_open.set(false)></button>
                <div class="language-menu dropdown-menu">
                    <p class="dropdown-title">{move || state.locale.get().common().select_language}</p>
                    {Locale::ALL.into_iter().map(|locale| view! {
                        <button
                            class="language-option"
                            class:selected=move || state.locale.get() == locale
                            on:click=move |_| {
                                state.locale.set(locale);
                                persist_locale(locale);
                                state.language_menu_open.set(false);
                            }
                        >
                            <span>{locale.flag()}</span><span><strong>{locale.native_name()}</strong><small>{locale.english_name()}</small></span>
                            <Show when=move || state.locale.get() == locale><Icon kind=IconKind::Check size=14 /></Show>
                        </button>
                    }).collect_view()}
                </div>
            </Show>
        </div>
    }
}

fn is_known_path(path: &str) -> bool {
    matches!(
        path,
        "/" | "/home"
            | "/landing"
            | "/login"
            | "/register"
            | "/buyer"
            | "/buyer/overview"
            | "/buyer/playground"
            | "/buyer/marketplace"
            | "/buyer/api-keys"
            | "/buyer/usage"
            | "/buyer/billing"
            | "/buyer/logs"
            | "/playground"
            | "/marketplace"
            | "/models"
            | "/keys"
            | "/usage"
            | "/billing"
            | "/logs"
            | "/supplier"
            | "/supplier/overview"
            | "/supplier/resources"
            | "/supplier/deployments"
            | "/supplier/earnings"
            | "/supplier/settlements"
            | "/supplier/reliability"
            | "/supplier/settings"
            | "/admin"
            | "/admin/overview"
            | "/admin/supply"
            | "/admin/capacity"
            | "/admin/demand"
            | "/admin/models"
            | "/admin/revenue"
            | "/admin/settlements"
            | "/admin/suppliers"
            | "/admin/customers"
            | "/admin/operations"
            | "/admin/settings"
    )
}

fn role_metric_label(locale: Locale, role: Role) -> &'static str {
    match (locale, role) {
        (Locale::En, Role::Buyer) => locale.common().balance,
        (Locale::En, Role::Supplier) => "Today Net Earnings",
        (Locale::En, Role::Admin) => "Today GMV (Gross)",
        (Locale::Zh, Role::Buyer) => locale.common().balance,
        (Locale::Zh, Role::Supplier) => "今日净收益",
        (Locale::Zh, Role::Admin) => "今日平台 GMV",
        (Locale::ZhTw, Role::Buyer) => locale.common().balance,
        (Locale::ZhTw, Role::Supplier) => "今日淨收益",
        (Locale::ZhTw, Role::Admin) => "今日平台 GMV",
        (Locale::Ja, Role::Buyer) => locale.common().balance,
        (Locale::Ja, Role::Supplier) => "本日の純収益",
        (Locale::Ja, Role::Admin) => "本日の GMV",
    }
}

fn role_metric_value(role: Role) -> String {
    match role {
        Role::Buyer => format!("${BALANCE:.2}"),
        Role::Supplier => "$382.40".to_string(),
        Role::Admin => "$18,450".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_guard_accepts_original_public_contract() {
        assert!(is_known_path("/buyer/overview"));
        assert!(is_known_path("/supplier/resources"));
        assert!(is_known_path("/admin/operations"));
        assert!(!is_known_path("/not-a-real-route"));
    }
}
