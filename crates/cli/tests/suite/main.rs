//! The CLI's tests, as one test binary rather than one per file.
//!
//! A test binary is a link, and on Windows a link of the server and everything
//! under it is most of what building a test costs — so the files are modules of
//! this one, and each is the suite it was when it was a binary of its own.
//! `support` is the fixture they share, declared here once. The `verkstead`
//! they run is still `CARGO_BIN_EXE_verkstead`, the manifest's binary, whatever
//! this one is called.

mod support;

mod answers;
mod ask;
mod done;
mod guide;
mod launcher_windows;
mod memo;
mod pipe_windows;
mod preview;
mod repo;
mod sandbox;
mod sandbox_windows;
mod serve;
mod transfer;
mod waiting;
