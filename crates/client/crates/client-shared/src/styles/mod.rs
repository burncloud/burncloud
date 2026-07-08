//! BurnCloud design system CSS — assembled from `styles/*.css`.
//!
//! - **Tokens & component skin**: edit fragments here (not scattered in pages).
//! - **Layout**: prefer Tailwind utilities in RSX (`flex`, `gap-bc-4`, `text-bc-text`) — see `docs/ui/naming.md`
//! - **Rules**: `docs/ui/system.md` · **Maintenance**: `docs/ui/README.md`

pub const DESIGN_SYSTEM_CSS: &str = concat!(
    include_str!("00_burncloud_design_system_apple_inspired.css"),
    include_str!("01_base_element_styles.css"),
    include_str!("02_acrylic_glass_effects.css"),
    include_str!("03_card_styles.css"),
    include_str!("04_button_styles.css"),
    include_str!("05_input_styles.css"),
    include_str!("06_progress_bar.css"),
    include_str!("07_status_indicators.css"),
    include_str!("08_animations.css"),
    include_str!("09_layout_helpers_burncloud_specific_not_tailwind_d.css"),
    include_str!("10_typography.css"),
    include_str!("11_app_layout.css"),
    include_str!("12_navigation.css"),
    include_str!("13_model_cards_metrics.css"),
    include_str!("14_log_viewer.css"),
    include_str!("15_macos_style_scrollbars_hidden_by_default_show_on.css"),
    include_str!("16_macos_window_control_colors.css"),
    include_str!("17_scrollbar_theme.css"),
    include_str!("18_login_register_page_50_50_split_screen.css"),
    include_str!("19_component_utility_classes_migrated_from_inline_s.css"),
    include_str!("20_landing_marketing_page.css"),
    include_str!("21_landing_page_semantic_component_classes_home_rs.css"),
    include_str!("22_legacy_daisyui_class_aliases_migrate_to_bc_over.css"),
    include_str!("23_page_level_helpers_from_design_kit_styles_css.css"),
    include_str!("24_semantic_component_classes_register_users_api_mi.css"),
    include_str!("25_token_compliant_utility_classes_issue_179_migrat.css"),
);
