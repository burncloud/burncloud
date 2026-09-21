#[derive(Debug, Clone, PartialEq)]
pub struct HardwareProfile { pub os: OsInfo, pub cpu: CpuInfo, pub memory: MemoryInfo, pub gpu: Vec<GpuInfo>, pub disk: DiskInfo }
#[derive(Debug, Clone, PartialEq)]
pub struct OsInfo { pub name: String, pub version: Option<String>, pub arch: String }
#[derive(Debug, Clone, PartialEq)]
pub struct CpuInfo { pub arch: String, pub physical_cores: Option<u32>, pub logical_cores: Option<u32>, pub instruction_sets: Vec<String> }
#[derive(Debug, Clone, PartialEq)]
pub struct MemoryInfo { pub total_mb: Option<u64>, pub available_mb: Option<u64> }
#[derive(Debug, Clone, PartialEq)]
pub struct GpuInfo { pub vendor: GpuVendor, pub model: String, pub vram_total_mb: Option<u64>, pub vram_free_mb: Option<u64>, pub driver_version: Option<String>, pub diagnostic: Option<String> }
#[derive(Debug, Clone, PartialEq)]
pub enum GpuVendor { Nvidia, Amd, Apple, Intel, Unknown }
#[derive(Debug, Clone, PartialEq)]
pub struct DiskInfo { pub model_cache_path: String, pub total_mb: Option<u64>, pub available_mb: Option<u64> }

// For all optional numeric fields, `None` means the value was not measured,
// the command was unavailable, or it could not be parsed reliably.
