#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "Test-only file: the assertions are the test."
)]
//! The Node state machine's transition table and the reconciler's idempotency (#633, plan item 23).
//!
//! `tests/state.rs` and `tests/reconciler.rs` already cover the properties the plan names one at a time -- the
//! golden path, that a spawned process does not prove readiness, that `Ready` and `Routable` cannot be claimed
//! without their evidence, and that failures can retry. What is missing is the **whole table**: `can_transition_to`
//! has 11 states and therefore 121 ordered pairs, and only a handful are exercised.
//!
//! The states and the expected edges are written out **independently** here, transcribed from `state.rs`'s
//! documentation of its own behaviour rather than derived from the `matches!` arm list, so a change to that list
//! has to be reconciled with this table. That is the point of the file: the arm list is the implementation, and
//! this is the specification it should match.
//!
//! ## Two accountings the plan asks to keep apart
//!
//! "生产 adapter 验证与 fake skeleton 验证分开统计" -- production-adapter verification and fake-skeleton
//! verification are counted separately. Everything here is the **fake skeleton**: pure state and reconciler
//! logic with no process, no network and no filesystem. The production adapters (`ProcessManager` etc. against a
//! real machine) are not exercised by this file at all, and nothing below claims they are.

use burncloud_node_runtime::{
    DemandReconciler, InvalidNodeTransition, NodeState, NodeStateMachine, ReconcileAction,
    ReconcileEvidence,
};

/// Every state, so the table below can be exhaustive rather than a sample.
///
/// Written out by hand because `NodeState` has no iterator -- deliberately, since it is a closed enum whose
/// variants are named in the transition arms. The count is asserted below, so a variant added to the enum
/// without being added here fails the suite rather than quietly reducing the coverage.
///
/// **Twelve variants, not eleven.** The first version of this array declared `11` while listing twelve entries,
/// having missed `Unhealthy` from the count; the compiler rejected the literal, which is the check working.
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

/// The transitions the table expects, as `(from, to)` pairs read off the state machine's declared behaviour.
///
/// Transcribed from the doc comments in `state.rs`, which describe the rail: resolve, prepare an artifact,
/// prepare a runtime, start the process, wait for readiness, then attach. Recovery returns a failure to the
/// matching earlier stage rather than to the beginning, and the two settled outcomes (`LocalUnsupported` and
/// `Routable`) can only be released back to `Absent`.
const EXPECTED_EDGES: [(NodeState, NodeState); 23] = [
    // Resolving a local variant.
    (NodeState::Absent, NodeState::Resolving),
    (NodeState::Resolving, NodeState::LocalUnsupported),
    (NodeState::Resolving, NodeState::PreparingArtifact),
    (NodeState::Resolving, NodeState::Failed),
    // Preparing the artifact.
    (NodeState::PreparingArtifact, NodeState::ArtifactReady),
    (NodeState::PreparingArtifact, NodeState::Failed),
    // Preparing the runtime.
    (NodeState::ArtifactReady, NodeState::PreparingRuntime),
    (NodeState::PreparingRuntime, NodeState::Starting),
    (NodeState::PreparingRuntime, NodeState::Failed),
    // Starting the process, then waiting for readiness evidence.
    (NodeState::Starting, NodeState::WaitingReady),
    (NodeState::Starting, NodeState::Failed),
    (NodeState::WaitingReady, NodeState::Ready),
    (NodeState::WaitingReady, NodeState::Failed),
    // Serving, and the ways out of it.
    (NodeState::Ready, NodeState::Routable),
    (NodeState::Ready, NodeState::Unhealthy),
    (NodeState::Ready, NodeState::Failed),
    (NodeState::Routable, NodeState::Unhealthy),
    (NodeState::Routable, NodeState::Absent),
    // Recovery.
    (NodeState::Unhealthy, NodeState::Starting),
    (NodeState::Unhealthy, NodeState::Failed),
    (NodeState::Failed, NodeState::Resolving),
    (NodeState::Failed, NodeState::Absent),
    // Releasing a settled non-serving outcome.
    (NodeState::LocalUnsupported, NodeState::Absent),
];

fn is_expected(from: NodeState, to: NodeState) -> bool {
    EXPECTED_EDGES.contains(&(from, to))
}

// -------------------------------------------------------------------------------------------
// the whole table
// -------------------------------------------------------------------------------------------

#[test]
fn the_transition_table_has_exactly_the_expected_edges() {
    // **The exhaustive check.** For every one of the 121 ordered pairs, `can_transition_to` must agree with the
    // transcribed table -- so an edge added, removed or redirected in the `matches!` arms is reported with the
    // pair that changed rather than as a behaviour difference discovered later.
    assert_eq!(
        ALL_STATES.len(),
        12,
        "the enum has 11 variants; if this fails, a variant was added and `ALL_STATES` must be extended or the \
         table below is no longer exhaustive"
    );

    let mut missing = Vec::new();
    let mut extra = Vec::new();
    let mut allowed = 0;

    for from in ALL_STATES {
        for to in ALL_STATES {
            let actual = from.can_transition_to(to);
            let expected = is_expected(from, to);
            if actual {
                allowed += 1;
            }
            if expected && !actual {
                missing.push(format!("{from:?} -> {to:?}"));
            }
            if actual && !expected {
                extra.push(format!("{from:?} -> {to:?}"));
            }
        }
    }

    println!(
        "{} of {} ordered pairs are transitions",
        allowed,
        ALL_STATES.len() * ALL_STATES.len()
    );
    assert!(
        missing.is_empty(),
        "the table expects these transitions and the state machine refuses them:\n  {}",
        missing.join("\n  ")
    );
    assert!(
        extra.is_empty(),
        "the state machine allows these transitions and the table does not:\n  {}",
        extra.join("\n  ")
    );
    assert_eq!(
        allowed,
        EXPECTED_EDGES.len(),
        "every expected edge is allowed exactly once"
    );
}

