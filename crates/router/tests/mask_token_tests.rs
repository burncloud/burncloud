//! Tests for mask_token function - Bug #015 fix verification

// 测试 mask_token 函数，需要将其暴露给测试
// 由于 mask_token 在 lib.rs 中是私有函数，我们需要通过测试模块访问

use burncloud_router::mask_token;

#[test]
fn test_mask_token_short() {
    // 短 token（<= 8 字符）应该完全脱敏
    assert_eq!(mask_token(""), "***");
    assert_eq!(mask_token("abcd"), "***");
    assert_eq!(mask_token("abcdefgh"), "***");
}

#[test]
fn test_mask_token_normal() {
    // 正常长度的 token 应该保留前4后4个字符
    assert_eq!(mask_token("sk-1234567890"), "sk-1***7890");
    assert_eq!(mask_token("sk-proj-abcdefghijklmnop"), "sk-p***mnop");
    assert_eq!(mask_token("abcdefghij"), "abcd***ghij");
}

#[test]
fn test_mask_token_exactly_9_chars() {
    // 刚好9个字符的情况
    assert_eq!(mask_token("123456789"), "1234***6789");
}

#[test]
fn test_mask_token_api_key_formats() {
    // 常见 API Key 格式
    assert_eq!(mask_token("pk_live_abcdefghijklmnopqrstuv"), "pk_l***stuv");
    assert_eq!(mask_token("sk_test_1234567890abcdef"), "sk_t***cdef");
    assert_eq!(mask_token("AKIAIOSFODNN7EXAMPLE"), "AKIA***MPLE");
    assert_eq!(mask_token("GOOG1234567890abcdef"), "GOOG***cdef");
}

#[test]
fn test_mask_token_unicode() {
    // Unicode 字符测试（边界情况）
    // "测试密钥12345678" = 4个中文字符 + 8个数字 = 12个字符
    // 保留前4个字符和后4个字符
    assert_eq!(mask_token("测试密钥12345678"), "测试密钥***5678");
}

#[test]
fn test_mask_token_security() {
    // 安全验证：脱敏后不应包含完整敏感信息
    let original = "sk-secret-key-very-long-1234567890";
    let masked = mask_token(original);
    
    // 脱敏后不应包含完整的中间部分
    assert!(!masked.contains("secret"));
    assert!(!masked.contains("very-long"));
    
    // 应该保留前4和后4
    assert!(masked.starts_with("sk-s"));
    assert!(masked.ends_with("7890"));
}
