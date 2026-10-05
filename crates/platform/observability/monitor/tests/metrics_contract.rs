#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "Test-only file: the assertions are the test."
)]
//! Metric units, the freshness boundary, and the collectors' shape (#633, plan item 25).
//!
//! The crate was ~1000 lines with **no tests**. The plan names six behaviours and adds two constraints that
//! shape this file:
//!
//! > 可控时钟/样本用于确定性校验 -- a controllable clock and sample, for deterministic checks
//! > 真实采集只验证形状与合理范围，不断言固定 CPU/内存数值 -- for real collection, verify **shape and plausible
//! > range** only; do not assert fixed CPU or memory numbers
//!
//! So the deterministic properties are asserted exactly, and the live collectors are asserted for shape and
//! range -- **no test below contains a machine-specific number**, and none would fail on a different machine, at
//! a different load, or with a different number of cores.
//!
//! ## Two defects this found
//!
//! **`format_size` divided by 1024 and labelled the results `KB`, `MB`, `GB`, `TB`.** Those are the **decimal**
//! units; the binary ones are `KiB`, `MiB`, `GiB`, `TiB`. `1024` bytes printed as `1.0 KB` is off by 2.4%, and
//! the error compounds with each step. Fixed here, and the arithmetic is pinned so the divisor and the label
//! cannot drift apart again.
//!
//! **The cache's freshness check subtracted two `u64`s.** `now - metrics.timestamp` underflows when the clock
//! moves **backwards** -- an NTP correction, a VM restored from a snapshot -- which panics in a debug build and
//! wraps to a near-`u64::MAX` age in a release build, making the cache look infinitely stale so every call
//! re-collects. It is now `is_fresh`, which treats a future timestamp as not fresh, and the boundary is tested
//! from both sides.

use burncloud_service_monitor::collectors::usage_percent;
use burncloud_service_monitor::{
    is_fresh, CpuCollector, CpuInfo, DiskCollector, DiskInfo, MemoryCollector, MemoryInfo,
    SystemMetrics, SystemMonitorService,
};

// -------------------------------------------------------------------------------------------
// unit conversion
// -------------------------------------------------------------------------------------------

#[test]
fn a_byte_count_is_formatted_in_binary_units_and_labelled_as_binary() {
    // The divisor is 1024, so the labels must be the binary ones. The previous labels were `KB`/`MB`/`GB`/`TB`,
    // which are decimal, so the number and the unit disagreed by 2.4% at every step -- and a reader comparing
    // against a disk vendor's "1 TB" would be out by 10%.
    let cases: [(u64, &str); 12] = [
        (0, "0 B"),
        (1, "1 B"),
        (1023, "1023 B"),  // one below the first step stays in bytes
        (1024, "1.0 KiB"), // 1024 bytes is 1 KiB, not 1 KB
        (1536, "1.5 KiB"),
        (1024 * 1024, "1.0 MiB"),
        (1024 * 1024 * 1024, "1.0 GiB"),
        (1024_u64.pow(4), "1.0 TiB"),
        // The last unit is the largest, so a larger count keeps growing rather than moving on.
        (1024_u64.pow(5), "1024.0 TiB"),
        // Rounding is one decimal place, and it truncates rather than rounding half up.
        (1024 + 51, "1.0 KiB"),
        (1024 + 52, "1.1 KiB"),
        (u64::MAX, "16777216.0 TiB"),
    ];

    for (bytes, expected) in cases {
        let got = MemoryInfo::format_size(bytes);
        println!("{bytes} -> {got}");
        assert_eq!(got, expected, "{bytes} bytes");
    }

    // And the labels carry the `i`, which is the whole point of the fix. Asserted separately from the numbers so
    // a change that reverted only the labels is caught even if the arithmetic stayed.
    for (bytes, unit) in [
        (1024_u64, "KiB"),
        (1024 * 1024, "MiB"),
        (1024 * 1024 * 1024, "GiB"),
        (1024_u64.pow(4), "TiB"),
    ] {
        let formatted = MemoryInfo::format_size(bytes);
        assert!(
            formatted.ends_with(unit),
            "{bytes} bytes must be labelled {unit}, got {formatted}"
        );
    }
}

