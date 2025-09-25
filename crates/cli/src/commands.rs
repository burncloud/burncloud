use anyhow::Result;
use burncloud_auto_update::AutoUpdater;
use burncloud_core::{ConfigManager, ModelManager};
use clap::{Arg, Command};
use log::{error, info};
use std::io::{self, Write};

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
    println!("");
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
    println!("");
    println!("示例:");
    println!("  burncloud client");
    println!("  burncloud pull llama3.2");
    println!("  burncloud run gemma3");
    println!("  burncloud update --check-only");
}
