use burncloud_service_inference::{InferenceService, InferenceConfig, InstanceStatus};
use burncloud_database::{Database, create_default_database};
use burncloud_database_router::RouterDatabase;
use std::env;
use std::path::PathBuf;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_inference_lifecycle_and_db_registration() -> anyhow::Result<()> {
    // 1. 设置测试环境
    // 指向我们的 mock_server.bat
    let mut mock_bin = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    mock_bin.push("tests");
    mock_bin.push("mock_server.bat");
    
    env::set_var("BURNCLOUD_LLAMA_BIN", mock_bin.to_str().unwrap());
    
    // 初始化数据库 (使用内存或临时文件，这里 create_default_database 使用默认位置，
    // 在测试环境中可能需要注意隔离，但为了验证 create_upstream 逻辑，我们直接用它)
    let db = create_default_database().await?;
    RouterDatabase::init(&db).await?;
    
    // 2. 初始化服务
    let service = InferenceService::new().await?;
    
    let model_id = "test-model-qwen";
    let port = 18080;
    
    let config = InferenceConfig {
        model_id: model_id.to_string(),
        file_path: "C:\\fake\\path\\model.gguf".to_string(),
        port,
        context_size: 2048,
        gpu_layers: 0,
    };
    
    println!(">>> Starting Instance...");
    // 3. 启动实例
    service.start_instance(config).await?;
    
    // 验证状态
    let status = service.get_status(model_id).await;
    assert_eq!(status, InstanceStatus::Running, "Instance should be running");
    
    // 4. 验证数据库注册 (Task 11.2 Key Validation)
    println!(">>> Verifying Database Registration...");
    let upstream_id = format!("local-{}", model_id);
    let upstream = RouterDatabase::get_upstream(&db, &upstream_id).await?;
    
    assert!(upstream.is_some(), "Upstream should be registered in DB");
    let u = upstream.unwrap();
    assert_eq!(u.base_url, format!("http://127.0.0.1:{}", port));
    assert_eq!(u.match_path, "/v1/chat/completions");
    println!(">>> Database registration verified: {:?}", u);
    
    // 稍微等待一下，模拟运行
    sleep(Duration::from_millis(500)).await;
    
    // 5. 停止实例
    println!(">>> Stopping Instance...");
    service.stop_instance(model_id).await?;
    
    // 验证状态
    let status_stopped = service.get_status(model_id).await;
    assert_eq!(status_stopped, InstanceStatus::Stopped, "Instance should be stopped");
    
    // 6. 验证数据库清理 (Task 11.2 Key Validation)
    println!(">>> Verifying Database Cleanup...");
    let upstream_after = RouterDatabase::get_upstream(&db, &upstream_id).await?;
    assert!(upstream_after.is_none(), "Upstream should be removed from DB after stop");
    println!(">>> Database cleanup verified.");
    
    Ok(())
}