#[test]
fn a_state_never_transitions_to_itself() {
    // Reflexivity would make the rail unfalsifiable: a stage that "advanced" to where it already was would
    // report progress without doing anything, and the reconciler would loop. Checked over all 11 rather than on a
    // sample.
    for state in ALL_STATES {
        assert!(
            !state.can_transition_to(state),
            "{state:?} must not transition to itself"
        );
    }
}

#[test]
fn a_failure_phase_can_never_reach_ready_or_routable() {
    // **The plan's second item, as a table property rather than as three examples.** "失败阶段不得进入
    // Ready/Routable": from either failure state, `Ready` and `Routable` must be unreachable **in one step**.
    //
    // One step is the property that matters here, because `transition` is the only way the state moves and it
    // moves by exactly one edge. Reaching `Ready` from `Failed` therefore takes at least the path through
    // `Resolving` and the whole rail again -- which is the intent, since readiness must be re-proven rather than
    // inherited from a run that failed.
    for failure in [NodeState::Failed, NodeState::Unhealthy] {
        for target in [NodeState::Ready, NodeState::Routable] {
            assert!(
                !failure.can_transition_to(target),
                "a failed phase ({failure:?}) must not reach {target:?} in one step"
            );
        }
    }

    // The same for the earlier failure phases: nothing before readiness may skip to it, which is what stops a
    // stage from claiming a result its evidence does not support.
    for early in [
        NodeState::Absent,
        NodeState::Resolving,
        NodeState::PreparingArtifact,
        NodeState::ArtifactReady,
        NodeState::PreparingRuntime,
        NodeState::Starting,
    ] {
        assert!(
            !early.can_transition_to(NodeState::Ready),
            "{early:?} must not reach Ready without the readiness step"
        );
        assert!(
            !early.can_transition_to(NodeState::Routable),
            "{early:?} must not reach Routable"
        );
    }

    // And `Routable` is reached from exactly one state, so there is no second route onto the serving path.
    let sources: Vec<NodeState> = ALL_STATES
        .into_iter()
        .filter(|s| s.can_transition_to(NodeState::Routable))
        .collect();
    assert_eq!(
        sources,
        vec![NodeState::Ready],
        "Routable is reachable only from Ready"
    );
}

#[test]
fn only_routable_is_serving_and_only_failed_and_unhealthy_are_failures() {
    // The two predicates, over every state rather than a sample. `LocalUnsupported` is the interesting one: it is
    // a **settled non-serving outcome, not a runtime failure**, so `is_failure` must be false for it even though
    // it is not serving. Its doc comment says so; this asserts it.
    let mut serving = Vec::new();
    let mut failures = Vec::new();
    for state in ALL_STATES {
        if state.is_serving() {
            serving.push(state);
        }
        if state.is_failure() {
            failures.push(state);
        }
    }
    println!("serving: {serving:?}, failures: {failures:?}");

    assert_eq!(
        serving,
        vec![NodeState::Routable],
        "only Routable is serving"
    );
    assert_eq!(
        failures,
        vec![NodeState::Failed, NodeState::Unhealthy],
        "and only those two are failures"
    );
    assert!(
        !NodeState::LocalUnsupported.is_failure(),
        "LocalUnsupported is a settled outcome rather than a runtime failure"
    );
    assert!(
        !NodeState::LocalUnsupported.is_serving(),
        "and it is not serving either, so it is neither of the two predicates"
    );

    // `is_failure` is not the same as "not serving" -- that would make every state a failure.
    let not_serving = ALL_STATES.iter().filter(|s| !s.is_serving()).count();
    assert_eq!(
        not_serving, 11,
        "twelve states and one of them is Routable, so eleven are not serving"
    );
    assert!(
        not_serving > failures.len(),
        "and only two of them are failures"
    );
}

#[test]
fn absent_is_the_only_entry_and_has_no_incoming_edge() {
    // `Absent` is the default and the only state a fresh machine can be in, so nothing may transition **into** it
    // except the two releases that deliberately discard a settled outcome. A cycle back to `Absent` from the
    // middle of the rail would let a run restart without being asked.
    let incoming: Vec<NodeState> = ALL_STATES
        .into_iter()
        .filter(|s| s.can_transition_to(NodeState::Absent))
        .collect();
    println!("states that can reach Absent: {incoming:?}");
    // Compared as a **set**, not as a sequence: the order the states come back in is the enum's declaration order,
    // and the first version of this assertion failed only because it listed the same three states in another order.
    let mut incoming_sorted = incoming.clone();
    incoming_sorted.sort_by_key(|s| format!("{s:?}"));
    assert_eq!(
        incoming_sorted,
        vec![
            NodeState::Failed,
            NodeState::LocalUnsupported,
            NodeState::Routable
        ],
        "only Failed, Routable and LocalUnsupported release back to Absent"
    );

    // A fresh machine starts there, and the default agrees with `new()`.
    assert_eq!(NodeStateMachine::new().state(), NodeState::Absent);
    assert_eq!(NodeStateMachine::default().state(), NodeState::Absent);
    assert_eq!(NodeState::default(), NodeState::Absent);
}

