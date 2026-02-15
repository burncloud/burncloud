use anyhow::Result;
use burncloud_auto_update::AutoUpdater;
use burncloud_core::{ConfigManager, ModelManager};
use burncloud_database::Database;
use clap::{Arg, Command};
use log::{error, info};
use std::io::{self, Write};

use crate::channel::handle_channel_command;
use crate::price::handle_price_command;

pub async fn handle_command(args: &[String]) -> Result<()> {
    let app = Command::new("burncloud")
        .version("0.1.0")
        .about("AI模型部署和管理平台")
        .subcommand_required(false)
        .subcommand(
            Command::new("pull")
                .about("下载模型")
                .arg(Arg::new("model").required(true).help("模型名称")),
        )
        .subcommand(
            Command::new("run")
                .about("运行模型")
                .arg(Arg::new("model").required(true).help("模型名称"))
                .arg(Arg::new("prompt").help("输入提示")),
        )
        .subcommand(Command::new("list").about("列出已下载的模型"))
        .subcommand(Command::new("server").about("启动服务器模式"))
        .subcommand(
            Command::new("update").about("检查并更新应用程序").arg(
                Arg::new("check-only")
                    .long("check-only")
                    .help("仅检查更新，不执行更新")
                    .action(clap::ArgAction::SetTrue),
            ),
        )
        .subcommand(
            Command::new("channel")
                .about("Manage API channels")
                .subcommand_required(true)
                .subcommand(
                    Command::new("add")
                        .about("Add a new channel")
                        .arg(
                            Arg::new("type")
                                .short('t')
                                .long("type")
                                .required(true)
                                .help("Channel type (openai, azure, anthropic, gemini, aws, vertexai, deepseek)"),
                        )
                        .arg(
                            Arg::new("key")
                                .short('k')
                                .long("key")
                                .required(true)
                                .help("API key for the channel"),
                        )
                        .arg(
                            Arg::new("models")
                                .short('m')
                                .long("models")
                                .help("Comma-separated list of supported models (uses defaults if not specified)"),
                        )
                        .arg(
                            Arg::new("url")
                                .short('u')
                                .long("url")
                                .help("Custom base URL for the channel"),
                        )
                        .arg(
                            Arg::new("name")
                                .short('n')
                                .long("name")
                                .help("Channel name (uses default if not specified)"),
                        ),
                )
                .subcommand(
                    Command::new("list")
                        .about("List all channels")
                        .arg(
                            Arg::new("format")
                                .long("format")
                                .default_value("table")
                                .value_parser(["table", "json"])
                                .help("Output format (table or json)"),
                        ),
                )
                .subcommand(
                    Command::new("delete")
                        .about("Delete a channel")
                        .arg(
                            Arg::new("id")
                                .required(true)
                                .help("Channel ID to delete"),
                        )
                        .arg(
                            Arg::new("yes")
                                .short('y')
                                .long("yes")
                                .action(clap::ArgAction::SetTrue)
                                .help("Skip confirmation prompt"),
                        ),
                )
                .subcommand(
                    Command::new("show")
                        .about("Show channel details")
                        .arg(
                            Arg::new("id")
                                .required(true)
                                .help("Channel ID to show"),
                        ),
                ),
        )
        .subcommand(
            Command::new("price")
                .about("Manage model pricing")
                .subcommand_required(true)
                .subcommand(
                    Command::new("list")
                        .about("List all prices")
                        .arg(
                            Arg::new("limit")
                                .long("limit")
                                .default_value("100")
                                .help("Maximum number of results"),
                        )
                        .arg(
                            Arg::new("offset")
                                .long("offset")
                                .default_value("0")
                                .help("Offset for pagination"),
                        ),
                )
                .subcommand(
                    Command::new("set")
                        .about("Set price for a model")
                        .arg(
                            Arg::new("model")
                                .required(true)
                                .help("Model name"),
                        )
                        .arg(
                            Arg::new("input")
                                .long("input")
                                .required(true)
                                .help("Input price per 1M tokens"),
                        )
                        .arg(
                            Arg::new("output")
                                .long("output")
                                .required(true)
                                .help("Output price per 1M tokens"),
                        )
                        .arg(
                            Arg::new("alias")
                                .long("alias")
                                .help("Alias to another model's pricing"),
                        ),
                )
                .subcommand(
                    Command::new("get")
                        .about("Get price for a model")
                        .arg(
                            Arg::new("model")
                                .required(true)
                                .help("Model name"),
                        ),
                )
                .subcommand(
                    Command::new("delete")
                        .about("Delete price for a model")
                        .arg(
                            Arg::new("model")
                                .required(true)
                                .help("Model name"),
                        ),
                ),
        );

    let matches = app.try_get_matches_from(
        std::iter::once("burncloud".to_string()).chain(args.iter().cloned()),
    )?;

    let config_manager = ConfigManager::new("config.json".to_string())?;
    let mut model_manager = ModelManager::new(config_manager.get_models_dir().to_string());

    match matches.subcommand() {
        Some(("pull", sub_m)) => {
            let model = sub_m.get_one::<String>("model").unwrap();
            model_manager.pull_model(model).await?;
        }
        Some(("run", sub_m)) => {
            let model = sub_m.get_one::<String>("model").unwrap();
            let prompt = sub_m.get_one::<String>("prompt");

            if prompt.is_none() {
                println!("进入交互模式，输入 'exit' 退出:");
                loop {
                    print!("> ");
                    io::stdout().flush()?;

                    let mut input = String::new();
                    io::stdin().read_line(&mut input)?;
                    let input = input.trim();

                    if input == "exit" {
                        break;
                    }

                    if !input.is_empty() {
                        let response = model_manager.run_model(model, Some(input)).await?;
                        println!("{}", response);
                    }
                }
            } else {
                let response = model_manager
                    .run_model(model, prompt.map(|s| s.as_str()))
                    .await?;
                println!("{}", response);
            }
        }
        Some(("list", _)) => {
            let models = model_manager.list_models();
            if models.is_empty() {
                println!("没有找到已下载的模型");
            } else {
                println!("已下载的模型:");
                for model in models {
                    println!("  {} ({}MB)", model.name, model.size / 1024 / 1024);
                }
            }
        }
        Some(("update", sub_m)) => {
            let check_only = sub_m.get_flag("check-only");
            let res = tokio::task::spawn_blocking(move || handle_update_command(check_only)).await;
            match res {
                Ok(Ok(())) => {}
                Ok(Err(e)) => return Err(e),
                Err(e) => return Err(anyhow::anyhow!(format!("更新线程失败: {:?}", e))),
            }
        }
        Some(("channel", sub_m)) => {
            let db = Database::new().await?;
            handle_channel_command(&db, sub_m).await?;
            db.close().await?;
        }
        Some(("price", sub_m)) => {
            let db = Database::new().await?;
            handle_price_command(&db, sub_m).await?;
            db.close().await?;
        }
        _ => {
            show_help();
        }
    }

    Ok(())
}

