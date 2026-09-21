use super::detectors::{cache_path, disk, parse_meminfo, parse_nvidia_smi_output, parse_vm_stat};
use super::{CpuInfo, DiskInfo, GpuInfo, GpuVendor, HardwareError, HardwareProfile, MemoryInfo, OsInfo};
use std::process::Command;
use std::time::SystemTime;

fn output(program:&str,args:&[&str])->Result<String,HardwareError>{let o=Command::new(program).args(args).output().map_err(HardwareError::IoError)?;if !o.status.success(){return Err(HardwareError::CommandExecutionFailed(String::from_utf8_lossy(&o.stderr).trim().into()))}Ok(String::from_utf8_lossy(&o.stdout).into())}

fn os_version(os_name: &str) -> Option<String> {
    match os_name {
        "windows" => {
            let script = r#"$o=Get-CimInstance Win32_OperatingSystem; if($o){"$($o.Caption) ($($o.Version))"}"#;
            output("powershell", &["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", script]).ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
        }
        "macos" => output("sw_vers", &["-productVersion"]).ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()),
        "linux" => std::fs::read_to_string("/etc/os-release").ok().and_then(|s| s.lines().find_map(|line| line.strip_prefix("PRETTY_NAME=")).map(|v| v.trim_matches('"').to_string())),
        _ => None,
    }
}
fn parse_windows_memory(input: &str) -> (Option<u64>, Option<u64>) {
    let values: Vec<u64> = input.trim().split(',').filter_map(|v| v.trim().parse().ok()).collect();
    if values.len() == 2 { (Some(values[0] / 1024), Some(values[1] / 1024)) } else { (None, None) }
}

fn windows_memory() -> (Option<u64>, Option<u64>) {
    let script = r#"$m=Get-CimInstance Win32_OperatingSystem; if($m){"$($m.TotalVisibleMemorySize),$($m.FreePhysicalMemory)"}"#;
    output("powershell", &["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", script])
        .map(|s| parse_windows_memory(&s)).unwrap_or((None, None))
}

fn memory()->Result<MemoryInfo,HardwareError>{if cfg!(target_os="linux"){let s=std::fs::read_to_string("/proc/meminfo")?;let (t,a)=parse_meminfo(&s)?;Ok(MemoryInfo{total_mb:t,available_mb:a})}else if cfg!(target_os="macos"){let t=output("sysctl",&["-n","hw.memsize"]).ok().and_then(|s|s.trim().parse::<u64>().ok()).map(|v|v/(1024*1024));let page=output("sysctl",&["-n","hw.pagesize"]).ok().and_then(|s|s.trim().parse::<u64>().ok()).unwrap_or(4096);let a=output("vm_stat",&[]).ok().and_then(|s|parse_vm_stat(&s,page).ok());Ok(MemoryInfo{total_mb:t,available_mb:a})}else if cfg!(target_os="windows"){let (t,a)=windows_memory();Ok(MemoryInfo{total_mb:t,available_mb:a})}else{Ok(MemoryInfo{total_mb:None,available_mb:None})}}
fn gpu()->Result<Vec<GpuInfo>,HardwareError>{match Command::new("nvidia-smi").args(["--query-gpu=name,memory.total,memory.free,driver_version","--format=csv,noheader,nounits"]).output(){Ok(o) if o.status.success()=>parse_nvidia_smi_output(&String::from_utf8_lossy(&o.stdout)),Ok(o)=>Err(HardwareError::GpuDriverUnavailable(String::from_utf8_lossy(&o.stderr).trim().into())),Err(e) if e.kind()==std::io::ErrorKind::NotFound=>Ok(Vec::new()),Err(e)=>Err(e.into())}}
fn windows_cpu() -> (Option<u32>, Vec<String>) {
    let cores_script = r#"(Get-CimInstance Win32_Processor | Measure-Object -Property NumberOfCores -Sum).Sum"#;
    let physical = output("powershell", &["-NoProfile", "-NonInteractive", "-Command", cores_script]).ok().and_then(|s| s.trim().parse().ok());
    let feature_script = r#"Add-Type -TypeDefinition 'using System; using System.Runtime.InteropServices; public static class NativeCpu { [DllImport("kernel32.dll")] public static extern bool IsProcessorFeaturePresent(uint f); }'; $f=@(@(6,'sse'),@(10,'sse2'),@(13,'sse3'),@(36,'ssse3'),@(38,'sse4.1'),@(40,'avx'),@(41,'avx2'),@(42,'avx512f')); foreach($x in $f){if([NativeCpu]::IsProcessorFeaturePresent($x[0])){$x[1]}}"#;
    let features = output("powershell", &["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", feature_script]).ok().map(|s| s.lines().map(str::trim).filter(|s| !s.is_empty()).map(String::from).collect()).unwrap_or_default();
    (physical, features)
}

pub fn detect()->Result<HardwareProfile,HardwareError>{let os_name=if cfg!(target_os="windows"){"windows"}else if cfg!(target_os="macos"){"macos"}else if cfg!(target_os="linux"){"linux"}else{"unknown"};let logical=std::thread::available_parallelism().ok().map(|n|n.get() as u32);let (physical,instruction_sets)=if cfg!(target_os="windows"){windows_cpu()}else{(logical,Vec::new())};let cpu=CpuInfo{arch:std::env::consts::ARCH.into(),physical_cores:physical,logical_cores:logical,instruction_sets};let path=cache_path();let (total,available)=disk(&path);Ok(HardwareProfile{os:OsInfo{name:os_name.into(),version:os_version(os_name),arch:std::env::consts::ARCH.into()},cpu,memory:memory()?,gpu:gpu()?,disk:DiskInfo{model_cache_path:path,total_mb:total,available_mb:available}})}
#[derive(Debug,Clone)] pub struct CachedHardwareProfile{profile:HardwareProfile,static_detected_at:SystemTime}
impl CachedHardwareProfile{pub fn detect_once()->Result<Self,HardwareError>{Ok(Self{profile:detect()?,static_detected_at:SystemTime::now()})}pub fn profile(&self)->&HardwareProfile{&self.profile}pub fn refresh_dynamic(&mut self)->Result<(),HardwareError>{let fresh=detect()?;self.profile.memory.available_mb=fresh.memory.available_mb;self.profile.disk.available_mb=fresh.disk.available_mb;for g in &mut self.profile.gpu{if g.vendor==GpuVendor::Nvidia{if let Some(n)=fresh.gpu.iter().find(|x|x.vendor==GpuVendor::Nvidia&&x.model==g.model){g.vram_free_mb=n.vram_free_mb;}}}Ok(())}pub fn static_detected_at(&self)->SystemTime{self.static_detected_at}}
