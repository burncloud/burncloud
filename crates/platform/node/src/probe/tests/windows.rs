use super::{
    classify_vendor, detect_cim_accelerators, detect_nvidia, parse_adapter_ram, parse_cim_devices,
};
use crate::{
    AcceleratorKind, AcceleratorProfile, HardwareProbe, HardwareProbeError, HardwareProfile,
    SystemHardwareProbe,
};
use serde_json::json;

const GIB: f64 = 1024.0 * 1024.0 * 1024.0;

// 以 GiB 展示字节数，便于人工核对探测结果。
fn format_bytes(bytes: u64) -> String {
    format!("{:.2} GiB ({bytes} bytes)", bytes as f64 / GIB)
}

fn format_accelerators(accelerators: &[AcceleratorProfile]) -> String {
    if accelerators.is_empty() {
        return "  (无)".to_owned();
    }
    accelerators
        .iter()
        .enumerate()
        .map(|(index, gpu)| {
            let memory = gpu
                .memory_bytes
                .map_or_else(|| "未知".to_owned(), format_bytes);
            format!("  [{index}] {:?} | {} | 显存: {memory}", gpu.kind, gpu.name)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn print_profile(title: &str, profile: &HardwareProfile) {
    println!("===== {title} =====");
    println!("CPU: {}", profile.cpu);
    println!("内存: {}", format_bytes(profile.memory_bytes));
    println!(
        "当前目录所在磁盘可用空间: {}",
        format_bytes(profile.disk_available_bytes)
    );
    println!("加速器:\n{}", format_accelerators(&profile.accelerators));
}

// ---------- 白盒：私有解析与分类函数 ----------

#[test]
fn cim_accepts_a_single_controller_and_vendor_fallback() {
    let devices = parse_cim_devices(&json!({
        "Name": "  Integrated Graphics  ",
        "AdapterCompatibility": "Intel Corporation",
        "AdapterRAM": 2147483648_u64
    }));
    assert_eq!(devices.len(), 1);
    assert_eq!(devices[0].kind, AcceleratorKind::Intel);
    assert_eq!(devices[0].name, "Integrated Graphics");
    assert_eq!(devices[0].memory_bytes, Some(2147483648));
}

#[test]
fn cim_skips_duplicates_and_invalid_records_but_keeps_unknown_devices() {
    let devices = parse_cim_devices(&json!([
        {"Name": "NVIDIA RTX", "AdapterRAM": 1024},
        {"Name": "   "},
        {"AdapterRAM": 1024},
        {"Name": "AMD Radeon", "AdapterRAM": 0},
        {"Name": "Example GPU", "AdapterRAM": 4294967295_u64}
    ]));
    assert_eq!(devices.len(), 2);
    assert_eq!(devices[0].kind, AcceleratorKind::Amd);
    assert_eq!(devices[0].memory_bytes, None);
    assert_eq!(devices[1].kind, AcceleratorKind::Other);
    assert_eq!(devices[1].name, "Example GPU");
    assert_eq!(devices[1].memory_bytes, None);
    assert!(parse_cim_devices(&json!(null)).is_empty());
}

#[test]
fn adapter_ram_rejects_missing_zero_and_overflowed_capacity() {
    assert_eq!(parse_adapter_ram(&json!(1073741824)), Some(1073741824));
    for value in [
        json!(null),
        json!(0),
        json!(-1),
        json!(4294967295_u64),
        json!(4294967296_u64),
    ] {
        assert_eq!(parse_adapter_ram(&value), None);
    }
}

#[test]
fn adapter_ram_accepts_values_just_below_overflow_and_rejects_non_integers() {
    assert_eq!(parse_adapter_ram(&json!(1)), Some(1));
    let below_max = u64::from(u32::MAX) - 1;
    assert_eq!(parse_adapter_ram(&json!(below_max)), Some(below_max));
    for value in [
        json!(1.5),
        json!("1024"),
        json!([1024]),
        json!({"bytes": 1024}),
    ] {
        assert_eq!(parse_adapter_ram(&value), None);
    }
}

#[test]
fn vendor_classification_covers_every_branch() {
    assert_eq!(
        classify_vendor("AMD Radeon RX 7900"),
        Some(AcceleratorKind::Amd)
    );
    assert_eq!(
        classify_vendor("Advanced Micro Devices, Inc."),
        Some(AcceleratorKind::Amd)
    );
    assert_eq!(
        classify_vendor("ATI Technologies Inc."),
        Some(AcceleratorKind::Amd)
    );
    assert_eq!(
        classify_vendor("INTEL(R) UHD Graphics"),
        Some(AcceleratorKind::Intel)
    );
    assert_eq!(classify_vendor("Apple"), Some(AcceleratorKind::Apple));
    assert_eq!(classify_vendor("Microsoft Basic Display Adapter"), None);
    assert_eq!(classify_vendor(""), None);
}

#[test]
fn vendor_classification_matches_ati_only_as_a_whole_word() {
    // "Corporation"、"Radiation" 中含有 "ati" 子串，不能被误判为 AMD。
    assert_eq!(classify_vendor("NVIDIA Corporation"), None);
    assert_eq!(classify_vendor("Radiation Labs"), None);
    assert_eq!(classify_vendor("ati-gpu"), Some(AcceleratorKind::Amd));
}

#[test]
fn cim_prefers_vendor_from_name_over_adapter_compatibility() {
    let devices = parse_cim_devices(&json!({
        "Name": "AMD Radeon Graphics",
        "AdapterCompatibility": "Intel Corporation"
    }));
    assert_eq!(devices.len(), 1);
    assert_eq!(devices[0].kind, AcceleratorKind::Amd);
    assert_eq!(devices[0].memory_bytes, None);
}

#[test]
fn cim_skips_nvidia_case_insensitively_and_non_string_names() {
    let devices = parse_cim_devices(&json!([
        {"Name": "nvidia geforce rtx 4090"},
        {"Name": 42},
        {"Name": null},
        {"Name": "Intel Arc", "AdapterCompatibility": 7, "AdapterRAM": "8GB"}
    ]));
    assert_eq!(devices.len(), 1);
    assert_eq!(devices[0].kind, AcceleratorKind::Intel);
    assert_eq!(devices[0].name, "Intel Arc");
    assert_eq!(devices[0].memory_bytes, None);
}

#[test]
fn cim_returns_empty_for_unexpected_top_level_values() {
    for value in [json!([]), json!(1), json!("Intel"), json!(true)] {
        assert!(parse_cim_devices(&value).is_empty());
    }
}

// ---------- 白盒：真实系统接口（结果依赖本机环境） ----------

#[test]
fn real_cim_detection_returns_clean_non_nvidia_devices() {
    match detect_cim_accelerators() {
        Ok(devices) => {
            println!("===== CIM (Win32_VideoController) =====");
            println!("{}", format_accelerators(&devices));
            for device in &devices {
                assert!(!device.name.is_empty());
                assert_eq!(device.name, device.name.trim());
                assert!(!device.name.to_ascii_lowercase().contains("nvidia"));
                assert_ne!(device.kind, AcceleratorKind::Nvidia);
            }
        }
        // CIM 不可用属于允许的降级路径，只要求错误类型正确。
        Err(error) => {
            println!("CIM 探测不可用: {error}");
            assert!(matches!(error, HardwareProbeError::GpuDriverUnavailable(_)));
        }
    }
}

#[test]
fn real_nvml_detection_reports_only_nvidia_devices() {
    match detect_nvidia() {
        Ok(devices) => {
            println!("===== NVML =====");
            println!("{}", format_accelerators(&devices));
            for device in &devices {
                assert_eq!(device.kind, AcceleratorKind::Nvidia);
                assert!(!device.name.is_empty());
            }
        }
        // 未安装 NVIDIA 驱动的机器会走这里，属于允许的降级路径。
        Err(error) => {
            println!("NVML 不可用: {error}");
            assert!(matches!(error, HardwareProbeError::GpuDriverUnavailable(_)));
        }
    }
}

// ---------- 黑盒：只通过公开的 HardwareProbe 接口 ----------

#[tokio::test]
async fn public_probe_reports_real_hardware_profile() {
    let probe: Box<dyn HardwareProbe> = Box::new(SystemHardwareProbe::new());
    let profile = probe.inspect().await.expect("硬件探测失败");

    print_profile("HardwareProbe::inspect 结果", &profile);

    assert!(!profile.cpu.trim().is_empty());
    assert!(profile.memory_bytes > 0);
    for gpu in &profile.accelerators {
        assert!(!gpu.name.trim().is_empty());
        assert_ne!(gpu.memory_bytes, Some(0));
    }
}

#[tokio::test]
async fn public_probe_is_stable_and_serializable() {
    let probe = SystemHardwareProbe::new();
    let first = HardwareProbe::inspect(&probe)
        .await
        .expect("第一次探测失败");
    let second = HardwareProbe::inspect(&probe)
        .await
        .expect("第二次探测失败");

    // 可用磁盘空间会随运行变化，只比较稳定的硬件事实。
    assert_eq!(first.cpu, second.cpu);
    assert_eq!(first.memory_bytes, second.memory_bytes);
    assert_eq!(first.accelerators, second.accelerators);

    let encoded = serde_json::to_string_pretty(&first).expect("序列化失败");
    println!("===== HardwareProfile JSON =====\n{encoded}");
    let decoded: HardwareProfile = serde_json::from_str(&encoded).expect("反序列化失败");
    assert_eq!(decoded, first);
}