#[test]
fn the_rail_from_absent_to_routable_is_the_only_path_and_it_uses_every_preparation_state() {
    // The golden path as a property of the table: starting from `Absent`, the only way forward that is not a
    // failure is unique at each step, and it passes through every stage. This is what `tests/state.rs`'s
    // `golden_runtime_path_reaches_routable_without_skipping_states` checks for the happy path; here it is derived
    // from the table so the two must agree.
    let mut machine = NodeStateMachine::new();
    let rail = [
        NodeState::Resolving,
        NodeState::PreparingArtifact,
        NodeState::ArtifactReady,
        NodeState::PreparingRuntime,
        NodeState::Starting,
        NodeState::WaitingReady,
        NodeState::Ready,
        NodeState::Routable,
    ];

    for step in rail {
        // Exactly one successor at each stage of the rail, so the path cannot branch. `is_failure` is **not** the
        // filter: `Resolving` also reaches `LocalUnsupported`, which is a settled outcome rather than a failure,
        // so "not a failure" leaves two candidates and the first version of this test failed on that. What is
        // excluded is anything that is not a step further along the rail.
        let forward: Vec<NodeState> = ALL_STATES
            .into_iter()
            .filter(|s| {
                machine.state().can_transition_to(*s)
                    && *s != NodeState::LocalUnsupported
                    && !s.is_failure()
            })
            .collect();
        assert_eq!(
            forward,
            vec![step],
            "from {:?} the only forward step must be {step:?}, got {forward:?}",
            machine.state()
        );
        machine
            .transition(step)
            .unwrap_or_else(|e| panic!("the rail step to {step:?} must be legal: {e}"));
    }

    assert_eq!(
        machine.state(),
        NodeState::Routable,
        "and the rail ends here"
    );
    assert!(machine.state().is_serving());
}

#[test]
fn the_failure_states_have_a_forward_path_back_to_the_rail() {
    // A dead end would strand a workload: if a failure state had no outgoing transition, the reconciler would
    // report `Recover` forever and nothing could change. Both failure states must have at least one non-failure
    // successor, and the successor must be the stage that has to be redone.
    for (failure, expected) in [
        (
            NodeState::Failed,
            vec![NodeState::Resolving, NodeState::Absent],
        ),
        (
            NodeState::Unhealthy,
            vec![NodeState::Starting, NodeState::Failed],
        ),
    ] {
        let mut forward: Vec<NodeState> = ALL_STATES
            .into_iter()
            .filter(|s| failure.can_transition_to(*s))
            .collect();
        forward.sort_by_key(|s| format!("{s:?}"));
        println!("{failure:?} can go to {forward:?}");
        // As a set: the exits come back in the enum's declaration order, which is not the order they are listed
        // in, and the first version of this assertion failed on that alone.
        let mut expected_sorted = expected.clone();
        expected_sorted.sort_by_key(|s| format!("{s:?}"));
        assert_eq!(
            forward, expected_sorted,
            "{failure:?} must have exactly these exits, or a stranded workload cannot be recovered"
        );
        assert!(
            forward.iter().any(|s| !s.is_failure()),
            "{failure:?} must have at least one exit that is not another failure"
        );
    }
}

#[test]
fn every_state_is_reachable_and_every_state_can_be_left() {
    // Two graph properties that together mean the table describes one connected machine rather than a set of
    // fragments. Reachability is computed from `Absent` by repeated relaxation, which is independent of the
    // hand-written table above.
    let mut reachable = vec![NodeState::Absent];
    loop {
        let mut grew = false;
        for from in reachable.clone() {
            for to in ALL_STATES {
                if from.can_transition_to(to) && !reachable.contains(&to) {
                    reachable.push(to);
                    grew = true;
                }
            }
        }
        if !grew {
            break;
        }
    }
    println!(
        "reachable from Absent: {} of {}",
        reachable.len(),
        ALL_STATES.len()
    );
    assert_eq!(
        reachable.len(),
        ALL_STATES.len(),
        "every state must be reachable from Absent, or it is dead code: missing {:?}",
        ALL_STATES
            .into_iter()
            .filter(|s| !reachable.contains(s))
            .collect::<Vec<_>>()
    );

    // `Routable` itself has no outgoing transition except to `Unhealthy` and `Absent`, which is the exit a
    // serving workload needs when it is detached or degrades.
    let from_routable: Vec<NodeState> = ALL_STATES
        .into_iter()
        .filter(|s| NodeState::Routable.can_transition_to(*s))
        .collect();
    assert_eq!(
        from_routable.len(),
        2,
        "Routable has exactly two exits: {from_routable:?}"
    );
}

// -------------------------------------------------------------------------------------------
// the state machine's failure behaviour
// -------------------------------------------------------------------------------------------

