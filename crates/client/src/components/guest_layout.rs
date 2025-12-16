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

            // Custom CSS with full animation system
            style {
                "
                :root {{
                    --font-sans: -apple-system, BlinkMacSystemFont, 'SF Pro Text', 'SF Pro Display', Inter, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif;
                }}
                html, body {{
                    font-family: var(--font-sans);
                    -webkit-font-smoothing: antialiased;
                    overflow: hidden;
                }}
                .app-drag-region {{ -webkit-app-region: drag; }}
                .app-no-drag {{ -webkit-app-region: no-drag; }}

                /* ========== AURORA - Ethereal flowing background ========== */
                @keyframes aurora {{
                    0%, 100% {{ transform: translateX(0) translateY(0) rotate(0deg) scale(1); opacity: 0.6; }}
                    25% {{ transform: translateX(50px) translateY(-30px) rotate(5deg) scale(1.1); opacity: 0.8; }}
                    50% {{ transform: translateX(-30px) translateY(50px) rotate(-5deg) scale(1.05); opacity: 0.7; }}
                    75% {{ transform: translateX(-50px) translateY(-20px) rotate(3deg) scale(0.95); opacity: 0.9; }}
                }}
                .animate-aurora {{
                    animation: aurora 20s ease-in-out infinite;
                }}

                /* ========== FLOAT - Gentle levitation ========== */
                @keyframes float {{
                    0%, 100% {{ transform: translateY(0px); }}
                    50% {{ transform: translateY(-12px); }}
                }}
                .animate-float {{
                    animation: float 6s ease-in-out infinite;
                }}

                /* ========== GLOW PULSE - Pulsating glow effect ========== */
                @keyframes glow-pulse {{
                    0%, 100% {{ box-shadow: 0 0 20px rgba(0, 113, 227, 0.3), 0 0 40px rgba(0, 113, 227, 0.1); }}
                    50% {{ box-shadow: 0 0 30px rgba(0, 113, 227, 0.5), 0 0 60px rgba(0, 113, 227, 0.2); }}
                }}
                .animate-glow-pulse {{
                    animation: glow-pulse 3s ease-in-out infinite;
                }}

                /* ========== SHIMMER - Light sweep effect ========== */
                @keyframes shimmer {{
                    0% {{ background-position: -200% 0; }}
                    100% {{ background-position: 200% 0; }}
                }}
                .animate-shimmer {{
                    background: linear-gradient(90deg, transparent, rgba(255,255,255,0.4), transparent);
                    background-size: 200% 100%;
                    animation: shimmer 2s linear infinite;
                }}

                /* ========== GRADIENT FLOW - Moving gradient background ========== */
                @keyframes gradient-flow {{
                    0% {{ background-position: 0% 50%; }}
                    50% {{ background-position: 100% 50%; }}
                    100% {{ background-position: 0% 50%; }}
                }}
                .animate-gradient-flow {{
                    background-size: 200% 200%;
                    animation: gradient-flow 8s ease infinite;
                }}

                /* ========== SCALE IN - Entrance animation ========== */
                @keyframes scale-in {{
                    0% {{ transform: scale(0.9); opacity: 0; }}
                    100% {{ transform: scale(1); opacity: 1; }}
                }}
                .animate-scale-in {{
                    animation: scale-in 0.6s cubic-bezier(0.16, 1, 0.3, 1) forwards;
                }}

                /* ========== SLIDE UP FADE - Staggered entrance ========== */
                @keyframes slide-up-fade {{
                    0% {{ transform: translateY(30px); opacity: 0; }}
                    100% {{ transform: translateY(0); opacity: 1; }}
                }}
                .animate-slide-up {{
                    animation: slide-up-fade 0.8s cubic-bezier(0.16, 1, 0.3, 1) forwards;
                }}

                /* ========== ANIMATE-IN - Combined fade-in, zoom-in, slide-up (Micro-Interaction Delight) ========== */
                @keyframes animate-in {{
                    0% {{ 
                        opacity: 0; 
                        transform: translateY(16px) scale(0.95); 
                    }}
                    100% {{ 
                        opacity: 1; 
                        transform: translateY(0) scale(1); 
                    }}
                }}
                .animate-in {{
                    animation: animate-in 0.5s cubic-bezier(0.16, 1, 0.3, 1) forwards;
                }}

                /* ========== SHAKE - Error feedback animation ========== */
                @keyframes shake {{
                    0%, 100% {{ transform: translateX(0); }}
                    10%, 30%, 50%, 70%, 90% {{ transform: translateX(-4px); }}
                    20%, 40%, 60%, 80% {{ transform: translateX(4px); }}
                }}
                .animate-shake {{
                    animation: shake 0.5s cubic-bezier(0.36, 0.07, 0.19, 0.97) both;
                }}
                .animate-delay-100 {{ animation-delay: 0.1s; opacity: 0; }}
                .animate-delay-200 {{ animation-delay: 0.2s; opacity: 0; }}
                .animate-delay-300 {{ animation-delay: 0.3s; opacity: 0; }}
                .animate-delay-400 {{ animation-delay: 0.4s; opacity: 0; }}
                .animate-delay-500 {{ animation-delay: 0.5s; opacity: 0; }}

                /* ========== TYPING CURSOR ========== */
                @keyframes blink {{
                    0%, 50% {{ opacity: 1; }}
                    51%, 100% {{ opacity: 0; }}
                }}
                .animate-blink {{
                    animation: blink 1s step-end infinite;
                }}

                /* ========== ORBIT - Rotating particles ========== */
                @keyframes orbit {{
                    0% {{ transform: rotate(0deg) translateX(100px) rotate(0deg); }}
                    100% {{ transform: rotate(360deg) translateX(100px) rotate(-360deg); }}
                }}
                .animate-orbit {{
                    animation: orbit 20s linear infinite;
                }}

                /* ========== MORPH - Shape morphing blob ========== */
                @keyframes morph {{
                    0%, 100% {{ border-radius: 60% 40% 30% 70% / 60% 30% 70% 40%; }}
                    25% {{ border-radius: 30% 60% 70% 40% / 50% 60% 30% 60%; }}
                    50% {{ border-radius: 50% 60% 30% 60% / 30% 60% 70% 40%; }}
                    75% {{ border-radius: 60% 40% 60% 30% / 70% 30% 50% 60%; }}
                }}
                .animate-morph {{
                    animation: morph 8s ease-in-out infinite;
                }}

                /* ========== COUNTER ANIMATION ========== */
                @keyframes count-up {{
                    from {{ opacity: 0; transform: translateY(10px); }}
                    to {{ opacity: 1; transform: translateY(0); }}
                }}
                .animate-count {{
                    animation: count-up 0.5s ease-out forwards;
                }}

                /* ========== RIPPLE EFFECT ========== */
                @keyframes ripple {{
                    0% {{ transform: scale(0.8); opacity: 1; }}
                    100% {{ transform: scale(2.5); opacity: 0; }}
                }}
                .animate-ripple {{
                    animation: ripple 2s ease-out infinite;
                }}

                /* ========== PULSE ========== */
                @keyframes pulse-soft {{
                    0%, 100% {{ opacity: 1; }}
                    50% {{ opacity: 0.5; }}
                }}
                .animate-pulse {{
                    animation: pulse-soft 2s ease-in-out infinite;
                }}

                /* ========== MAGNETIC HOVER EFFECT ========== */
                .magnetic-hover {{
                    transition: transform 0.3s cubic-bezier(0.16, 1, 0.3, 1);
                }}
                .magnetic-hover:hover {{
                    transform: scale(1.02) translateY(-2px);
                }}

                /* ========== GLASS MORPHISM ========== */
                .glass {{
                    background: rgba(255, 255, 255, 0.7);
                    backdrop-filter: blur(20px);
                    -webkit-backdrop-filter: blur(20px);
                    border: 1px solid rgba(255, 255, 255, 0.3);
                }}

                /* ========== SCROLLBAR ========== */
                ::-webkit-scrollbar {{
                    width: 10px;
                }}
                ::-webkit-scrollbar-track {{
                    background: transparent;
                }}
                ::-webkit-scrollbar-thumb {{
                    background: rgba(0, 0, 0, 0.1);
                    border-radius: 10px;
                }}
                ::-webkit-scrollbar-thumb:hover {{
                    background: rgba(0, 0, 0, 0.2);
                }}

                /* ========== MISSING TAILWIND JIT CLASSES ========== */
                .text-transparent {{
                    color: transparent;
                }}
                .bg-clip-text {{
                    -webkit-background-clip: text;
                    background-clip: text;
                }}
                .bg-gradient-to-r {{
                    background-image: linear-gradient(to right, var(--tw-gradient-stops));
                }}
                .bg-gradient-to-br {{
                    background-image: linear-gradient(to bottom right, var(--tw-gradient-stops));
                }}
                .bg-gradient-to-tr {{
                    background-image: linear-gradient(to top right, var(--tw-gradient-stops));
                }}
                .bg-gradient-to-l {{
                    background-image: linear-gradient(to left, var(--tw-gradient-stops));
                }}
                .from-\\[\\#007AFF\\] {{ --tw-gradient-from: #007AFF; --tw-gradient-stops: var(--tw-gradient-from), var(--tw-gradient-to, transparent); }}
                .via-\\[\\#5856D6\\] {{ --tw-gradient-stops: var(--tw-gradient-from), #5856D6, var(--tw-gradient-to, transparent); }}
                .to-\\[\\#AF52DE\\] {{ --tw-gradient-to: #AF52DE; }}
                .to-\\[\\#5856D6\\] {{ --tw-gradient-to: #5856D6; }}
                .from-\\[\\#0071E3\\] {{ --tw-gradient-from: #0071E3; --tw-gradient-stops: var(--tw-gradient-from), var(--tw-gradient-to, transparent); }}
                .from-\\[\\#FF2D55\\]\\/15 {{ --tw-gradient-from: rgba(255, 45, 85, 0.15); --tw-gradient-stops: var(--tw-gradient-from), var(--tw-gradient-to, transparent); }}
                .via-\\[\\#AF52DE\\]\\/12 {{ --tw-gradient-stops: var(--tw-gradient-from), rgba(175, 82, 222, 0.12), var(--tw-gradient-to, transparent); }}
                .to-\\[\\#007AFF\\]\\/15 {{ --tw-gradient-to: rgba(0, 122, 255, 0.15); }}
                .from-\\[\\#30B0C7\\]\\/20 {{ --tw-gradient-from: rgba(48, 176, 199, 0.2); --tw-gradient-stops: var(--tw-gradient-from), var(--tw-gradient-to, transparent); }}
                .via-\\[\\#5856D6\\]\\/15 {{ --tw-gradient-stops: var(--tw-gradient-from), rgba(88, 86, 214, 0.15), var(--tw-gradient-to, transparent); }}
                .from-\\[\\#5AC8FA\\]\\/20 {{ --tw-gradient-from: rgba(90, 200, 250, 0.2); --tw-gradient-stops: var(--tw-gradient-from), var(--tw-gradient-to, transparent); }}
                .to-\\[\\#007AFF\\]\\/10 {{ --tw-gradient-to: rgba(0, 122, 255, 0.1); }}
                .from-\\[\\#FF9500\\]\\/10 {{ --tw-gradient-from: rgba(255, 149, 0, 0.1); --tw-gradient-stops: var(--tw-gradient-from), var(--tw-gradient-to, transparent); }}
                .to-\\[\\#FF2D55\\]\\/10 {{ --tw-gradient-to: rgba(255, 45, 85, 0.1); }}
                .from-\\[\\#AF52DE\\]\\/10 {{ --tw-gradient-from: rgba(175, 82, 222, 0.1); --tw-gradient-stops: var(--tw-gradient-from), var(--tw-gradient-to, transparent); }}
                .to-\\[\\#5856D6\\]\\/10 {{ --tw-gradient-to: rgba(88, 86, 214, 0.1); }}
                .from-\\[\\#AF52DE\\]\\/15 {{ --tw-gradient-from: rgba(175, 82, 222, 0.15); --tw-gradient-stops: var(--tw-gradient-from), var(--tw-gradient-to, transparent); }}
                .to-\\[\\#5856D6\\]\\/15 {{ --tw-gradient-to: rgba(88, 86, 214, 0.15); }}
                .from-\\[\\#34C759\\]\\/10 {{ --tw-gradient-from: rgba(52, 199, 89, 0.1); --tw-gradient-stops: var(--tw-gradient-from), var(--tw-gradient-to, transparent); }}
                .to-\\[\\#30D158\\]\\/10 {{ --tw-gradient-to: rgba(48, 209, 88, 0.1); }}
                .from-\\[\\#34C759\\]\\/15 {{ --tw-gradient-from: rgba(52, 199, 89, 0.15); --tw-gradient-stops: var(--tw-gradient-from), var(--tw-gradient-to, transparent); }}
                .to-\\[\\#30D158\\]\\/15 {{ --tw-gradient-to: rgba(48, 209, 88, 0.15); }}
                .from-\\[\\#007AFF\\]\\/5 {{ --tw-gradient-from: rgba(0, 122, 255, 0.05); --tw-gradient-stops: var(--tw-gradient-from), var(--tw-gradient-to, transparent); }}
                .to-\\[\\#5856D6\\]\\/5 {{ --tw-gradient-to: rgba(88, 86, 214, 0.05); }}
                .from-\\[\\#D97757\\] {{ --tw-gradient-from: #D97757; --tw-gradient-stops: var(--tw-gradient-from), var(--tw-gradient-to, transparent); }}
                .to-\\[\\#C96442\\] {{ --tw-gradient-to: #C96442; }}
                .from-\\[\\#4285F4\\] {{ --tw-gradient-from: #4285F4; --tw-gradient-stops: var(--tw-gradient-from), var(--tw-gradient-to, transparent); }}
                .to-\\[\\#34A853\\] {{ --tw-gradient-to: #34A853; }}
                .from-\\[\\#0077ED\\] {{ --tw-gradient-from: #0077ED; --tw-gradient-stops: var(--tw-gradient-from), var(--tw-gradient-to, transparent); }}
                .to-\\[\\#6E6AE8\\] {{ --tw-gradient-to: #6E6AE8; }}
                .from-white\\/90 {{ --tw-gradient-from: rgba(255, 255, 255, 0.9); --tw-gradient-stops: var(--tw-gradient-from), var(--tw-gradient-to, transparent); }}
                .from-white\\/80 {{ --tw-gradient-from: rgba(255, 255, 255, 0.8); --tw-gradient-stops: var(--tw-gradient-from), var(--tw-gradient-to, transparent); }}
                .from-white {{ --tw-gradient-from: #fff; --tw-gradient-stops: var(--tw-gradient-from), var(--tw-gradient-to, transparent); }}
                .to-\\[\\#F8F9FA\\] {{ --tw-gradient-to: #F8F9FA; }}
                .to-transparent {{ --tw-gradient-to: transparent; }}
                .to-white {{ --tw-gradient-to: #fff; }}

                /* Animation delays with arbitrary values */
                .\\[animation-delay\\:2s\\] {{ animation-delay: 2s; }}
                .\\[animation-delay\\:4s\\] {{ animation-delay: 4s; }}
                .\\[animation-delay\\:5s\\] {{ animation-delay: 5s; }}
                .\\[animation-delay\\:7s\\] {{ animation-delay: 7s; }}
                .\\[animation-delay\\:10s\\] {{ animation-delay: 10s; }}
                .\\[animation-delay\\:0\\.1s\\] {{ animation-delay: 0.1s; }}
                .\\[animation-delay\\:0\\.2s\\] {{ animation-delay: 0.2s; }}
                .\\[animation-delay\\:0\\.3s\\] {{ animation-delay: 0.3s; }}
                .\\[animation-delay\\:0\\.6s\\] {{ animation-delay: 0.6s; }}
                .\\[animation-delay\\:0\\.9s\\] {{ animation-delay: 0.9s; }}
                .\\[animation-delay\\:1\\.2s\\] {{ animation-delay: 1.2s; }}
                .\\[animation-delay\\:1\\.5s\\] {{ animation-delay: 1.5s; }}
                .\\[animation-delay\\:1\\.8s\\] {{ animation-delay: 1.8s; }}
                .\\[animation-delay\\:2\\.1s\\] {{ animation-delay: 2.1s; }}
                .\\[animation-delay\\:2\\.4s\\] {{ animation-delay: 2.4s; }}
                .\\[animation-duration\\:15s\\] {{ animation-duration: 15s; }}
                .\\[animation-duration\\:25s\\] {{ animation-duration: 25s; }}

                /* Blur utilities */
                .blur-\\[2px\\] {{ filter: blur(2px); }}
                .blur-\\[1px\\] {{ filter: blur(1px); }}
                .blur-\\[50px\\] {{ filter: blur(50px); }}
                .blur-\\[60px\\] {{ filter: blur(60px); }}
                .blur-\\[80px\\] {{ filter: blur(80px); }}
                .blur-\\[100px\\] {{ filter: blur(100px); }}
                .blur-\\[120px\\] {{ filter: blur(120px); }}
                .blur-3xl {{ filter: blur(64px); }}

                /* Shadow utilities */
                .shadow-lg {{ box-shadow: 0 10px 15px -3px rgba(0, 0, 0, 0.1), 0 4px 6px -2px rgba(0, 0, 0, 0.05); }}
                .shadow-\\[0_4px_12px_rgba\\(0\\,0\\,0\\,0\\.1\\)\\] {{ box-shadow: 0 4px 12px rgba(0,0,0,0.1); }}
                .shadow-\\[0_4px_12px_rgba\\(0\\,0\\,0\\,0\\.15\\)\\] {{ box-shadow: 0 4px 12px rgba(0,0,0,0.15); }}
                .shadow-\\[0_8px_20px_rgba\\(0\\,0\\,0\\,0\\.2\\)\\] {{ box-shadow: 0 8px 20px rgba(0,0,0,0.2); }}
                .shadow-\\[0_8px_24px_rgba\\(0\\,0\\,0\\,0\\.25\\)\\] {{ box-shadow: 0 8px 24px rgba(0,0,0,0.25); }}
                .shadow-\\[0_10px_30px_-5px_rgba\\(0\\,113\\,227\\,0\\.4\\)\\] {{ box-shadow: 0 10px 30px -5px rgba(0,113,227,0.4); }}
                .shadow-\\[0_20px_40px_-5px_rgba\\(0\\,113\\,227\\,0\\.5\\)\\] {{ box-shadow: 0 20px 40px -5px rgba(0,113,227,0.5); }}
                .shadow-\\[0_20px_50px_-12px_rgba\\(0\\,0\\,0\\,0\\.08\\)\\] {{ box-shadow: 0 20px 50px -12px rgba(0,0,0,0.08); }}
                .shadow-\\[0_30px_60px_-12px_rgba\\(0\\,0\\,0\\,0\\.15\\)\\] {{ box-shadow: 0 30px 60px -12px rgba(0,0,0,0.15); }}
                .shadow-\\[0_4px_14px_-2px_rgba\\(29\\,29\\,31\\,0\\.25\\)\\,0_12px_32px_-4px_rgba\\(29\\,29\\,31\\,0\\.15\\)\\] {{ box-shadow: 0 4px 14px -2px rgba(29,29,31,0.25), 0 12px 32px -4px rgba(29,29,31,0.15); }}
                .hover\\:shadow-\\[0_8px_24px_-4px_rgba\\(29\\,29\\,31\\,0\\.35\\)\\,0_16px_40px_-8px_rgba\\(29\\,29\\,31\\,0\\.2\\)\\]:hover {{ box-shadow: 0 8px 24px -4px rgba(29,29,31,0.35), 0 16px 40px -8px rgba(29,29,31,0.2); }}
                .shadow-\\[0_8px_32px_-4px_rgba\\(0\\,0\\,0\\,0\\.08\\)\\,0_24px_56px_-8px_rgba\\(88\\,86\\,214\\,0\\.12\\)\\] {{ box-shadow: 0 8px 32px -4px rgba(0,0,0,0.08), 0 24px 56px -8px rgba(88,86,214,0.12); }}

                /* Hover glow effects */
                .hover\\:shadow-\\[0_0_20px_rgba\\(175\\,82\\,222\\,0\\.4\\)\\]:hover {{ box-shadow: 0 0 20px rgba(175,82,222,0.4); }}
                .hover\\:shadow-\\[0_0_20px_rgba\\(52\\,199\\,89\\,0\\.4\\)\\]:hover {{ box-shadow: 0 0 20px rgba(52,199,89,0.4); }}
                .group-hover\\:shadow-\\[0_0_20px_rgba\\(175\\,82\\,222\\,0\\.4\\)\\] {{ }}
                .group:hover .group-hover\\:shadow-\\[0_0_20px_rgba\\(175\\,82\\,222\\,0\\.4\\)\\] {{ box-shadow: 0 0 20px rgba(175,82,222,0.4); }}
                .group:hover .group-hover\\:shadow-\\[0_0_20px_rgba\\(52\\,199\\,89\\,0\\.4\\)\\] {{ box-shadow: 0 0 20px rgba(52,199,89,0.4); }}

                /* Tracking utilities */
                .tracking-\\[0\\.15em\\] {{ letter-spacing: 0.15em; }}
                .tracking-\\[0\\.2em\\] {{ letter-spacing: 0.2em; }}

                /* Rounded utilities */
                .rounded-\\[14px\\] {{ border-radius: 14px; }}
                .rounded-\\[24px\\] {{ border-radius: 24px; }}
                .rounded-\\[28px\\] {{ border-radius: 28px; }}
                .rounded-\\[32px\\] {{ border-radius: 32px; }}

                /* Custom colors */
                .bg-\\[\\#F5F5F7\\] {{ background-color: #F5F5F7; }}
                .bg-\\[\\#E8E8ED\\] {{ background-color: #E8E8ED; }}
                .bg-\\[\\#D2D2D7\\] {{ background-color: #D2D2D7; }}
                .bg-\\[\\#1D1D1F\\] {{ background-color: #1D1D1F; }}
                .bg-\\[\\#2C2C2E\\] {{ background-color: #2C2C2E; }}
                .hover\\:bg-\\[\\#2C2C2E\\]:hover {{ background-color: #2C2C2E; }}
                .bg-black\\/90 {{ background-color: rgba(0, 0, 0, 0.9); }}
                .text-\\[\\#1D1D1F\\] {{ color: #1D1D1F; }}
                .text-\\[\\#6E6E73\\] {{ color: #6E6E73; }}
                .text-\\[\\#86868B\\] {{ color: #86868B; }}
                .text-\\[\\#0071E3\\] {{ color: #0071E3; }}
                .text-\\[\\#AF52DE\\] {{ color: #AF52DE; }}
                .text-\\[\\#34C759\\] {{ color: #34C759; }}
                .text-\\[\\#FF6B35\\] {{ color: #FF6B35; }}
                .bg-\\[\\#007AFF\\]\\/40 {{ background-color: rgba(0, 122, 255, 0.4); }}
                .bg-\\[\\#AF52DE\\]\\/40 {{ background-color: rgba(175, 82, 222, 0.4); }}
                .bg-\\[\\#34C759\\]\\/40 {{ background-color: rgba(52, 199, 89, 0.4); }}
                .bg-\\[\\#34C759\\] {{ background-color: #34C759; }}

                .hover\\:bg-\\[\\#D2D2D7\\]:hover {{ background-color: #D2D2D7; }}
                .hover\\:bg-\\[\\#1D1D1F\\]:hover {{ background-color: #1D1D1F; }}
                .hover\\:text-\\[\\#0071E3\\]:hover {{ color: #0071E3; }}
                .hover\\:from-\\[\\#0077ED\\]:hover {{ --tw-gradient-from: #0077ED; }}
                .hover\\:to-\\[\\#6E6AE8\\]:hover {{ --tw-gradient-to: #6E6AE8; }}

                /* Text sizes */
                .text-\\[10px\\] {{ font-size: 10px; }}
                .text-\\[11px\\] {{ font-size: 11px; }}
                .text-\\[12px\\] {{ font-size: 12px; }}
                .text-\\[13px\\] {{ font-size: 13px; }}
                .text-\\[14px\\] {{ font-size: 14px; }}
                .text-\\[15px\\] {{ font-size: 15px; }}
                .text-\\[16px\\] {{ font-size: 16px; }}
                .text-\\[17px\\] {{ font-size: 17px; }}
                .text-\\[20px\\] {{ font-size: 20px; }}
                .text-\\[22px\\] {{ font-size: 22px; }}

                /* Selection color */
                .selection\\:bg-\\[\\#0071E3\\] ::selection {{ background-color: #0071E3; }}
                .selection\\:text-white ::selection {{ color: white; }}

                /* Opacity utilities */
                .opacity-\\[0\\.008\\] {{ opacity: 0.008; }}
                .opacity-\\[0\\.03\\] {{ opacity: 0.03; }}
                .text-\\[\\#86868B\\]\\/30 {{ color: rgba(134, 134, 139, 0.3); }}
                .text-\\[\\#86868B\\]\\/40 {{ color: rgba(134, 134, 139, 0.4); }}

                /* Scale transforms */
                .hover\\:scale-\\[1\\.02\\]:hover {{ transform: scale(1.02); }}
                .hover\\:scale-\\[1\\.015\\]:hover {{ transform: scale(1.015); }}
                .hover\\:scale-\\[1\\.05\\]:hover {{ transform: scale(1.05); }}
                .hover\\:scale-105:hover {{ transform: scale(1.05); }}
                .hover\\:scale-110:hover {{ transform: scale(1.10); }}
                .active\\:scale-\\[0\\.985\\]:active {{ transform: scale(0.985); }}
                .group:hover .group-hover\\:scale-110 {{ transform: scale(1.10); }}
                .group:hover .group-hover\\:opacity-100 {{ opacity: 1; }}
                .group:hover .group-hover\\:opacity-20 {{ opacity: 0.2; }}
                .group:hover .group-hover\\:opacity-25 {{ opacity: 0.25; }}
                .group:hover .group-hover\\:opacity-80 {{ opacity: 0.8; }}

                /* Hover rotate */
                .hover\\:rotate-6:hover {{ transform: rotate(6deg); }}
                .hover\\:-rotate-6:hover {{ transform: rotate(-6deg); }}
                .group-hover\\:rotate-6 {{ }}
                .group:hover .group-hover\\:rotate-6 {{ transform: rotate(6deg); }}

                /* Hover/Active translate */
                .hover\\:-translate-y-0\\.5:hover {{ transform: translateY(-0.125rem); }}
                .active\\:translate-y-0:active {{ transform: translateY(0); }}
                .disabled\\:hover\\:translate-y-0:disabled:hover {{ transform: translateY(0); }}
                .group:hover .group-hover\\:translate-x-1 {{ transform: translateX(0.25rem); }}
                .group:hover .group-hover\\:translate-x-2 {{ transform: translateX(0.5rem); }}
                "
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
