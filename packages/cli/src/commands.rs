use clap::{Arg, Command};
use burncloud_core::{ModelManager, ConfigManager};
use anyhow::Result;
use std::io::{self, Write};

pub async fn handle_command(args: &[String]) -> Result<()> {
    let app = Command::new("burncloud")
        .version("0.1.0")
        .about("AI模型部署和管理平台")
        .subcommand_required(false)
        .subcommand(
            Command::new("pull")
                .about("下载模型")
                .arg(Arg::new("model").required(true).help("模型名称"))
        )
        .subcommand(
            Command::new("run")
                .about("运行模型")
                .arg(Arg::new("model").required(true).help("模型名称"))
                .arg(Arg::new("prompt").help("输入提示"))
        )
        .subcommand(
            Command::new("list")
                .about("列出已下载的模型")
        )
        .subcommand(
            Command::new("server")
                .about("启动服务器模式")
        );

    let matches = app.try_get_matches_from(std::iter::once("burncloud".to_string()).chain(args.iter().cloned()))?;

    let config_manager = ConfigManager::new("config.json".to_string())?;
    let mut model_manager = ModelManager::new(config_manager.get_models_dir().to_string());

    match matches.subcommand() {
        Some(("pull", sub_m)) => {
            let model = sub_m.get_one::<String>("model").unwrap();
            model_manager.pull_model(model).await?;
        },
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
                let response = model_manager.run_model(model, prompt.map(|s| s.as_str())).await?;
                println!("{}", response);
            }
        },
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
        },
        _ => {
            show_help();
        }
    }

    Ok(())
}

pub fn show_help() {
    println!("BurnCloud - AI模型部署和管理平台");
    println!("");
    println!("用法:");
    println!("  burncloud                 - 启动GUI (Windows) / 显示帮助 (Linux)");
    println!("  burncloud client          - 启动GUI客户端");
    println!("  burncloud server          - 启动服务器");
    println!("  burncloud code            - 编程模式");
    println!("  burncloud pull <model>    - 下载模型");
    println!("  burncloud run <model>     - 运行模型");
    println!("  burncloud list            - 列出模型");
    println!("");
    println!("示例:");
    println!("  burncloud client");
    println!("  burncloud pull llama3.2");
    println!("  burncloud run gemma3");
}