/// 处理更新命令（使用同步版本避免运行时冲突）
fn handle_update_command(check_only: bool) -> Result<()> {
    info!("初始化自动更新器...");

    let updater = AutoUpdater::with_default_config();

    if check_only {
        println!("检查更新中...");
        match updater.sync_check_for_updates() {
            Ok(true) => {
                println!("✅ 发现新版本可用！");
                println!("运行 'burncloud update' 来更新到最新版本");
            }
            Ok(false) => {
                println!("✅ 已是最新版本");
            }
            Err(e) => {
                error!("检查更新失败: {}", e);
                println!("❌ 检查更新失败: {}", e);
                let (github_url, gitee_url) = updater.get_download_links();
                println!("你可以手动从以下地址下载最新版本:");
                println!("  GitHub: {}", github_url);
                println!("  Gitee:  {}", gitee_url);
                return Err(anyhow::anyhow!("检查更新失败: {}", e));
            }
        }
    } else {
        println!("正在更新 BurnCloud...");
        match updater.sync_update() {
            Ok(_) => {
                println!("✅ 更新成功！");
                println!("请重新启动应用程序以使用新版本");
            }
            Err(e) => {
                error!("更新失败: {}", e);
                println!("❌ 更新失败: {}", e);
                let (github_url, gitee_url) = updater.get_download_links();
                println!("你可以手动从以下地址下载最新版本:");
                println!("  GitHub: {}", github_url);
                println!("  Gitee:  {}", gitee_url);
                return Err(anyhow::anyhow!("更新失败: {}", e));
            }
        }
    }

    Ok(())
}

pub fn show_help() {
    println!("BurnCloud - AI模型部署和管理平台");
    println!();
    println!("用法:");
    println!("  burncloud                     - 启动GUI (Windows) / 显示帮助 (Linux)");
    println!("  burncloud client              - 启动GUI客户端");
    println!("  burncloud server              - 启动服务器");
    println!("  burncloud code                - 编程模式");
    println!("  burncloud pull <model>        - 下载模型");
    println!("  burncloud run <model>         - 运行模型");
    println!("  burncloud list                - 列出模型");
    println!("  burncloud update              - 更新应用程序");
    println!("  burncloud update --check-only - 仅检查更新");
    println!();
    println!("示例:");
    println!("  burncloud client");
    println!("  burncloud pull llama3.2");
    println!("  burncloud run gemma3");
    println!("  burncloud update --check-only");
}
