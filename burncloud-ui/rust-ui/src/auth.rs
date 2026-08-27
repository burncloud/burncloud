use axum::http::HeaderMap;

const SESSION_COOKIE: &str = "burncloud_ui_session";

pub fn token_from_headers(headers: &HeaderMap) -> Option<String> {
    headers
        .get(axum::http::header::COOKIE)
        .and_then(|value| value.to_str().ok())
        .and_then(|cookies| {
            cookies.split(';').find_map(|cookie| {
                let (name, value) = cookie.trim().split_once('=')?;
                (name == SESSION_COOKIE && !value.is_empty()).then(|| value.to_string())
            })
        })
}

pub fn session_cookie(token: &str) -> String {
    let secure = if std::env::var("BURNCLOUD_UI_SECURE_COOKIE").as_deref() == Ok("true") {
        "; Secure"
    } else {
        ""
    };
    format!("{SESSION_COOKIE}={token}; Path=/; HttpOnly; SameSite=Lax; Max-Age=28800{secure}")
}

pub fn expired_session_cookie() -> String {
    format!("{SESSION_COOKIE}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0")
}

pub fn safe_next(value: Option<&str>) -> &str {
    value
        .filter(|next| next.starts_with('/') && !next.starts_with("//") && !next.starts_with("/\\"))
        .unwrap_or("/buyer/overview")
}

pub fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

pub fn render_language_switcher(class_name: &str) -> String {
    format!(
        r#"<div class="language-switcher {class_name}" data-language-switcher><button class="language-trigger" type="button" data-language-trigger aria-label="选择语言" aria-haspopup="true" aria-expanded="false"><span class="language-flag" data-language-current-flag>CN</span><span class="language-current-name" data-language-current-name>简体中文</span><span class="language-current-short" data-language-current-short>中</span><svg viewBox="0 0 24 24" aria-hidden="true"><path d="m6 9 6 6 6-6"/></svg></button><div class="language-panel" data-language-panel role="radiogroup" aria-label="选择语言" hidden><button type="button" class="language-option" role="radio" data-language-option="en" aria-checked="false"><span>US</span><strong>English</strong><small>EN</small></button><button type="button" class="language-option selected" role="radio" data-language-option="zh" aria-checked="true"><span>CN</span><strong>简体中文</strong><small>中</small></button><button type="button" class="language-option" role="radio" data-language-option="zh-TW" aria-checked="false"><span>HK</span><strong>繁體中文</strong><small>繁</small></button><button type="button" class="language-option" role="radio" data-language-option="ja" aria-checked="false"><span>JP</span><strong>日本語</strong><small>日</small></button></div></div>"#
    )
}

pub fn render_login(next: &str, show_error: bool, backend_error: bool) -> String {
    let error = if backend_error {
        r#"<div class="login-error" role="alert">无法连接 BurnCloud 服务，请确认后端已启动后重试。</div>"#
    } else if show_error {
        r#"<div class="login-error" role="alert">用户名或密码不正确，请重新输入。</div>"#
    } else {
        ""
    };
    format!(
        r#"<!doctype html><html lang="zh-CN"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><meta name="description" content="登录 BurnCloud Buyer 工作区"><title>登录 - BurnCloud</title><link rel="stylesheet" href="/assets/styles.css"><script src="/assets/i18n.js" defer></script></head><body class="auth-body">{}
        <main class="login-shell"><section class="login-brand"><div class="login-brand-mark">BC</div><p class="eyebrow">BURN CLOUD CONTROL PLANE</p><h1>连接模型、用量与账单的统一工作区</h1><p>登录后将从 BurnCloud 数据库读取账户、模型目录、账单和 API 密钥状态。</p><ul><li>管理面 JWT 与推理 API 密钥严格隔离</li><li>真实路由操练场不会向浏览器暴露密钥</li><li>角色权限每次从后端账户记录确认</li></ul></section>
        <section class="login-panel"><form method="post" action="/session/login" class="login-form"><div><p class="eyebrow">SECURE SIGN IN</p><h2>登录 BurnCloud</h2><p>使用 BurnCloud 管理账户继续。</p></div>{error}<input type="hidden" name="next" value="{}"><label>用户名<input name="username" autocomplete="username" required autofocus></label><label>密码<input name="password" type="password" autocomplete="current-password" required></label><button class="button primary login-submit" type="submit">登录工作区</button><p class="login-footnote">会话保存在 HttpOnly Cookie 中，浏览器脚本无法读取。</p></form></section></main></body></html>"#,
        render_language_switcher("auth-language-switcher"),
        escape_html(safe_next(Some(next)))
    )
}

#[cfg(test)]
mod tests {
    use super::{escape_html, render_login, safe_next};

    #[test]
    fn rejects_external_login_redirects() {
        assert_eq!(safe_next(Some("//example.com")), "/buyer/overview");
        assert_eq!(safe_next(Some("/\\example.com")), "/buyer/overview");
        assert_eq!(safe_next(Some("https://example.com")), "/buyer/overview");
        assert_eq!(safe_next(Some("/buyer/playground")), "/buyer/playground");
    }

    #[test]
    fn escapes_database_backed_text() {
        assert_eq!(
            escape_html("<script>'x'</script>"),
            "&lt;script&gt;&#39;x&#39;&lt;/script&gt;"
        );
    }

    #[test]
    fn login_includes_four_language_switcher() {
        let page = render_login("/buyer/overview", false, false);
        assert!(page.contains("/assets/i18n.js"));
        for language in ["en", "zh", "zh-TW", "ja"] {
            assert!(page.contains(&format!("data-language-option=\"{language}\"")));
        }
    }
}
