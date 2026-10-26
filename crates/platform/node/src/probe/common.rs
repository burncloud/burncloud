use crate::{error::HardwareProbeError, types::HardwareProfile};
use sysinfo::{Disks, System};

pub(crate) struct CommonHardware {
    pub cpu: String,
    pub memory_bytes: u64,
    pub disk_available_bytes: u64,
}

pub(crate) fn collect_common() -> Result<CommonHardware, HardwareProbeError> {
    let system = System::new_all();
    let current_dir = std::env::current_dir().map_err(|error| {
        HardwareProbeError::DiskError(format!("cannot read current directory: {error}"))
    })?;
    let disks = Disks::new_with_refreshed_list();
    let disk = disks
        .list()
        .iter()
        .filter(|disk| current_dir.starts_with(disk.mount_point()))
        .max_by_key(|disk| disk.mount_point().components().count())
        .ok_or_else(|| {
            HardwareProbeError::DiskError(format!(
                "no visible disk contains current directory {}",
                current_dir.display()
            ))
        })?;

    let cpu = system
        .cpus()
        .first()
        .map(|cpu| cpu.brand().trim())
        .filter(|brand| !brand.is_empty())
        .unwrap_or("Unknown CPU")
        .to_owned();

    Ok(CommonHardware {
        cpu,
        memory_bytes: system.total_memory(),
        disk_available_bytes: disk.available_space(),
    })
}

impl CommonHardware {
    pub(crate) fn with_accelerators(
        self,
        accelerators: Vec<crate::types::AcceleratorProfile>,
    ) -> HardwareProfile {
        HardwareProfile {
            cpu: self.cpu,
            memory_bytes: self.memory_bytes,
            disk_available_bytes: self.disk_available_bytes,
            accelerators,
        }
    }
}
