//! MN-01: System Monitoring Types Tests (P2)
//!
//! Tests for verifying system metrics data structures.
//!
//! Key Scenarios:
//! - Memory info calculation and formatting
//! - CPU info structure
//! - Disk info structure
//! - System metrics aggregation
//! - JSON serialization/deserialization

use burncloud_service_monitor::{CpuInfo, DiskInfo, MemoryInfo, SystemMetrics};

/// Test: MemoryInfo creation and calculation
#[test]
fn test_memory_info_creation() {
    let mem = MemoryInfo::new(16_000_000_000, 8_000_000_000, 8_000_000_000); // 16GB total, 8GB used

    assert_eq!(mem.total, 16_000_000_000);
    assert_eq!(mem.used, 8_000_000_000);
    assert_eq!(mem.available, 8_000_000_000);
    assert!((mem.usage_percent - 50.0).abs() < 0.1);
}

/// Test: MemoryInfo zero usage
#[test]
fn test_memory_info_zero_usage() {
    let mem = MemoryInfo::new(16_000_000_000, 0, 16_000_000_000);

    assert_eq!(mem.used, 0);
    assert_eq!(mem.available, 16_000_000_000);
    assert!((mem.usage_percent - 0.0).abs() < f32::EPSILON);
}

/// Test: MemoryInfo full usage
#[test]
fn test_memory_info_full_usage() {
    let mem = MemoryInfo::new(16_000_000_000, 16_000_000_000, 0);

    assert_eq!(mem.used, 16_000_000_000);
    assert_eq!(mem.available, 0);
    assert!((mem.usage_percent - 100.0).abs() < 0.1);
}

/// Test: MemoryInfo format_size function
#[test]
fn test_memory_format_size() {
    // Bytes
    assert_eq!(MemoryInfo::format_size(500), "500 B");

    // Kilobytes
    assert_eq!(MemoryInfo::format_size(1024), "1.0 KB");
    assert_eq!(MemoryInfo::format_size(1536), "1.5 KB");

    // Megabytes
    assert_eq!(MemoryInfo::format_size(1_048_576), "1.0 MB");
    assert_eq!(MemoryInfo::format_size(100_000_000), "95.4 MB");

    // Gigabytes
    assert_eq!(MemoryInfo::format_size(1_073_741_824), "1.0 GB");
    assert_eq!(MemoryInfo::format_size(16_000_000_000), "14.9 GB");
}

/// Test: MemoryInfo formatted methods
#[test]
fn test_memory_formatted_methods() {
    let mem = MemoryInfo::new(16_000_000_000, 8_000_000_000, 8_000_000_000);

    let used = mem.used_formatted();
    let total = mem.total_formatted();

    assert!(used.contains("GB"));
    assert!(total.contains("GB"));
}

/// Test: MemoryInfo serialization
#[test]
fn test_memory_info_serialization() {
    let mem = MemoryInfo::new(8_000_000_000, 4_000_000_000, 4_000_000_000);

    let json = serde_json::to_string(&mem).expect("Should serialize");
    let deserialized: MemoryInfo =
        serde_json::from_str(&json).expect("Should deserialize");

    assert_eq!(mem.total, deserialized.total);
    assert_eq!(mem.used, deserialized.used);
    assert_eq!(mem.available, deserialized.available);
}

/// Test: CpuInfo structure
#[test]
fn test_cpu_info_structure() {
    let cpu = CpuInfo {
        usage_percent: 45.5,
        core_count: 8,
        frequency: 3200,
        brand: "Intel Core i7".to_string(),
    };

    assert!((cpu.usage_percent - 45.5).abs() < f32::EPSILON);
    assert_eq!(cpu.core_count, 8);
    assert_eq!(cpu.frequency, 3200);
    assert_eq!(cpu.brand, "Intel Core i7");
}

/// Test: CpuInfo serialization
#[test]
fn test_cpu_info_serialization() {
    let cpu = CpuInfo {
        usage_percent: 75.0,
        core_count: 16,
        frequency: 4500,
        brand: "AMD Ryzen 9".to_string(),
    };

    let json = serde_json::to_string(&cpu).expect("Should serialize");
    assert!(json.contains("\"usage_percent\":75.0"));
    assert!(json.contains("\"core_count\":16"));
    assert!(json.contains("\"frequency\":4500"));
    assert!(json.contains("\"brand\":\"AMD Ryzen 9\""));
}

