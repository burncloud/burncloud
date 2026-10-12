#![cfg(unix)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic_in_result_fn,
    clippy::let_underscore_must_use,
    clippy::disallowed_types,
    reason = "Test-only integration file. A failed assertion is the intended failure signal for an \
              RPC contract, and these tests compare real returned values and real wire bytes rather \
              than trusting the function under test. `unwrap`/`expect` appear where the test itself \
              is what would break (reading its own scripted responses), and `let _ =` drives a call \
              whose return value is deliberately discarded. `serde_json::Value` is how this file \
              reads a JSON-RPC message off the wire, which is an untrusted document by definition: a \
              test server for that protocol is itself the protocol boundary the lint carves out, and \
              `src/rpc.rs` takes the same exception for the same reason."
)]
//! The `add_uri` contract, driven end to end over a real loopback socket (#635 plan v2).
//!
//! `add_uri` is the one entry point that has to make two decisions in sequence -- is this URI
//! already downloading, and if not, what exactly do I put on the wire -- and neither decision had
//! a test that read the request the client actually sent. The inline suites in `src/rpc.rs` cover
//! the response *decoding* paths; this file covers the outcome branches (`Existing`, `Created`,
//! `RpcError`) and, for `Created`, the request shape.
//!
//! Every test here talks to a **scripted in-process HTTP server** bound to `127.0.0.1:0`. No test
//! downloads anything, starts `aria2c`, or runs `taskkill`: the real side effects live in
//! `start_aria2_rpc`, `Aria2Daemon::start`, `download_aria2` and `kill_existing_aria2`, and this
//! increment's plan forbids invoking them.
//!
//! Why the server is scripted rather than "answer whatever comes": `add_uri` calls `tellActive`
//! first, and only calls `aria2.addUri` when nothing matched. A server that always answered the
//! same way could not distinguish "the duplicate check ran and passed" from "the duplicate check
//! was skipped", which is exactly the distinction these tests exist to make.

use burncloud_download_aria2::{AddUriOutcome, Aria2Error, Aria2RpcClient, DownloadOptions};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// Read one complete HTTP request and return its JSON body as text.
///
/// A single `read` is not a message: it can return only part of the headers, and the body length is
/// only known from `Content-Length`. Reading until the header terminator and then for the counted
/// number of bytes is what makes the request assertions below reliable.
async fn read_request_body(
    stream: &mut tokio::net::TcpStream,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let mut buffer = Vec::new();
    let mut chunk = [0_u8; 512];

    let header_end = loop {
        if let Some(position) = find_subslice(&buffer, b"\r\n\r\n") {
            break position + 4;
        }
        let read = stream.read(&mut chunk).await?;
        if read == 0 {
            return Err("the connection closed before the headers ended".into());
        }
        buffer.extend_from_slice(&chunk[..read]);
    };

    let headers = String::from_utf8_lossy(&buffer[..header_end]).to_string();
    let length: usize = headers
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            if name.eq_ignore_ascii_case("content-length") {
                value.trim().parse().ok()
            } else {
                None
            }
        })
        .ok_or("the request had no Content-Length")?;

    while buffer.len() < header_end + length {
        let read = stream.read(&mut chunk).await?;
        if read == 0 {
            return Err("the connection closed before the body ended".into());
        }
        buffer.extend_from_slice(&chunk[..read]);
    }

    Ok(String::from_utf8_lossy(&buffer[header_end..header_end + length]).to_string())
}

/// The offset of `needle` in `haystack`, if present.
fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// A server that answers up to `calls` requests with `respond` and records every request it saw.
///
/// `respond` receives the zero-based call index and the parsed request, so a test can answer
/// `tellActive` differently from `aria2.addUri`. The recorded bodies are what the "what did the
/// client actually send" assertions read.
async fn start_scripted_server<F>(
    calls: usize,
    respond: F,
) -> Result<(u16, Arc<Mutex<Vec<serde_json::Value>>>), Box<dyn std::error::Error + Send + Sync>>
where
    F: Fn(usize) -> String + Send + Sync + 'static,
{
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();
    let seen: Arc<Mutex<Vec<serde_json::Value>>> = Arc::new(Mutex::new(Vec::new()));
    let recorded = Arc::clone(&seen);
    let respond = Arc::new(respond);

    tokio::spawn(async move {
        for index in 0..calls {
            let Ok((mut stream, _)) = listener.accept().await else {
                break;
            };
            let body = match read_request_body(&mut stream).await {
                Ok(body) => body,
                Err(_) => continue,
            };
            let parsed: serde_json::Value =
                serde_json::from_str(&body).unwrap_or(serde_json::Value::Null);
            if let Ok(mut guard) = recorded.lock() {
                guard.push(parsed);
            }
            let response = respond(index);
            let http = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response}",
                response.len()
            );
            let _ = stream.write_all(http.as_bytes()).await;
        }
    });

    Ok((port, seen))
}

