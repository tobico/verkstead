//! The words a Conversation's derived conditions are said in.
//!
//! A condition is something true *of* a state rather than a state of its own —
//! the ladder is untouched and nothing new is stored — so it is drawn beside the
//! lifecycle word rather than in place of one on the wire. What it is called is
//! this module's, in one place, because the same condition is said twice: once
//! on the card the human opens and once in the sidebar row they find it by, and
//! a condition worded two ways is two conditions to the person reading them.

/// A wrap-up that has got down to its checks: the review answered, nothing said
/// on the pull request left unaddressed, and nothing running in the Worktree.
///
/// *Checks* rather than *CI*, which is the codebase's word throughout: a
/// GitHub check may be a required review or a deploy gate as easily as a test
/// suite, and the human is waiting on all of them alike.
export const WAITING_ON_CHECKS = "Waiting on checks";
