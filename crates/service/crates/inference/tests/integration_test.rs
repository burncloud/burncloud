#![allow(clippy::unwrap_used)]

use burncloud_database::create_default_database;
use burncloud_database_channel::ChannelProviderModel;
use burncloud_database_router::RouterDatabase;
use burncloud_service_inference::{InferenceConfig, InferenceService, InstanceStatus};
use std::env;
use std::path::PathBuf;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
#[ignore = "requires mock_server binary (Windows-only)"]
async fn test_inference_lifecycle_and_db_registration() -> anyhow::Result<()> {
    // 1. ??????
    // ????? mock_server.bat
    let mut mock_bin = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    mock_bin.push("tests");
    mock_bin.push("mock_server.bat");

    env::set_var(
        "BURNCLOUD_LLAMA_BIN",
        mock_bin
            .to_str()
            .unwrap_or_else(|| panic!("mock_bin path is not valid UTF-8")),
    );

    // ?????? (???????????? create_default_database ???????
    // ???????????????????? create_upstream ?????????)
    let db = create_default_database().await?;
    RouterDatabase::init(&db).await?;

    // 2. ?????
    let service = InferenceService::new();

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
    // 3. ????
    service.start_instance(&db, config).await?;

    // ????
    let status = service.get_status(model_id).await;
    assert_eq!(
        status,
        InstanceStatus::Running,
        "Instance should be running"
    );

    // 4. ??????? (Task 11.2 Key Validation)
    println!(">>> Verifying Database Registration...");
    let upstream = ChannelProviderModel::list(&db, 1000, 0)
        .await?
        .into_iter()
        .find(|channel| channel.name == format!("Local: {}", model_id));

    assert!(upstream.is_some(), "Upstream should be registered in DB");
    let u = upstream.unwrap_or_else(|| panic!("upstream should exist after is_some() check"));
    let expected_base_url = format!("http://127.0.0.1:{}", port);
    assert_eq!(u.base_url.as_deref(), Some(expected_base_url.as_str()));
    assert_eq!(u.models, model_id);
    assert_eq!(u.tag.as_deref(), Some("local-inference"));
    println!(">>> Database registration verified: {:?}", u);

    // ???????????
    sleep(Duration::from_millis(500)).await;

    // 5. ????
    println!(">>> Stopping Instance...");
    service.stop_instance(&db, model_id).await?;

    // ????
    let status_stopped = service.get_status(model_id).await;
    assert_eq!(
        status_stopped,
        InstanceStatus::Stopped,
        "Instance should be stopped"
    );

    // 6. ??????? (Task 11.2 Key Validation)
    println!(">>> Verifying Database Cleanup...");
    let upstream_after = ChannelProviderModel::list(&db, 1000, 0)
        .await?
        .into_iter()
        .find(|channel| channel.name == format!("Local: {}", model_id));
    assert!(
        upstream_after.is_none(),
        "Upstream should be removed from DB after stop"
    );
    println!(">>> Database cleanup verified.");

    Ok(())
}
