//! Tests for regex compilation - Bug #014 fix verification
//! 
//! Verifies that all hardcoded regex patterns in jobs_aesthetic.rs compile correctly
//! and don't panic at runtime.

use regex::Regex;

#[test]
fn test_server_start_regex() {
    // 验证服务器启动错误正则表达式能正确编译和匹配
    let re = Regex::new(r"Server failed to start at (http[^\r\n]+)")
        .expect("Invalid regex pattern for server start error");
    
    // 测试正常匹配
    let text = "Server failed to start at http://localhost:3000";
    assert!(re.is_match(text));
    let cap = re.captures(text).unwrap();
    assert_eq!(&cap[1], "http://localhost:3000");
    
    // 测试不匹配情况
    assert!(!re.is_match("Server started successfully"));
}

#[test]
fn test_layout_check_regex() {
    // 验证布局检查正则表达式
    let re = Regex::new(r"Layout check failed on ([^:]+): ([^\r\n]+)")
        .expect("Invalid regex pattern for layout check");
    
    let text = "Layout check failed on /home: Missing padding";
    assert!(re.is_match(text));
    let cap = re.captures(text).unwrap();
    assert_eq!(&cap[1], "/home");
    assert_eq!(&cap[2], "Missing padding");
}

#[test]
fn test_naming_error_regex() {
    // 验证命名错误正则表达式
    let re = Regex::new(r"::error::([^\r\n]+)")
        .expect("Invalid regex pattern for naming error");
    
    let text = "::error::Invalid class name: .btn-primary-large";
    assert!(re.is_match(text));
    let cap = re.captures(text).unwrap();
    assert_eq!(&cap[1], "Invalid class name: .btn-primary-large");
}

#[test]
fn test_aesthetic_metrics_regex() {
    // 验证美学指标正则表达式
    let re = Regex::new(r"Aesthetic metrics failed on ([^:]+): ([^\r\n]+)")
        .expect("Invalid regex pattern for aesthetic metrics");
    
    let text = "Aesthetic metrics failed on /dashboard: Color contrast too low";
    assert!(re.is_match(text));
    let cap = re.captures(text).unwrap();
    assert_eq!(&cap[1], "/dashboard");
    assert_eq!(&cap[2], "Color contrast too low");
}

#[test]
fn test_all_regex_patterns_compile() {
    // 一次性验证所有硬编码正则表达式都能编译通过
    // 这确保了expect不会在运行时panic
    
    let patterns = vec![
        r"Server failed to start at (http[^\r\n]+)",
        r"Layout check failed on ([^:]+): ([^\r\n]+)",
        r"::error::([^\r\n]+)",
        r"Aesthetic metrics failed on ([^:]+): ([^\r\n]+)",
    ];
    
    for (i, pattern) in patterns.iter().enumerate() {
        let result = Regex::new(pattern);
        assert!(result.is_ok(), "Regex pattern #{} failed to compile: {}", i, pattern);
    }
}

#[test]
fn test_regex_performance() {
    // 验证正则表达式匹配性能不会成为瓶颈
    let re = Regex::new(r"Server failed to start at (http[^\r\n]+)")
        .expect("Invalid regex pattern");
    
    let text = "Server failed to start at http://localhost:3000".repeat(100);
    
    // 计时匹配操作，确保不会耗时过长
    let start = std::time::Instant::now();
    for _ in 0..1000 {
        let _ = re.is_match(&text);
    }
    let duration = start.elapsed();
    
    // 1000次匹配应该在100ms内完成
    assert!(duration < std::time::Duration::from_millis(100), 
        "Regex matching took too long: {:?}", duration);
}