#[test]
fn the_formatted_accessors_agree_with_the_raw_counts() {
    // `used_formatted` and `total_formatted` are what a caller displays, so they must be the formatting of the
    // fields they name rather than of each other.
    let info = MemoryInfo::new(
        8 * 1024 * 1024 * 1024,
        3 * 1024 * 1024 * 1024,
        5 * 1024 * 1024 * 1024,
    );
    assert_eq!(info.total_formatted(), "8.0 GiB");
    assert_eq!(info.used_formatted(), "3.0 GiB");
    assert_ne!(
        info.used_formatted(),
        info.total_formatted(),
        "the two accessors read different fields"
    );
}

// -------------------------------------------------------------------------------------------
// zero totals and impossible inputs
// -------------------------------------------------------------------------------------------

#[test]
fn a_zero_total_is_zero_percent_rather_than_a_division_by_zero() {
    // The plan's "零总量". A machine with no swap, an unmounted device and a container with a zero-size overlay
    // all produce a zero total, and the percentage must be zero rather than `NaN`.
    assert_eq!(usage_percent(0, 0), 0.0, "zero used of zero total");
    assert_eq!(
        usage_percent(1, 0),
        0.0,
        "a non-zero used with a zero total is still zero"
    );
    assert_eq!(usage_percent(u64::MAX, 0), 0.0);
    assert!(
        usage_percent(1, 0).is_finite(),
        "the result must be a number, not NaN or an infinity"
    );

    // The same through the type's own constructor, which is what the collectors call.
    let empty = MemoryInfo::new(0, 0, 0);
    assert_eq!(empty.usage_percent, 0.0);
    assert!(empty.usage_percent.is_finite());

    let inconsistent = MemoryInfo::new(0, 1024, 0);
    assert_eq!(
        inconsistent.usage_percent, 0.0,
        "a total of zero is zero percent whatever `used` says"
    );
}

#[test]
fn a_used_larger_than_the_total_is_clamped_rather_than_exceeding_one_hundred() {
    // `used` and `total` come from separate fields of a platform structure, so nothing guarantees `used <=
    // total`. The unclamped division produced values above 100, which a gauge or a progress bar renders as
    // overflowing and which a threshold check reads as "definitely full" at any level.
    assert_eq!(usage_percent(200, 100), 100.0, "clamped at the ceiling");
    assert_eq!(usage_percent(u64::MAX, 1), 100.0);
    assert!(
        usage_percent(u64::MAX, 1) <= 100.0,
        "a percentage of a whole cannot exceed 100"
    );

    // And the ordinary cases are unchanged.
    assert_eq!(usage_percent(50, 100), 50.0);
    assert_eq!(usage_percent(0, 100), 0.0);
    assert_eq!(usage_percent(100, 100), 100.0);
    assert_eq!(usage_percent(1, 3), 33.333332, "a third, at f32 precision");

    // The type's constructor applies its own arithmetic and is **not** clamped -- it divides without one. That is
    // recorded rather than changed: the constructor is fed by the collectors, and clamping there would hide a
    // platform reporting more used than installed. `usage_percent` is what the collectors now use.
    let over = MemoryInfo::new(100, 200, 0);
    println!(
        "MemoryInfo::new with used > total gives usage_percent = {}",
        over.usage_percent
    );
    assert!(
        over.usage_percent > 100.0,
        "the constructor does not clamp, which is why the collectors call `usage_percent` instead"
    );
}

// -------------------------------------------------------------------------------------------
// the freshness boundary, with a controllable clock
// -------------------------------------------------------------------------------------------

#[test]
fn the_freshness_boundary_is_exact_from_both_sides() {
    // The plan's "缓存有效/过期" and "缓存 TTL 边界". The comparison is `<`, so a timestamp exactly `ttl` old is
    // **stale** and one a second younger is fresh. Both sides are asserted, because a boundary tested from one
    // side only is satisfied by an off-by-one.
    let now = 1_000_000_u64;
    let ttl = 10_u64;

    assert!(is_fresh(now, now, ttl), "collected at `now` is fresh");
    assert!(is_fresh(now - 1, now, ttl), "one second old");
    assert!(
        is_fresh(now - 9, now, ttl),
        "one second inside the boundary"
    );
    assert!(
        !is_fresh(now - 10, now, ttl),
        "exactly `ttl` old is stale: the comparison is `<`, as it was before the extraction"
    );
    assert!(
        !is_fresh(now - 11, now, ttl),
        "and older than that is stale"
    );

    // A zero TTL means nothing is fresh, including a timestamp from this very second.
    assert!(!is_fresh(now, now, 0), "a zero TTL expires immediately");
    assert!(!is_fresh(now - 1, now, 0));

    // A very large TTL keeps an old value fresh, and the arithmetic does not overflow on the way.
    assert!(
        is_fresh(0, now, u64::MAX),
        "a TTL of u64::MAX accepts any past timestamp"
    );
}

