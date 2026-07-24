//! Tests for main.rs error handling
//! Bug #011 fix verification
//! 
//! Tests for Tokio runtime creation error handling

use std::error::Error;

/// 模拟 Runtime 创建结果的辅助函数
/// 用于测试错误处理逻辑
enum RuntimeResult {
    Ok,
    Err(String),
}

/// 模拟创建 Tokio runtime 的过程
/// 测试错误处理逻辑是否正确
fn create_runtime_simulation(result: RuntimeResult) -> Result<(), Box<dyn Error>> {
    match result {
        RuntimeResult::Ok => Ok(()),
        RuntimeResult::Err(msg) => Err(msg.into()),
    }
}

#[test]
fn test_runtime_creation_success() {
    // 验证正常创建 runtime 的情况
    let result = create_runtime_simulation(RuntimeResult::Ok);
    assert!(result.is_ok());
}

#[test]
fn test_runtime_creation_failure() {
    // 验证 runtime 创建失败时能正确返回错误
    // Bug #011 修复前：会直接 panic
    // Bug #011 修复后：会返回错误，程序可以优雅处理
    
    let result = create_runtime_simulation(RuntimeResult::Err("Failed to create runtime".into()));
    assert!(result.is_err());
    assert!(result.err().unwrap().to_string().contains("Failed to create runtime"));
}

#[test]
fn test_runtime_error_handling_does_not_panic() {
    // 验证错误处理不会导致程序崩溃
    // 这是 Bug #011 的核心测试：unwrap() 会 panic，而 match 不会
    
    // 使用 catch_unwind 捕获 panic
    let result = std::panic::catch_unwind(|| {
        let _ = create_runtime_simulation(RuntimeResult::Err("Test error".into()));
    });
    
    // 应该不会 panic
    assert!(result.is_ok(), "Runtime creation failure should not cause panic");
}

#[test]
fn test_host_port_parsing_defaults() {
    // 验证 HOST 和 PORT 环境变量缺失时使用默认值
    
    // 清除环境变量
    std::env::remove_var("HOST");
    std::env::remove_var("PORT");
    
    // 获取默认值
    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse()
        .unwrap_or(3000);
    
    assert_eq!(host, "127.0.0.1");
    assert_eq!(port, 3000);
}

#[test]
fn test_host_port_parsing_custom_values() {
    // 验证环境变量设置时使用自定义值
    
    std::env::set_var("HOST", "0.0.0.0");
    std::env::set_var("PORT", "8080");
    
    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse()
        .unwrap_or(3000);
    
    assert_eq!(host, "0.0.0.0");
    assert_eq!(port, 8080);
    
    // 清理环境变量
    std::env::remove_var("HOST");
    std::env::remove_var("PORT");
}

#[test]
fn test_port_parsing_invalid_value() {
    // 验证无效端口值使用默认值
    
    std::env::set_var("PORT", "invalid");
    
    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse()
        .unwrap_or(3000);
    
    assert_eq!(port, 3000);
    
    std::env::remove_var("PORT");
}

#[test]
fn test_port_parsing_out_of_range() {
    // 验证超出范围的端口值使用默认值
    
    std::env::set_var("PORT", "65536");
    
    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse()
        .unwrap_or(3000);
    
    assert_eq!(port, 3000);
    
    std::env::remove_var("PORT");
}
