mod common;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
mod unsupported;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
pub use self::linux::SystemHardwareProbe;
#[cfg(target_os = "macos")]
pub use self::macos::SystemHardwareProbe;
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
pub use self::unsupported::SystemHardwareProbe;
#[cfg(target_os = "windows")]
pub use self::windows::SystemHardwareProbe;

impl Default for SystemHardwareProbe {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl crate::HardwareProbe for SystemHardwareProbe {
    async fn inspect(&self) -> Result<crate::HardwareProfile, crate::HardwareProbeError> {
        SystemHardwareProbe::inspect(self).await
    }
}

#[cfg(test)]
mod tests;
