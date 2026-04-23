//! L3 Affinity layer — Rendezvous Hash + dual-TTL flow cache.
//!
//! Maps `(affinity_key, model)` → `channel_id` so consecutive requests from
//! the same user land on the same upstream, preserving KV cache and reducing
//! cost variance.
//!
//! See [`docs/code/GLOSSARY.md`](../../docs/code/GLOSSARY.md) § 5 for the
//! Rendezvous (HRW / Highest Random Weight) algorithm — chosen over consistent
//! hashing because the candidate set is small (≤ 20) and changes frequently.
//!
//! # Dual TTL ("粘而不僵")
//!
//! - **Sticky TTL** (default 5 min): inside this window, [`AffinityCache::lookup`]
//!   returns the cached channel even if HRW would now pick something else.
//! - **Hard TTL** (default 30 min): past this window the entry is forcibly
//!   re-evaluated. Caps the worst-case staleness when channel weights change.
//!
//! # Failure eviction (circuit-breaker联动)
//!
//! When a request to the affined channel fails (5xx, timeout, 429), the caller
//! must invoke [`AffinityCache::evict`] so the next request goes back through
//! HRW. This keeps affinity from pinning users to a sick channel.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::{Duration, Instant};

use dashmap::DashMap;

/// Default sticky TTL — within this window the cached channel is always returned.
pub const DEFAULT_STICKY_TTL: Duration = Duration::from_secs(5 * 60);
/// Default hard TTL — entry is re-evaluated past this point.
pub const DEFAULT_HARD_TTL: Duration = Duration::from_secs(30 * 60);

/// Compound cache key: `(affinity_key, model)` — both required because the
/// same user routes independently per model.
type CacheKey = (String, String);

/// Cache entry — channel id + creation time (for TTL bookkeeping).
#[derive(Debug, Clone, Copy)]
struct CacheEntry {
    channel_id: i32,
    created_at: Instant,
}

/// Lock-free flow cache (DashMap-backed) with dual TTL.
///
/// Use [`AffinityCache::with_ttls`] to override defaults for tests.
pub struct AffinityCache {
    entries: DashMap<CacheKey, CacheEntry>,
    sticky_ttl: Duration,
    hard_ttl: Duration,
}

impl Default for AffinityCache {
    fn default() -> Self {
        Self::with_ttls(DEFAULT_STICKY_TTL, DEFAULT_HARD_TTL)
    }
}

impl AffinityCache {
    /// Construct with explicit TTLs (panics if `sticky > hard`).
    pub fn with_ttls(sticky_ttl: Duration, hard_ttl: Duration) -> Self {
        assert!(
            sticky_ttl <= hard_ttl,
            "sticky_ttl must be <= hard_ttl, got sticky={:?} hard={:?}",
            sticky_ttl,
            hard_ttl
        );
        Self {
            entries: DashMap::new(),
            sticky_ttl,
            hard_ttl,
        }
    }

    /// Look up the affined channel id for `(key, model)`.
    ///
    /// Returns:
    /// - `Some(id)` if a sticky-fresh entry exists
    /// - `None` if no entry, or the entry is past `hard_ttl` (entry is removed)
    /// - `None` if past `sticky_ttl` but within `hard_ttl` — caller should
    ///   re-pick via HRW; if HRW picks the same channel, [`Self::insert`] keeps
    ///   it warm. (We don't return the stale id directly because past sticky
    ///   the affinity hint is "soft".)
    pub fn lookup(&self, key: &str, model: &str) -> Option<i32> {
        let compound = (key.to_string(), model.to_string());
        let entry = self.entries.get(&compound)?;
        let age = entry.created_at.elapsed();
        if age > self.hard_ttl {
            drop(entry);
            self.entries.remove(&compound);
            return None;
        }
        if age > self.sticky_ttl {
            return None;
        }
        Some(entry.channel_id)
    }

    /// Insert or refresh an affinity entry for `(key, model) → channel_id`.
    pub fn insert(&self, key: &str, model: &str, channel_id: i32) {
        let compound = (key.to_string(), model.to_string());
        self.entries.insert(
            compound,
            CacheEntry {
                channel_id,
                created_at: Instant::now(),
            },
        );
    }

    /// Evict the entry for `(key, model)`. Used by failover so a sick channel
    /// isn't re-affined on the next request.
    pub fn evict(&self, key: &str, model: &str) {
        let compound = (key.to_string(), model.to_string());
        self.entries.remove(&compound);
    }

