use crate::{
    app::router::routes::Route,
    domains::buyer::overview::model::OverviewModel,
    i18n::{formatter::currency, strings, Locale, LocaleStrings},
    shared::{
        types::{nav_items, NavKey, Role},
        ui::{GlobalStyle, Icon, IconName, Logo},
    },
};
use dioxus::prelude::*;

fn route_path(route: &Route) -> &'static str {
    match route {
        Route::Home {} => "/buyer/overview",
        Route::PublicHome {} => "/home",
        Route::Landing {} => "/landing",
        Route::Login {} => "/login",
        Route::Register {} => "/register",
        Route::Buyer {}
        | Route::BuyerOverviewRoute {}
        | Route::Console {}
        | Route::ConsoleDashboard {}
        | Route::ConsoleBuyerOverview {} => "/buyer/overview",
        Route::Playground {} => "/buyer/playground",
        Route::Marketplace {} => "/buyer/marketplace",
        Route::ApiKeys {} => "/buyer/api-keys",
        Route::Usage {} => "/buyer/usage",
        Route::PublicPlayground {} => "/playground",
        Route::PublicMarketplace {} => "/marketplace",
        Route::Models {} => "/models",
        Route::Keys {} => "/keys",
        Route::PublicUsage {} => "/usage",
        Route::PublicBilling {} => "/billing",
        Route::PublicLogs {} => "/logs",
        Route::Supplier {} => "/supplier",
        Route::Billing {} => "/buyer/billing",
        Route::Logs {} => "/buyer/logs",
        Route::SupplierOverview {} => "/supplier/overview",
        Route::SupplierResources {} => "/supplier/resources",
        Route::SupplierDeployments {} => "/supplier/deployments",
        Route::SupplierEarnings {} => "/supplier/earnings",
        Route::SupplierSettlements {} => "/supplier/settlements",
        Route::SupplierReliability {} => "/supplier/reliability",
        Route::SupplierSettings {} => "/supplier/settings",
        Route::Admin {} => "/admin",
        Route::AdminOverview {} => "/admin/overview",
        Route::AdminSupply {} => "/admin/supply",
        Route::AdminCapacity {} => "/admin/capacity",
        Route::AdminDemand {} => "/admin/demand",
        Route::AdminModels {} => "/admin/models",
        Route::AdminRevenue {} => "/admin/revenue",
        Route::AdminSettlements {} => "/admin/settlements",
        Route::AdminSuppliers {} => "/admin/suppliers",
        Route::AdminCustomers {} => "/admin/customers",
        Route::AdminOperations {} => "/admin/operations",
        Route::AdminSettings {} => "/admin/settings",
        Route::NotFound { .. } => "",
    }
}

fn role_label(role: Role, copy: &LocaleStrings) -> &'static str {
    match role {
        Role::Buyer => copy.buyer,
        Role::Supplier => copy.supplier,
        Role::Admin => copy.admin,
    }
}

fn role_flow(role: Role, copy: &LocaleStrings) -> &'static str {
    match role {
        Role::Buyer => copy.buyer_flow,
        Role::Supplier => copy.supplier_flow,
        Role::Admin => copy.admin_flow,
    }
}

fn role_subtext(role: Role, copy: &LocaleStrings) -> &'static str {
    match role {
        Role::Buyer => copy.buyer_subtext,
        Role::Supplier => copy.supplier_subtext,
        Role::Admin => copy.admin_subtext,
    }
}

fn search_placeholder(role: Role, copy: &LocaleStrings) -> &'static str {
    match role {
        Role::Buyer => copy.search,
        Role::Supplier => copy.search_supplier,
        Role::Admin => copy.search_admin,
    }
}

