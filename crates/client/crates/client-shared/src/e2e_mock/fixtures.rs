//! Static fixtures for E2E preview routes (visual/aesthetic tests only).

use crate::api_client::TokenDto;
use crate::billing_service::{BillingModelSummary, BillingSummary};
use crate::channel_service::Channel;
use crate::log_service::LogEntry;
use crate::monitor_service::{
    FilterConfig, RiskEvent, RiskEventPage, SecuritySummary, SystemMetrics,
};
use crate::usage_service::{Recharge, UsageStats};

use super::E2eMockPage;

pub fn registry_for_page(page: E2eMockPage) -> super::E2eMockRegistry {
    let mut reg = super::E2eMockRegistry::default();
    match page {
        E2eMockPage::Home | E2eMockPage::Login => {}
        E2eMockPage::Dashboard => {
            reg.system_metrics = Some(sample_system_metrics());
            reg.channels = Some(sample_channels());
            reg.usage = Some(sample_usage());
            reg.billing = Some(sample_billing());
            reg.logs = Some(sample_logs());
        }
        E2eMockPage::Models => {
            reg.channels = Some(sample_channels());
        }
        E2eMockPage::Access | E2eMockPage::Playground => {
            reg.tokens = Some(sample_tokens());
            reg.channels = Some(sample_channels());
        }
        E2eMockPage::Finance => {
            reg.billing = Some(sample_billing());
            reg.recharges = Some(sample_recharges());
        }
        E2eMockPage::Monitor => {
            reg.security_summary = Some(sample_security_summary());
            reg.risk_events = Some(sample_risk_events());
            reg.filter_config = Some(sample_filter_config());
        }
        E2eMockPage::Settings => {}
    }
    reg
}

fn sample_channels() -> Vec<Channel> {
    vec![
        Channel {
            id: 1,
            type_: 1,
            key: "sk-preview".into(),
            name: "OpenAI Primary".into(),
            base_url: "https://api.openai.com".into(),
            models: "gpt-4o-mini,gpt-4o".into(),
            group: Some("default".into()),
            status: 1,
            priority: 1,
            weight: 100,
            param_override: None,
            header_override: None,
        },
        Channel {
            id: 2,
            type_: 14,
            key: "sk-preview-2".into(),
            name: "Anthropic Backup".into(),
            base_url: "https://api.anthropic.com".into(),
            models: "claude-3-5-sonnet".into(),
            group: Some("default".into()),
            status: 1,
            priority: 2,
            weight: 50,
            param_override: None,
            header_override: None,
        },
    ]
}

fn sample_tokens() -> Vec<TokenDto> {
    vec![TokenDto {
        token: "bc-preview-token-001".into(),
        user_id: "preview-admin".into(),
        status: "active".into(),
        quota_limit: 100_000,
        used_quota: 12_400,
    }]
}

fn sample_system_metrics() -> SystemMetrics {
    SystemMetrics {
        cpu: crate::monitor_service::CpuInfo {
            usage_percent: 24.5,
            core_count: 8,
            frequency: 3_200,
            brand: "Preview CPU".into(),
        },
        memory: crate::monitor_service::MemoryInfo {
            total: 16_000_000_000,
            used: 8_500_000_000,
            available: 7_500_000_000,
            usage_percent: 53.1,
        },
        disks: vec![crate::monitor_service::DiskInfo {
            total: 512_000_000_000,
            used: 210_000_000_000,
            available: 302_000_000_000,
            usage_percent: 41.0,
            mount_point: "/".into(),
        }],
        timestamp: 1_700_000_000,
    }
}

fn sample_usage() -> UsageStats {
    UsageStats {
        prompt_tokens: 1_240_000,
        completion_tokens: 380_000,
        total_tokens: 1_620_000,
    }
}

fn sample_billing() -> BillingSummary {
    BillingSummary {
        period_start: Some("2026-07-01".into()),
        period_end: Some("2026-07-31".into()),
        pre_migration_requests: 0,
        models: vec![BillingModelSummary {
            model: "gpt-4o-mini".into(),
            requests: 12_400,
            prompt_tokens: 900_000,
            cache_read_tokens: 0,
            completion_tokens: 280_000,
            reasoning_tokens: 0,
            cost_usd: 4.82,
        }],
        total_cost_usd: 4.82,
    }
}

fn sample_recharges() -> Vec<Recharge> {
    vec![Recharge {
        id: 1,
        user_id: "preview-admin".into(),
        amount: 5_000_000_000,
        description: Some("Preview recharge".into()),
        created_at: Some("2026-07-01T10:00:00Z".into()),
    }]
}

fn sample_logs() -> Vec<LogEntry> {
    vec![LogEntry {
        request_id: "req-preview-001".into(),
        user_id: Some("preview-admin".into()),
        path: "/v1/chat/completions".into(),
        method: Some("POST".into()),
        upstream_id: Some("1".into()),
        status_code: 200,
        latency_ms: 420,
        model: Some("gpt-4o-mini".into()),
        total_tokens: Some(128),
        created_at: Some("2026-07-07T12:00:00Z".into()),
    }]
}

fn sample_security_summary() -> SecuritySummary {
    SecuritySummary {
        score: 86,
        blocked_count: 42,
        threat_source_count: 3,
        sparkline: vec![4, 6, 3, 8, 5, 7, 4],
    }
}

fn sample_risk_events() -> RiskEventPage {
    RiskEventPage {
        data: vec![RiskEvent {
            id: 1,
            time: "2026-07-07 11:00".into(),
            source: "10.0.0.5".into(),
            target: "gpt-4o-mini".into(),
            event_type: "rate_limit".into(),
            severity: "warning".into(),
            status: "blocked".into(),
            detail: "Preview risk event".into(),
        }],
        total: 1,
        page: 1,
        page_size: 20,
    }
}

fn sample_filter_config() -> FilterConfig {
    FilterConfig {
        content_filter_enabled: true,
        blacklist_enabled: true,
        custom_rules: vec!["preview-rule".into()],
    }
}
