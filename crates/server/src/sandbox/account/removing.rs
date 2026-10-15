//! What taking the account away came to, entry by entry: the count the verb
//! reports, the paths that kept an entry, and whether the account may go.
//!
//! **Every record is read and every entry in it taken off before the account
//! is deleted** (ADR-0014, *Amended: removing the account reads every
//! record*). The records hold the SID their entries name as text, so an entry
//! comes off whether or not the account is still there to resolve it; what
//! matters is that something reads them, and after an uninstall nothing else
//! will. This module is the arithmetic over what came of each one, and is
//! built on every platform so that it is tested on every platform: the taking
//! off itself is Windows' own — see [`super::super::granting::removing`].
//!
//! **A grant or a refusal that would not come off stops the removal**, before
//! the profile or the account goes, unless the human said `--force`. A step
//! never does: a record names every step a description asked for, including
//! ones the machine never wrote, so a step with nothing to take off is
//! ordinary — see [`Kind::Step`].

use std::path::{Path, PathBuf};

/// What an entry was, as far as removing the account cares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A grant, read-only or read-write: a session's reach into a directory of
    /// the human's.
    Grant,

    /// A refusal, which an older build wrote and which cut a directory's
    /// inheritance as well as denying the account.
    Refusal,

    /// A step through a directory on the way to another, which inherits to
    /// nothing and stops nothing when it will not come off.
    Step,
}

impl Kind {
    /// The kind of entry `wanted` is.
    #[cfg_attr(not(windows), allow(dead_code))]
    pub(crate) fn of(wanted: super::super::granting::Wanted) -> Kind {
        use super::super::granting::Wanted;

        match wanted {
            Wanted::Granted(_) => Kind::Grant,
            Wanted::Refused => Kind::Refusal,
            Wanted::Stepped => Kind::Step,
        }
    }
}

/// What came of one entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Came {
    /// It was taken off, or there was nothing of the account's on the
    /// directory to take.
    Off,

    /// The path is no longer there, so neither is the entry: a closed
    /// Worktree's directory goes with its entry on it.
    NotThere,

    /// It would not come off, and this is what the machine said.
    Kept(String),
}

/// One path that is still holding an entry naming the account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Kept {
    /// The directory.
    pub path: PathBuf,

    /// What the entry on it was.
    pub kind: Kind,

    /// And what the machine said when it was asked to take it off.
    pub why: String,
}

/// Every entry every record named, and what came of them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Tally {
    /// How many entries the records named, all of them together.
    pub total: usize,

    /// How many of those came off, counting the ones whose path had already
    /// gone: there is nothing left on the machine either way.
    pub off: usize,

    /// The ones that did not, in the order they were tried.
    pub kept: Vec<Kept>,

    /// How many Conversations' records still held entries when the removal
    /// read them.
    ///
    /// **Not how many were open.** Telling an open Conversation from one the
    /// next server would have swept takes the store, and this is a count that
    /// answers the question without one: these were entries that would have
    /// stood until a close or a sweep, which after an uninstall is never.
    pub conversations: usize,
}

impl Tally {
    /// One entry counted, at `path`, of `kind`, with what came of it.
    pub fn counted(&mut self, path: &Path, kind: Kind, came: Came) {
        self.total += 1;

        match came {
            Came::Off | Came::NotThere => self.off += 1,
            Came::Kept(why) => self.kept.push(Kept {
                path: path.to_owned(),
                kind,
                why,
            }),
        }
    }

    /// Whether anything that would not come off is a reason to leave the
    /// account where it is: a grant or a refusal, and never a step.
    pub fn stops(&self) -> bool {
        self.kept.iter().any(|kept| kept.kind != Kind::Step)
    }

    /// Whether the account, its profile and its password go: when nothing
    /// stops it, or when the human said to go on regardless.
    pub fn goes_on(&self, force: bool) -> bool {
        force || !self.stops()
    }

    /// The `entries` line: how many came off out of how many there were.
    pub fn entries(&self) -> String {
        if self.total == 0 {
            return "no record named any, so there were none to take off".to_owned();
        }

        format!("{} of {} taken off", self.off, self.total)
    }

    /// The `records` line: how many Conversations were still holding entries
    /// of their own.
    pub fn records(&self) -> String {
        match self.conversations {
            0 => "no Conversation's record still held entries".to_owned(),
            1 => "1 Conversation's record still held entries, counted above".to_owned(),
            many => format!("{many} Conversations' records still held entries, counted above"),
        }
    }

    /// One line per path that kept an entry, for stderr.
    pub fn kept_lines(&self) -> Vec<String> {
        self.kept
            .iter()
            .map(|kept| {
                let path = kept.path.display();
                let why = &kept.why;

                match kept.kind {
                    Kind::Grant => format!("a grant on {path} would not come off: {why}"),
                    Kind::Refusal => format!("a refusal on {path} would not come off: {why}"),
                    Kind::Step => format!(
                        "a step through {path} would not come off, which leaves nothing \
                         reachable and stops nothing: {why}"
                    ),
                }
            })
            .collect()
    }

