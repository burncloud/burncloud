#[cfg(test)]
mod tests {
    use super::super::detectors::{ensure_ram, ensure_vram, max_available_nvidia_vram_bytes, parse_df_output, parse_meminfo, parse_nvidia_smi_output, parse_vm_stat};
    use super::super::hardware_profile::{GpuInfo, GpuVendor, HardwareProfile};

    #[test]
    fn parses_linux_meminfo() { assert_eq!(parse_meminfo("MemTotal:       8192 kB\nMemAvailable:   4096 kB\n").unwrap(), (Some(8), Some(4))); }
    #[test]
    fn parses_vm_stat_page_sizes() { let s="Pages free: 10.\nPages inactive: 20."; assert_eq!(parse_vm_stat(s,4096).unwrap(),0); assert_eq!(parse_vm_stat(s,16384).unwrap(),0); }
    #[test]
    fn parses_df_and_nvidia() { assert_eq!(parse_df_output("Filesystem 1024-blocks Used Available Capacity Mounted on\n/dev/x 100 20 80 20% /a path").unwrap(),(100,80)); let g=parse_nvidia_smi_output("RTX,24000,12000,550").unwrap(); assert_eq!(g[0].vram_free_mb,Some(12000)); }
    #[test]
    fn resource_none_is_unknown() { assert!(ensure_ram(1,None).is_err()); assert!(ensure_vram(1,None).is_err()); }
    #[test]
    fn max_gpu_uses_free_vram() { let p=HardwareProfile{os:super::super::hardware_profile::OsInfo{name:"x".into(),version:None,arch:"x".into()},cpu:super::super::hardware_profile::CpuInfo{arch:"x".into(),physical_cores:None,logical_cores:None,instruction_sets:vec![]},memory:super::super::hardware_profile::MemoryInfo{total_mb:None,available_mb:None},gpu:vec![GpuInfo{vendor:GpuVendor::Nvidia,model:"a".into(),vram_total_mb:Some(1),vram_free_mb:Some(2),driver_version:None,diagnostic:None}],disk:super::super::hardware_profile::DiskInfo{model_cache_path:"x".into(),total_mb:None,available_mb:None}}; assert_eq!(max_available_nvidia_vram_bytes(&p),Some(2*1024*1024)); }
}
