pub mod cpu;
pub mod disk;
pub mod memory;

pub use cpu::CpuCollector;
pub use disk::DiskCollector;
pub use memory::{DetailedMemoryInfo, MemoryCollector};

/// A usage percentage, clamped to `0.0..=100.0`, with a zero total reported as zero.
///
/// Extracted so the two edge cases the crate's plan names -- a **zero total** and a total the platform reports
/// inconsistently -- are decided in one place and can be tested without a disk. The previous form was inline in
/// each collector as `if total > 0 { used / total * 100 } else { 0.0 }`, which handled the zero but not a `used`
/// larger than `total`: the unclamped division then exceeded 100.
///
/// A `used` greater than `total` is reachable, because the numbers come from separate fields of a platform
/// structure and need not agree.
pub fn usage_percent(used: u64, total: u64) -> f32 {
    if total == 0 {
        return 0.0;
    }
    let percent = (used as f64 / total as f64 * 100.0) as f32;
    percent.clamp(0.0, 100.0)
}