    /// What the verb says when it has stopped short of deleting anything.
    pub fn held_back(&self) -> String {
        let stopping = self
            .kept
            .iter()
            .filter(|kept| kept.kind != Kind::Step)
            .count();

        let entries = if stopping == 1 { "entry" } else { "entries" };

        format!(
            "{stopping} {entries} above would not come off, so the account, its profile and its \
             password were left where they are; put those paths right and run this again, or \
             pass --force to remove the account regardless"
        )
    }
}

/// The line said before entry `at` of `of` is taken off, so that a directory
/// Windows is walking for seconds has a line in front of it.
pub fn announced(at: usize, of: usize, path: &Path) -> String {
    format!("taking off {at} of {of}: {}", path.display())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tally(outcomes: &[(Kind, Came)]) -> Tally {
        let mut tally = Tally::default();

        for (at, (kind, came)) in outcomes.iter().enumerate() {
            tally.counted(&PathBuf::from(format!("/d{at}")), *kind, came.clone());
        }

        tally
    }

    fn kept() -> Came {
        Came::Kept("Access is denied. (os error 5)".to_owned())
    }

    /// Every entry off, or gone with its directory, is `n of n`, and the
    /// account goes.
    #[test]
    fn every_entry_off_or_gone_is_all_of_them_and_the_account_goes() {
        let tally = tally(&[
            (Kind::Grant, Came::Off),
            (Kind::Grant, Came::NotThere),
            (Kind::Step, Came::Off),
            (Kind::Refusal, Came::Off),
        ]);

        assert_eq!(tally.entries(), "4 of 4 taken off");
        assert!(!tally.stops());
        assert!(tally.goes_on(false));
        assert!(tally.kept_lines().is_empty());
    }

    /// A grant that would not come off stops the removal, says how many came
    /// off, and names the path; `--force` goes on regardless.
    #[test]
    fn a_grant_that_stays_stops_the_removal_unless_forced() {
        let tally = tally(&[
            (Kind::Step, Came::Off),
            (Kind::Step, Came::Off),
            (Kind::Grant, Came::Off),
            (Kind::Grant, Came::Off),
            (Kind::Grant, kept()),
            (Kind::Grant, kept()),
        ]);

        assert_eq!(tally.entries(), "4 of 6 taken off");
        assert!(tally.stops());
        assert!(!tally.goes_on(false), "nothing goes without --force");
        assert!(tally.goes_on(true), "and the account goes with it");

        assert_eq!(
            tally.kept_lines(),
            vec![
                "a grant on /d4 would not come off: Access is denied. (os error 5)",
                "a grant on /d5 would not come off: Access is denied. (os error 5)",
            ],
        );
        assert!(
            tally
                .held_back()
                .starts_with("2 entries above would not come off")
        );
    }

    /// A refusal stops it the same way.
    #[test]
    fn a_refusal_that_stays_stops_the_removal() {
        let tally = tally(&[(Kind::Refusal, kept())]);

        assert!(tally.stops());
        assert!(
            tally
                .held_back()
                .starts_with("1 entry above would not come off")
        );
    }

    /// A step that will not come off is named and counted, and stops nothing.
    #[test]
    fn a_step_that_stays_stops_nothing() {
        let tally = tally(&[(Kind::Grant, Came::Off), (Kind::Step, kept())]);

        assert_eq!(tally.entries(), "1 of 2 taken off");
        assert!(!tally.stops());
        assert!(tally.goes_on(false));
        assert_eq!(tally.kept_lines().len(), 1);
        assert!(tally.kept_lines()[0].contains("stops nothing"));
    }

    /// No record at all is said in words rather than as `0 of 0`.
    #[test]
    fn no_records_is_said_rather_than_counted() {
        let tally = Tally::default();

        assert_eq!(
            tally.entries(),
            "no record named any, so there were none to take off"
        );
        assert!(tally.goes_on(false));
    }

    /// The Conversations' line counts records, in the singular and the plural.
    #[test]
    fn the_conversations_still_holding_entries_are_counted() {
        let mut tally = Tally::default();

        assert_eq!(
            tally.records(),
            "no Conversation's record still held entries"
        );

        tally.conversations = 1;
        assert_eq!(
            tally.records(),
            "1 Conversation's record still held entries, counted above"
        );

        tally.conversations = 3;
        assert_eq!(
            tally.records(),
            "3 Conversations' records still held entries, counted above"
        );
    }

    /// Each entry is announced with where it is in the whole.
    #[test]
    fn an_entry_is_announced_with_its_place_in_the_whole() {
        assert_eq!(
            announced(34, 60, Path::new("/home/ada/.claude")),
            "taking off 34 of 60: /home/ada/.claude"
        );
    }
}