/// The requests the server actually received.
fn requests_seen(seen: &Arc<Mutex<Vec<serde_json::Value>>>) -> Vec<serde_json::Value> {
    seen.lock().map(|guard| guard.clone()).unwrap_or_default()
}

/// A `tellActive` response carrying exactly the tasks given.
fn active_tasks(gids: &[&str]) -> String {
    let tasks: Vec<serde_json::Value> = gids
        .iter()
        .map(|gid| {
            serde_json::json!({
                "gid": gid,
                "status": "active",
                "totalLength": "0",
                "completedLength": "0",
                "downloadSpeed": "0"
            })
        })
        .collect();
    serde_json::json!({ "jsonrpc": "2.0", "id": "1", "result": tasks }).to_string()
}

/// A `getFiles` response claiming `uri` was downloaded to `path`.
///
/// The GID is not part of a `getFiles` reply -- the client already knows which GID it asked about --
/// so this takes only what the response actually carries.
fn files_for(uri: &str, path: &str) -> String {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": "1",
        "result": [{ "path": path, "uris": [{ "uri": uri, "status": "used" }] }]
    })
    .to_string()
}

/// A `tellStatus` response for `gid`.
fn status_for(gid: &str) -> String {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": "1",
        "result": {
            "gid": gid,
            "status": "active",
            "totalLength": "0",
            "completedLength": "0",
            "downloadSpeed": "0"
        }
    })
    .to_string()
}

/// A `aria2.addUri` response whose result is the new GID.
fn added(gid: &str) -> String {
    serde_json::json!({ "jsonrpc": "2.0", "id": "1", "result": gid }).to_string()
}

// -------------------------------------------------------------------------------------------
// AC4a -- an already-downloading URI is reported as Existing, not added a second time
// -------------------------------------------------------------------------------------------

/// A matching active task short-circuits into `Existing(gid)`, and the GID is the existing one.
#[tokio::test]
async fn a_matching_active_task_is_reported_as_existing_with_its_gid(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let uri = "https://example.invalid/already-downloading.bin";
    let dir = "/tmp/bc-635-existing";
    let existing_gid = "gid-existing-1";

    let (port, seen) = start_scripted_server(3, {
        let uri = uri.to_string();
        let dir = dir.to_string();
        let gid = existing_gid.to_string();
        move |index| match index {
            0 => active_tasks(&[&gid]),
            1 => status_for(&gid),
            _ => files_for(&uri, &format!("{dir}/{}", uri.trim_start_matches("https://"))),
        }
    })
    .await?;

    let client = Aria2RpcClient::new(port, None);
    let options = DownloadOptions {
        dir: Some(dir.to_string()),
        out: None,
        split: None,
        max_connection_per_server: None,
        continue_download: None,
        allow_overwrite: None,
        auto_file_renaming: None,
    };

    let outcome = client.add_uri(vec![uri.to_string()], Some(options)).await?;

    match outcome {
        AddUriOutcome::Existing(gid) => assert_eq!(
            gid, existing_gid,
            "the reported GID must be the task that already exists"
        ),
        AddUriOutcome::Created(gid) => panic!(
            "an already-active identical URI was added again as {gid}, so a second download of the \
             same file would start"
        ),
    }

    // Three calls only: tellActive, tellStatus, getFiles. A fourth would be the add.
    let seen = requests_seen(&seen);
    assert_eq!(
        seen.len(),
        3,
        "the duplicate check must not fall through to aria2.addUri: {seen:#?}"
    );
    assert!(
        !seen
            .iter()
            .any(|request| request["method"] == "aria2.addUri"),
        "aria2.addUri must not be called for an already-downloading URI: {seen:#?}"
    );

    Ok(())
}

// -------------------------------------------------------------------------------------------
// AC4b -- a new URI is added, and the request says exactly what the plan requires
// -------------------------------------------------------------------------------------------

