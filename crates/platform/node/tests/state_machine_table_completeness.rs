#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "Test-only file: the assertions are the test."
)]
//! The illegal half of the Node state machine's transition table, completed (#635, scope
//! `node-state-machine-coverage`).
//!
//! `tests/state_machine_table.rs` compares every ordered pair against a transcribed edge table, and has a
//! test that a refused transition leaves the machine where it was. That second test cannot build a machine
//! in `Failed`, `Unhealthy` or `LocalUnsupported`: its route array holds only the forward rail, so it
//! `continue`s over those three states. Its comment says their pairs are covered "by the direct
//! construction below", but no such construction follows it. 27 of the 121 illegal pairs were therefore
//! never asserted -- the half of "所有合法/非法状态转移" that the plan's first acceptance condition names.
//!
//! This file closes that gap rather than editing the sibling file's assertions:
//!
//! 1. every state is reached by a legal route found with breadth-first search over the machine's own
//!    `can_transition_to`, so no state can be skipped silently, and each route is asserted to have arrived;
//! 2. for every state, every illegal target must be refused, must leave the state unchanged, and must be
//!    reported with both ends -- with the pair counts asserted, so coverage is a number, not a claim;
//! 3. the failure closure is asserted at path level: no route from a failure state reaches `Ready` or
//!    `Routable` without passing the readiness stage again, while the same search with that stage removed
//!    does reach them, so the property is falsifiable rather than vacuous.
//!
//! Everything here is the fake skeleton: pure state logic, no process, no network and no filesystem. The
//! production adapters (`ProcessManager` and friends against a real machine) are not exercised here.

use std::collections::VecDeque;

use burncloud_node_runtime::{NodeState, NodeStateMachine};

/// Every state, transcribed by hand because `NodeState` deliberately has no iterator.
///
/// The count is asserted below, so a variant added to the enum without being added here fails the suite
/// rather than quietly reducing the matrix this file claims to cover.
const ALL_STATES: [NodeState; 12] = [
    NodeState::Absent,
    NodeState::Resolving,
    NodeState::LocalUnsupported,
    NodeState::PreparingArtifact,
    NodeState::ArtifactReady,
    NodeState::PreparingRuntime,
    NodeState::Starting,
    NodeState::WaitingReady,
    NodeState::Ready,
    NodeState::Routable,
    NodeState::Failed,
    NodeState::Unhealthy,
];

/// The two states that mean the workload is not healthy, and the one that means the machine decided it
/// cannot serve at all. These are the three the sibling file's route array cannot reach.
const OFF_RAIL_STATES: [NodeState; 3] = [
    NodeState::Failed,
    NodeState::Unhealthy,
    NodeState::LocalUnsupported,
];

/// The shortest legal route from `Absent` to `target`, searched over the machine's own edge predicate.
///
/// `None` means no legal route exists; every caller asserts rather than skips, which is the difference
/// between this file and the `continue` it is written to replace.
fn legal_route(target: NodeState) -> Option<Vec<NodeState>> {
    let mut queue = VecDeque::from([(NodeState::Absent, Vec::new())]);
    let mut seen = vec![NodeState::Absent];
    while let Some((state, path)) = queue.pop_front() {
        if state == target {
            return Some(path);
        }
        for next in ALL_STATES {
            if !state.can_transition_to(next) || seen.contains(&next) {
                continue;
            }
            seen.push(next);
            let mut extended = path.clone();
            extended.push(next);
            queue.push_back((next, extended));
        }
    }
    None
}

/// A fresh machine driven to `state` along a legal route, with every step asserted to be accepted.
///
/// Panics rather than returning a machine in the wrong state: a test that silently continued here would
/// report coverage it did not have.
fn machine_at(state: NodeState) -> NodeStateMachine {
    let route =
        legal_route(state).unwrap_or_else(|| panic!("no legal route from Absent to {state:?}"));
    let mut machine = NodeStateMachine::new();
    for step in route {
        machine
            .transition(step)
            .expect("a route found by the edge predicate must be accepted");
    }
    assert_eq!(
        machine.state(),
        state,
        "the route must arrive exactly at {state:?}"
    );
    machine
}

/// Every state reachable from `start` over legal edges, optionally treating `wall` as unenterable.
///
/// Callers pass `Some(wall)` to ask "can this happen without that stage" and `None` as the positive
/// control that proves the search still finds the target when the stage is allowed.
fn reachable_from(start: NodeState, wall: Option<NodeState>) -> Vec<NodeState> {
    let mut visited = vec![start];
    let mut queue = VecDeque::from([start]);
    while let Some(state) = queue.pop_front() {
        for next in ALL_STATES {
            if Some(next) == wall || !state.can_transition_to(next) || visited.contains(&next) {
                continue;
            }
            visited.push(next);
            queue.push_back(next);
        }
    }
    visited
}

