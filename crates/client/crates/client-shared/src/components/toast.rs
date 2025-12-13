use dioxus::prelude::*;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq)]
pub enum ToastType {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Toast {
    pub id: String,
    pub message: String,
    pub type_: ToastType,
    pub timestamp: u64,
    pub duration: u64, // milliseconds
}

// Global signal for Toasts
// In Dioxus 0.5+, we can use a Signal in a Context or just a global/static if appropriate,
// but Context is safer for multiple windows/instances.
// We'll use a Context approach.

#[derive(Clone, Copy)]
pub struct ToastManager(Signal<Vec<Toast>>);

impl ToastManager {
    pub fn new() -> Self {
        Self(Signal::new(Vec::new()))
    }

    pub fn add(mut self, message: String, type_: ToastType) {
        let id = Uuid::new_v4().to_string();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let toast = Toast {
            id: id.clone(),
            message,
            type_,
            timestamp: now,
            duration: 3000, // Default 3s
        };

        self.0.write().push(toast);
    }

    pub fn remove(mut self, id: &str) {
        self.0.write().retain(|t| t.id != id);
    }

    pub fn success(self, msg: &str) {
        self.add(msg.to_string(), ToastType::Success);
    }
    pub fn error(self, msg: &str) {
        self.add(msg.to_string(), ToastType::Error);
    }
    pub fn info(self, msg: &str) {
        self.add(msg.to_string(), ToastType::Info);
    }
    pub fn warning(self, msg: &str) {
        self.add(msg.to_string(), ToastType::Warning);
    }
}

impl Default for ToastManager {
    fn default() -> Self {
        Self::new()
    }
}

pub fn use_toast() -> ToastManager {
    use_context::<ToastManager>()
}

pub fn use_init_toast() -> ToastManager {
    use_context_provider(ToastManager::new)
}

#[component]
pub fn ToastContainer() -> Element {
    let toast_manager = use_toast();
    let mut toasts = toast_manager.0;

    // Cleanup timer
    use_effect(move || {
        let interval = tokio::time::interval(Duration::from_millis(500));
        spawn(async move {
            let mut timer = interval;
            loop {
                timer.tick().await;
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64;

                // We need to check if we need to write to avoid unnecessary locks/updates
                // Using a read lock first
                let should_cleanup = toasts.read().iter().any(|t| now > t.timestamp + t.duration);

                if should_cleanup {
                    toasts.write().retain(|t| now <= t.timestamp + t.duration);
                }
            }
        });
    });

    let current_toasts = toasts.read();

    rsx! {
        div { class: "toast-container",
            {current_toasts.iter().map(|toast| {
                let id = toast.id.clone();
                rsx! {
                    div {
                        key: "{toast.id}",
                        class: "toast toast-{toast.type_.to_string().to_lowercase()}",
                        onclick: move |_| toast_manager.remove(&id),
                        div { class: "toast-icon",
                            match toast.type_ {
                                ToastType::Success => "✓",
                                ToastType::Error => "✕",
                                ToastType::Warning => "⚠",
                                ToastType::Info => "ℹ",
                            }
                        }
                        div { class: "toast-message", "{toast.message}" }
                    }
                }
            })}
        }
    }
}

impl std::fmt::Display for ToastType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ToastType::Info => write!(f, "Info"),
            ToastType::Success => write!(f, "Success"),
            ToastType::Warning => write!(f, "Warning"),
            ToastType::Error => write!(f, "Error"),
        }
    }
}
