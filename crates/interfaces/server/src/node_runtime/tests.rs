#![allow(
    clippy::unwrap_used,
    reason = "测试使用临时工件并通过断言呈现计划错误。"
)]

use std::path::Path;

use burncloud_node_runtime::{
    deployment::{DeploymentEventPublisher, DeploymentService, HttpProbe, LocalProcessManager},
    ArtifactVerificationStatus, HealthProbe, NodeState, PreparedArtifact, PreparedRuntime,
    ProcessManager, ReadinessProbe, RuntimeAdapter,
};
use serde_json::json;
use std::{
    fs,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tempfile::NamedTempFile;
use tokio::time::sleep;

use super::LlamaCppRuntimeAdapter;

#[tokio::test]
async fn llama_runtime_adapter_plans_verified_local_artifact() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .unwrap();
    let model_path = workspace
        .join("iso/DeepSeek-R1-Distill-Qwen-1.5B-Q2_K.gguf")
        .canonicalize()
        .unwrap();
    let executable = workspace
        .join("llama-b12099/llama-server")
        .canonicalize()
        .unwrap();
    let artifact =
        PreparedArtifact::new(&model_path, ArtifactVerificationStatus::Verified).unwrap();
    let runtime = PreparedRuntime {
        executable: executable.to_string_lossy().into_owned(),
    };
    let adapter = LlamaCppRuntimeAdapter::new("local-model".to_string());

    let plan = adapter.plan(&runtime, &artifact).await.unwrap();
    let port = plan.process.args[5].parse::<u16>().unwrap();

    println!("actual_plan={plan:?}");
    assert_eq!(plan.process.program, runtime.executable);
    assert_eq!(plan.process.args[0], "-m");
    assert_eq!(
        plan.process.args[1],
        artifact.local_path().to_string_lossy()
    );
    assert_eq!(plan.process.args[2], "--host");
    assert_eq!(plan.process.args[3], "127.0.0.1");
    assert_eq!(plan.process.args[4], "--port");
    assert_ne!(port, 0);
    assert_eq!(plan.local_endpoint, format!("http://127.0.0.1:{port}"));
    assert_eq!(
        plan.readiness.endpoint,
        format!("{}/health", plan.local_endpoint)
    );
}

#[tokio::test]
async fn llama_runtime_adapter_rejects_unverified_or_missing_artifacts() {
    let runtime = PreparedRuntime {
        executable: "/opt/burncloud/runtime/llama-server".to_string(),
    };
    let adapter = LlamaCppRuntimeAdapter::new("local-model".to_string());
    let unverified = PreparedArtifact::new("", ArtifactVerificationStatus::Failed).unwrap();
    assert!(adapter.plan(&runtime, &unverified).await.is_err());

    let missing_file = NamedTempFile::new().unwrap();
    let missing_path = Path::new(missing_file.path()).to_path_buf();
    drop(missing_file);
    let missing =
        PreparedArtifact::new(missing_path, ArtifactVerificationStatus::Verified).unwrap();
    assert!(adapter.plan(&runtime, &missing).await.is_err());
}