/// Test: CpuInfo usage range
#[test]
fn test_cpu_usage_range() {
    // Test valid range
    let valid_cpu = CpuInfo {
        usage_percent: 50.0,
        core_count: 4,
        frequency: 2400,
        brand: String::new(),
    };
    assert!(valid_cpu.usage_percent >= 0.0 && valid_cpu.usage_percent <= 100.0);

    // Test idle
    let idle_cpu = CpuInfo {
        usage_percent: 0.0,
        core_count: 4,
        frequency: 2400,
        brand: String::new(),
    };
    assert!((idle_cpu.usage_percent - 0.0).abs() < f32::EPSILON);

    // Test full load
    let full_cpu = CpuInfo {
        usage_percent: 100.0,
        core_count: 4,
        frequency: 2400,
        brand: String::new(),
    };
    assert!((full_cpu.usage_percent - 100.0).abs() < f32::EPSILON);
}

/// Test: DiskInfo structure
#[test]
fn test_disk_info_structure() {
    let disk = DiskInfo {
        total: 500_000_000_000, // 500GB
        used: 250_000_000_000,  // 250GB
        available: 250_000_000_000,
        usage_percent: 50.0,
        mount_point: "/".to_string(),
    };

    assert_eq!(disk.total, 500_000_000_000);
    assert_eq!(disk.used, 250_000_000_000);
    assert_eq!(disk.available, 250_000_000_000);
    assert!((disk.usage_percent - 50.0).abs() < f32::EPSILON);
    assert_eq!(disk.mount_point, "/");
}

/// Test: DiskInfo serialization
#[test]
fn test_disk_info_serialization() {
    let disk = DiskInfo {
        total: 1_000_000_000_000,
        used: 750_000_000_000,
        available: 250_000_000_000,
        usage_percent: 75.0,
        mount_point: "/mnt/data".to_string(),
    };

    let json = serde_json::to_string(&disk).expect("Should serialize");
    let deserialized: DiskInfo =
        serde_json::from_str(&json).expect("Should deserialize");

    assert_eq!(disk.total, deserialized.total);
    assert_eq!(disk.used, deserialized.used);
    assert_eq!(disk.available, deserialized.available);
    assert_eq!(disk.mount_point, deserialized.mount_point);
}

/// Test: Multiple disks
#[test]
fn test_multiple_disks() {
    let disks = vec![
        DiskInfo {
            total: 500_000_000_000,
            used: 100_000_000_000,
            available: 400_000_000_000,
            usage_percent: 20.0,
            mount_point: "/".to_string(),
        },
        DiskInfo {
            total: 1_000_000_000_000,
            used: 800_000_000_000,
            available: 200_000_000_000,
            usage_percent: 80.0,
            mount_point: "/home".to_string(),
        },
    ];

    assert_eq!(disks.len(), 2);

    // Find root disk
    let root = disks.iter().find(|d| d.mount_point == "/").unwrap();
    assert_eq!(root.usage_percent, 20.0);

    // Find home disk
    let home = disks.iter().find(|d| d.mount_point == "/home").unwrap();
    assert_eq!(home.usage_percent, 80.0);
}

/// Test: SystemMetrics structure
#[test]
fn test_system_metrics_structure() {
    let cpu = CpuInfo {
        usage_percent: 35.0,
        core_count: 8,
        frequency: 3200,
        brand: "Intel Core i7".to_string(),
    };

    let memory = MemoryInfo::new(16_000_000_000, 8_000_000_000, 8_000_000_000);

    let disks = vec![DiskInfo {
        total: 500_000_000_000,
        used: 250_000_000_000,
        available: 250_000_000_000,
        usage_percent: 50.0,
        mount_point: "/".to_string(),
    }];

    let metrics = SystemMetrics::new(cpu.clone(), memory.clone(), disks.clone());

    assert_eq!(metrics.cpu.usage_percent, cpu.usage_percent);
    assert_eq!(metrics.memory.total, memory.total);
    assert_eq!(metrics.disks.len(), 1);
    assert!(metrics.timestamp > 0);
}

