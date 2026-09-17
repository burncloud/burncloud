//! Traffic-owned preference between local Node and external Provider routes.
//!
//! Node may publish a READY local channel, but Node must never decide whether
//! that channel wins a request. Preference is routing policy owned here.

/// High-level preference applied before the existing scheduler ranks candidates
/// inside the preferred class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoutePreference {
    /// Preserve existing scheduler behavior across all candidates.
    ExistingPolicy,
    /// Prefer healthy/routable local Node candidates when any exist; otherwise
    /// fall back to external Provider candidates.
    LocalFirst,
}

/// Candidate origin is routing metadata, not Node state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteOrigin {
    Local,
    External,
}

/// Decide which candidate class may enter normal scheduler ranking.
///
/// This does not pick a concrete channel. Health, cost, RPM, weight, affinity,
/// failover and the final channel choice remain responsibilities of the
/// existing Traffic scheduler.
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
    fn existing_policy_does_not_force_local_or_external() {
        assert_eq!(
            preferred_origins(RoutePreference::ExistingPolicy, true, true),
            &[RouteOrigin::Local, RouteOrigin::External]
        );
    }

    #[test]
    fn local_first_selects_local_class_when_local_candidate_exists() {
        assert_eq!(
            preferred_origins(RoutePreference::LocalFirst, true, true),
            &[RouteOrigin::Local]
        );
    }

    #[test]
    fn local_first_falls_back_to_external_when_local_candidate_is_absent() {
        assert_eq!(
            preferred_origins(RoutePreference::LocalFirst, false, true),
            &[RouteOrigin::External]
        );
    }

    #[test]
    fn local_first_returns_no_class_when_no_candidate_exists() {
        assert!(preferred_origins(RoutePreference::LocalFirst, false, false).is_empty());
    }
}