#[test]
fn the_three_off_rail_states_are_reachable_so_the_matrix_cannot_skip_them() {
    // The gap this file exists to close, asserted directly. The sibling file's `a_refused_transition_...`
    // test `continue`s over these three because its fixed route array cannot build them; the search here
    // finds a legal route for each, so their illegal pairs can be asserted like every other state's.
    for state in OFF_RAIL_STATES {
        let route = legal_route(state)
            .unwrap_or_else(|| panic!("{state:?} must be reachable by a legal route"));
        assert!(
            !route.is_empty(),
            "{state:?} is not the entry state, so the route is not empty"
        );
        assert_eq!(machine_at(state).state(), state);
        println!("reached {state:?} in {} steps: {route:?}", route.len());
    }
}

#[test]
fn every_illegal_pair_is_refused_and_leaves_the_state_unchanged_for_all_twelve_states() {
    // The completion of the sibling file's matrix check. That one asserts the property over the states its
    // route array can build; this one asserts it over all twelve, so the nonsense pairs from `Failed`,
    // `Unhealthy` and `LocalUnsupported` -- for example jumping straight to `Routable`, or skipping ahead
    // to `PreparingRuntime` -- are checked too.
    let expected_illegal = ALL_STATES.len() * ALL_STATES.len() - 23;
    let mut legal = 0_usize;
    let mut illegal = 0_usize;
    let mut from_states = 0_usize;

    for from in ALL_STATES {
        from_states += 1;
        for to in ALL_STATES {
            if from.can_transition_to(to) {
                legal += 1;
                continue;
            }
            illegal += 1;
            let mut machine = machine_at(from);
            let before = machine.state();
            let result = machine.transition(to);
            assert!(result.is_err(), "{from:?} -> {to:?} must be refused");
            assert_eq!(
                machine.state(),
                before,
                "a refused transition must leave the machine at {before:?}, not move it part-way"
            );
            let error = result.expect_err("just asserted to be an error");
            assert_eq!(error.from, before, "{from:?} -> {to:?} reports its source");
            assert_eq!(error.to, to, "{from:?} -> {to:?} reports its target");
            assert!(
                error.to_string().contains(&format!("{before:?}"))
                    && error.to_string().contains(&format!("{to:?}")),
                "the message names both ends: {error}"
            );
        }
    }

    println!(
        "{from_states} states, {legal} legal pairs, {illegal} illegal pairs refused and left unchanged"
    );
    assert_eq!(
        from_states,
        ALL_STATES.len(),
        "every state is the source of its own row"
    );
    assert_eq!(
        legal + illegal,
        ALL_STATES.len() * ALL_STATES.len(),
        "the matrix is the whole product of the state set with itself"
    );
    assert_eq!(
        illegal, expected_illegal,
        "144 ordered pairs minus the 23 legal edges; a change here means an edge moved and the \
         transcribed table in the sibling file must be reconciled"
    );
    assert_eq!(illegal, 121, "twelve states have 121 illegal ordered pairs");
}

#[test]
fn no_failure_phase_reaches_serving_without_passing_the_readiness_stage_again() {
    // "失败阶段不得进入 Ready/Routable", at path level rather than one step. Blocking `WaitingReady` removes
    // every route onto the readiness stage, so a failure that could inherit readiness from the run that
    // failed would show up here as `Ready` reached without proving readiness again.
    for failure in [NodeState::Failed, NodeState::Unhealthy] {
        let without_readiness = reachable_from(failure, Some(NodeState::WaitingReady));
        for target in [NodeState::Ready, NodeState::Routable] {
            assert!(
                !without_readiness.contains(&target),
                "{failure:?} must not reach {target:?} without the readiness stage; reached via \
                 {without_readiness:?}"
            );
        }
        assert!(
            !without_readiness.contains(&NodeState::WaitingReady),
            "the blocked stage is not itself reachable while blocked"
        );

        // The positive control: with the readiness stage allowed, serving is reachable again. Without this
        // the assertion above would also pass on a machine whose failure states led nowhere at all.
        let with_readiness = reachable_from(failure, None);
        for target in [NodeState::Ready, NodeState::Routable] {
            assert!(
                with_readiness.contains(&target),
                "{failure:?} must be able to recover to {target:?} by retrying the rail"
            );
        }
        println!(
            "{failure:?} reaches {} of {} states with the readiness stage blocked",
            without_readiness.len(),
            ALL_STATES.len()
        );
    }
}

#[test]
fn refusing_a_pair_never_moves_the_machine_even_when_the_pair_is_tried_twice() {
    // The same property from the other side: a refusal is not a partial write that a second attempt
    // completes. An implementation that assigned before validating would pass a single attempt on some
    // pairs and fail here.
    let mut checked = 0_usize;
    for from in ALL_STATES {
        for to in ALL_STATES {
            if from.can_transition_to(to) {
                continue;
            }
            let mut machine = machine_at(from);
            assert!(machine.transition(to).is_err());
            assert!(
                machine.transition(to).is_err(),
                "the second attempt is refused too"
            );
            assert_eq!(
                machine.state(),
                from,
                "still at {from:?} after two refusals"
            );
            checked += 1;
        }
    }
    assert_eq!(
        checked, 121,
        "every illegal pair is covered by this check as well"
    );
}
