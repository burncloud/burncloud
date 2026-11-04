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