fn nav_label(locale: Locale, copy: &LocaleStrings, key: NavKey) -> &'static str {
    match key {
        NavKey::Overview => copy.navigation[0],
        NavKey::Playground => copy.navigation[1],
        NavKey::Marketplace => copy.navigation[2],
        NavKey::ApiKeys => copy.navigation[3],
        NavKey::Usage => copy.navigation[4],
        NavKey::Billing => copy.navigation[5],
        NavKey::Logs => copy.navigation[6],
        NavKey::Resources => match locale {
            Locale::Zh => "GPU 算力资源",
            Locale::ZhTw => "GPU 算力資源",
            Locale::Ja => "GPU リソース",
            Locale::En => "GPU Resources",
        },
        NavKey::Deployments => match locale {
            Locale::Zh => "Autopilot 部署",
            Locale::ZhTw => "Autopilot 部署",
            Locale::Ja => "Autopilot デプロイ",
            Locale::En => "Autopilot Deployments",
        },
        NavKey::Earnings => match locale {
            Locale::Zh => "算力收益明细",
            Locale::ZhTw => "算力收益明細",
            Locale::Ja => "収益と支払い",
            Locale::En => "Revenue & Payouts",
        },
        NavKey::Settlements => match locale {
            Locale::Zh => "结算打款批次",
            Locale::ZhTw => "結算打款批次",
            Locale::Ja => "決済バッチ",
            Locale::En => "Settlement Batches",
        },
        NavKey::Reliability => match locale {
            Locale::Zh => "SLA 可用性审计",
            Locale::ZhTw => "SLA 可用性稽核",
            Locale::Ja => "SLA と信頼性",
            Locale::En => "SLA & Reliability",
        },
        NavKey::Settings => copy.settings,
        NavKey::Supply => match locale {
            Locale::Zh => "全局算力池",
            Locale::ZhTw => "全域算力池",
            Locale::Ja => "供給フリート",
            Locale::En => "Supply Fleet",
        },
        NavKey::Capacity => match locale {
            Locale::Zh => "容量与弹性伸缩",
            Locale::ZhTw => "容量與彈性伸縮",
            Locale::Ja => "キャパシティと拡張",
            Locale::En => "Capacity & Autoscale",
        },
        NavKey::Demand => match locale {
            Locale::Zh => "全网 Token 需求",
            Locale::ZhTw => "全網 Token 需求",
            Locale::Ja => "トークン需要",
            Locale::En => "Token Demand",
        },
        NavKey::Models => match locale {
            Locale::Zh => "模型定价目录",
            Locale::ZhTw => "模型定價目錄",
            Locale::Ja => "モデルカタログ",
            Locale::En => "Model Catalog",
        },
        NavKey::Revenue => match locale {
            Locale::Zh => "平台利润与分成",
            Locale::ZhTw => "平台利潤與分成",
            Locale::Ja => "プラットフォーム収益",
            Locale::En => "Platform Revenue",
        },
        NavKey::Suppliers => match locale {
            Locale::Zh => "供应商名录档案",
            Locale::ZhTw => "供應商名錄檔案",
            Locale::Ja => "サプライヤー一覧",
            Locale::En => "Supplier Accounts",
        },
        NavKey::Customers => match locale {
            Locale::Zh => "企业客户账户",
            Locale::ZhTw => "企業客戶帳戶",
            Locale::Ja => "エンタープライズ顧客",
            Locale::En => "Enterprise Customers",
        },
        NavKey::Operations => match locale {
            Locale::Zh => "应急安全熔断",
            Locale::ZhTw => "緊急安全熔斷",
            Locale::Ja => "緊急制御",
            Locale::En => "Emergency Controls",
        },
    }
}

fn role_metric_label(locale: Locale, role: Role, copy: &LocaleStrings) -> &'static str {
    match (locale, role) {
        (_, Role::Buyer) => copy.prepaid_balance,
        (Locale::Zh, Role::Supplier) => "今日净收益",
        (Locale::ZhTw, Role::Supplier) => "今日淨收益",
        (Locale::Ja, Role::Supplier) => "本日の純収益",
        (_, Role::Supplier) => "Today Net Earnings",
        (Locale::Zh, Role::Admin) => "今日平台 GMV",
        (Locale::ZhTw, Role::Admin) => "今日平台 GMV",
        (Locale::Ja, Role::Admin) => "本日の GMV",
        (_, Role::Admin) => "Today GMV (Gross)",
    }
}

fn role_metric_value(role: Role, model: &OverviewModel) -> String {
    match role {
        Role::Buyer => model
            .prepaid_balance
            .map(currency)
            .unwrap_or_else(|| "—".to_string()),
        Role::Supplier => "$382.40".to_string(),
        Role::Admin => "$18,450".to_string(),
    }
}

