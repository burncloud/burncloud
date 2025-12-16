use dioxus::prelude::*;

/// Variant of Glass Card styling
#[derive(Clone, Copy, PartialEq, Default)]
pub enum GlassVariant {
    /// Light variant - white background with low opacity (default)
    #[default]
    Light,
    /// Dark variant - black/dark background with low opacity
    Dark,
}

/// Glass Card container component with glassmorphism effects.
///
/// This component creates a "frosted glass" container effect with:
/// - Backdrop blur effect (`backdrop-blur-md`)
/// - Low opacity background (white or black)
/// - Subtle border (`border-white/10`)
/// - Rounded corners and shadow
///
/// # Properties
///
/// - `variant`: Light (white) or Dark (black) glass effect
/// - `class`: Additional CSS classes to apply
/// - `blur_intensity`: Blur intensity - "sm", "md" (default), "lg", "xl"
/// - `children`: Child elements to render inside the card
///
/// # Example
///
/// ```rust
/// rsx! {
///     GlassCard {
///         variant: GlassVariant::Light,
///         class: "p-10 max-w-md",
///         
///         h1 { "Welcome" }
///         p { "This is inside a glass card" }
///     }
/// }
/// ```
#[component]
pub fn GlassCard(
    #[props(default)] variant: GlassVariant,
    #[props(default)] class: String,
    #[props(default = "md".to_string())] blur_intensity: String,
    children: Element,
) -> Element {
    // Define styles based on variant
    let bg_class = match variant {
        GlassVariant::Light => "bg-white/70",
        GlassVariant::Dark => "bg-black/70",
    };

    let border_class = match variant {
        GlassVariant::Light => "border-white/10",
        GlassVariant::Dark => "border-white/5",
    };

    // Map blur intensity to Tailwind classes
    let blur_class = match blur_intensity.as_str() {
        "sm" => "backdrop-blur-sm",
        "md" => "backdrop-blur-md",
        "lg" => "backdrop-blur-lg",
        "xl" => "backdrop-blur-xl",
        _ => "backdrop-blur-md",
    };

    rsx! {
        div {
            class: "{blur_class} {bg_class} border {border_class} rounded-[32px] shadow-[0_30px_60px_-12px_rgba(0,0,0,0.12)] relative overflow-hidden {class}",

            // Glossy reflection effect (top highlight)
            div {
                class: "absolute top-0 right-0 w-48 h-48 bg-gradient-to-br from-white/40 to-transparent opacity-60 pointer-events-none rounded-full blur-2xl -translate-y-1/2 translate-x-1/2"
            }

            // Content wrapper with relative z-index to appear above the glossy effect
            div { class: "relative z-10",
                {children}
            }
        }
    }
}

/// Zen Mode container that ensures absolute centering of its contents.
///
/// This component provides a full-screen container with perfect vertical
/// and horizontal centering for its children, creating a focused "Zen Mode" layout.
///
/// # Properties
///
/// - `class`: Additional CSS classes to apply
/// - `children`: Child elements to center
///
/// # Example
///
/// ```rust
/// rsx! {
///     ZenContainer {
///         GlassCard {
///             // Form content here
///         }
///     }
/// }
/// ```
#[component]
pub fn ZenContainer(#[props(default)] class: String, children: Element) -> Element {
    rsx! {
        div {
            class: "h-full w-full min-h-screen flex items-center justify-center {class}",
            {children}
        }
    }
}
