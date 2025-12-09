use dioxus::prelude::*;

use crate::app::Route;
use burncloud_client_shared::components::{Sidebar, TitleBar};

#[component]
pub fn Layout() -> Element {
    rsx! {
        head {
            link { href: "https://npm.elemecdn.com/daisyui@4.12.10/dist/full.min.css", rel: "stylesheet", r#type: "text/css" }
            script { src: "https://cdn.tailwindcss.com" }
            // Inject Custom CSS for Apple-like Aesthetics
            style {
                "
                :root {{
                    --font-sans: -apple-system, BlinkMacSystemFont, 'SF Pro Text', 'Segoe UI', Roboto, Helvetica, Arial, sans-serif;
                }}
                body {{
                    font-family: var(--font-sans);
                    -webkit-font-smoothing: antialiased;
                    overflow: hidden; /* Prevent body scroll, handle in containers */
                }}
                /* Fallback for SVGs if Tailwind fails to load */
                svg {{
                    width: 1.5rem;
                    height: 1.5rem;
                    max-width: 100%;
                }}
                .app-drag-region {{
                    -webkit-app-region: drag;
                }}
                .app-no-drag {{
                    -webkit-app-region: no-drag;
                }}
                /* Custom Scrollbar to look more native/hidden */
                ::-webkit-scrollbar {{
                    width: 8px;
                    height: 8px;
                }}
                ::-webkit-scrollbar-track {{
                    background: transparent;
                }}
                ::-webkit-scrollbar-thumb {{
                    background: rgba(0, 0, 0, 0.1);
                    border-radius: 4px;
                }}
                ::-webkit-scrollbar-thumb:hover {{
                    background: rgba(0, 0, 0, 0.2);
                }}
                "
            }
        }

        // Main App Container - macOS Split View Style
        div { class: "flex h-screen w-screen bg-base-100 text-base-content overflow-hidden select-none relative", "data-theme": "light",

            // Floating TitleBar (Z-Index 50)
            div { class: "absolute top-0 left-0 w-full z-50 pointer-events-none",
                // TitleBar needs pointer-events-auto for buttons to work
                div { class: "pointer-events-auto",
                    TitleBar {}
                }
            }

            // Sidebar Panel
            div { class: "w-64 flex-shrink-0 flex flex-col border-r border-base-300/50 bg-base-200/50 backdrop-blur-xl pt-8", // pt-8 for TitleBar space
                div { class: "flex-1 overflow-y-auto px-2 py-4",
                    Sidebar {}
                }
            }

            // Main Content Panel
            div { class: "flex-1 flex flex-col bg-base-100 relative min-w-0 pt-8", // pt-8 for TitleBar space
                // Scrollable Content Area
                main { class: "flex-1 overflow-y-auto overflow-x-hidden p-6 md:p-10",
                    Outlet::<Route> {}
                }
            }
        }
    }
}
