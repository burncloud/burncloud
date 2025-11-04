use anyhow::Result;
use std::env;

fn main() -> Result<()> {
    // 初始化日志
    env_logger::init();

    let args: Vec<String> = env::args().collect();

    match args.as_slice() {
        [_] => {
            // burncloud.exe (无参数)
            #[cfg(windows)]
            burncloud_client::launch_gui();

            #[cfg(not(windows))]
            burncloud_cli::show_help();
        }
        [_, subcommand, _rest @ ..] => {
            match subcommand.as_str() {
                "client" => {
                    burncloud_client::launch_gui();
                }
                "server" => {
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
    burncloud_server::start_server().await
}

#[tokio::main]
async fn run_async_code() -> Result<()> {
    burncloud_code::start_cli().await
}

#[tokio::main]
async fn run_async_cli(args: &[String]) -> Result<()> {
    burncloud_cli::handle_command(args).await
}
