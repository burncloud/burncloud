#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "Test-only integration file: unwrap/expect is the intended failure signal, and the \
              assertions compare real accessor output rather than trusting the constructor."
)]
//! Constructor and default-state contract for the aria2 manager and daemon (#635).
//!
//! Everything in this file is deliberately **side-effect free**: it constructs the two owners and
//! reads their accessors, and never calls `start_aria2_rpc`, `Aria2Daemon::start` or
//! `download_aria2`. Those three reach the network or spawn `aria2c`, and this increment's plan
//! forbids them in tests. The machine-wide process sweep that this list used to name was removed in
//! #898, so no function this file can reach terminates a process any more.
//!
//! Why this exists: `manager.rs` and `daemon.rs` previously had no tests in any form, and these
//! two accessors are exactly what a caller uses to decide whether it may talk to a daemon.
//! "A manager/daemon that has never started is not running and hands out no RPC client" is real
//! behaviour of the API, not a tautology about a freshly built struct -- a stub returning `true`
//! or `Some` would send callers to a daemon that does not exist.
//!
//! What this file deliberately does **not** claim: the daemon's stored configuration. `Aria2Daemon`
//! exposes no config accessor, and inventing one would be a production change, which this
//! increment is not allowed to make. The config round-trip is covered on the manager instead.

use burncloud_download_aria2::{Aria2Config, Aria2Daemon, Aria2Manager};

#[test]
fn a_fresh_manager_is_neither_running_nor_connected() {
    let manager = Aria2Manager::new();

    assert!(
        !manager.is_running(),
        "a manager that was never started must not report itself as running, or callers will \
         address a daemon that does not exist"
    );
    assert!(
        manager.create_rpc_client().is_none(),
        "a manager with no daemon must not hand out an RPC client"
    );
    assert!(
        manager.get_rpc_client().is_none(),
        "the borrowed accessor must agree with create_rpc_client"
    );
}

#[test]
fn default_is_the_same_construction_as_new() {
    // `Default` documents itself as reusing `new`, so both paths must be observably equivalent.
    let from_new = Aria2Manager::new();
    let from_default = Aria2Manager::default();

    assert_eq!(
        from_new.is_running(),
        from_default.is_running(),
        "Default and new must agree on the running state"
    );
    assert_eq!(
        from_new.create_rpc_client().is_some(),
        from_default.create_rpc_client().is_some(),
        "Default and new must agree on whether a client is available"
    );
}

#[test]
fn a_manager_built_from_a_config_is_still_not_running_and_keeps_the_client_slot_empty() {
    // `with_config` is the constructor a caller reaches for when it does not want the defaults.
    // It must not start anything either: only `start_daemon` may move the manager to running.
    let manager = Aria2Manager::with_config(Aria2Config::default());

    assert!(
        !manager.is_running(),
        "configuring a manager must not start a daemon"
    );
    assert!(
        manager.create_rpc_client().is_none(),
        "a configured-but-unstarted manager must not hand out an RPC client"
    );
}

#[test]
fn a_daemon_built_from_a_config_starts_stopped() {
    // `Aria2Daemon::start` reaches the network and spawns aria2c, so the only assertion available
    // here is the documented pre-start state -- which is precisely the state the restart monitor
    // branches on.
    let daemon = Aria2Daemon::new(Aria2Config::default());

    assert!(
        !daemon.is_running(),
        "a daemon that was never started must report itself as not running"
    );
    assert!(
        daemon.get_rpc_client().is_none(),
        "a stopped daemon must not expose an RPC client bound to a nonexistent instance"
    );
}

#[test]
fn the_default_config_describes_a_real_daemon_that_could_be_started() {
    // Guards against a default that cannot work: port 0 cannot be bound and an empty split size
    // would be rejected by aria2. Both would only surface at daemon start otherwise.
    let config = Aria2Config::default();

    assert!(
        config.port > 0,
        "the default port must be a real port, not 0"
    );
    assert!(
        !config.split_size.is_empty(),
        "the default split size must not be empty"
    );
    assert!(
        !config.download_dir.as_os_str().is_empty(),
        "the default download directory must not be empty"
    );
}
