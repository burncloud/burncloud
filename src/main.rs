use anyhow::Result;
use std::env;

fn main() -> Result<()> {
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
                        let port = std::env::var("PORT").unwrap_or("3000".to_string()).parse().unwrap_or(3000);
                        if let Err(e) = burncloud_server::start_server(port, false).await {
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
                    burncloud_client::launch_gui_with_tray();
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

#[tokio::main]
async fn run_async_server() -> Result<()> {
    let port = std::env::var("PORT").unwrap_or("3000".to_string()).parse().unwrap_or(3000);
    burncloud_server::start_server(port, true).await
}

#[tokio::main]
async fn run_async_code() -> Result<()> {
    burncloud_code::start_cli().await
}

#[tokio::main]
async fn run_async_cli(args: &[String]) -> Result<()> {
    burncloud_cli::handle_command(args).await
}