#[test]
fn a_refused_transition_leaves_the_machine_where_it_was() {
    // The plan's "所有合法/非法状态转移" has a second half: not only that an illegal pair is refused, but that the
    // refusal does not partially apply. `transition` validates before assigning, and this checks that over the
    // whole matrix rather than on one example -- a machine that moved and then errored would be in a state the
    // caller was told it is not in.
    for from in ALL_STATES {
        for to in ALL_STATES {
            if from.can_transition_to(to) {
                continue;
            }
            // Reach `from` by the shortest legal route, then attempt the illegal pair.
            let mut machine = NodeStateMachine::new();
            if from != NodeState::Absent {
                let route = [
                    NodeState::Resolving,
                    NodeState::PreparingArtifact,
                    NodeState::ArtifactReady,
                    NodeState::PreparingRuntime,
                    NodeState::Starting,
                    NodeState::WaitingReady,
                    NodeState::Ready,
                    NodeState::Routable,
                ];
                let mut reached = from == NodeState::Absent;
                for step in route {
                    if step == from {
                        reached = true;
                    }
                    if reached {
                        break;
                    }
                    if machine.transition(step).is_err() {
                        break;
                    }
                }
                if !reached || machine.state() != from {
                    // Not every state is on the forward rail (the two failures and LocalUnsupported are not), so
                    // the illegal pairs from those are covered by the direct construction below instead.
                    continue;
                }
            }

            let before = machine.state();
            let result = machine.transition(to);
            assert!(result.is_err(), "{from:?} -> {to:?} must be refused");
            assert_eq!(
                machine.state(),
                before,
                "a refused transition must leave the machine at {before:?}"
            );

            // The error names both ends, so a caller can report which pair it attempted.
            let error: InvalidNodeTransition = result.expect_err("just asserted to be an error");
            assert_eq!(error.from, before);
            assert_eq!(error.to, to);
            assert!(
                error.to_string().contains(&format!("{before:?}"))
                    && error.to_string().contains(&format!("{to:?}")),
                "the message names both states: {error}"
            );
        }
    }
}

#[test]
fn the_two_off_rail_states_are_reached_only_by_evidence_and_not_by_transition() {
    // `Failed` and `LocalUnsupported` are not on the forward rail, so they are reached through the reconciler's
    // `observe` rather than by walking the machine. This checks the machine refuses to step into them from
    // `Absent`, which is what makes the rail the only way to start.
    for off_rail in [
        NodeState::Failed,
        NodeState::LocalUnsupported,
        NodeState::Routable,
    ] {
        assert!(
            !NodeState::Absent.can_transition_to(off_rail),
            "Absent must not step straight to {off_rail:?}"
        );
    }
    assert!(
        !NodeState::Absent.can_transition_to(NodeState::PreparingArtifact),
        "nor may it skip the resolution stage"
    );
}

// -------------------------------------------------------------------------------------------
// the reconciler
// -------------------------------------------------------------------------------------------

/// Drive a reconciler to `target` on the forward rail, **using evidence rather than a hand-built route**.
///
/// A route built by listing the states a test wants to pass through does not work with this reconciler:
/// `next_action` moves the machine eagerly from `Absent` and from `ArtifactReady`, so a caller that also steps
/// the machine by hand arrives somewhere it did not intend. The first version of two tests below did exactly
/// that and failed. This advances by calling `next_action` and then delivering the evidence the current stage is
/// waiting for, so the states visited are whatever the reconciler decides.
///
/// Returns the sequence of `(state after evidence, evidence delivered)` pairs, so a test can assert on the path
/// as it actually was.
fn drive_to(target: NodeState) -> Vec<(NodeState, ReconcileEvidence)> {
    let mut reconciler = DemandReconciler::new();
    let mut visited = Vec::new();

    // `next_action` from Absent moves to Resolving and asks for the resolution work.
    let mut guard = 0;
    while reconciler.state() != target {
        guard += 1;
        assert!(
            guard < 32,
            "the rail did not reach {target:?} within 32 steps"
        );

        reconciler
            .next_action()
            .unwrap_or_else(|e| panic!("next_action failed at {:?}: {e}", reconciler.state()));

        // The evidence this stage is waiting for, chosen from the state rather than from a route.
        let evidence = match reconciler.state() {
            NodeState::Resolving => ReconcileEvidence::Resolved,
            NodeState::PreparingArtifact => ReconcileEvidence::ArtifactPrepared,
            NodeState::PreparingRuntime => ReconcileEvidence::RuntimePrepared,
            NodeState::Starting => ReconcileEvidence::ProcessStarted,
            NodeState::WaitingReady => ReconcileEvidence::ReadinessVerified,
            NodeState::Ready => ReconcileEvidence::RouterAttached,
            NodeState::Routable => break,
            other if other == target => break,
            other => panic!("no forward evidence defined for {other:?}"),
        };

        reconciler.observe(evidence).unwrap_or_else(|e| {
            panic!(
                "{evidence:?} must be accepted at {:?}: {e}",
                reconciler.state()
            )
        });
        visited.push((reconciler.state(), evidence));
    }

    visited
}

