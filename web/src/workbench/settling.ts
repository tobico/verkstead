//! How long a pause in typing is, before a field that saves itself keeps what
//! is in it.
//!
//! One number, shared by every self-saving field on a drafting Conversation —
//! the Brief and the branch name — because they are the same card and a human
//! typing across both should not meet two different ideas of what a pause is.
//! Its own module because the Brief's card draws the setup the branch stands
//! in, so neither of the two files can own it without the other importing it
//! back.

/// Long enough that a sentence is one save rather than a save a word, and short
/// enough that a human who typed and then sat back has a saved draft by the
/// time they have read it over. Leaving the field saves it whatever the timer
/// was about to do.
export const SETTLE = 800;
