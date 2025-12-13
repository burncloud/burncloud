use dioxus::prelude::*;
use dioxus_router::components::Outlet;

use crate::app::Route;
use burncloud_client_shared::components::TitleBar;

#[component]
pub fn GuestLayout() -> Element {
    rsx! {
        head {
            // Embed Tailwind v2 and DaisyUI v4 CSS locally
            style { "{include_str!(\"../assets/tailwind.css\")}" }
            style { "{include_str!(\"../assets/daisyui.css\")}" }

            // Custom CSS
            style {
                "\n                :root {{\n                    --font-sans: -apple-system, BlinkMacSystemFont, 'SF Pro Text', 'Segoe UI', Roboto, Helvetica, Arial, sans-serif;\n                }}\n                html, body {{\n                    font-family: var(--font-sans);\n                    -webkit-font-smoothing: antialiased;\n                    overflow: hidden; /* Prevent global scrollbar */\n                }}\n                .app-drag-region {{ -webkit-app-region: drag; }}\n                .app-no-drag {{ -webkit-app-region: no-drag; }}\n                \n                @keyframes fade-in {{\n                    from {{ opacity: 0; transform: translateY(10px); }}\n                    to {{ opacity: 1; transform: translateY(0); }}\n                }}\n                .animate-fade-in {{\n                    animation: fade-in 0.6s ease-out forwards;\n                }}\n                "
            }
        }

        div { class: "h-screen w-screen overflow-hidden flex flex-col bg-base-100 text-base-content relative", "data-theme": "light",
            // Floating TitleBar (Draggable)
            div { class: "absolute top-0 left-0 w-full z-50",
                TitleBar {}
            }

            // Main Content (Centered)
            main { class: "flex-1 flex flex-col relative z-0 overflow-y-auto",
                Outlet::<Route> {}
            }
        }
    }
}