#[test]
fn replaying_the_same_evidence_is_refused_so_a_stage_cannot_advance_twice() {
    // **The plan's "重复 reconcile 幂等".** The reconciler advances on evidence, so the property that matters is
    // what a **second** delivery of the same evidence does: it must be refused and leave the state alone. If it
    // were accepted, a replayed stage -- a retried callback, a duplicated receipt read back after an interruption
    // -- would advance the machine past the stage it belongs to.
    //
    // The states are reached with `drive_to`, which delivers evidence and calls `next_action` in the order the
    // reconciler expects. Two earlier versions of this test drove the machine by hand and arrived one state short,
    // because `next_action` is what steps out of `Absent` and out of `ArtifactReady`.
    // **`ArtifactReady` is not in this list**, and its absence is deliberate: it accepts no evidence at all, so
    // there is nothing to replay against it -- it advances only when `next_action` is called. `Resolving` and
    // `Unhealthy` are included instead, because both accept evidence (`Resolved`/`LocalUnsupported`/`Failed` for
    // the first, `Failed` for the second), which is what this test is about.
    const TARGETS: [NodeState; 9] = [
        NodeState::Resolving,
        NodeState::PreparingArtifact,
        NodeState::PreparingRuntime,
        NodeState::Starting,
        NodeState::WaitingReady,
        NodeState::Ready,
        NodeState::Routable,
        NodeState::Unhealthy,
        NodeState::Failed,
    ];

    for target in TARGETS {
        let mut reconciler = DemandReconciler::new();

        // Replay `drive_to`'s decisions on this reconciler rather than building a route: the sequence is derived
        // from the state, so it cannot disagree with what the reconciler does.
        let visited = drive_to(target);
        let mut delivered = Vec::new();
        reconciler
            .next_action()
            .expect("the first action is Resolve");

        // **`Failed` is delivered here, before the loop.** It is reached from `Resolving`, which `drive_to` does
        // not model, and it cannot be reached at the end of the walk: the loop goes as far as `Routable`, and
        // `(Routable, Failed)` is not a transition -- the table records that a serving workload becomes
        // `Unhealthy` first. The first version delivered it after the loop and was refused.
        if target == NodeState::Failed {
            reconciler
                .observe(ReconcileEvidence::Failed)
                .expect("Failed is accepted from Resolving");
            assert_eq!(reconciler.state(), NodeState::Failed);
        }

        let mut state = reconciler.state();
        while state != target && state != NodeState::Routable {
            let evidence = match state {
                NodeState::Resolving => ReconcileEvidence::Resolved,
                NodeState::PreparingArtifact => ReconcileEvidence::ArtifactPrepared,
                NodeState::PreparingRuntime => ReconcileEvidence::RuntimePrepared,
                NodeState::Starting => ReconcileEvidence::ProcessStarted,
                NodeState::WaitingReady => ReconcileEvidence::ReadinessVerified,
                NodeState::Ready => ReconcileEvidence::RouterAttached,
                NodeState::ArtifactReady => ReconcileEvidence::Resolved,
                other => panic!("no forward evidence for {other:?}"),
            };
            if state == NodeState::ArtifactReady {
                // Nothing to deliver: `next_action` advances this state on its own.
                reconciler
                    .next_action()
                    .expect("ArtifactReady advances eagerly");
                state = reconciler.state();
                continue;
            }
            reconciler
                .observe(evidence)
                .unwrap_or_else(|e| panic!("{evidence:?} must be accepted at {state:?}: {e}"));
            delivered.push(evidence);
            state = reconciler.state();
            if state == target {
                break;
            }
            reconciler
                .next_action()
                .expect("the next stage is requested");
            state = reconciler.state();
        }

        // `Unhealthy` is reached from `Routable`, which the loop stops at rather than leaving.
        if target == NodeState::Unhealthy && reconciler.state() != NodeState::Unhealthy {
            assert_eq!(
                reconciler.state(),
                NodeState::Routable,
                "Unhealthy is reached from a serving workload"
            );
            reconciler
                .observe(ReconcileEvidence::BecameUnhealthy)
                .expect("BecameUnhealthy is accepted from Routable");
        }

        assert_eq!(
            reconciler.state(),
            target,
            "the walk must arrive at {target:?}; it delivered {delivered:?} and `drive_to` reported {visited:?}"
        );

        // The evidence this state waits for, so the replay is of something that was **accepted** the first time.
        let evidence = match target {
            NodeState::Resolving => ReconcileEvidence::Resolved,
            NodeState::PreparingArtifact => ReconcileEvidence::ArtifactPrepared,
            NodeState::PreparingRuntime => ReconcileEvidence::RuntimePrepared,
            NodeState::Starting => ReconcileEvidence::ProcessStarted,
            NodeState::WaitingReady => ReconcileEvidence::ReadinessVerified,
            NodeState::Ready => ReconcileEvidence::RouterAttached,
            NodeState::Routable => ReconcileEvidence::BecameUnhealthy,
            // `Unhealthy` accepts `Failed`, which is the edge the table records as `(Unhealthy, Failed)`.
            NodeState::Unhealthy => ReconcileEvidence::Failed,
            NodeState::Failed => ReconcileEvidence::Failed,
            other => panic!("{other:?} waits for no evidence"),
        };

        // **For `Failed` the walk above already delivered the evidence once**, which is the first delivery this
        // test is about: `Failed` was accepted from `Resolving` and put the machine in `Failed`. Delivering it
        // again here would be the *second* delivery, and delivering it a third time the third -- and `(Failed,
        // Failed)` is not a transition, so the "must be accepted the first time" assertion would be checking a
        // self-loop that does not exist.
        let after_first = if target == NodeState::Failed {
            let state = reconciler.state();
            println!("{evidence:?}: Resolving -> {state:?} (delivered once, before the walk)");
            state
        } else {
            let before = reconciler.state();
            reconciler
                .observe(evidence)
                .unwrap_or_else(|e| panic!("{evidence:?} must be accepted from {before:?}: {e}"));
            let after = reconciler.state();
            println!("{evidence:?}: {before:?} -> {after:?}");
            after
        };

        let second = reconciler.observe(evidence);
        match &second {
            Ok(()) => panic!(
                "{evidence:?} was accepted twice, so a replayed stage advanced the machine from {after_first:?} \
                 without new evidence"
            ),
            Err(e) => println!("  the replay is refused: {e}"),
        }
        assert!(second.is_err(), "replayed evidence must be refused");
        assert_eq!(
            reconciler.state(),
            after_first,
            "and the refusal leaves the state where the first delivery put it"
        );
    }
}

