//! The schema's tests, as one test binary rather than one per file.
//!
//! A test binary is a link, and on Windows a link is most of what building a
//! test costs — so the files are modules of this one, and each is the suite it
//! was when it was a binary of its own.

mod examples;
mod question_set;
mod response;
