use super::{detect, HardwareError, HardwareProfile};
use async_trait::async_trait;

#[async_trait]
pub trait HardwareFactsProbe: Send + Sync {
    async fn inspect_full(&self) -> Result<HardwareProfile, HardwareError>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct RealHardwareFactsProbe;

#[async_trait]
impl HardwareFactsProbe for RealHardwareFactsProbe {
    async fn inspect_full(&self) -> Result<HardwareProfile, HardwareError> { detect() }
}

#[derive(Debug, Clone)]
pub struct FakeHardwareFactsProbe { profile: HardwareProfile }

impl FakeHardwareFactsProbe {
    pub fn new(profile: HardwareProfile) -> Self { Self { profile } }
}

#[async_trait]
impl HardwareFactsProbe for FakeHardwareFactsProbe {
    async fn inspect_full(&self) -> Result<HardwareProfile, HardwareError> { Ok(self.profile.clone()) }
}