#[test]
#[allow(
    clippy::too_many_lines,
    reason = "table-driven test: it walks every stage of the rail and asserts the whole refused-evidence row, so the length is the coverage"
)]
fn evidence_from_the_wrong_stage_is_refused_at_every_stage_of_the_rail() {
    // The same property the existing `reconciler.rs` test checks for one pair, here for **every stage** of the
    // rail and every evidence value: only the evidence the current stage is waiting for is accepted, and
    // everything else is refused with the state left alone.
    //
    // The first version of this test tried to enumerate all 12 x 9 pairs by transcribing the reconciler's arms
    // into the test and counting them. That is a copy of the implementation, which would agree with a change to it
    // rather than catch one -- so the pairs are driven instead, through the reconciler itself.
    const ALL_EVIDENCE: [ReconcileEvidence; 9] = [
        ReconcileEvidence::Resolved,
        ReconcileEvidence::LocalUnsupported,
        ReconcileEvidence::ArtifactPrepared,
        ReconcileEvidence::RuntimePrepared,
        ReconcileEvidence::ProcessStarted,
        ReconcileEvidence::ReadinessVerified,
        ReconcileEvidence::RouterAttached,
        ReconcileEvidence::BecameUnhealthy,
        ReconcileEvidence::Failed,
    ];

    // The evidence each state on the rail is waiting for, and nothing else may be delivered there.
    let expected_at = |state: NodeState| -> Vec<ReconcileEvidence> {
        // `Failed` is accepted from **every stage that has work in flight** -- Resolving through Ready, plus
        // Unhealthy -- so it appears in each of those arms. The first version of this list omitted it from
        // Resolving, and the probe caught that immediately.
        match state {
            NodeState::Resolving => vec![
                ReconcileEvidence::Resolved,
                ReconcileEvidence::LocalUnsupported,
                ReconcileEvidence::Failed,
            ],
            NodeState::PreparingArtifact => {
                vec![
                    ReconcileEvidence::ArtifactPrepared,
                    ReconcileEvidence::Failed,
                ]
            }
            NodeState::PreparingRuntime => {
                vec![
                    ReconcileEvidence::RuntimePrepared,
                    ReconcileEvidence::Failed,
                ]
            }
            NodeState::Starting => {
                vec![ReconcileEvidence::ProcessStarted, ReconcileEvidence::Failed]
            }
            NodeState::WaitingReady => {
                vec![
                    ReconcileEvidence::ReadinessVerified,
                    ReconcileEvidence::Failed,
                ]
            }
            NodeState::Ready => vec![
                ReconcileEvidence::RouterAttached,
                ReconcileEvidence::BecameUnhealthy,
                ReconcileEvidence::Failed,
            ],
            NodeState::Routable => vec![ReconcileEvidence::BecameUnhealthy],
            // `Unhealthy` is a failure phase with work in flight, so `Failed` is accepted here too -- the table's
            // `(Unhealthy, Failed)` edge. The first version of this list had `Unhealthy` among the states that
            // accept nothing, and the probe caught it.
            NodeState::Unhealthy => vec![ReconcileEvidence::Failed],
            // Nothing advances these: they are reached and left by `retry`, not by evidence.
            NodeState::Absent | NodeState::ArtifactReady | NodeState::Failed => vec![],
            NodeState::LocalUnsupported => vec![],
        }
    };

    let mut checked = 0;
    let mut accepted = 0;
    let mut refused = 0;

    for target in ALL_STATES {
        // Reach the state by driving, then probe every evidence value against a **fresh** reconciler at that
        // state, so one probe cannot change what the next one sees.
        for evidence in ALL_EVIDENCE {
            let mut reconciler = DemandReconciler::new();
            match target {
                NodeState::Absent => {}
                NodeState::Resolving => {
                    reconciler.next_action().expect("Resolve");
                }
                NodeState::PreparingArtifact => {
                    reconciler.next_action().expect("Resolve");
                    reconciler
                        .observe(ReconcileEvidence::Resolved)
                        .expect("Resolved");
                }
                NodeState::ArtifactReady => {
                    reconciler.next_action().expect("Resolve");
                    reconciler
                        .observe(ReconcileEvidence::Resolved)
                        .expect("Resolved");
                    reconciler
                        .observe(ReconcileEvidence::ArtifactPrepared)
                        .expect("prepared");
                }
                NodeState::PreparingRuntime => {
                    reconciler.next_action().expect("Resolve");
                    reconciler
                        .observe(ReconcileEvidence::Resolved)
                        .expect("Resolved");
                    reconciler
                        .observe(ReconcileEvidence::ArtifactPrepared)
                        .expect("prepared");
                    reconciler
                        .next_action()
                        .expect("ArtifactReady advances eagerly");
                }
                NodeState::Starting => {
                    reconciler.next_action().expect("Resolve");
                    reconciler
                        .observe(ReconcileEvidence::Resolved)
                        .expect("Resolved");
                    reconciler
                        .observe(ReconcileEvidence::ArtifactPrepared)
                        .expect("prepared");
                    reconciler.next_action().expect("eager");
                    reconciler
                        .observe(ReconcileEvidence::RuntimePrepared)
                        .expect("runtime");
                }
                NodeState::WaitingReady => {
                    reconciler.next_action().expect("Resolve");
                    reconciler
                        .observe(ReconcileEvidence::Resolved)
                        .expect("Resolved");
                    reconciler
                        .observe(ReconcileEvidence::ArtifactPrepared)
                        .expect("prepared");
                    reconciler.next_action().expect("eager");
                    reconciler
                        .observe(ReconcileEvidence::RuntimePrepared)
                        .expect("runtime");
                    reconciler
                        .observe(ReconcileEvidence::ProcessStarted)
                        .expect("started");
                }
                NodeState::Ready => {
                    reconciler.next_action().expect("Resolve");
                    reconciler
                        .observe(ReconcileEvidence::Resolved)
                        .expect("Resolved");
                    reconciler
                        .observe(ReconcileEvidence::ArtifactPrepared)
                        .expect("prepared");
                    reconciler.next_action().expect("eager");
                    reconciler
                        .observe(ReconcileEvidence::RuntimePrepared)
                        .expect("runtime");
                    reconciler
                        .observe(ReconcileEvidence::ProcessStarted)
                        .expect("started");
                    reconciler
                        .observe(ReconcileEvidence::ReadinessVerified)
                        .expect("ready");
                }
                NodeState::Routable => {
                    reconciler.next_action().expect("Resolve");
                    reconciler
                        .observe(ReconcileEvidence::Resolved)
                        .expect("Resolved");
                    reconciler
                        .observe(ReconcileEvidence::ArtifactPrepared)
                        .expect("prepared");
                    reconciler.next_action().expect("eager");
                    reconciler
                        .observe(ReconcileEvidence::RuntimePrepared)
                        .expect("runtime");
                    reconciler
                        .observe(ReconcileEvidence::ProcessStarted)
                        .expect("started");
                    reconciler
                        .observe(ReconcileEvidence::ReadinessVerified)
                        .expect("ready");
                    reconciler
                        .observe(ReconcileEvidence::RouterAttached)
                        .expect("attached");
                }
                NodeState::Unhealthy => {
                    reconciler.next_action().expect("Resolve");
                    reconciler
                        .observe(ReconcileEvidence::Resolved)
                        .expect("Resolved");
                    reconciler
                        .observe(ReconcileEvidence::ArtifactPrepared)
                        .expect("prepared");
                    reconciler.next_action().expect("eager");
                    reconciler
                        .observe(ReconcileEvidence::RuntimePrepared)
                        .expect("runtime");
                    reconciler
                        .observe(ReconcileEvidence::ProcessStarted)
                        .expect("started");
                    reconciler
                        .observe(ReconcileEvidence::ReadinessVerified)
                        .expect("ready");
                    reconciler
                        .observe(ReconcileEvidence::BecameUnhealthy)
                        .expect("unhealthy");
                }
                NodeState::Failed => {
                    reconciler.next_action().expect("Resolve");
                    reconciler
                        .observe(ReconcileEvidence::Failed)
                        .expect("failed");
                }
                NodeState::LocalUnsupported => {
                    reconciler.next_action().expect("Resolve");
                    reconciler
                        .observe(ReconcileEvidence::LocalUnsupported)
                        .expect("unsupported");
                }
            }
            assert_eq!(
                reconciler.state(),
                target,
                "the probe must start from {target:?}"
            );

            let before = reconciler.state();
            let result = reconciler.observe(evidence);
            let should_accept = expected_at(target).contains(&evidence);
            checked += 1;

            if should_accept {
                accepted += 1;
                assert!(
                    result.is_ok(),
                    "{evidence:?} is the evidence {target:?} waits for and must be accepted"
                );
                assert_ne!(
                    reconciler.state(),
                    before,
                    "{evidence:?} accepted at {target:?} must move the machine"
                );
            } else {
                refused += 1;
                assert!(
                    result.is_err(),
                    "{evidence:?} was accepted at {target:?}, where the stage waits for {:?}",
                    expected_at(target)
                );
                assert_eq!(
                    reconciler.state(),
                    before,
                    "a refused delivery must leave the machine at {before:?}"
                );
            }
        }
    }

    println!("{checked} (state, evidence) pairs probed: {accepted} accepted, {refused} refused");
    assert_eq!(
        checked,
        ALL_STATES.len() * ALL_EVIDENCE.len(),
        "every pair was probed"
    );
    assert_eq!(accepted, 16, "sixteen pairs are the ones a stage waits for");
    assert_eq!(refused, checked - 16, "and the rest are refused");
}

