use once_cell::sync::Lazy;
use regex::Regex;

#[allow(clippy::expect_used)]
pub static USERNAME_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[a-zA-Z0-9_]+$")
        .expect("用户名正则语法错误，请修复正则表达式")
});

#[allow(clippy::expect_used)]
pub static EMAIL_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9]([a-zA-Z0-9.-]*[a-zA-Z0-9])?\.[a-zA-Z]{2,}$")
        .expect("邮箱正则语法错误，请修复正则表达式")
});

/// 用户名验证：3-50 个字符，只能包含字母、数字、下划线
pub fn validate_username(username: &str) -> Result<(), String> {
    if username.is_empty() {
        return Err("用户名不能为空".to_string());
    }
    if username.len() < 3 {
        return Err("用户名长度不能少于 3 个字符".to_string());
    }
    if username.len() > 50 {
        return Err("用户名长度不能超过 50 个字符".to_string());
    }
    if !USERNAME_REGEX.is_match(username) {
        return Err("用户名只能包含字母、数字和下划线".to_string());
    }
    Ok(())
}

/// 密码验证：6-128 个字符
pub fn validate_password(password: &str) -> Result<(), String> {
    if password.is_empty() {
        return Err("密码不能为空".to_string());
    }
    if password.len() < 6 {
        return Err("密码长度不能少于 6 个字符".to_string());
    }
    if password.len() > 128 {
        return Err("密码长度不能超过 128 个字符".to_string());
    }
    Ok(())
}

/// 邮箱验证：基本格式检查
pub fn validate_email(email: &str) -> Result<(), String> {
    if email.is_empty() {
        return Err("邮箱不能为空".to_string());
    }
    if !EMAIL_REGEX.is_match(email) {
        return Err("无效的邮箱格式".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_username() {
        // 合法用户名
        assert!(validate_username("user123").is_ok());
        assert!(validate_username("user_name").is_ok());
        assert!(validate_username("User123").is_ok());
        assert!(validate_username("_user").is_ok());
        assert!(validate_username("user_").is_ok());
        assert!(validate_username(&"a".repeat(3)).is_ok()); // 最小长度
        assert!(validate_username(&"a".repeat(50)).is_ok()); // 最大长度

        // 非法用户名
        assert!(validate_username("ab").is_err()); // 太短
        assert!(validate_username(&"a".repeat(51)).is_err()); // 太长
        assert!(validate_username("user@name").is_err()); // 包含特殊字符 @
        assert!(validate_username("user name").is_err()); // 包含空格
        assert!(validate_username("user#name").is_err()); // 包含特殊字符 #
        assert!(validate_username("").is_err()); // 空
        assert!(validate_username("user-name").is_err()); // 包含连字符
        assert!(validate_username("user.name").is_err()); // 包含点
    }

    #[test]
    fn test_validate_password() {
        // 合法密码
        assert!(validate_password("password123").is_ok());
        assert!(validate_password(&"a".repeat(6)).is_ok()); // 最小长度
        assert!(validate_password(&"a".repeat(128)).is_ok()); // 最大长度

        // 非法密码
        assert!(validate_password("abc").is_err()); // 太短
        assert!(validate_password(&"a".repeat(129)).is_err()); // 太长
        assert!(validate_password("").is_err()); // 空
        assert!(validate_password("     ").is_err()); // 全空格
    }

    #[test]
    fn test_validate_email() {
        // 合法邮箱
        assert!(validate_email("test@example.com").is_ok());
        assert!(validate_email("test.name@example.co.uk").is_ok());
        assert!(validate_email("test+tag@example.com").is_ok());
        assert!(validate_email("test_test@example.com").is_ok());
        assert!(validate_email("test-test@example.com").is_ok());
        assert!(validate_email("a@b.co").is_ok()); // 最小域名

        // 非法邮箱
        assert!(validate_email("invalid-email").is_err()); // 无效格式
        assert!(validate_email("").is_err()); // 空
        assert!(validate_email("@example.com").is_err()); // 缺少用户名
        assert!(validate_email("test@").is_err()); // 缺少域名
        assert!(validate_email("test@.com").is_err()); // 域名格式错误
        assert!(validate_email("test@example").is_err()); // 缺少顶级域名
        assert!(validate_email("test@example..com").is_err()); // 连续点
    }
}
