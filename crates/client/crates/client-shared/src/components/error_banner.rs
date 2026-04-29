use dioxus::prelude::*;

#[component]
pub fn ErrorBanner(message: String, on_retry: Option<EventHandler<()>>) -> Element {
    let mut retrying = use_signal(|| false);

    rsx! {
        div { class: "error-banner",
            span { "{message}" }
            if let Some(on_retry) = on_retry {
                button {
                    class: "retry-btn",
                    disabled: *retrying.read(),
                    onclick: move |_| {
                        if !*retrying.read() {
                            retrying.set(true);
                            on_retry.call(());
                            // Debounce: re-enable after 2s
                            spawn(async move {
                                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                                retrying.set(false);
                            });
                        }
                    },
                    if *retrying.read() { "重试中..." } else { "重试" }
                }
            }
        }
    }
}
