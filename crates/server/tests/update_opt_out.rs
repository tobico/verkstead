//! `VERKSTEAD_NO_UPDATE_CHECK` — the environment's half of turning the update
//! check off.
//!
//! Its own test binary, and the only test in it, because proving that an
//! environment variable does anything means setting one, and a process may only
//! safely write its environment while no other thread of it is reading one.
//! What the setting then decides — no task, no request, and an endpoint that
//! still answers — is `updates.rs`'s subject; this is the wire between the two.

use clap::Parser;
use verkstead_server::Config;

#[test]
fn the_env_var_turns_the_update_check_off() {
    // SAFETY: the only test in this binary, so nothing else in this process is
    // reading the environment while it is written.
    unsafe { std::env::set_var("VERKSTEAD_NO_UPDATE_CHECK", "1") };

    let config = Config::parse_from(["verkstead serve"]);

    assert!(config.no_update_check);
    assert_eq!(
        config.releases(),
        None,
        "with the check off there is nowhere to ask, so nothing to ask",
    );
}
