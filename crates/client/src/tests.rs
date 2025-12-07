#[cfg(test)]
mod tests {
    use dioxus::prelude::*;
    use crate::app::App;

    #[test]
    fn test_app_renders() {
        // Use dioxus_ssr::render_element to render the App component.
        // This handles VirtualDom creation and runtime setup.
        
        let html = dioxus_ssr::render_element(rsx! {
            App {}
        });
        
        // Basic assertion: contains common layout elements or initial route content
        // Default route is Dashboard
        assert!(html.contains("div"));
    }
}
