mod error;
mod hardware_detector;
mod hardware_profile;
mod detectors;
mod probe;
#[cfg(test)]
mod tests;

pub use error::HardwareError;
pub use hardware_detector::{detect, CachedHardwareProfile};
pub use probe::{FakeHardwareFactsProbe, HardwareFactsProbe, RealHardwareFactsProbe};
pub use hardware_profile::{CpuInfo, DiskInfo, GpuInfo, GpuVendor, HardwareProfile, MemoryInfo, OsInfo};
pub use detectors::{ensure_disk, ensure_ram, ensure_runtime_compatible, ensure_vram, max_available_nvidia_vram_bytes, parse_df_output, parse_meminfo, parse_nvidia_smi_output, parse_vm_stat};