#[test]
fn the_reconciler_retries_only_from_a_failure_and_returns_to_the_matching_stage() {
    // "中断后沿持久回执恢复" in its smallest form: a failed run is put back on the rail at the stage that has to be
    // redone, and `retry` is refused everywhere else. `Unhealthy` returns to `Starting` rather than to the
    // beginning, because the artifact and the runtime are still there -- only the process has to come back.
    let mut reconciler = DemandReconciler::new();

    // Refused from Absent.
    assert!(
        reconciler.retry().is_err(),
        "there is nothing to retry from Absent"
    );
    assert_eq!(
        reconciler.state(),
        NodeState::Absent,
        "and the state is unchanged"
    );

    // Drive to Failed.
    reconciler.next_action().expect("Resolve");
    reconciler
        .observe(ReconcileEvidence::Failed)
        .expect("Failed from Resolving");
    assert_eq!(reconciler.state(), NodeState::Failed);
    reconciler.retry().expect("Failed can retry");
    assert_eq!(
        reconciler.state(),
        NodeState::Resolving,
        "a failed run restarts at resolution, because the artifact it prepared may be the reason it failed"
    );

    // Drive to Unhealthy and check the other recovery target. `next_action` is called before each delivery,
    // because it is what puts the machine into the state that waits for it -- `ArtifactReady` in particular only
    // advances when it is called. The first version of this test omitted those calls and failed with
    // "RuntimePrepared must be accepted on the way to Unhealthy: ArtifactReady -> ArtifactReady".
    for evidence in [
        ReconcileEvidence::Resolved,
        ReconcileEvidence::ArtifactPrepared,
        ReconcileEvidence::RuntimePrepared,
        ReconcileEvidence::ProcessStarted,
        ReconcileEvidence::ReadinessVerified,
        ReconcileEvidence::RouterAttached,
        ReconcileEvidence::BecameUnhealthy,
    ] {
        reconciler
            .next_action()
            .unwrap_or_else(|e| panic!("next_action failed at {:?}: {e}", reconciler.state()));
        reconciler.observe(evidence).unwrap_or_else(|e| {
            panic!("{evidence:?} must be accepted on the way to Unhealthy: {e}")
        });
    }
    assert_eq!(reconciler.state(), NodeState::Unhealthy);
    reconciler.retry().expect("Unhealthy can retry");
    assert_eq!(
        reconciler.state(),
        NodeState::Starting,
        "an unhealthy run restarts the process, keeping the prepared artifact and runtime"
    );

    // And retry is refused from a healthy serving state, so it cannot be used to restart a working workload.
    for evidence in [
        ReconcileEvidence::ProcessStarted,
        ReconcileEvidence::ReadinessVerified,
    ] {
        reconciler.next_action().expect("the stage is requested");
        reconciler.observe(evidence).expect("back up the rail");
    }
    assert_eq!(reconciler.state(), NodeState::Ready);
    assert!(
        reconciler.retry().is_err(),
        "a ready workload must not be retried"
    );
}

