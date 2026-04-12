use anyhow::Result;
use std::env;
use std::path::Path;

fn main() -> Result<()> {
    // Load .env file if present
    dotenvy::dotenv().ok();

    // Auto-generate MASTER_KEY if missing
    ensure_master_key();

    // 初始化日志
    env_logger::init();

    let args: Vec<String> = env::args().collect();

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
                "code" => {
                    run_async_code()?;
                }
                _ => {
                    // 处理其他命令 (pull, run, list 等)
                    run_async_cli(&args[1..])?;
                }
            }
        }
        [] => {
            // 空参数数组 (理论上不应该发生)
            burncloud_cli::show_help();
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
    hex::decode(val.trim())
        .ok()
        .map(|bytes| bytes.len() == 32)
        .unwrap_or(false)
}

/// Ensure MASTER_KEY exists and is valid: if missing or malformed, generate a
/// 32-byte random key, write it to `.env`, and set it in the process environment.
fn ensure_master_key() {
    if is_valid_master_key() {
        return;
    }

    // Generate 32 random bytes as hex (64 chars)
    let key: [u8; 32] = rand::random();
    let hex_key = hex::encode(key);

    // Locate .env: prefer CWD (where dotenvy reads), fall back to exe dir
    let env_path = std::fs::canonicalize(".env").unwrap_or_else(|_| {
        env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join(".env")))
            .unwrap_or_else(|| Path::new(".env").to_path_buf())
    });

    // Replace or append MASTER_KEY line in .env
    let line = format!("MASTER_KEY={hex_key}");
    let content = if env_path.exists() {
        let existing = std::fs::read_to_string(&env_path).unwrap_or_default();
        let mut found = false;
        let lines: String = existing
            .lines()
            .map(|l| {
                if l.starts_with("MASTER_KEY=") {
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
        Ok(_) => eprintln!("Generated MASTER_KEY → {}", env_path.display()),
        Err(e) => eprintln!("Warning: failed to write .env: {e}"),
    }

    env::set_var("MASTER_KEY", &hex_key);
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
async fn run_async_code() -> Result<()> {
    burncloud_code::start_cli().await
}

#[tokio::main]
async fn run_async_cli(args: &[String]) -> Result<()> {
    burncloud_cli::handle_command(args).await
}
