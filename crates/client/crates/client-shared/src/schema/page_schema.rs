/// Background variant for alternating section styles.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BgVariant {
    Dark,
    Light,
}

/// Identifies which of the six canonical page layouts this schema describes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PageType {
    /// Page 1 — Hero Landing Page.
    HeroLanding,
    /// Page 2 — Product Grid Page.
    ProductGrid,
    /// Page 3 — Product Detail Page.
    ProductDetail,
    /// Page 4 — Form / Auth Page.
    FormAuth,
    /// Page 5 — Feature/Content Page.
    FeatureContent,
    /// Page 6 — Error Page (404/500).
    Error,
}

impl PageType {
    /// Maximum number of CTAs allowed per region for this page type.
    /// Jobs review: Error = 1, all others = 2.
    pub fn cta_limit(self) -> u8 {
        match self {
            PageType::Error => 1,
            _ => 2,
        }
    }
}

/// Universal page structure schema.
///
/// Encodes the structural contract for each of the six canonical page types:
/// background sequence, section count limit, and CTA budget per region.
#[derive(Clone, PartialEq, Debug)]
pub struct PageSchema {
    /// The page type this schema governs.
    pub page_type: PageType,
    /// Sequence of background variants for each section, alternating as required.
    pub bg_sequence: Vec<BgVariant>,
    /// Maximum number of sections allowed on this page.
    pub max_sections: usize,
    /// Maximum number of CTAs allowed per interactive region.
    pub cta_limit: u8,
}

impl PageSchema {
    /// Returns the canonical schema for a given page type.
    pub fn for_type(page_type: PageType) -> Self {
        let (bg_sequence, max_sections) = match page_type {
            PageType::HeroLanding => (vec![BgVariant::Dark, BgVariant::Light, BgVariant::Dark], 6),
            PageType::ProductGrid => (vec![BgVariant::Light], 1),
            PageType::ProductDetail => {
                (vec![BgVariant::Dark, BgVariant::Light, BgVariant::Dark], 6)
            }
            PageType::FormAuth => (vec![BgVariant::Light], 1),
            PageType::FeatureContent => (
                vec![
                    BgVariant::Light,
                    BgVariant::Dark,
                    BgVariant::Light,
                    BgVariant::Dark,
                ],
                8,
            ),
            PageType::Error => (vec![BgVariant::Light], 1),
        };
        let cta_limit = page_type.cta_limit();
        Self {
            page_type,
            bg_sequence,
            max_sections,
            cta_limit,
        }
    }
}
