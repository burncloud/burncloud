use std::{env, error::Error, fs, path::PathBuf};

use burncloud_node_runtime::{AcceleratorKind, AcceleratorProfile, HardwareProfile};
use burncloud_supply_model::{
    CandidateResolutionOutcome, ModelManifest, RealModelResolver, Variant,
};

struct Arguments {
    manifest: Option<PathBuf>,
    model: String,
    ram_bytes: u64,
    vram_bytes: Vec<u64>,
    disk_bytes: u64,
    runtimes: Vec<String>,
}

fn usage() -> &'static str {
    "用法: cargo run -p burncloud-supply-model --example phase_three -- \
[--manifest <manifest.json>] --model <model> --ram-bytes <bytes> \
--vram-bytes <bytes[,bytes...]> --disk-bytes <bytes> --runtime <backend> [--runtime <backend>]"
}

fn next_value<I>(args: &mut I, option: &str) -> Result<String, Box<dyn Error>>
where
    I: Iterator<Item = String>,
{
    args.next()
        .ok_or_else(|| format!("参数 {option} 缺少值\n{}", usage()).into())
}

fn parse_u64(value: String, option: &str) -> Result<u64, Box<dyn Error>> {
    value
        .parse::<u64>()
        .map_err(|error| format!("参数 {option} 不是有效的非负整数: {error}").into())
}

fn parse_arguments() -> Result<Arguments, Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let mut manifest = None;
    let mut model = None;
    let mut ram_bytes = None;
    let mut vram_bytes = None;
    let mut disk_bytes = None;
    let mut runtimes = Vec::new();

    while let Some(option) = args.next() {
        match option.as_str() {
            "--manifest" => manifest = Some(PathBuf::from(next_value(&mut args, &option)?)),
            "--model" => model = Some(next_value(&mut args, &option)?),
            "--ram-bytes" => {
                ram_bytes = Some(parse_u64(next_value(&mut args, &option)?, &option)?);
            }
            "--vram-bytes" => {
                let value = next_value(&mut args, &option)?;
                let values = value
                    .split(',')
                    .map(|item| {
                        if item.trim().is_empty() {
                            return Err(format!("参数 {option} 包含空的显存值").into());
                        }
                        parse_u64(item.trim().to_owned(), &option)
                    })
                    .collect::<Result<Vec<_>, Box<dyn Error>>>()?;
                vram_bytes = Some(values);
            }
            "--disk-bytes" => {
                disk_bytes = Some(parse_u64(next_value(&mut args, &option)?, &option)?);
            }
            "--runtime" => {
                let runtime = next_value(&mut args, &option)?;
                if runtime.trim().is_empty() {
                    return Err(format!("参数 {option} 不能是空字符串").into());
                }
                runtimes.push(runtime);
            }
            "--help" | "-h" => {
                println!("{}", usage());
                std::process::exit(0);
            }
            _ => return Err(format!("未知参数: {option}\n{}", usage()).into()),
        }
    }

    Ok(Arguments {
        manifest,
        model: model.ok_or_else(|| format!("缺少 --model\n{}", usage()))?,
        ram_bytes: ram_bytes.ok_or_else(|| format!("缺少 --ram-bytes\n{}", usage()))?,
        vram_bytes: vram_bytes.ok_or_else(|| format!("缺少 --vram-bytes\n{}", usage()))?,
        disk_bytes: disk_bytes.ok_or_else(|| format!("缺少 --disk-bytes\n{}", usage()))?,
        runtimes: if runtimes.is_empty() {
            return Err(format!("至少需要一个 --runtime\n{}", usage()).into());
        } else {
            runtimes
        },
    })
}

fn demo_manifests() -> Vec<ModelManifest> {
    vec![ModelManifest {
        model_name: "phase-three-model".into(),
        description: "阶段三内置示例模型".into(),
        supported_backends: vec!["llama.cpp".into(), "vllm".into()],
        version: "1".into(),
        variants: vec![
            Variant {
                id: "phase-three-q4".into(),
                model_size: Some("7b".into()),
                quantization: "Q4_K_M".into(),
                backend: "llama.cpp".into(),
                min_ram_bytes: 8 * 1024 * 1024 * 1024,
                min_vram_bytes: 4 * 1024 * 1024 * 1024,
                disk_size_bytes: 5 * 1024 * 1024 * 1024,
                checksum: "sha256:phase-three-q4".into(),
                artifact_uri: "https://example.invalid/phase-three-q4.gguf".into(),
            },
            Variant {
                id: "phase-three-q8".into(),
                model_size: Some("7b".into()),
                quantization: "Q8_0".into(),
                backend: "llama.cpp".into(),
                min_ram_bytes: 16 * 1024 * 1024 * 1024,
                min_vram_bytes: 8 * 1024 * 1024 * 1024,
                disk_size_bytes: 10 * 1024 * 1024 * 1024,
                checksum: "sha256:phase-three-q8".into(),
                artifact_uri: "https://example.invalid/phase-three-q8.gguf".into(),
            },
        ],
    }]
}

fn main() -> Result<(), Box<dyn Error>> {
    let arguments = parse_arguments()?;
    let manifests = match arguments.manifest {
        Some(path) => {
            let manifest_json = fs::read_to_string(path)?;
            serde_json::from_str(&manifest_json)?
        }
        None => demo_manifests(),
    };
    let resolver = RealModelResolver::new(manifests);
    let hardware = HardwareProfile {
        cpu_threads: 0,
        memory_bytes: arguments.ram_bytes,
        disk_available_bytes: arguments.disk_bytes,
        accelerators: arguments
            .vram_bytes
            .iter()
            .map(|memory_bytes| AcceleratorProfile {
                kind: AcceleratorKind::Other,
                name: "cli-accelerator".into(),
                memory_bytes: Some(*memory_bytes),
            })
            .collect(),
    };

    match resolver.resolve_with_capabilities(&arguments.model, &hardware, &arguments.runtimes)? {
        CandidateResolutionOutcome::Candidates(candidates) => {
            println!("兼容候选: {}", candidates.len());
            for (index, candidate) in candidates.iter().enumerate() {
                println!(
                    "{}. model={} quantization={} runtime={} min_ram_bytes={} min_vram_bytes={} disk_size_bytes={} artifact_uri={} checksum={}",
                    index + 1,
                    candidate.model_name,
                    candidate.quantization,
                    candidate.runtime,
                    candidate.min_ram_bytes,
                    candidate.min_vram_bytes,
                    candidate.disk_size_bytes,
                    candidate.artifact_uri,
                    candidate.checksum,
                );
            }
        }
        CandidateResolutionOutcome::Unsupported(unsupported) => {
            println!(
                "不支持: model={} reason={:?}",
                unsupported.model, unsupported.reason
            );
        }
    }

    Ok(())
}
