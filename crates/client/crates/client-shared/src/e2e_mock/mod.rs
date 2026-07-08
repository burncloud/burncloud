//! Opt-in mock data layer for E2E visual/aesthetic preview routes (`/preview/*`).
//!
//! **Production default:** inactive — all service calls hit real APIs.
//! Mock data activates only while an [`E2eMockPageShell`] preview route is mounted
//! (debug builds or `e2e-preview` feature).

mod fixtures;
mod shell;

pub use shell::E2eMockPageShell;

use crate::api_client::TokenDto;
use crate::billing_service::BillingSummary;
use crate::channel_service::Channel;
use crate::log_service::LogEntry;
use crate::monitor_service::{
    FilterConfig, RiskEventPage, SecuritySummary, SystemMetrics,
};
use crate::usage_service::{Recharge, UsageStats};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum E2eMockPage {
    Home,
    Login,
    Dashboard,
    Models,
    Access,
    Settings,
    Finance,
    Monitor,
    Playground,
}

#[derive(Clone, Default)]
pub struct E2eMockRegistry {
    pub channels: Option<Vec<Channel>>,
    pub tokens: Option<Vec<TokenDto>>,
    pub system_metrics: Option<SystemMetrics>,
    pub security_summary: Option<SecuritySummary>,
    pub risk_events: Option<RiskEventPage>,
    pub filter_config: Option<FilterConfig>,
    pub usage: Option<UsageStats>,
    pub billing: Option<BillingSummary>,
    pub recharges: Option<Vec<Recharge>>,
    pub logs: Option<Vec<LogEntry>>,
}

#[cfg(any(debug_assertions, feature = "e2e-preview"))]
mod state {
    use super::E2eMockRegistry;
    use std::sync::RwLock;

    static ACTIVE: RwLock<Option<E2eMockRegistry>> = RwLock::new(None);

    pub fn activate(registry: E2eMockRegistry) {
        if let Ok(mut guard) = ACTIVE.write() {
            *guard = Some(registry);
        }
    }

    pub fn deactivate() {
        if let Ok(mut guard) = ACTIVE.write() {
            *guard = None;
        }
    }

    pub fn with_active<R>(f: impl FnOnce(&E2eMockRegistry) -> Option<R>) -> Option<R> {
        let guard = ACTIVE.read().ok()?;
        guard.as_ref().and_then(f)
    }
}

#[cfg(any(debug_assertions, feature = "e2e-preview"))]
pub fn activate(registry: E2eMockRegistry) {
    state::activate(registry);
}

#[cfg(any(debug_assertions, feature = "e2e-preview"))]
pub fn deactivate() {
    state::deactivate();
}

#[cfg(not(any(debug_assertions, feature = "e2e-preview")))]
pub fn activate(_registry: E2eMockRegistry) {}

#[cfg(not(any(debug_assertions, feature = "e2e-preview")))]
pub fn deactivate() {}

/// True when preview routes are compiled into this binary.
pub const PREVIEW_ROUTES_AVAILABLE: bool = cfg!(any(debug_assertions, feature = "e2e-preview"));

pub fn channels() -> Option<Vec<Channel>> {
    #[cfg(any(debug_assertions, feature = "e2e-preview"))]
    {
        return state::with_active(|r| r.channels.clone());
    }
    #[cfg(not(any(debug_assertions, feature = "e2e-preview")))]
    {
        None
    }
}

pub fn tokens() -> Option<Vec<TokenDto>> {
    #[cfg(any(debug_assertions, feature = "e2e-preview"))]
    {
        return state::with_active(|r| r.tokens.clone());
    }
    #[cfg(not(any(debug_assertions, feature = "e2e-preview")))]
    {
        None
    }
}

pub fn system_metrics() -> Option<SystemMetrics> {
    #[cfg(any(debug_assertions, feature = "e2e-preview"))]
    {
        return state::with_active(|r| r.system_metrics.clone());
    }
    #[cfg(not(any(debug_assertions, feature = "e2e-preview")))]
    {
        None
    }
}

pub fn security_summary() -> Option<SecuritySummary> {
    #[cfg(any(debug_assertions, feature = "e2e-preview"))]
    {
        return state::with_active(|r| r.security_summary.clone());
    }
    #[cfg(not(any(debug_assertions, feature = "e2e-preview")))]
    {
        None
    }
}

pub fn risk_events() -> Option<RiskEventPage> {
    #[cfg(any(debug_assertions, feature = "e2e-preview"))]
    {
        return state::with_active(|r| r.risk_events.clone());
    }
    #[cfg(not(any(debug_assertions, feature = "e2e-preview")))]
    {
        None
    }
}

pub fn filter_config() -> Option<FilterConfig> {
    #[cfg(any(debug_assertions, feature = "e2e-preview"))]
    {
        return state::with_active(|r| r.filter_config.clone());
    }
    #[cfg(not(any(debug_assertions, feature = "e2e-preview")))]
    {
        None
    }
}

pub fn usage_stats() -> Option<UsageStats> {
    #[cfg(any(debug_assertions, feature = "e2e-preview"))]
    {
        return state::with_active(|r| r.usage.clone());
    }
    #[cfg(not(any(debug_assertions, feature = "e2e-preview")))]
    {
        None
    }
}

pub fn billing_summary() -> Option<BillingSummary> {
    #[cfg(any(debug_assertions, feature = "e2e-preview"))]
    {
        return state::with_active(|r| r.billing.clone());
    }
    #[cfg(not(any(debug_assertions, feature = "e2e-preview")))]
    {
        None
    }
}

pub fn recharges() -> Option<Vec<Recharge>> {
    #[cfg(any(debug_assertions, feature = "e2e-preview"))]
    {
        return state::with_active(|r| r.recharges.clone());
    }
    #[cfg(not(any(debug_assertions, feature = "e2e-preview")))]
    {
        None
    }
}

pub fn logs() -> Option<Vec<LogEntry>> {
    #[cfg(any(debug_assertions, feature = "e2e-preview"))]
    {
        return state::with_active(|r| r.logs.clone());
    }
    #[cfg(not(any(debug_assertions, feature = "e2e-preview")))]
    {
        None
    }
}

/// Map a production path to its preview entry (E2E only). Returns `None` for unmapped paths.
pub fn preview_path_for(production_path: &str) -> Option<&'static str> {
    if !PREVIEW_ROUTES_AVAILABLE {
        return None;
    }
    match production_path {
        "/" => Some("/preview/home"),
        "/login" => Some("/preview/login"),
        "/console/dashboard" => Some("/preview/console/dashboard"),
        "/console/models" => Some("/preview/console/models"),
        "/console/access" => Some("/preview/console/access"),
        "/console/settings" => Some("/preview/console/settings"),
        "/console/finance" => Some("/preview/console/finance"),
        "/console/monitor" => Some("/preview/console/monitor"),
        "/console/playground" => Some("/preview/console/playground"),
        _ => None,
    }
}
