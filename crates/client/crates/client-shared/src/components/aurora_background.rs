use dioxus::prelude::*;

/// Variant of Aurora background effect
#[derive(Clone, Copy, PartialEq, Default)]
pub enum AuroraVariant {
    /// Deep space theme - purple and blue tones (default)
    #[default]
    DeepSpace,
    /// Aurora theme - pink, purple, and cyan
    Aurora,
    /// Warm theme - orange and red tones
    Warm,
}

/// Aurora mesh gradient background component that creates depth and atmosphere.
/// 
/// This component renders an animated mesh gradient background with multiple
/// flowing gradient blobs, creating a "Deep Space" or "Aurora" visual effect.
/// 
/// # Properties
/// 
/// - `variant`: The color theme variant (DeepSpace, Aurora, Warm)
/// - `with_grid`: Whether to show a subtle dot grid overlay (default: true)
/// - `class`: Additional CSS classes to apply
/// 
/// # Example
/// 
/// ```rust
/// rsx! {
///     AuroraBackground {
///         variant: AuroraVariant::DeepSpace,
///         with_grid: true,
///     }
/// }
/// ```
#[component]
pub fn AuroraBackground(
    #[props(default)] variant: AuroraVariant,
    #[props(default = true)] with_grid: bool,
    #[props(default)] class: String,
) -> Element {
    // Define gradient colors based on variant
    let (primary_gradient, secondary_gradient, accent_gradient) = match variant {
        AuroraVariant::DeepSpace => (
            "from-[#FF2D55]/12 via-[#AF52DE]/10 to-[#007AFF]/12",
            "from-[#30B0C7]/15 via-[#5856D6]/12 to-transparent",
            "from-[#5AC8FA]/15 to-[#007AFF]/8",
        ),
        AuroraVariant::Aurora => (
            "from-[#007AFF]/15 via-[#5856D6]/12 to-[#AF52DE]/15",
            "from-[#5AC8FA]/20 via-[#30B0C7]/15 to-transparent",
            "from-[#AF52DE]/15 to-[#FF2D55]/10",
        ),
        AuroraVariant::Warm => (
            "from-[#FF9500]/15 via-[#FF2D55]/12 to-[#AF52DE]/10",
            "from-[#FF6B35]/15 via-[#FF9500]/12 to-transparent",
            "from-[#FF2D55]/15 to-[#FF9500]/10",
        ),
    };

    rsx! {
        div { class: "absolute inset-0 pointer-events-none overflow-hidden {class}",
            // Layer 1: Primary Aurora Blob - large morphing shape
            div {
                class: "absolute top-[-20%] left-[-10%] w-[800px] h-[800px] bg-gradient-to-r {primary_gradient} rounded-full blur-[100px] animate-aurora animate-morph"
            }

            // Layer 2: Secondary Flow - opposite corner
            div {
                class: "absolute bottom-[-15%] right-[-10%] w-[700px] h-[700px] bg-gradient-to-l {secondary_gradient} rounded-full blur-[80px] animate-aurora [animation-delay:7s] [animation-duration:25s]"
            }

            // Layer 3: Accent Orb - floating accent
            div {
                class: "absolute top-[20%] right-[20%] w-[300px] h-[300px] bg-gradient-to-br {accent_gradient} rounded-full blur-[60px] animate-float [animation-delay:2s]"
            }

            // Grid pattern overlay (optional)
            if with_grid {
                div {
                    class: "absolute inset-0 opacity-[0.02]",
                    style: "background-image: radial-gradient(circle at 1px 1px, #1D1D1F 1px, transparent 0); background-size: 40px 40px;"
                }
            }
        }
    }
}
