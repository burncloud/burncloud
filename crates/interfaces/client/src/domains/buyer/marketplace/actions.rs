//! Interaction helpers for the buyer marketplace.

use super::model::ModelStatus;

pub const fn status_tone(status: ModelStatus) -> &'static str {
    match status {
        ModelStatus::Healthy => "healthy",
        ModelStatus::Degraded => "warning",
    }
}