/// `Created(gid)` uses the GID the server returned, and the request carries the documented shape.
#[tokio::test]
async fn a_new_uri_is_created_and_the_request_shape_matches_the_contract(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let new_gid = "gid-created-1";
    let secret = "s3cr3t-token";
    let first_uri = "https://example.invalid/first.bin";
    let second_uri = "https://example.invalid/second.bin";

    let (port, seen) = start_scripted_server(2, move |index| {
        if index == 0 {
            // Nothing is downloading: an empty active list is what makes the add path reachable.
            active_tasks(&[])
        } else {
            added(new_gid)
        }
    })
    .await?;

    let client = Aria2RpcClient::new(port, Some(secret.to_string()));
    let outcome = client
        .add_uri(vec![first_uri.to_string(), second_uri.to_string()], None)
        .await?;

    match outcome {
        AddUriOutcome::Created(gid) => assert_eq!(
            gid, new_gid,
            "the created GID must be the one the server returned"
        ),
        AddUriOutcome::Existing(gid) => {
            panic!("nothing was downloading, so no task could already exist; got Existing({gid})")
        }
    }

    let seen = requests_seen(&seen);
    assert_eq!(seen.len(), 2, "expected tellActive then addUri: {seen:#?}");

    let add = &seen[1];
    assert_eq!(
        add["method"], "aria2.addUri",
        "the created task must be requested through aria2.addUri"
    );

    let params = add["params"]
        .as_array()
        .unwrap_or_else(|| panic!("params must be an array: {add:#?}"));

    // aria2's JSON-RPC form is a flat parameter list: `token:<secret>` first (when configured),
    // then the URIs as one parameter each -- not a nested array. Pinned as observed on the wire, so
    // a future change that starts wrapping the URIs in an array is caught here.
    let expected_token = format!("token:{secret}");
    assert_eq!(
        params.first().and_then(|value| value.as_str()),
        Some(expected_token.as_str()),
        "a configured secret must be sent first as token:<secret>: {params:#?}"
    );

    let sent: Vec<&str> = params
        .iter()
        .skip(1)
        .filter_map(|value| value.as_str())
        .collect();
    assert_eq!(
        sent,
        vec![first_uri, second_uri],
        "every URI must be its own parameter, in the caller's order and without duplicates: \
         {params:#?}"
    );

    Ok(())
}

// -------------------------------------------------------------------------------------------
// AC4c -- error propagation from the RPC layer
// -------------------------------------------------------------------------------------------

/// A JSON-RPC `error` object becomes `RpcError`, and nothing is treated as created.
#[tokio::test]
async fn a_json_rpc_error_object_becomes_an_rpc_error_and_no_created_outcome(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (port, seen) = start_scripted_server(2, |index| {
        if index == 0 {
            active_tasks(&[])
        } else {
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": "1",
                "error": { "code": 1, "message": "Unauthorized" }
            })
            .to_string()
        }
    })
    .await?;

    let client = Aria2RpcClient::new(port, None);
    let outcome = client
        .add_uri(vec!["https://example.invalid/denied.bin".to_string()], None)
        .await;

    match outcome {
        Err(Aria2Error::RpcError(message)) => assert!(
            message.contains("Unauthorized"),
            "the server's message must survive into the error: {message}"
        ),
        Err(other) => panic!("expected RpcError, got {other:?}"),
        Ok(created) => panic!(
            "a rejected addUri must not report success; got {:?}",
            created.gid()
        ),
    }

    assert_eq!(
        requests_seen(&seen).len(),
        2,
        "the add was attempted and then refused; no silent retry is expected"
    );

    Ok(())
}

// -------------------------------------------------------------------------------------------
// AC4d -- an empty URI list never touches the network
// -------------------------------------------------------------------------------------------

/// Empty input is refused locally: no request is sent at all.
///
/// The server is given one call slot so that a request, if one were sent, would be recorded. The
/// assertion is that the log stays empty -- which is the difference between "the client validated
/// its input" and "the client asked aria2 to reject it".
#[tokio::test]
async fn an_empty_uri_list_is_refused_without_contacting_the_server(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (port, seen) = start_scripted_server(1, |_| added("should-never-be-requested")).await?;

    let client = Aria2RpcClient::new(port, None);
    let outcome = client.add_uri(Vec::new(), None).await;

    match outcome {
        Err(Aria2Error::RpcError(message)) => assert!(
            message.contains("不能为空"),
            "the refusal must explain that the URI list is empty: {message}"
        ),
        Err(other) => panic!("expected RpcError for empty input, got {other:?}"),
        Ok(outcome) => panic!(
            "an empty URI list must not produce a task; got {:?}",
            outcome.gid()
        ),
    }

    // Give any (incorrectly) spawned request a chance to land before asserting on the log.
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;
    assert!(
        requests_seen(&seen).is_empty(),
        "an empty URI list must be rejected before any network call: {:#?}",
        requests_seen(&seen)
    );

    Ok(())
}
