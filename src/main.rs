use std::env;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    match args.as_slice() {
        [_] => {
            // burncloud.exe (无参数)
            #[cfg(all(windows, feature = "gui"))]
            burncloud_client::launch_gui();

            #[cfg(not(all(windows, feature = "gui")))]
            burncloud_cli::show_help();
        },
        [_, subcommand, _rest @ ..] => {
            match subcommand.as_str() {
                "client" => {
                    #[cfg(feature = "gui")]
                    burncloud_client::launch_gui();

                    #[cfg(not(feature = "gui"))]
                    {
                        println!("GUI功能未启用，请使用 --features gui 重新构建");
                        std::process::exit(1);
                    }
                },
                "server" => {
                    burncloud_server::start_server().await?;
                },
                "code" => {
                    burncloud_code::start_cli().await?;
                },
                _ => {
                    // 处理其他命令 (pull, run, list 等)
                    burncloud_cli::handle_command(&args[1..]).await?;
                }
            }
        },
        [] => {
            // 空参数数组 (理论上不应该发生)
            burncloud_cli::show_help();
        }
    }

    Ok(())
}