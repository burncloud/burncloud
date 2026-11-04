use super::STYLE;

#[test]
fn chat_empty_logo_has_no_background_or_shadow() {
    let rule = STYLE
        .split_once(".chat-empty-logo {")
        .expect("空状态图标样式必须存在")
        .1
        .split_once('}')
        .expect("空状态图标样式必须完整")
        .0;

    assert!(rule.contains("background: transparent;"));
    assert!(rule.contains("box-shadow: none;"));
    assert!(rule.contains("width: 48px;"));
    assert!(rule.contains("height: 48px;"));
    assert!(rule.contains("margin-bottom: 16px;"));
}

#[test]
fn retired_picker_styles_are_removed_without_dropping_current_controls() {
    for class in [
        ".chat-catalog-detail",
        ".chat-model-context",
        ".chat-model-picker",
        ".chat-model-picker-back",
        ".chat-model-picker-heading",
        ".chat-model-settings",
        ".chat-model-settings-model",
        ".chat-model-settings-model-copy",
        ".chat-search-heading",
    ] {
        assert!(!STYLE.contains(class), "旧选择器样式不应残留：{class}");
    }
    assert!(STYLE.contains(".chat-reasoning-heading {"));
    assert!(STYLE.contains(".chat-catalog-heading {"));
    assert!(STYLE.contains(".model-chat-app {"));
}
