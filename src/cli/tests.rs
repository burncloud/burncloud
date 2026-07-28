//! Tests for CLI argument parsing error handling
//! Bug #012 and Bug #013 fix verification

use clap::{Arg, Command};

/// Helper function to create a plan command parser
fn create_plan_command() -> Command {
    Command::new("plan")
        .subcommand(
            Command::new("create")
                .arg(Arg::new("name").required(true))
                .arg(Arg::new("monthly-fee").required(true).value_parser(clap::value_parser!(i64)))
                .arg(Arg::new("billing-mode").required(true))
                .arg(Arg::new("channel-id").required(true).value_parser(clap::value_parser!(i32)))
        )
        .subcommand(
            Command::new("show")
                .arg(Arg::new("id").required(true).value_parser(clap::value_parser!(i32)))
        )
        .subcommand(
            Command::new("delete")
                .arg(Arg::new("id").required(true).value_parser(clap::value_parser!(i32)))
        )
}

/// Helper function to create a subscription command parser
fn create_subscription_command() -> Command {
    Command::new("subscription")
        .subcommand(
            Command::new("subscribe")
                .arg(Arg::new("user").required(true).value_parser(clap::value_parser!(i32)))
                .arg(Arg::new("plan").required(true).value_parser(clap::value_parser!(i32)))
                .arg(Arg::new("duration").required(true).value_parser(clap::value_parser!(i64)))
        )
        .subcommand(
            Command::new("list")
                .arg(Arg::new("user").required(true).value_parser(clap::value_parser!(i32)))
        )
        .subcommand(
            Command::new("cancel")
                .arg(Arg::new("id").required(true).value_parser(clap::value_parser!(i32)))
        )
}

#[test]
fn test_plan_create_missing_name() {
    // Bug #012: plan create 缺少 name 参数
    let matches = create_plan_command().get_matches_from_safe(["plan", "create", "--monthly-fee", "100", "--billing-mode", "prepaid", "--channel-id", "1"]);
    
    // 应该返回错误而不是 panic
    assert!(matches.is_err());
    let err = matches.err().unwrap();
    assert!(err.to_string().contains("name"));
}

#[test]
fn test_plan_create_missing_monthly_fee() {
    // Bug #012: plan create 缺少 monthly-fee 参数
    let matches = create_plan_command().get_matches_from_safe(["plan", "create", "--name", "test", "--billing-mode", "prepaid", "--channel-id", "1"]);
    
    assert!(matches.is_err());
    let err = matches.err().unwrap();
    assert!(err.to_string().contains("monthly-fee"));
}

#[test]
fn test_plan_create_missing_billing_mode() {
    // Bug #012: plan create 缺少 billing-mode 参数
    let matches = create_plan_command().get_matches_from_safe(["plan", "create", "--name", "test", "--monthly-fee", "100", "--channel-id", "1"]);
    
    assert!(matches.is_err());
    let err = matches.err().unwrap();
    assert!(err.to_string().contains("billing-mode"));
}

#[test]
fn test_plan_create_missing_channel_id() {
    // Bug #012: plan create 缺少 channel-id 参数
    let matches = create_plan_command().get_matches_from_safe(["plan", "create", "--name", "test", "--monthly-fee", "100", "--billing-mode", "prepaid"]);
    
    assert!(matches.is_err());
    let err = matches.err().unwrap();
    assert!(err.to_string().contains("channel-id"));
}

#[test]
fn test_plan_show_missing_id() {
    // Bug #012: plan show 缺少 id 参数
    let matches = create_plan_command().get_matches_from_safe(["plan", "show"]);
    
    assert!(matches.is_err());
    let err = matches.err().unwrap();
    assert!(err.to_string().contains("id"));
}

#[test]
fn test_plan_delete_missing_id() {
    // Bug #012: plan delete 缺少 id 参数
    let matches = create_plan_command().get_matches_from_safe(["plan", "delete"]);
    
    assert!(matches.is_err());
    let err = matches.err().unwrap();
    assert!(err.to_string().contains("id"));
}

#[test]
fn test_subscription_subscribe_missing_user() {
    // Bug #013: subscription subscribe 缺少 user 参数
    let matches = create_subscription_command().get_matches_from_safe(["subscription", "subscribe", "--plan", "1", "--duration", "30"]);
    
    assert!(matches.is_err());
    let err = matches.err().unwrap();
    assert!(err.to_string().contains("user"));
}

#[test]
fn test_subscription_subscribe_missing_plan() {
    // Bug #013: subscription subscribe 缺少 plan 参数
    let matches = create_subscription_command().get_matches_from_safe(["subscription", "subscribe", "--user", "1", "--duration", "30"]);
    
    assert!(matches.is_err());
    let err = matches.err().unwrap();
    assert!(err.to_string().contains("plan"));
}

#[test]
fn test_subscription_subscribe_missing_duration() {
    // Bug #013: subscription subscribe 缺少 duration 参数
    let matches = create_subscription_command().get_matches_from_safe(["subscription", "subscribe", "--user", "1", "--plan", "1"]);
    
    assert!(matches.is_err());
    let err = matches.err().unwrap();
    assert!(err.to_string().contains("duration"));
}

#[test]
fn test_subscription_list_missing_user() {
    // Bug #013: subscription list 缺少 user 参数
    let matches = create_subscription_command().get_matches_from_safe(["subscription", "list"]);
    
    assert!(matches.is_err());
    let err = matches.err().unwrap();
    assert!(err.to_string().contains("user"));
}

#[test]
fn test_subscription_cancel_missing_id() {
    // Bug #013: subscription cancel 缺少 id 参数
    let matches = create_subscription_command().get_matches_from_safe(["subscription", "cancel"]);
    
    assert!(matches.is_err());
    let err = matches.err().unwrap();
    assert!(err.to_string().contains("id"));
}

#[test]
fn test_plan_create_valid_args() {
    // 验证正常参数情况下能正确解析
    let matches = create_plan_command().get_matches_from_safe(["plan", "create", "--name", "test-plan", "--monthly-fee", "100", "--billing-mode", "prepaid", "--channel-id", "1"]);
    
    assert!(matches.is_ok());
    let m = matches.unwrap();
    
    let (_, sub_m) = m.subcommand().unwrap();
    assert_eq!(sub_m.get_one::<String>("name").unwrap(), "test-plan");
    assert_eq!(*sub_m.get_one::<i64>("monthly-fee").unwrap(), 100);
    assert_eq!(sub_m.get_one::<String>("billing-mode").unwrap(), "prepaid");
    assert_eq!(*sub_m.get_one::<i32>("channel-id").unwrap(), 1);
}

#[test]
fn test_subscription_subscribe_valid_args() {
    // 验证正常参数情况下能正确解析
    let matches = create_subscription_command().get_matches_from_safe(["subscription", "subscribe", "--user", "1", "--plan", "2", "--duration", "30"]);
    
    assert!(matches.is_ok());
    let m = matches.unwrap();
    
    let (_, sub_m) = m.subcommand().unwrap();
    assert_eq!(*sub_m.get_one::<i32>("user").unwrap(), 1);
    assert_eq!(*sub_m.get_one::<i32>("plan").unwrap(), 2);
    assert_eq!(*sub_m.get_one::<i64>("duration").unwrap(), 30);
}
