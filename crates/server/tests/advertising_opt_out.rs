//! `VERKSTEAD_NO_ADVERTISING` — the environment's half of turning the
//! advertisement off (ADR-0020, *Discovery*).
//!
//! Its own test binary, and the only test in it, for the reason
//! `update_opt_out.rs` beside it is: proving that an environment variable does
//! anything means setting one, and a process may only safely write its
//! environment while no other thread of it is reading one. Which is also why
//! both halves of the switch are asked about inside one test rather than in two
//! beside each other.
//!
//! What the setting then decides — a device that says nothing about itself on
//! the LAN, and is found only by an address somebody types — is
//! `advertising.rs`'s subject; this is the wire between the two.

use clap::Parser;
use verkstead_server::Config;

#[test]
fn the_env_var_turns_the_advertisement_off() {
    // SAFETY: the only test in this binary, so nothing else in this process is
    // reading the environment while it is written.
    unsafe { std::env::remove_var("VERKSTEAD_NO_ADVERTISING") };

    assert!(
        Config::parse_from(["verkstead serve"]).advertises(),
        "on where nobody has said otherwise, for the reason `openFirewall` is on by \
         default: a discovery nothing can hear is a feature that silently does not work, \
         with nothing on either machine saying why",
    );

    // SAFETY: as above.
    unsafe { std::env::set_var("VERKSTEAD_NO_ADVERTISING", "1") };

    let config = Config::parse_from(["verkstead serve"]);

    assert!(config.no_advertising);
    assert!(
        !config.advertises(),
        "and with the switch thrown there is nothing to say and nowhere to say it",
    );
}
