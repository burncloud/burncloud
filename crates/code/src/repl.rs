use anyhow::Result;
use std::io::{self, Write};

pub async fn start_cli() -> Result<()> {
    println!("🚀 BurnCloud Code - 编程辅助工具");
    println!("输入 'help' 查看可用命令，输入 'exit' 退出");

    loop {
        print!("burncloud-code> ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        match input {
            "exit" | "quit" => {
                println!("再见！");
                break;
            },
            "help" => {
                show_help();
            },
            "generate" => {
                println!("代码生成功能开发中...");
            },
            "analyze" => {
                println!("代码分析功能开发中...");
            },
            "format" => {
                println!("代码格式化功能开发中...");
            },
            "" => continue,
            _ => {
                println!("未知命令: {}，输入 'help' 查看可用命令", input);
            }
        }
    }

    Ok(())
}

fn show_help() {
    println!("可用命令:");
    println!("  help      - 显示帮助信息");
    println!("  generate  - 代码生成");
    println!("  analyze   - 代码分析");
    println!("  format    - 代码格式化");
    println!("  exit      - 退出程序");
}