fn is_active(path: &str, item_path: &str) -> bool {
    path == item_path
        || (item_path == "/buyer/overview" && path == "/buyer/overview")
        || path.starts_with(item_path)
}

fn start_window_drag(event: Event<MouseData>) {
    if event.data().trigger_button() == Some(dioxus::html::input_data::MouseButton::Primary) {
        crate::platform::desktop::drag_window();
    }
}

fn stop_window_drag(event: Event<MouseData>) {
    event.stop_propagation();
}

#[component]
pub fn BuyerShell(children: Element) -> Element {
    let mut locale = use_context::<Signal<Locale>>();
    let copy = strings(locale());
    let mut role = use_context::<Signal<Role>>();
    let current_route = use_route::<Route>();
    let current_path = route_path(&current_route);
    let current_role = role();
    use_effect(move || {
        let next = Role::from_path(current_path);
        if role() != next {
            role.set(next);
        }
    });
    let mut role_menu_open = use_signal(|| false);
    let mut language_menu_open = use_signal(|| false);
    let mut drawer_open = use_signal(|| false);
    let mut search = use_context::<Signal<String>>();
    let navigator = use_navigator();
    let model = OverviewModel::mock();
    let menu_is_open = role_menu_open() || language_menu_open();
    let attention_path = current_role.overview_path().to_string();

    rsx! {
        GlobalStyle {}
        div { class: "app-shell",
            if menu_is_open { div { class: "dropdown-scrim", role: "presentation", onclick: move |_| { role_menu_open.set(false); language_menu_open.set(false); } } }
            if drawer_open() { div { class: "mobile-scrim visible", role: "presentation", onclick: move |_| drawer_open.set(false) } }
            aside { class: if drawer_open() { "sidebar open" } else { "sidebar" },
                div { class: "sidebar-brand-area desktop-drag-region", onmousedown: start_window_drag,
                    button { class: "icon-button sidebar-close desktop-no-drag", aria_label: copy.close, onmousedown: stop_window_drag, onclick: move |_| drawer_open.set(false), Icon { name: IconName::X, size: 18 } }
                    div { class: "role-switcher desktop-no-drag", onmousedown: stop_window_drag,
                        button { class: "brand-button", aria_haspopup: "true", aria_expanded: role_menu_open(), onclick: move |_| { language_menu_open.set(false); role_menu_open.set(!role_menu_open()); },
                            span { class: "brand-identity", Logo { size: 26 } span { class: "brand-copy",
                                span { class: "brand-name-row", strong { "BurnCloud" } span { class: if current_role == Role::Supplier { "role-color-dot supplier" } else if current_role == Role::Admin { "role-color-dot admin" } else { "role-color-dot" } } }
                                span { class: "brand-role-row", span { {role_label(current_role, copy)} } small { "• Pro" } }
                            } }
                            Icon { name: IconName::ChevronDown, size: 16 }
                        }
                        if role_menu_open() {
                            div { class: "role-menu dropdown-menu",
                                p { class: "dropdown-title", {copy.switch_role} }
                                for candidate in Role::ALL {
                                    button { class: if candidate == current_role { "role-option selected" } else { "role-option" }, onclick: move |_| {
                                            role_menu_open.set(false);
                                            drawer_open.set(false);
                                            navigator.push(match candidate { Role::Buyer => Route::BuyerOverviewRoute {}, Role::Supplier => Route::SupplierOverview {}, Role::Admin => Route::AdminOverview {} });
                                        },
                                        span { class: "role-option-main",
                                            span { class: "role-option-dot {candidate.code()}" }
                                            span { class: "role-option-copy", strong { {role_label(candidate, copy)} } small { {role_subtext(candidate, copy)} } }
                                        }
                                        if candidate == current_role { span { class: "active-pill", {copy.active} } }
                                    }
                                }
                            }
                        }
                    }
                }
                div { class: "workflow-strip", {role_flow(current_role, copy)} }
                nav { class: "side-nav",
                    div { class: "nav-list",
                        for item in nav_items(current_role) {
                            Link { class: if is_active(current_path, item.path) { "nav-link active" } else { "nav-link" }, to: item.path, onclick: move |_| drawer_open.set(false),
                                span { class: "nav-label", Icon { name: item.icon, size: 16 } span { {nav_label(locale(), copy, item.key)} } }
                                if item.key == NavKey::Playground {
                                    span { class: "nav-badge", {copy.live} }
                                } else if let Some(badge) = item.badge {
                                    span { class: "nav-badge", {badge} }
                                }
                            }
                        }
                    }
                }
                div { class: "sidebar-footer",
                    div { class: "role-metric", span { {role_metric_label(locale(), current_role, copy)} } div { strong { {role_metric_value(current_role, &model)} } if current_role == Role::Buyer { a { role: "button", class: "top-up-link", href: "/buyer/billing", onclick: move |_| drawer_open.set(false), "+ Top Up" } } } }
                    div { class: "sidebar-meta", a { href: "/home", Icon { name: IconName::Globe, size: 14 } span { {copy.public_portal} } } span { class: "sla", i {} {copy.sla} } }
                }
            }
            div { class: "app-main",
                header { class: "topbar desktop-drag-region", onmousedown: start_window_drag,
                    button { class: "icon-button mobile-menu-button desktop-no-drag", aria_label: copy.menu, onmousedown: stop_window_drag, onclick: move |_| drawer_open.set(true), Icon { name: IconName::Menu, size: 18 } }
                    div { class: "global-search desktop-no-drag", onmousedown: stop_window_drag, Icon { name: IconName::Search, size: 14 } input { r#type: "text", placeholder: search_placeholder(current_role, copy), value: search(), oninput: move |event| search.set(event.value()) } }
                    div { class: "topbar-actions desktop-no-drag", onmousedown: stop_window_drag,
                        div { class: "autopilot", span {}, span { class: "autopilot-label", {copy.autopilot} } }
                        div { class: "language-switcher",
                            button { id: "language-switcher-button", class: "language-button", title: copy.select_language, aria_haspopup: "true", aria_expanded: language_menu_open(), onclick: move |_| { role_menu_open.set(false); language_menu_open.set(!language_menu_open()); }, Icon { name: IconName::Globe, size: 14 } span { class: "language-copy", span { class: "language-flag", {locale().flag()} } span { class: "language-name", {locale().native_name()} } } Icon { name: IconName::ChevronDown, size: 14 } }
                            if language_menu_open() {
                        div { class: "language-menu dropdown-menu",
                                    p { class: "dropdown-title", {copy.language} }
                                    for lang in Locale::ALL {
                                        button { id: match lang { Locale::Zh => "lang-opt-zh", Locale::En => "lang-opt-en", Locale::ZhTw => "lang-opt-zh-TW", Locale::Ja => "lang-opt-ja" }, class: if lang == locale() { "language-option selected" } else { "language-option" }, onclick: move |_| { locale.set(lang); language_menu_open.set(false); dioxus::document::eval(&format!("localStorage.setItem('burncloud_selected_language', '{}'); document.documentElement.lang = '{}';", lang.storage_code(), lang.browser_code())); }, span { class: "language-option-flag", {lang.flag()} } span { class: "language-option-copy", strong { {lang.native_name()} } small { {lang.english_name()} } } if lang == locale() { Icon { name: IconName::Check, size: 14 } } }
                                    }
                                }
                            }
                        }
                        button {
                            class: "icon-button notification",
                            title: copy.attention_needed,
                            onclick: move |_| {
                                let script = format!(
                                    "window.history.pushState({{}}, '', '{}#attention'); document.getElementById('attention')?.scrollIntoView({{behavior:'smooth',block:'center'}});",
                                    attention_path
                                );
                                dioxus::document::eval(&script);
                            },
                            Icon { name: IconName::Bell, size: 16 }
                            span { class: "notification-dot" }
                        }
                        span { class: "topbar-divider" }
                        div { class: "profile", span { class: "avatar", {match current_role { Role::Buyer => "BY", Role::Supplier => "SP", Role::Admin => "AD" }} } span { class: "profile-copy", strong { "burncloud.com" } small { {role_label(current_role, copy)} } } }
                    }
                }
                main { class: "page-viewport", div { class: "content-width", if current_role == Role::Buyer { {children} } else { div { class: "placeholder-panel", span { class: "placeholder-icon", Icon { name: IconName::Layers, size: 22 } } h1 { {format!("{} Overview", role_label(current_role, copy))} } p { {copy.placeholder} } } } } }
            }
        }
    }
}
