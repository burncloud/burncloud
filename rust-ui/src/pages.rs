use crate::app::AppState;
use crate::components::{ButtonLink, Icon, Status};
use crate::models::{
    ACTIVITY, BALANCE, IconKind, MODEL_USAGE, Metric, TODAY_SPEND, is_low_balance,
};
use leptos::prelude::*;
use leptos_router::hooks::use_location;

#[component]
pub fn OverviewPage() -> impl IntoView {
    let state = expect_context::<AppState>();

    view! {
        {move || {
            let text = state.locale.get().overview();
            let low_balance = is_low_balance(BALANCE);
            let spend = Metric { value: format!("${TODAY_SPEND:.2}"), unit: None, trend: Some("+8.4% vs yesterday"), positive: false };
            let balance = Metric { value: format!("${BALANCE:.2}"), unit: None, trend: None, positive: true };
            let availability = Metric { value: "99.99%".into(), unit: None, trend: None, positive: true };
            let tokens = Metric { value: "1.84M".into(), unit: Some("tokens"), trend: Some("85.4 req/min peak"), positive: true };

            view! {
                <div class="overview-stack">
                    <section class="page-header">
                        <div class="page-heading-copy">
                            <h1>{text.title}</h1>
                            <p>{text.subtitle}</p>
                        </div>
                        <div class="page-actions">
                            <ButtonLink href="/buyer/playground" label=text.open_playground icon=IconKind::Terminal />
                            <ButtonLink href="/buyer/marketplace" label=text.browse_marketplace icon=IconKind::Store primary=true />
                        </div>
                    </section>

                    <div class=if low_balance { "conclusion conclusion-warning" } else { "conclusion conclusion-healthy" } role="status">
                        <Icon kind=if low_balance { IconKind::AlertTriangle } else { IconKind::CheckCircle } size=16 />
                        <span>{if low_balance { text.conclusion_warning } else { text.conclusion_healthy }}</span>
                    </div>

                    <section class="metric-grid" aria-label="Overview metrics">
                        <MetricCard label=text.today_spend subtitle=text.today_spend_sub metric=spend />
                        <MetricCard label=text.balance subtitle=text.balance_sub metric=balance badge="HEALTHY" />
                        <MetricCard label=text.availability subtitle=text.availability_sub metric=availability status="Healthy" />
                        <MetricCard label=text.tokens subtitle=text.tokens_sub metric=tokens />
                    </section>

                    {low_balance.then(|| view! {
                        <section id="attention" class="attention" role="alert">
                            <div class="attention-copy">
                                <Icon kind=IconKind::AlertTriangle size=20 />
                                <div><h2>{text.attention_title}</h2><p>{text.attention_desc}</p></div>
                            </div>
                            <ButtonLink href="/buyer/billing" label=text.attention_top_up icon=IconKind::CreditCard primary=true />
                        </section>
                    })}

                    <section class="panel models-panel">
                        <div class="section-header">
                            <div><h2>{text.models_title}</h2><p>{text.models_desc}</p></div>
                            <a href="/buyer/marketplace" rel="external" class="ghost-link">
                                <span>{text.explore_models}</span><Icon kind=IconKind::ArrowRight size=14 />
                            </a>
                        </div>
                        <div class="table-scroll">
                            <table>
                                <thead><tr>
                                    <th>{text.col_model}</th><th>{text.col_tier}</th><th>{text.col_tokens}</th>
                                    <th>{text.col_latency}</th><th>{text.col_cost}</th><th>{text.col_status}</th>
                                    <th class="align-right">{text.col_action}</th>
                                </tr></thead>
                                <tbody>
                                    {MODEL_USAGE.into_iter().map(|item| view! {
                                        <tr>
                                            <td><div class="model-cell">
                                                <span class="model-mark">{item.name.chars().next().unwrap_or('M')}</span>
                                                <span><strong>{item.name}</strong><small>{item.family}</small></span>
                                            </div></td>
                                            <td><span class="tier">{item.tier}</span></td>
                                            <td class="mono strong">{item.tokens}</td>
                                            <td class="mono">{item.latency}</td>
                                            <td class="mono strong">{item.cost}</td>
                                            <td><Status /></td>
                                            <td class="align-right"><a href="/buyer/playground" rel="external" class="table-link">{text.test}</a></td>
                                        </tr>
                                    }).collect_view()}
                                </tbody>
                            </table>
                        </div>
                    </section>

                    <section class="panel activity-panel">
                        <div class="section-header">
                            <div><h2>{text.activity_title}</h2><p>{text.activity_desc}</p></div>
                            <a href="/buyer/logs" rel="external" class="ghost-link">
                                <span>{text.view_logs}</span><Icon kind=IconKind::ArrowRight size=14 />
                            </a>
                        </div>
                        <div class="activity-list">
                            {ACTIVITY.into_iter().map(|event| view! {
                                <article class="activity-item">
                                    <span class="activity-icon"><Icon kind=event.kind size=14 /></span>
                                    <div class="activity-copy"><strong>{event.title}</strong><p>{event.description}</p></div>
                                    <time>{event.time}</time>
                                </article>
                            }).collect_view()}
                        </div>
                    </section>
                </div>
            }
        }}
    }
}

#[component]
fn MetricCard(
    label: &'static str,
    subtitle: &'static str,
    metric: Metric,
    #[prop(optional)] badge: Option<&'static str>,
    #[prop(optional)] status: Option<&'static str>,
) -> impl IntoView {
    view! {
        <article class="metric-card">
            <div class="metric-label-row">
                <span class="metric-label">{label}</span>
                {badge.map(|label| view! { <span class="badge-success">{label}</span> })}
                {status.map(|label| view! { <Status label=label /> })}
            </div>
            <div class="metric-value-row"><strong>{metric.value}</strong>{metric.unit.map(|unit| view! { <span>{unit}</span> })}</div>
            <div class="metric-meta">
                {metric.trend.map(|trend| {
                    let class = if metric.positive { "trend trend-positive" } else { "trend" };
                    view! { <span class=class>{trend}</span> }
                })}
                <span class="metric-subtitle">{subtitle}</span>
            </div>
        </article>
    }
}

#[component]
pub fn PlaceholderPage() -> impl IntoView {
    let state = expect_context::<AppState>();
    let location = use_location();

    view! {
        {move || {
            let common = state.locale.get().common();
            let role = state.role.get();
            let path = location.pathname.get();
            view! {
                <section class="placeholder-panel">
                    <span class="placeholder-icon"><Icon kind=IconKind::Layers size=22 /></span>
                    <p class="eyebrow">{path}</p>
                    <h1>{common.coming_soon}</h1>
                    <p>{common.coming_soon_desc}</p>
                    <a href=role.overview_path() rel="external" class="button button-primary">
                        <Icon kind=IconKind::ArrowRight size=14 class="icon-reverse" />
                        <span>{common.back_overview}</span>
                    </a>
                </section>
            }
        }}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overview_mock_values_match_source_page() {
        assert_eq!(crate::models::TODAY_SPEND, 14.28);
        assert_eq!(BALANCE, 128.50);
        assert_eq!(MODEL_USAGE.len(), 3);
        assert_eq!(ACTIVITY.len(), 3);
    }

    #[test]
    fn placeholders_return_to_current_role() {
        assert_eq!(
            crate::models::Role::Supplier.overview_path(),
            "/supplier/overview"
        );
        assert_eq!(
            crate::models::Role::Admin.overview_path(),
            "/admin/overview"
        );
    }
}
