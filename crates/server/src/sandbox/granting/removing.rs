//! Every record under the Data Directory read, and every entry in it taken
//! off by the SID that record names: the half of removing the account that is
//! about the human's directories rather than about the account.
//!
//! **Every record, whichever identity it names.** The installation's own, every
//! identity it stood for before the current one, every Conversation's and the
//! Compile Server's. A Conversation's entries are otherwise taken off by a
//! close or by the sweep a server starts with, and neither runs after an
//! uninstall; an earlier identity's are taken off by nothing else at all. So
//! this is the last thing that will read them, and it reads them all.
//!
//! **Nothing here needs the account to be there.** Each record holds its SID as
//! text and an entry is taken off by that — see [`super::writing::stripped`],
//! whose `Sid::of` parses one that resolves to nobody as readily as one that
//! resolves to somebody. Which is what makes a machine whose account is
//! already gone, with entries still standing that name it, one this repairs.
//!
//! **Entries first and each record after**, in the order a boundary ending
//! keeps — see [`super::super::entries`]'s `given_back`: a record is what a
//! later run would take its entries off by, so it goes only once there is
//! nothing in it that stops the removal.
//!
//! Not elevated, and nothing of it needs to be: the entries are on the human's
//! own directories, which is why the suite can run it as the human.

use std::path::{Path, PathBuf};

use super::super::account::removing::{Came, Kind, Tally};
use super::super::entries::NOBODYS;
use super::{Entry, remembering, writing};

/// One record's worth of entries, and what to do with the record once they
/// have come off.
struct Held {
    sid: String,
    entries: Vec<Entry>,
    cut: Vec<PathBuf>,

    /// The Conversation the record is under, or nothing where it is the
    /// installation's own.
    conversation: Option<i64>,
}

/// Every entry every record under `data_dir` names, taken off, and each record
/// that came off whole forgotten.
///
/// `before` is told, ahead of each entry, where it is in the whole and which
/// directory it is on. What came of every one is in the [`Tally`] handed back.
pub(crate) fn taken_off(data_dir: &Path, before: &mut dyn FnMut(usize, usize, &Path)) -> Tally {
    let mut held = Vec::new();

    let standing = remembering::standing_read(data_dir);

    if let Some(standing) = &standing {
        held.push(Held {
            sid: standing.sid.clone(),
            entries: standing.entries.clone(),
            cut: Vec::new(),
            conversation: None,
        });

        for earlier in &standing.earlier {
            held.push(Held {
                sid: earlier.sid.clone(),
                entries: earlier.entries.clone(),
                cut: Vec::new(),
                conversation: None,
            });
        }
    }

    let mut tally = Tally::default();

    for remembered in remembering::left_behind(data_dir) {
        if remembered.conversation != NOBODYS && !remembered.entries.is_empty() {
            tally.conversations += 1;
        }

        held.push(Held {
            sid: remembered.sid,
            entries: remembered.entries,
            cut: remembered.cut,
            conversation: Some(remembered.conversation),
        });
    }

    let total = held.iter().map(|held| held.entries.len()).sum();
    let mut at = 0usize;

    // The installation's record is one file for every identity in it, so it
    // goes only if none of them kept an entry that stops the removal.
    let mut standing_whole = true;

    for one in &held {
        let came = writing::stripped(&one.entries, &one.cut, &one.sid, &mut |entry| {
            at += 1;
            before(at, total, &entry.path);
        });

        let mut whole = true;

        for (entry, came) in came {
            let kind = Kind::of(entry.wanted);

            if matches!(came, Came::Kept(_)) && kind != Kind::Step {
                whole = false;
            }

            tally.counted(&entry.path, kind, came);
        }

        match one.conversation {
            Some(conversation) if whole => remembering::forget(data_dir, conversation),
            Some(_) => {}
            None => standing_whole &= whole,
        }
    }

    if standing.is_some() && standing_whole {
        remembering::standing_forget(data_dir);
    }

    tally
}
