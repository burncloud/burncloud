#![allow(dead_code)]

use burncloud_common::ModelInfo;

pub fn format_model_list(models: &[&ModelInfo]) {
    if models.is_empty() {
        println!("No models found");
        return;
    }

    println!("{:<20} {:<10} {:<10}", "Name", "Size", "Status");
    println!("{}", "-".repeat(40));

    for model in models {
        let size_str = format!("{}MB", model.size / 1024 / 1024);
        let status = if model.downloaded {
            "Downloaded"
        } else {
            "Not downloaded"
        };
        println!("{:<20} {:<10} {:<10}", model.name, size_str, status);
    }
}

pub fn print_success(message: &str) {
    println!("✓ {}", message);
}

pub fn print_error(message: &str) {
    eprintln!("✗ {}", message);
}

pub fn print_info(message: &str) {
    println!("ℹ {}", message);
}
