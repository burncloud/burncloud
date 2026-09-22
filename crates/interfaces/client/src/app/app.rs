use super::router::routes::Route;
use crate::{i18n::Locale, shared::types::Role};
use dioxus::prelude::*;

#[component]
fn DesktopChrome() -> Element {
    #[cfg(all(feature = "desktop", target_os = "windows"))]
    {
        return rsx! { crate::desktop_chrome::DesktopTitleBar {} };
    }

    #[cfg(not(all(feature = "desktop", target_os = "windows")))]
    {
        rsx! {}
    }
}

/// Root component shared by LiveView and desktop targets.
#[component]
pub fn App() -> Element {
    let locale = use_context_provider(|| Signal::new(Locale::default()));
    use_context_provider(|| Signal::new(Role::Buyer));
    use_context_provider(|| Signal::new(String::new()));
    use_effect(move || {
        let mut locale = locale;
        spawn(async move {
            let script = "const saved = localStorage.getItem('burncloud_selected_language'); const browser = (navigator.language || 'zh-CN').toLowerCase(); const value = saved || (browser.includes('zh-tw') || browser.includes('zh-hk') || browser.includes('zh-hant') ? 'zh-TW' : browser.startsWith('zh') ? 'zh' : browser.startsWith('ja') ? 'ja' : 'zh'); document.documentElement.lang = value === 'zh' ? 'zh-CN' : value; return value;";
            if let Ok(value) = dioxus::document::eval(script).join::<String>().await {
                locale.set(Locale::from_code(&value));
            }
        });
    });
    rsx! {
        DesktopChrome {}
        Router::<Route> {}
    }
}
