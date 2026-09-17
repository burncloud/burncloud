//! Traffic-owned preference between local Node and external Provider routes.
//!
//! Node may publish a READY local channel, but Node does not decide whether
//! that channel wins a request. That choice belongs to Traffic/Scheduler.

/// High-level preference applied before the existing scheduler ranks channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoutePreference {
    /// Keep the current scheduler behavior across all candidates.
    ExistingPolicy,
    /// Prefer local candidates when any are available; otherwise use external candidates.
    LocalFirst,
}

/// Where a routable candidate comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteOrigin {
    Local,
    External,
}

/// Select which origin class may enter normal scheduler ranking.
///
/// This does not select a concrete channel. Health, cost, RPM, weight,
/// affinity and failover remain responsibilities of the existing scheduler.
pub fn preferred_origins(
    preference: RoutePreference,
    has_local_candidate: bool,
    has_external_candidate: bool,
) -> &'static [RouteOrigin] {
    const BOTH: &[RouteOrigin] = &[RouteOrigin::Local, RouteOrigin::External];
    const LOCAL: &[RouteOrigin] = &[RouteOrigin::Local];
    const EXTERNAL: &[RouteOrigin] = &[RouteOrigin::External];
    const NONE: &[RouteOrigin] = &[];

    match preference {
        RoutePreference::ExistingPolicy => BOTH,
        RoutePreference::LocalFirst if has_local_candidate => LOCAL,
        RoutePreference::LocalFirst if has_external_candidate => EXTERNAL,
        RoutePreference::LocalFirst => NONE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn existing_policy_keeps_both_origin_classes() {
        assert_eq!(
            preferred_origins(RoutePreference::ExistingPolicy, true, true),
            &[RouteOrigin::Local, RouteOrigin::External]
        );
    }

    #[test]
    fn local_first_prefers_local_when_available() {
        assert_eq!(
            preferred_origins(RoutePreference::LocalFirst, true, true),
            &[RouteOrigin::Local]
        );
    }

    #[test]
    fn local_first_falls_back_to_external_when_local_is_absent() {
        assert_eq!(
            preferred_origins(RoutePreference::LocalFirst, false, true),
            &[RouteOrigin::External]
        );
    }

    #[test]
    fn local_first_returns_none_when_no_candidate_exists() {
        assert!(preferred_origins(RoutePreference::LocalFirst, false, false).is_empty());
    }
}