#[test]
fn a_timestamp_in_the_future_is_stale_rather_than_fresh_or_a_panic() {
    // **The defect the extraction fixed.** `now - collected_at` on `u64` underflows when `collected_at` is ahead
    // of `now` -- a clock that stepped backwards, a VM restored from a snapshot, metrics read from another
    // process whose clock differs. In a debug build the original expression **panicked**; this test would have
    // failed the whole run. In a release build it wrapped and the cache looked infinitely stale.
    //
    // The chosen reading is **stale**: a timestamp ahead of the clock cannot be evidence that the data is
    // recent. Treating it as age zero would pin a stale value in the cache until the clock caught up.
    let now = 1_000_000_u64;
    for ahead in [1_u64, 10, 1_000, u64::MAX - now] {
        let collected_at = now + ahead;
        let fresh = is_fresh(collected_at, now, 10);
        println!("collected {ahead} seconds in the future -> fresh: {fresh}");
        assert!(
            !fresh,
            "a timestamp {ahead} seconds ahead of the clock must not be treated as fresh"
        );
    }

    // The extreme case, so the subtraction is exercised at the widest possible gap: the original expression
    // would have underflowed by nearly the whole range.
    assert!(!is_fresh(u64::MAX, 0, u64::MAX));
    assert!(!is_fresh(u64::MAX, 0, 10));
}

#[test]
fn metrics_can_carry_a_chosen_timestamp_and_still_default_to_the_clock() {
    // "可控时钟/样本": the injectable constructor is what makes the boundary testable end to end, and `new` must
    // still read the clock so the public behaviour is unchanged.
    let cpu = CpuInfo {
        usage_percent: 12.5,
        core_count: 4,
        frequency: 2400,
        brand: "test".to_string(),
    };
    let memory = MemoryInfo::new(1024, 512, 512);
    let disks = Vec::new();

    let chosen = SystemMetrics::with_timestamp(cpu.clone(), memory.clone(), disks.clone(), 42);
    assert_eq!(chosen.timestamp, 42, "the timestamp is the one given");

    let from_clock = SystemMetrics::new(cpu, memory, disks);
    assert!(
        from_clock.timestamp > 1_700_000_000,
        "`new` reads the clock, so its timestamp is a plausible Unix time: {}",
        from_clock.timestamp
    );

    // And the two feed the same decision, which is the point of the injection.
    assert!(
        is_fresh(chosen.timestamp, 42 + 5, 10),
        "the injected timestamp is fresh five seconds later"
    );
    assert!(
        !is_fresh(chosen.timestamp, 42 + 10, 10),
        "and stale at the boundary"
    );
}

// -------------------------------------------------------------------------------------------
// the collectors: shape and range only
// -------------------------------------------------------------------------------------------