    /// Approximate live entry count (DashMap len is approximate under concurrency).
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the cache holds zero entries (lock-free, approximate).
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Pick a channel for `key` using Rendezvous (HRW) hashing.
///
/// `score(ch_i, key) = hash(key, ch_i.id) × weight_i × health_i`
///
/// - `candidates` is the post-OrderType-filter pool with admin weights.
/// - `health_of` returns each channel's health score in [0, 1]; channels with
///   `health == 0.0` are excluded so a dead candidate cannot win HRW.
///
/// Returns `None` only if the candidate pool is empty or every candidate has
/// zero health.
pub fn pick_hrw<F>(
    key: &str,
    candidates: &[(burncloud_common::types::Channel, i32)],
    health_of: F,
) -> Option<i32>
where
    F: Fn(i32) -> f64,
{
    let mut best: Option<(i32, f64)> = None;
    for (ch, weight) in candidates {
        let health = health_of(ch.id);
        if health <= 0.0 {
            continue;
        }
        let h = mix_hash(key, ch.id);
        // Map u64 hash → (0, 1] uniform float, then weight × health.
        let r = (h as f64 + 1.0) / (u64::MAX as f64 + 1.0);
        let score = r * (*weight as f64).max(1.0) * health;
        match best {
            Some((_, best_score)) if best_score >= score => {}
            _ => best = Some((ch.id, score)),
        }
    }
    best.map(|(id, _)| id)
}

fn mix_hash(key: &str, channel_id: i32) -> u64 {
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    channel_id.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::scheduler::tests::make_channel;

    #[test]
    fn hrw_is_deterministic() {
        let cands = vec![make_channel(1, 1), make_channel(2, 1), make_channel(3, 1)];
        let pick1 = pick_hrw("user-A", &cands, |_| 1.0);
        let pick2 = pick_hrw("user-A", &cands, |_| 1.0);
        assert_eq!(pick1, pick2);
        assert!(pick1.is_some());
    }

    #[test]
    fn hrw_skips_dead_candidates() {
        let cands = vec![make_channel(1, 100), make_channel(2, 1)];
        // Channel 1 has 100x weight but is dead; HRW must pick channel 2.
        let pick = pick_hrw("user-A", &cands, |id| if id == 1 { 0.0 } else { 1.0 });
        assert_eq!(pick, Some(2));
    }

    #[test]
    fn hrw_returns_none_on_empty() {
        let pick = pick_hrw("user-A", &[], |_| 1.0);
        assert_eq!(pick, None);
    }

    #[test]
    fn hrw_returns_none_when_all_dead() {
        let cands = vec![make_channel(1, 1), make_channel(2, 1)];
        let pick = pick_hrw("user-A", &cands, |_| 0.0);
        assert_eq!(pick, None);
    }

    #[test]
    fn cache_round_trip() {
        let cache = AffinityCache::default();
        cache.insert("user-A", "glm-5.1", 42);
        assert_eq!(cache.lookup("user-A", "glm-5.1"), Some(42));
    }

    #[test]
    fn cache_separates_per_model() {
        let cache = AffinityCache::default();
        cache.insert("user-A", "glm-5.1", 1);
        cache.insert("user-A", "claude", 2);
        assert_eq!(cache.lookup("user-A", "glm-5.1"), Some(1));
        assert_eq!(cache.lookup("user-A", "claude"), Some(2));
    }

    #[test]
    fn cache_evict_removes_entry() {
        let cache = AffinityCache::default();
        cache.insert("user-A", "glm-5.1", 7);
        cache.evict("user-A", "glm-5.1");
        assert_eq!(cache.lookup("user-A", "glm-5.1"), None);
    }

    #[test]
    fn sticky_ttl_returns_cached_value_then_drops() {
        let cache = AffinityCache::with_ttls(
            Duration::from_millis(50),
            Duration::from_secs(60),
        );
        cache.insert("u", "m", 9);
        assert_eq!(cache.lookup("u", "m"), Some(9));
        std::thread::sleep(Duration::from_millis(80));
        // Past sticky but within hard → returns None (caller re-picks via HRW).
        assert_eq!(cache.lookup("u", "m"), None);
    }

    #[test]
    fn hard_ttl_removes_entry() {
        let cache = AffinityCache::with_ttls(
            Duration::from_millis(20),
            Duration::from_millis(40),
        );
        cache.insert("u", "m", 9);
        std::thread::sleep(Duration::from_millis(60));
        let _ = cache.lookup("u", "m");
        assert_eq!(cache.len(), 0, "hard TTL should physically remove entry");
    }

    #[test]
    #[should_panic(expected = "sticky_ttl must be <= hard_ttl")]
    fn ttls_must_be_ordered() {
        let _ = AffinityCache::with_ttls(Duration::from_secs(60), Duration::from_secs(30));
    }
}
