use regex::Regex;
use once_cell::sync::Lazy;

static EMAIL_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(?:\.[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)*$"
    ).unwrap()
});

static USERNAME_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[a-zA-Z0-9_]{3,20}$").unwrap()
});

/// Password strength levels
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PasswordStrength {
    Weak,
    Medium,
    Strong,
}

impl PasswordStrength {
    pub fn as_str(&self) -> &'static str {
        match self {
            PasswordStrength::Weak => "弱",
            PasswordStrength::Medium => "中",
            PasswordStrength::Strong => "强",
        }
    }

    pub fn color_class(&self) -> &'static str {
        match self {
            PasswordStrength::Weak => "bg-red-500",
            PasswordStrength::Medium => "bg-yellow-500",
            PasswordStrength::Strong => "bg-green-500",
        }
    }

    pub fn width_class(&self) -> &'static str {
        match self {
            PasswordStrength::Weak => "w-1/3",
            PasswordStrength::Medium => "w-2/3",
            PasswordStrength::Strong => "w-full",
        }
    }
}

/// Validate email format
pub fn validate_email(email: &str) -> bool {
    if email.is_empty() {
        return true; // Email is optional
    }
    
    EMAIL_REGEX.is_match(email)
}

/// Get email validation error message
pub fn get_email_error(email: &str) -> Option<String> {
    if email.is_empty() {
        return None;
    }
    
    if !validate_email(email) {
        Some("邮箱格式不正确".to_string())
    } else {
        None
    }
}

/// Validate username format
pub fn validate_username(username: &str) -> bool {
    if username.is_empty() {
        return false;
    }
    
    USERNAME_REGEX.is_match(username)
}

/// Get username validation error message
pub fn get_username_error(username: &str) -> Option<String> {
    if username.is_empty() {
        return Some("用户名不能为空".to_string());
    }
    
    if username.len() < 3 {
        return Some("用户名至少需要3个字符".to_string());
    }
    
    if username.len() > 20 {
        return Some("用户名不能超过20个字符".to_string());
    }
    
    if !validate_username(username) {
        return Some("用户名只能包含字母、数字和下划线".to_string());
    }
    
    None
}

/// Calculate password strength
pub fn calculate_password_strength(password: &str) -> PasswordStrength {
    if password.is_empty() {
        return PasswordStrength::Weak;
    }
    
    let mut score = 0;
    
    // Length check
    if password.len() >= 8 {
        score += 1;
    }
    if password.len() >= 12 {
        score += 1;
    }
    
    // Character variety checks
    if password.chars().any(|c| c.is_lowercase()) {
        score += 1;
    }
    if password.chars().any(|c| c.is_uppercase()) {
        score += 1;
    }
    if password.chars().any(|c| c.is_numeric()) {
        score += 1;
    }
    if password.chars().any(|c| !c.is_alphanumeric()) {
        score += 1;
    }
    
    match score {
        0..=2 => PasswordStrength::Weak,
        3..=4 => PasswordStrength::Medium,
        _ => PasswordStrength::Strong,
    }
}

/// Get password validation error message
pub fn get_password_error(password: &str) -> Option<String> {
    if password.is_empty() {
        return Some("密码不能为空".to_string());
    }
    
    if password.len() < 6 {
        return Some("密码至少需要6个字符".to_string());
    }
    
    None
}

/// Sanitize input to prevent XSS attacks
pub fn sanitize_input(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_validation() {
        assert!(validate_email(""));
        assert!(validate_email("test@example.com"));
        assert!(validate_email("user.name+tag@example.co.uk"));
        assert!(!validate_email("invalid.email"));
        assert!(!validate_email("@example.com"));
        assert!(!validate_email("test@"));
    }

    #[test]
    fn test_username_validation() {
        assert!(validate_username("user123"));
        assert!(validate_username("test_user"));
        assert!(!validate_username("ab")); // too short
        assert!(!validate_username("user@name")); // invalid char
        assert!(!validate_username("")); // empty
    }

    #[test]
    fn test_password_strength() {
        assert_eq!(calculate_password_strength(""), PasswordStrength::Weak);
        assert_eq!(calculate_password_strength("abc"), PasswordStrength::Weak);
        assert_eq!(calculate_password_strength("password"), PasswordStrength::Weak);
        assert_eq!(calculate_password_strength("Password1"), PasswordStrength::Medium);
        assert_eq!(calculate_password_strength("Password123!"), PasswordStrength::Strong);
    }

    #[test]
    fn test_sanitization() {
        assert_eq!(sanitize_input("<script>alert('xss')</script>"), 
                   "&lt;script&gt;alert(&#x27;xss&#x27;)&lt;/script&gt;");
        assert_eq!(sanitize_input("normal text"), "normal text");
    }
}
