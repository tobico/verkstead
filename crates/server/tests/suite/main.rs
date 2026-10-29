//! The server's tests, as one test binary rather than one per file.
//!
//! A test binary is a link, and on Windows a link of the server and everything
//! under it is most of what building a test costs — so the files are modules of
//! this one, and each is the suite it was when it was a binary of its own. One
//! suite is run by its module's name: `cargo test -p verkstead-server --test
//! suite -- conversations::`.
//!
//! **Seven stay out**, each a test binary of its own beside this one:
//! `per_user_path`, `servers_path`, `session_path` and `vendor_installers` move
//! `HOME` and `PATH` on the process and look again, `advertising_opt_out` and
//! `update_opt_out` set and clear an opt-out variable, and `withdrawing` raises
//! `SIGTERM` at the process it is in. What makes each of them safe is being the
//! only test in its process, which a module of this one would not be — every
//! file here runs on threads of the one process under `cargo test`, beside
//! every other.
//!
//! One module is not a suite: `fixture_repositories` is the git repository a
//! test copies rather than makes, for the suites that can reach it.

mod account_windows;
mod accounts;
mod adding;
mod advertised;
mod advertising;
mod annex_forward;
mod announcing;
mod asking;
mod attaching;
mod bridging;
mod browsing;
mod conversations;
mod device;
mod devices;
mod dialling;
mod discovering;
mod exchange;
mod files;
mod fixture_repositories;
mod freshness;
mod installing;
mod joining;
mod kept;
mod logins;
mod logins_windows;
mod matching;
mod member_updates;
mod merging;
mod mirroring;
mod news;
mod notices;
mod nudges;
mod onboarding;
mod package_stores;
mod peer;
mod pictures;
mod pipe_windows;
mod ports;
mod preflight;
mod probing;
mod profiles;
mod push_delivery;
mod relayed;
mod relaying;
mod remote;
mod renewing;
mod replicating;
mod repos;
mod responses;
mod sandbox;
mod sandbox_macos;
mod sandbox_windows;
mod screen;
mod sessions;
mod sessions_windows;
mod sets;
mod settings;
mod sharing;
mod skeleton;
mod terminal;
mod terminal_windows;
mod transcripts;
mod transfer;
mod ui_api;
mod ui_content;
mod unlinking;
mod updates;
mod viewer;
mod watching;
mod workbench_key;