#[tokio::test]
#[ignore = "需要本机 llama-server 与 GGUF 模型；运行后会在 deploy/records 生成实测 JSON"]
#[allow(
    clippy::disallowed_types,
    clippy::too_many_lines,
    reason = "实机测试需完整保留外部 llama.cpp JSON 响应和部署事件记录。"
)]
async fn live_local_llama_deployment_records_process_and_heartbeat() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .unwrap();
    let model_path = workspace
        .join("iso/DeepSeek-R1-Distill-Qwen-1.5B-Q2_K.gguf")
        .canonicalize()
        .unwrap();
    let executable = std::env::var_os("BURNCLOUD_LLAMA_SERVER")
        .map(std::path::PathBuf::from)
        .expect("BURNCLOUD_LLAMA_SERVER must point to llama-server")
        .canonicalize()
        .unwrap();
    let artifact =
        PreparedArtifact::new(&model_path, ArtifactVerificationStatus::Verified).unwrap();
    let runtime = PreparedRuntime {
        executable: executable.to_string_lossy().into_owned(),
    };
    let model_id = model_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap()
        .to_string();

    let processes = Arc::new(LocalProcessManager::with_kill_on_drop());
    let probe = Arc::new(HttpProbe::default());
    let events = Arc::new(crate::node_deployment::DeploymentEventStore::default());
    let publisher: Arc<dyn DeploymentEventPublisher> = events.clone();
    let (service, mut monitor_errors) =
        DeploymentService::new(processes.clone(), probe.clone(), probe.clone(), publisher);

    let outcome = service
        .deploy_prepared(
            model_id.clone(),
            artifact.clone(),
            runtime.clone(),
            Arc::new(LlamaCppRuntimeAdapter::new(model_id.clone())),
        )
        .await
        .unwrap();
    probe
        .wait_ready(outcome.plan.readiness.clone())
        .await
        .unwrap();
    let healthy = probe
        .is_healthy(outcome.plan.readiness.clone())
        .await
        .unwrap();
    let running = processes.is_running(outcome.process).await.unwrap();
    let inference_request = json!({
        "model": model_id,
        "messages": [{ "role": "user", "content": "只输出：BurnCloud 本机模型部署成功。" }],
        "temperature": 0,
        "max_tokens": 64,
        "stream": false,
    });
    let inference_response = reqwest::Client::new()
        .post(format!(
            "{}/v1/chat/completions",
            outcome.plan.local_endpoint
        ))
        .json(&inference_request)
        .send()
        .await
        .unwrap();
    let inference_status = inference_response.status();
    let inference_body: serde_json::Value = inference_response.json().await.unwrap();

    sleep(Duration::from_secs(12)).await;
    let heartbeat_snapshot = events.get(&model_id).await.unwrap();
    let heartbeat_events = heartbeat_snapshot
        .events
        .iter()
        .filter(|event| event.kind == "heartbeat")
        .cloned()
        .collect::<Vec<_>>();
    let keep_running = std::env::var("BURNCLOUD_KEEP_DEPLOYMENT_RUNNING").as_deref() == Ok("1");
    let (stopped, running_after_stop, final_snapshot) = if keep_running {
        (false, running, heartbeat_snapshot.clone())
    } else {
        let stopped = service.stop_model(&model_id).await.unwrap();
        let running_after_stop = processes.is_running(outcome.process).await.unwrap();
        let final_snapshot = events.get(&model_id).await.unwrap();
        (stopped, running_after_stop, final_snapshot)
    };
    let monitor_error = monitor_errors
        .try_recv()
        .map(|error| error.to_string())
        .ok();
    let runtime_version = std::process::Command::new(&executable)
        .arg("--version")
        .output()
        .unwrap();
    let runtime_version = json!({
        "exit_code": runtime_version.status.code(),
        "stdout": String::from_utf8_lossy(&runtime_version.stdout).trim(),
        "stderr": String::from_utf8_lossy(&runtime_version.stderr).trim(),
    });
    let recorded_at_unix_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    assert_eq!(outcome.state, NodeState::Ready);
    assert!(healthy);
    assert!(running);
    assert!(inference_status.is_success(), "{inference_body}");
    assert!(!heartbeat_events.is_empty());
    assert_eq!(stopped, !keep_running);
    assert_eq!(running_after_stop, keep_running);
    assert!(monitor_error.is_none());

    let mut record = json!({
        "recorded_at_unix_ms": recorded_at_unix_ms,
        "platform": std::env::consts::OS,
        "model": {
            "model_id": model_id,
            "path": artifact.local_path(),
            "size_bytes": fs::metadata(artifact.local_path()).unwrap().len(),
            "verification_status": format!("{:?}", artifact.verification_status()),
        },
        "runtime": {
            "executable": runtime.executable,
            "version": runtime_version,
        },
        "inference": {
            "http_status": inference_status.as_u16(),
            "request": inference_request,
            "response": inference_body,
        },
        "deployment_outcome": {
            "state": format!("{:?}", outcome.state),
            "process": {
                "pid": outcome.process.pid,
            },
            "plan": {
                "process": {
                    "program": outcome.plan.process.program,
                    "args": outcome.plan.process.args,
                },
                "readiness": {
                    "endpoint": outcome.plan.readiness.endpoint,
                },
                "local_endpoint": outcome.plan.local_endpoint,
            },
        },
        "process_manager": {
            "is_running_before_stop": running,
            "stop_model_returned": stopped,
            "is_running_after_stop": running_after_stop,
        },
        "observation": {
            "keep_running": keep_running,
            "record_refresh_interval_seconds": 10,
        },
        "probes": {
            "wait_ready": "Ok(())",
            "is_healthy": healthy,
        },
        "heartbeat": {
            "interval_seconds": 10,
            "snapshot": heartbeat_snapshot,
            "events": heartbeat_events,
        },
        "final_snapshot": final_snapshot,
        "monitor_error": monitor_error,
    });
    let record_dir = workspace.join("deploy/records");
    fs::create_dir_all(&record_dir).unwrap();
    let record_path = record_dir.join(format!("local-llama-deployment-{recorded_at_unix_ms}.json"));
    fs::write(&record_path, serde_json::to_vec_pretty(&record).unwrap()).unwrap();

    println!("record_path={}", record_path.display());
    println!(
        "actual_execution_record={}",
        serde_json::to_string_pretty(&record).unwrap()
    );

    if keep_running {
        println!(
            "deployment_observe_mode=running; pid={}; endpoint={}",
            outcome.process.pid, outcome.plan.local_endpoint
        );
        loop {
            sleep(Duration::from_secs(10)).await;
            let live_snapshot = events.get(&model_id).await.unwrap();
            let live_heartbeat_events = live_snapshot
                .events
                .iter()
                .filter(|event| event.kind == "heartbeat")
                .cloned()
                .collect::<Vec<_>>();
            let live_running = processes.is_running(outcome.process).await.unwrap_or(false);
            record["heartbeat"]["snapshot"] = serde_json::to_value(&live_snapshot).unwrap();
            record["heartbeat"]["events"] = serde_json::to_value(&live_heartbeat_events).unwrap();
            record["process_manager"]["is_running_now"] = json!(live_running);
            record["observation"]["last_refresh_unix_ms"] = json!(SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis());
            fs::write(&record_path, serde_json::to_vec_pretty(&record).unwrap()).unwrap();
            println!(
                "heartbeat_refresh pid={} running={} last_heartbeat_unix_ms={:?}",
                outcome.process.pid, live_running, live_snapshot.last_heartbeat_unix_ms
            );
            if !live_running {
                break;
            }
        }
    }
}
