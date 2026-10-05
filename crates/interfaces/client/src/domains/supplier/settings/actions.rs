//! Browser-side actions for the supplier/settings page.

use super::model::SavedSupplierSettings;

pub const STORAGE_KEY: &str = "burncloud_supplier_settings";

fn run_script(script: String) {
    dioxus::prelude::spawn(async move {
        let _ = dioxus::document::eval(&script).await;
    });
}

pub async fn load_saved_settings() -> Option<SavedSupplierSettings> {
    let script = format!(
        "const storage = typeof localStorage === 'undefined' ? null : localStorage; \
         return storage?.getItem({STORAGE_KEY:?}) || \"\";"
    );
    let serialized = dioxus::document::eval(&script)
        .join::<String>()
        .await
        .ok()?;

    if serialized.is_empty() {
        return None;
    }

    serde_json::from_str(&serialized).ok()
}

pub fn persist_settings(alert_email: &str, webhook_url: &str) {
    let Ok(serialized) = serde_json::to_string(&serde_json::json!({
        "alert_email": alert_email,
        "webhook_url": webhook_url,
    })) else {
        return;
    };

    let script = format!(
        "const storage = typeof localStorage === 'undefined' ? null : localStorage; \
         storage?.setItem({STORAGE_KEY:?}, {serialized:?});"
    );
    run_script(script);
}

pub fn copy_to_clipboard(value: &str) {
    let Ok(serialized) = serde_json::to_string(value) else {
        return;
    };

    run_script(format!("navigator.clipboard?.writeText({serialized});"));
}
