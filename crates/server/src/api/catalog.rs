use crate::api::response::{err, ok};
use crate::AppState;
use axum::{
    extract::State,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use burncloud_database_billing::BillingPriceModel;
use burncloud_service_channel::ChannelService;
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct CatalogModel {
    pub id: String,
    pub providers: Vec<String>,
    pub available_channels: usize,
    pub p95_latency_ms: Option<i32>,
    pub input_price_per_million: Option<f64>,
    pub output_price_per_million: Option<f64>,
    pub context_window: Option<i64>,
    pub max_output_tokens: Option<i64>,
    pub supports_vision: bool,
    pub supports_function_calling: bool,
    pub model_type: Option<String>,
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/api/models/catalog", get(model_catalog))
}

async fn model_catalog(State(state): State<AppState>) -> Response {
    let channels = match ChannelService::list(&state.db, 100, 0).await {
        Ok(channels) => channels,
        Err(error) => {
            tracing::error!(%error, "Failed to load the buyer model catalog");
            return err("Failed to load model catalog").into_response();
        }
    };
    let prices = match BillingPriceModel::list(&state.db, 2_000, 0, Some("USD"), None).await {
        Ok(prices) => prices,
        Err(error) => {
            tracing::error!(%error, "Failed to load model prices for the buyer catalog");
            return err("Failed to load model pricing").into_response();
        }
    };

    let mut price_by_model = HashMap::new();
    for price in prices {
        let is_universal = price.region.as_deref().unwrap_or_default().is_empty();
        match price_by_model.entry(price.model.clone()) {
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(price);
            }
            std::collections::hash_map::Entry::Occupied(mut entry) if is_universal => {
                entry.insert(price);
            }
            _ => {}
        }
    }

    let mut catalog = BTreeMap::<String, CatalogModel>::new();
    for channel in channels.into_iter().filter(|channel| channel.status == 1) {
        for model_id in channel
            .models
            .split(',')
            .map(str::trim)
            .filter(|model| !model.is_empty())
        {
            let item = catalog.entry(model_id.to_string()).or_insert_with(|| {
                let price = price_by_model.get(model_id);
                CatalogModel {
                    id: model_id.to_string(),
                    providers: Vec::new(),
                    available_channels: 0,
                    p95_latency_ms: None,
                    input_price_per_million: price
                        .map(|value| value.input_price as f64 / 1_000_000_000.0),
                    output_price_per_million: price
                        .map(|value| value.output_price as f64 / 1_000_000_000.0),
                    context_window: price.and_then(|value| value.context_window),
                    max_output_tokens: price.and_then(|value| value.max_output_tokens),
                    supports_vision: price
                        .and_then(|value| value.supports_vision)
                        .unwrap_or_default()
                        != 0,
                    supports_function_calling: price
                        .and_then(|value| value.supports_function_calling)
                        .unwrap_or_default()
                        != 0,
                    model_type: price.and_then(|value| value.model_type.clone()),
                }
            });
            item.available_channels += 1;
            if !item.providers.contains(&channel.name) {
                item.providers.push(channel.name.clone());
                item.providers.sort();
            }
            if let Some(latency) = channel.response_time.filter(|value| *value > 0) {
                item.p95_latency_ms = Some(
                    item.p95_latency_ms
                        .map_or(latency, |current| current.min(latency)),
                );
            }
        }
    }

    ok(catalog.into_values().collect::<Vec<_>>()).into_response()
}

#[cfg(test)]
mod tests {
    use super::CatalogModel;

    #[test]
    fn catalog_contract_contains_only_safe_model_metadata() {
        let value = serde_json::to_value(CatalogModel {
            id: "model-a".to_string(),
            providers: vec!["provider-a".to_string()],
            available_channels: 1,
            p95_latency_ms: Some(120),
            input_price_per_million: Some(0.1),
            output_price_per_million: Some(0.2),
            context_window: Some(128_000),
            max_output_tokens: Some(8_192),
            supports_vision: true,
            supports_function_calling: true,
            model_type: Some("chat".to_string()),
        })
        .expect("catalog serialization");

        let rendered = value.to_string();
        assert!(!rendered.contains("api_key"));
        assert!(!rendered.contains("base_url"));
        assert!(!rendered.contains("header_override"));
        assert!(!rendered.contains("param_override"));
    }
}
