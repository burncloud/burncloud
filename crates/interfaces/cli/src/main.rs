use anyhow::Result;
use std::env;
use std::path::PathBuf;

mod cli;

fn main() -> Result<()> {
    // Load .env file if present
    dotenvy::dotenv().ok();

    // Generate and persist required secrets on first local startup.
    ensure_runtime_secrets();

    let args: Vec<String> = env::args().collect();

    // For CLI commands (not server/router), suppress INFO logs on stdout
    let is_server = args.len() >= 2 && matches!(args[1].as_str(), "server" | "router" | "client");
    if !is_server {
        env::set_var("RUST_LOG", "error");
    }

    // 初始化日志（tracing-subscriber + 文件输出）
    let _logging_guards = burncloud_server::logging::init_logging();

    match args.as_slice() {
        [_] => {
            // burncloud.exe (No args)
            #[cfg(windows)]
            {
                // Start Server in background thread
                std::thread::spawn(|| {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(async {
                        let host =
                            std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
                        let port = std::env::var("PORT")
                            .unwrap_or_else(|_| {
                                burncloud_common::constants::DEFAULT_PORT.to_string()
                            })
                            .parse()
                            .unwrap_or(burncloud_common::constants::DEFAULT_PORT);
                        if let Err(e) = burncloud_server::start_server(&host, port, false).await {
                            eprintln!("Server failed to start: {}", e);
                        }
                    });
                });

                burncloud_client::launch_gui_with_tray();
            }

            #[cfg(not(windows))]
            {
                println!("Starting BurnCloud Server with LiveView (Headless Mode)...");
                run_async_server()?;
            }
        }
        [_, subcommand, _rest @ ..] => {
            match subcommand.as_str() {
                "client" => {
                    #[cfg(windows)]
                    burncloud_client::launch_gui_with_tray();

                    #[cfg(not(windows))]
                    {
                        println!("Desktop GUI is only available on Windows.");
                        println!("On Linux, use 'burncloud server' to start the web dashboard.");
                    }
                }
                "server" | "router" => {
                    run_async_server()?;
                }
                _ => {
                    // 处理其他命令
                    run_async_cli(&args[1..])?;
                }
            }
        }
        [] => {
            // 空参数数组 (理论上不应该发生)
            crate::cli::commands::show_help();
        }
    }

    Ok(())
}

/// Check whether the current MASTER_KEY is valid (present, valid hex, exactly 32 bytes).
fn is_valid_master_key() -> bool {
    let val = match env::var("MASTER_KEY") {
        Ok(v) => v,
        Err(_) => return false,
    };
    is_valid_master_key_value(&val)
}

fn is_valid_master_key_value(value: &str) -> bool {
    hex::decode(value.trim())
        .ok()
        .map(|bytes| bytes.len() == 32)
        .unwrap_or(false)
}

/// Return the `.env` path used by `dotenvy` for this process.
fn env_file_path() -> PathBuf {
    env::current_dir()
        .map(|dir| dir.join(".env"))
        .unwrap_or_else(|_| PathBuf::from(".env"))
}

fn persisted_env_value(name: &str) -> Option<String> {
    dotenvy::from_path_iter(env_file_path())
        .ok()?
        .filter_map(Result::ok)
        .find_map(|(key, value)| (key == name && !value.trim().is_empty()).then_some(value))
}

/// Replace or append a value in `.env`, then expose it to the current process.
fn persist_env_value(name: &str, value: &str) {
    let env_path = env_file_path();
    let line = format!("{name}={value}");
    let prefix = format!("{name}=");
    let content = if env_path.exists() {
        let existing = std::fs::read_to_string(&env_path).unwrap_or_default();
        let mut found = false;
        let lines: String = existing
            .lines()
            .map(|l| {
                if l.starts_with(&prefix) {
                    found = true;
                    line.clone()
                } else {
                    l.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        if found {
            lines + "\n"
        } else {
            lines + "\n" + &line + "\n"
        }
    } else {
        line + "\n"
    };

    match std::fs::write(&env_path, content) {
        Ok(_) => eprintln!("Generated {name} in {}", env_path.display()),
        Err(e) => eprintln!("Warning: failed to write .env: {e}"),
    }

    env::set_var(name, value);
}

fn random_secret() -> String {
    let key: [u8; 32] = rand::random();
    hex::encode(key)
}

/// Ensure all required runtime secrets exist and remain stable across restarts.
fn ensure_runtime_secrets() {
    if !is_valid_master_key() {
        if let Some(value) =
            persisted_env_value("MASTER_KEY").filter(|value| is_valid_master_key_value(value))
        {
            env::set_var("MASTER_KEY", value);
        } else {
            persist_env_value("MASTER_KEY", &random_secret());
        }
    }

    for name in ["JWT_SECRET", "BURNCLOUD_INTERNAL_SECRET"] {
        let is_present = env::var(name)
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false);
        if !is_present {
            if let Some(value) = persisted_env_value(name) {
                env::set_var(name, value);
            } else {
                persist_env_value(name, &random_secret());
            }
        }
    }
}

#[tokio::main]
async fn run_async_server() -> Result<()> {
    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| burncloud_common::constants::DEFAULT_PORT.to_string())
        .parse()
        .unwrap_or(burncloud_common::constants::DEFAULT_PORT);
    burncloud_server::start_server(&host, port, true).await
}

#[tokio::main]
async fn run_async_cli(args: &[String]) -> Result<()> {
    crate::cli::commands::handle_command(args).await
}