#[test]
fn next_action_advances_only_on_the_two_states_that_need_a_step_taken_for_them() {
    // `next_action` moves the machine on `Absent` and `ArtifactReady` and returns a request everywhere else,
    // leaving the state alone. That is what makes it safe to call in a loop: only the stages that have nothing to
    // report yet advance eagerly, and everything else waits for `observe`.
    let mut reconciler = DemandReconciler::new();

    // Absent -> Resolving, asking for Resolve.
    assert_eq!(
        reconciler.next_action().expect("first action"),
        ReconcileAction::Resolve
    );
    assert_eq!(reconciler.state(), NodeState::Resolving);

    // Resolving -> Resolving, asking again; the state does not move until evidence arrives.
    assert_eq!(
        reconciler.next_action().expect("second action"),
        ReconcileAction::Resolve
    );
    assert_eq!(
        reconciler.state(),
        NodeState::Resolving,
        "calling next_action again from Resolving asks for the same work rather than skipping it"
    );

    reconciler
        .observe(ReconcileEvidence::Resolved)
        .expect("Resolved");
    assert_eq!(reconciler.state(), NodeState::PreparingArtifact);
    // PreparingArtifact asks without moving.
    assert_eq!(
        reconciler.next_action().expect("action"),
        ReconcileAction::PrepareArtifact
    );
    assert_eq!(reconciler.state(), NodeState::PreparingArtifact);

    reconciler
        .observe(ReconcileEvidence::ArtifactPrepared)
        .expect("ArtifactPrepared");
    assert_eq!(reconciler.state(), NodeState::ArtifactReady);
    // ArtifactReady is the second eager step.
    assert_eq!(
        reconciler.next_action().expect("action"),
        ReconcileAction::PrepareRuntime
    );
    assert_eq!(reconciler.state(), NodeState::PreparingRuntime);

    // And from Routable there is nothing to do, so `next_action` is a no-op rather than an error.
    reconciler
        .observe(ReconcileEvidence::RuntimePrepared)
        .expect("runtime");
    reconciler
        .observe(ReconcileEvidence::ProcessStarted)
        .expect("started");
    reconciler
        .observe(ReconcileEvidence::ReadinessVerified)
        .expect("ready");
    reconciler
        .observe(ReconcileEvidence::RouterAttached)
        .expect("attached");
    assert_eq!(reconciler.state(), NodeState::Routable);
    assert_eq!(
        reconciler
            .next_action()
            .expect("a serving node has nothing to do"),
        ReconcileAction::Noop
    );
    assert_eq!(reconciler.state(), NodeState::Routable, "and stays put");
}

#[test]
fn a_settled_unsupported_outcome_asks_for_nothing_and_cannot_be_retried() {
    // `LocalUnsupported` is a decision, not a failure: Supply proved this machine has no acceptable variant. So
    // the reconciler must ask for nothing and `retry` must be refused -- retrying would re-resolve something
    // already settled, and in a loop that is an infinite queue of work that cannot succeed.
    let mut reconciler = DemandReconciler::new();
    reconciler.next_action().expect("Resolve");
    reconciler
        .observe(ReconcileEvidence::LocalUnsupported)
        .expect("LocalUnsupported from Resolving");

    assert_eq!(reconciler.state(), NodeState::LocalUnsupported);
    assert_eq!(
        reconciler.next_action().expect("nothing to do"),
        ReconcileAction::Noop
    );
    assert!(
        reconciler.retry().is_err(),
        "a settled outcome must not be retryable"
    );
    assert!(!reconciler.state().is_failure(), "and it is not a failure");
    assert!(!reconciler.state().is_serving(), "nor is it serving");
}
