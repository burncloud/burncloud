use super::{HardwareError, HardwareProfile, GpuVendor};
use std::process::Command;

fn command(program: &str, args: &[&str]) -> Result<String, HardwareError> {
    let out = Command::new(program).args(args).output().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound { HardwareError::GpuDriverUnavailable(format!("{program} not found")) } else if e.kind() == std::io::ErrorKind::PermissionDenied { HardwareError::DevicePermissionDenied(format!("{program}: {e}")) } else { HardwareError::CommandExecutionFailed(format!("{program}: {e}")) }
    })?;
    if !out.status.success() { return Err(HardwareError::CommandExecutionFailed(format!("{program}: {}", String::from_utf8_lossy(&out.stderr).trim()))); }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

pub fn parse_meminfo(input: &str) -> Result<(Option<u64>, Option<u64>), HardwareError> {
    let mut total=None; let mut available=None;
    for line in input.lines() { let mut p=line.split_whitespace(); let key=p.next().unwrap_or(""); let value=p.next(); if let Some(v)=value { let n=v.parse::<u64>().map_err(|_| HardwareError::ParseError(format!("invalid {key}")))? * 1024 / (1024*1024); match key { "MemTotal:"=>total=Some(n), "MemAvailable:"=>available=Some(n), _=>{} } } }
    Ok((total, available))
}
pub fn parse_df_output(input: &str) -> Result<(u64,u64), HardwareError> { let line=input.lines().skip(1).find(|l| !l.trim().is_empty()).ok_or_else(|| HardwareError::ParseError("df output is empty".into()))?; let p:Vec<_>=line.split_whitespace().collect(); if p.len()<5 { return Err(HardwareError::ParseError("invalid df row".into())); } let nums=p.iter().filter_map(|x| x.parse::<u64>().ok()).collect::<Vec<_>>(); if nums.len()<3 { return Err(HardwareError::ParseError("df numeric columns missing".into())); } Ok((nums[0], nums[2])) }
pub fn parse_vm_stat(input: &str, page_size: u64) -> Result<u64, HardwareError> { let mut pages=0u64; for line in input.lines() { if line.contains("Pages free") || line.contains("Pages inactive") { if let Some(v)=line.split(':').nth(1) { pages=pages.saturating_add(v.trim().trim_end_matches('.').replace('.', "").parse::<u64>().map_err(|_| HardwareError::ParseError("invalid vm_stat page count".into()))?); } } } Ok(pages.saturating_mul(page_size)/(1024*1024)) }
pub fn parse_nvidia_smi_output(input: &str) -> Result<Vec<super::GpuInfo>, HardwareError> { let mut out=Vec::new(); for line in input.lines().filter(|l| !l.trim().is_empty()) { let p:Vec<_>=line.split(',').map(str::trim).collect(); if p.len()!=4 { out.push(super::GpuInfo{vendor:GpuVendor::Unknown,model:"Unknown NVIDIA GPU".into(),vram_total_mb:None,vram_free_mb:None,driver_version:None,diagnostic:Some("NVIDIA CSV parse failed".into())}); continue; } let parse=|s:&str| s.parse::<u64>().ok(); out.push(super::GpuInfo{vendor:GpuVendor::Nvidia,model:p[0].into(),vram_total_mb:parse(p[1]),vram_free_mb:parse(p[2]),driver_version:Some(p[3].into()),diagnostic:None}); } Ok(out) }

pub fn ensure_vram(required_mb:u64, available_mb:Option<u64>)->Result<(),HardwareError>{match available_mb{Some(v) if v>=required_mb=>Ok(()),Some(v)=>Err(HardwareError::InsufficientVram{required_mb,available_mb:v}),None=>Err(HardwareError::SystemInfoError("VRAM not measured".into()))}}
pub fn ensure_ram(required_mb:u64, available_mb:Option<u64>)->Result<(),HardwareError>{match available_mb{Some(v) if v>=required_mb=>Ok(()),Some(v)=>Err(HardwareError::InsufficientRam{required_mb,available_mb:v}),None=>Err(HardwareError::SystemInfoError("RAM not measured".into()))}}
pub fn ensure_disk(required_mb:u64, available_mb:Option<u64>)->Result<(),HardwareError>{match available_mb{Some(v) if v>=required_mb=>Ok(()),Some(v)=>Err(HardwareError::InsufficientDisk{required_mb,available_mb:v}),None=>Err(HardwareError::SystemInfoError("disk not measured".into()))}}
pub fn ensure_runtime_compatible(runtime:&str,compatible:bool,reason:&str)->Result<(),HardwareError>{if compatible{Ok(())}else{Err(HardwareError::RuntimeIncompatible{runtime:runtime.into(),reason:reason.into()})}}
pub fn max_available_nvidia_vram_bytes(profile:&HardwareProfile)->Option<u64>{profile.gpu.iter().filter(|g|g.vendor==GpuVendor::Nvidia).filter_map(|g|g.vram_free_mb).map(|m|m.saturating_mul(1024*1024)).max()}

pub(crate) fn cache_path()->String { if let Ok(v)=std::env::var("BURNCLOUD_MODEL_CACHE"){return v} if cfg!(windows){std::env::var("LOCALAPPDATA").unwrap_or_else(|_|".".into())+"/BurnCloud/models"}else if cfg!(target_os="macos"){std::env::var("HOME").unwrap_or_else(|_|".".into())+"/Library/Application Support/BurnCloud/models"}else{std::env::var("HOME").unwrap_or_else(|_|".".into())+"/.local/share/burncloud/models"} }
pub(crate) fn disk(path:&str)->(Option<u64>,Option<u64>){if cfg!(unix){if let Ok(s)=command("df", &["-m","-P",path]){if let Ok((t,a))=parse_df_output(&s){return(Some(t),Some(a))}}} (None,None)}
