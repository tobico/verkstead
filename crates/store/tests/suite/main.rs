//! The store's tests, as one test binary rather than one per file.
//!
//! A test binary is a link, and on Windows a link of everything the store
//! stands on is most of what building a test costs — so the files are modules
//! of this one, and each is the suite it was when it was a binary of its own.
//! `owning` is the fixture two of them share, declared here once.

mod owning;

mod arrivals;
mod banners;
mod births;
mod cleanup;
mod commits;
mod continuations;
mod conversations;
mod database;
mod deferrals;
mod deliveries;
mod direction;
mod endings;
mod joins;
mod landings;
mod manual_tasks;
mod members;
mod memos;
mod migrations;
mod pairings;
mod pauses;
mod pending_steers;
mod previews;
mod process;
mod profiles;
mod push;
mod ranks;
mod reports;
mod repos;
mod settling;
mod shares;
mod slices;
mod steers;
mod stops;
mod unseen;
mod waits;
mod wrap_up;
mod wrapping;