/// Test: SystemMetrics timestamp
#[test]
fn test_system_metrics_timestamp() {
    let cpu = CpuInfo {
        usage_percent: 0.0,
        core_count: 1,
        frequency: 1000,
        brand: String::new(),
    };
    let memory = MemoryInfo::new(0, 0, 0);
    let disks = vec![];

    let before = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let metrics = SystemMetrics::new(cpu, memory, disks);

    let after = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    assert!(metrics.timestamp >= before);
    assert!(metrics.timestamp <= after);
}

/// Test: SystemMetrics serialization
#[test]
fn test_system_metrics_serialization() {
    let cpu = CpuInfo {
        usage_percent: 50.0,
        core_count: 4,
        frequency: 2400,
        brand: "Test CPU".to_string(),
    };
    let memory = MemoryInfo::new(8_000_000_000, 4_000_000_000, 4_000_000_000);
    let disks = vec![DiskInfo {
        total: 256_000_000_000,
        used: 128_000_000_000,
        available: 128_000_000_000,
        usage_percent: 50.0,
        mount_point: "/".to_string(),
    }];

    let metrics = SystemMetrics::new(cpu, memory, disks);

    let json = serde_json::to_string(&metrics).expect("Should serialize");
    let deserialized: SystemMetrics =
        serde_json::from_str(&json).expect("Should deserialize");

    assert_eq!(metrics.cpu.core_count, deserialized.cpu.core_count);
    assert_eq!(metrics.memory.total, deserialized.memory.total);
    assert_eq!(metrics.disks.len(), deserialized.disks.len());
    assert_eq!(metrics.timestamp, deserialized.timestamp);
}

/// Test: SystemMetrics JSON structure
#[test]
fn test_system_metrics_json_structure() {
    let cpu = CpuInfo {
        usage_percent: 25.5,
        core_count: 8,
        frequency: 3600,
        brand: "Test".to_string(),
    };
    let memory = MemoryInfo::new(16_000_000_000, 4_000_000_000, 12_000_000_000);
    let disks = vec![];

    let metrics = SystemMetrics::new(cpu, memory, disks);
    let json = serde_json::to_string(&metrics).unwrap();

    // Verify JSON structure
    assert!(json.contains("\"cpu\""));
    assert!(json.contains("\"memory\""));
    assert!(json.contains("\"disks\""));
    assert!(json.contains("\"timestamp\""));
    assert!(json.contains("\"usage_percent\""));
    assert!(json.contains("\"core_count\""));
    assert!(json.contains("\"total\""));
    assert!(json.contains("\"used\""));
}

/// Test: Real-world system metrics scenario
#[test]
fn test_real_world_system_metrics() {
    // Simulate a real server's metrics
    let cpu = CpuInfo {
        usage_percent: 65.5,
        core_count: 32,
        frequency: 2800,
        brand: "AMD EPYC 7542".to_string(),
    };

    let memory = MemoryInfo::new(128_000_000_000, 96_000_000_000, 32_000_000_000);

    let disks = vec![
        DiskInfo {
            total: 2_000_000_000_000, // 2TB
            used: 1_500_000_000_000,  // 1.5TB
            available: 500_000_000_000,
            usage_percent: 75.0,
            mount_point: "/".to_string(),
        },
        DiskInfo {
            total: 4_000_000_000_000, // 4TB
            used: 2_000_000_000_000,  // 2TB
            available: 2_000_000_000_000,
            usage_percent: 50.0,
            mount_point: "/data".to_string(),
        },
    ];

    let metrics = SystemMetrics::new(cpu, memory, disks);

    // Verify CPU
    assert!(metrics.cpu.usage_percent > 50.0);
    assert_eq!(metrics.cpu.core_count, 32);

    // Verify Memory
    assert!((metrics.memory.usage_percent - 75.0).abs() < 0.1);

    // Verify Disks
    assert_eq!(metrics.disks.len(), 2);

    let total_disk_space: u64 = metrics.disks.iter().map(|d| d.total).sum();
    assert_eq!(total_disk_space, 6_000_000_000_000); // 6TB total
}