#[test]
fn collected_metrics_have_plausible_shape_and_ranges() {
    // **No machine-specific number appears in this test.** The plan is explicit: for real collection, verify the
    // shape and a plausible range, and do not assert fixed CPU or memory values. Nothing below would fail on a
    // laptop, a 64-core server or a container.
    let runtime = tokio::runtime::Runtime::new().expect("a runtime");
    let metrics = runtime.block_on(async {
        let service = SystemMonitorService::new();
        service.get_metrics().await
    });

    let metrics = match metrics {
        Ok(m) => m,
        Err(e) => {
            // A container without `/proc` or without a loadable platform API can legitimately fail; the shape of
            // the failure is all that can be asserted there.
            println!("collection failed on this host, which is allowed: {e}");
            return;
        }
    };

    // The timestamp is a Unix time near now, not a default or a zero.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("the clock is after the epoch")
        .as_secs();
    assert!(
        metrics.timestamp <= now + 5,
        "the collection timestamp must not be in the future by more than a small tolerance: {} vs {now}",
        metrics.timestamp
    );
    assert!(
        metrics.timestamp > now - 300,
        "nor minutes in the past: {} vs {now}",
        metrics.timestamp
    );

    // CPU: a percentage in range and a plausible core count.
    assert!(
        (0.0..=100.0).contains(&metrics.cpu.usage_percent),
        "CPU usage must be a percentage, got {}",
        metrics.cpu.usage_percent
    );
    assert!(!metrics.cpu.usage_percent.is_nan(), "and not NaN");
    assert!(
        (1..=4096).contains(&metrics.cpu.core_count),
        "a core count between 1 and 4096, got {}",
        metrics.cpu.core_count
    );

    // Memory: the fields are self-consistent rather than fixed. `used <= total` is the invariant a caller relies
    // on; `available <= total` likewise.
    let mem = &metrics.memory;
    println!(
        "memory: {} used of {} ({} available), {}%",
        mem.used_formatted(),
        mem.total_formatted(),
        MemoryInfo::format_size(mem.available),
        mem.usage_percent
    );
    assert!(
        mem.total > 0,
        "a machine with no memory at all is not a case to test here"
    );
    assert!(
        mem.used <= mem.total,
        "used must not exceed total: {} > {}",
        mem.used,
        mem.total
    );
    assert!(
        mem.available <= mem.total,
        "available must not exceed total: {} > {}",
        mem.available,
        mem.total
    );
    assert!(
        (0.0..=100.0).contains(&mem.usage_percent),
        "memory usage must be a percentage, got {}",
        mem.usage_percent
    );

    // Disks: each entry is self-consistent, and the collection is allowed to be empty (a container with no
    // mounted filesystem the API reports).
    for disk in &metrics.disks {
        assert!(
            disk.used <= disk.total,
            "disk {} reports {} used of {}",
            disk.mount_point,
            disk.used,
            disk.total
        );
        assert!(
            disk.available <= disk.total,
            "disk {} reports {} available of {}",
            disk.mount_point,
            disk.available,
            disk.total
        );
        assert!(
            (0.0..=100.0).contains(&disk.usage_percent),
            "disk {} usage must be a percentage, got {}",
            disk.mount_point,
            disk.usage_percent
        );
        assert!(
            !disk.mount_point.is_empty(),
            "every disk names a mount point"
        );
    }
    println!("collected {} disk(s)", metrics.disks.len());
}

#[tokio::test]
async fn the_individual_collectors_agree_with_the_service() {
    // The service composes the three collectors, so a collector that returned something structurally different
    // from what the service assembles would be a second definition of the same data.
    let mut cpu_collector = CpuCollector::new();
    let cpu = cpu_collector.collect().await;
    if let Ok(cpu) = &cpu {
        assert!((0.0..=100.0).contains(&cpu.usage_percent));
        assert!(cpu.core_count >= 1);
    }

    let memory_collector = MemoryCollector::new();
    let memory = memory_collector.collect().await;
    if let Ok(memory) = &memory {
        assert!(memory.used <= memory.total);
        assert!((0.0..=100.0).contains(&memory.usage_percent));
    }

    let disk_collector = DiskCollector::new();
    let disks = disk_collector.collect_all().await;
    if let Ok(disks) = &disks {
        for disk in disks {
            assert!(disk.used <= disk.total);
            assert!((0.0..=100.0).contains(&disk.usage_percent));
        }
    }

    // A zero-size disk built by hand must render as a zero percentage, which is the case a container overlay
    // produces and the case `usage_percent` exists for.
    let empty = DiskInfo {
        total: 0,
        used: 0,
        available: 0,
        usage_percent: usage_percent(0, 0),
        mount_point: "/nowhere".to_string(),
    };
    assert_eq!(empty.usage_percent, 0.0);
    assert_eq!(MemoryInfo::format_size(empty.total), "0 B");
}

#[tokio::test]
async fn a_cached_reading_is_returned_again_without_collecting() {
    // The cache path end to end: two calls in quick succession return the same timestamp, which is the observable
    // consequence of the second being served from the cache rather than re-collected.
    let service = SystemMonitorService::new();
    let first = match service.get_metrics().await {
        Ok(m) => m,
        Err(e) => {
            println!("collection failed on this host, which is allowed: {e}");
            return;
        }
    };
    let second = service.get_metrics().await.expect("the second call");

    println!("first {} second {}", first.timestamp, second.timestamp);
    assert_eq!(
        first.timestamp, second.timestamp,
        "the second call within the update interval must be served from the cache; a different timestamp means \
         it collected again"
    );

    // `refresh_metrics` is the documented way to bypass the cache, and it must produce a fresh reading.
    let refreshed = service.refresh_metrics().await.expect("a refresh");
    assert!(
        refreshed.timestamp >= first.timestamp,
        "a refresh must not move the timestamp backwards: {} then {}",
        first.timestamp,
        refreshed.timestamp
    );
}
