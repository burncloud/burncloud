use burncloud_tool_monitor::{SystemMonitorService, SystemMonitor};

fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 BurnCloud Tool Monitor Example");
    println!("=====================================");

    // 创建监控服务
    let monitor = SystemMonitorService::new();

    // 启动自动更新
    monitor.start_auto_update().await?;

    // 获取系统指标
    println!("📊 Getting system metrics...");
    let metrics = monitor.get_system_metrics().await?;

    // 显示CPU信息
    println!("\n🖥️  CPU Information:");
    println!("   Brand: {}", metrics.cpu.brand);
    println!("   Cores: {}", metrics.cpu.core_count);
    println!("   Frequency: {} MHz", metrics.cpu.frequency);
    println!("   Usage: {:.1}%", metrics.cpu.usage_percent);

    // 显示内存信息
    println!("\n💾 Memory Information:");
    println!("   Total: {}", metrics.memory.total_formatted());
    println!("   Used: {}", metrics.memory.used_formatted());
    println!("   Usage: {:.1}%", metrics.memory.usage_percent);

    // 显示磁盘信息
    println!("\n💿 Disk Information:");
    for (i, disk) in metrics.disks.iter().enumerate() {
        println!("   Disk {}: {} ({:.1}% used)",
                i + 1,
                disk.mount_point,
                disk.usage_percent);
        println!("      Total: {} | Used: {} | Available: {}",
                format_bytes(disk.total),
                format_bytes(disk.used),
                format_bytes(disk.available));
    }

    println!("\n✅ Tool monitor test completed!");

    Ok(())
}