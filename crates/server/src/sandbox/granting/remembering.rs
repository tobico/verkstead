//! What was written for a container, written down: the one thing a server that
//! did not make a profile has to have before it can take that profile away.
//!
//! **Because an access-control entry outlives the process that wrote it.** A
//! grant is a change to a directory on the human's own disk, and the identity it
//! names is a profile on their machine — neither goes when Verkstead does. So a
//! server that died holding a Conversation's container left a boundary standing
//! that nothing in memory can describe any more, and the next server has to be
//! able to say, of a directory it never granted, *this entry is one of mine and
//! here is what it names*. This is where that is said (ADR-0014).
//!
//! **One record per Conversation**, under the Data Directory, named by the
//! Conversation's id — which is what the sweep at startup reads back, one
//! candidate per file, the way the orphaned worktrees are one candidate per
//! directory (see [`crate::containers`]).
//!
//! **What is in it is the profile and every entry written for it.** The name,
//! because that is what deletes a profile; the SID, because that is what an
//! entry names and what a grant is taken back for; and the entries themselves,
//! because a description cannot be worked out again after the fact — a closed
//! Conversation has no Worktree left to build one from, and what is on those
//! directories was written by a description that no longer exists anywhere else.
//! And beside them, of the paths a description refuses, the ones that were
//! taking entries from above before any of this was written: a refusal cuts
//! that, so it is the one thing about those directories that stops being
//! readable off them the moment a container is made.
//!
//! **In a spelling of its own.** A record is written by one build and read by
//! the next one, so what goes on the disk is [`Word`] rather than whatever the
//! types it comes from are called this week: a rename inside the crate is a
//! rename, and a rename of what is written here is a record the server after it
//! cannot read.
//!
//! **A record that will not read is left exactly as it is.** Nothing here
//! guesses: a half-understood record would be a profile deleted with entries
//! still on somebody's directories naming it, which is the one outcome worse
//! than the entries staying — see [`read`].
//!
//! Built on every machine, for the reason [`super::entries`] is: what was
//! written down is a file, and only the taking-back of an entry is a call one
//! platform has.

use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::super::surface::Reach;
use super::{Entry, Wanted};

/// The directory the records go in, under the Data Directory.
///
/// Named here rather than composed by its callers, for the reason
/// [`crate::worktrees::directory`] is: two things want it whole — what writes a
/// record, and the sweep that reads every one there is.
pub(crate) fn directory(data_dir: &Path) -> PathBuf {
    data_dir.join("containers")
}

/// One Conversation's record: the profile its sessions run inside, and every
/// entry written on a real directory for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Remembered {
    /// Whose it is, which is the file's own name.
    pub(crate) conversation: i64,

    /// What the profile is called on this machine, which is what deletes it.
    pub(crate) name: String,

    /// And the identity it is, which is what every entry below names.
    pub(crate) sid: String,

    /// Those entries, in the order they were written.
    pub(crate) entries: Vec<Entry>,

    /// And which of the refused paths among them were taking entries from above
    /// when the first of this Conversation's sessions started.
    ///
    /// **The one thing about a directory that cannot be read off it later.** A
    /// refusal cuts the inheritance on the path it refuses, so a server looking
    /// at one afterwards — this one at the end of the Conversation, or the next
    /// one after a crash — finds a directory inheriting nothing whether it was
    /// inheriting anything before or not, and putting an inheritance back that
    /// was never cut takes the directory's own entries off it. See
    /// [`super::writing::inheriting`], which is what reads this while there is
    /// still an answer to read.
    pub(crate) cut: Vec<PathBuf>,
}

/// Write `remembered` down, whole.
///
/// **Rewritten rather than added to.** A Conversation's second session
/// describes its own surface and the container remembers both descriptions
/// together — see [`super::super::container::Container::wrote`] — so what is
/// written here is that whole list each time, and a record is never half of
/// one.
///
/// **Refusing matters**, which is why this hands back an error at all: a
/// container nothing wrote down is one no later server can take away, and its
/// entries would sit on the human's own directories for good. So the caller
/// that cannot write one refuses the session rather than granting anything —
/// which is the same answer a profile that will not be created gets (ADR-0014,
/// Q18).
///
/// **And atomically**, through the settings files' own
/// [`crate::settings::write_atomically`]: a record is rewritten at every
/// session start, so a truncate-then-write would put a window on every one of
/// them in which the file is half of a record. What a crash inside that window
/// leaves is the one thing this module exists to prevent — a record the next
/// server cannot parse, and so a profile and a set of entries on the human's
/// own directories that nothing will ever take back, because a Conversation
/// that has finished never runs another session to write the record again. A
/// rename makes it the old record or the new one and never a half of either.
pub(crate) fn wrote(data_dir: &Path, remembered: &Remembered) -> io::Result<()> {
    let directory = directory(data_dir);

    std::fs::create_dir_all(&directory)?;

    let written = Written {
        name: remembered.name.clone(),
        sid: remembered.sid.clone(),
        entries: remembered.entries.iter().map(Line::of).collect(),
        cut: remembered.cut.clone(),
    };

    let body = serde_json::to_string_pretty(&written).map_err(io::Error::other)?;

    crate::settings::write_atomically(
        &record(data_dir, remembered.conversation),
        &body,
        crate::settings::ORDINARY_MODE,
    )
}

/// Read one back, or nothing where there is none to read.
///
/// **Nothing is a record that is not there and a record that will not read
/// alike**, and the difference is in the log rather than in what comes back:
/// what a caller does about either is leave the machine as it found it, and a
/// record this build cannot make sense of is one whose entries it cannot take
/// back safely.
pub(crate) fn read(data_dir: &Path, conversation: i64) -> Option<Remembered> {
    at(&record(data_dir, conversation), conversation)
}

/// And every record there is, for the sweep that decides between them.
///
/// **Every candidate comes out of reading this one directory**, which is the
/// rule the other sweeps keep: no path here is built from a record or from
/// anything a request carried, so what is considered is what Verkstead itself
/// wrote in the one directory it writes them in.
///
/// A directory that is not there is a Verkstead that has never made a container
/// — every one on a Unix, and a Windows one before its first session — and is
/// not worth a word in the log.
pub(crate) fn left_behind(data_dir: &Path) -> Vec<Remembered> {
    let mut found = Vec::new();

    let entries = match std::fs::read_dir(directory(data_dir)) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return found,
        Err(error) => {
            tracing::error!(
                error = ?error,
                path = %directory(data_dir).display(),
                "the containers directory could not be read, so nothing is being swept",
            );

            return found;
        }
    };

    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                tracing::error!(
                    error = ?error,
                    path = %directory(data_dir).display(),
                    "an entry of the containers directory could not be read, so it is being \
                     left alone",
                );

                continue;
            }
        };

        let path = entry.path();

        let name = path.file_name().and_then(|name| name.to_str());

        // The neighbouring file a record is written through, caught before the
        // arm below says it is a stranger: [`wrote`] writes into this same
        // directory and renames, so a sweep running while a session starts can
        // see one — and one left behind is a rename that failed rather than
        // anything to warn about twice.
        if name.is_some_and(|name| name.starts_with('.')) {
            continue;
        }

        // A name that is not a Conversation's id is not a record this wrote:
        // every one of them is named by [`record`] and by nothing else.
        let Some(conversation) = name.and_then(|name| name.parse::<i64>().ok()) else {
            tracing::warn!(
                path = %path.display(),
                "something that is not a container's record is in the containers directory, \
                 so it is being left alone",
            );

            continue;
        };

        if let Some(remembered) = at(&path, conversation) {
            found.push(remembered);
        }
    }

    found
}

/// Take a record away, whatever it said.
///
/// The last thing that happens to a container: the entries have come off and
/// the profile has gone, so what is left is a record of a boundary that is no
/// longer anywhere. A record that will not go is named in the log and nothing
/// else — the next sweep reads it, finds a profile that is not there and takes
/// nothing back, which is the same nothing this failed to do.
pub(crate) fn forget(data_dir: &Path, conversation: i64) {
    let path = record(data_dir, conversation);

    match std::fs::remove_file(&path) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => tracing::warn!(
            error = ?error,
            path = %path.display(),
            "the record of a container that has gone could not be removed, so the next sweep \
             will read it again",
        ),
    }
}

/// Where one Conversation's record is.
fn record(data_dir: &Path, conversation: i64) -> PathBuf {
    directory(data_dir).join(conversation.to_string())
}

/// One record read off `path`, or nothing with the reason in the log.
fn at(path: &Path, conversation: i64) -> Option<Remembered> {
    let body = match std::fs::read(path) {
        Ok(body) => body,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return None,
        Err(error) => {
            tracing::warn!(
                error = ?error,
                path = %path.display(),
                "a container's record could not be read, so what it names is being left alone",
            );

            return None;
        }
    };

    let written: Written = match serde_json::from_slice(&body) {
        Ok(written) => written,
        Err(error) => {
            tracing::warn!(
                error = ?error,
                path = %path.display(),
                "a container's record is not one this build can read, so what it names is \
                 being left alone rather than half taken back",
            );

            return None;
        }
    };

    Some(Remembered {
        conversation,
        name: written.name,
        sid: written.sid,
        entries: written.entries.iter().map(Line::entry).collect(),
        cut: written.cut,
    })
}

/// A record as it is written down — see this module's own documentation for why
/// it is a shape of its own rather than the types it is made from.
#[derive(Debug, Serialize, Deserialize)]
struct Written {
    name: String,
    sid: String,
    entries: Vec<Line>,

    /// Missing from a record the build before this one wrote, and nothing there
    /// rather than a record that will not read: what it says is about paths
    /// whose inheritance was cut, and a record that does not say leaves them
    /// protected — the entries on them come off either way, which is what a
    /// sweep is for.
    #[serde(default)]
    cut: Vec<PathBuf>,
}

/// And one entry in it.
#[derive(Debug, Serialize, Deserialize)]
struct Line {
    path: PathBuf,
    wanted: Word,
}

/// What an entry says, in the spelling the disk holds.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Word {
    ReadOnly,
    ReadWrite,

    /// **Every step a description asked for, rather than the ones the machine
    /// took.** A record is written before a word of it is — see
    /// [`super::super::container::Container::wrote`] — so what is remembered
    /// here is the whole list, and an ancestor the machine refused an entry on
    /// is one there was never anything to take back from. Which is the safe way
    /// round: a step written and not remembered would sit on a directory of the
    /// human's naming an identity that has gone, and that is the one outcome
    /// this file exists to prevent.
    Stepped,

    Refused,
}

impl Line {
    /// One entry as it is written down.
    fn of(entry: &Entry) -> Line {
        Line {
            path: entry.path.clone(),
            wanted: match entry.wanted {
                Wanted::Granted(Reach::ReadOnly) => Word::ReadOnly,
                Wanted::Granted(Reach::ReadWrite) => Word::ReadWrite,
                Wanted::Stepped => Word::Stepped,
                Wanted::Refused => Word::Refused,
            },
        }
    }

    /// And as it is read back.
    fn entry(&self) -> Entry {
        Entry {
            path: self.path.clone(),
            wanted: match self.wanted {
                Word::ReadOnly => Wanted::Granted(Reach::ReadOnly),
                Word::ReadWrite => Wanted::Granted(Reach::ReadWrite),
                Word::Stepped => Wanted::Stepped,
                Word::Refused => Wanted::Refused,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One of everything a description comes to, remembered under Conversation
    /// 7.
    fn remembered(conversation: i64) -> Remembered {
        Remembered {
            conversation,
            name: format!("verkstead-0123456789abcdef-{conversation}"),
            sid: "S-1-15-2-1234567890-1234567890-1234567890-1234567890".to_owned(),
            entries: vec![
                Entry {
                    path: PathBuf::from(r"C:\state\worktrees\verkstead-rate-limiting"),
                    wanted: Wanted::Granted(Reach::ReadWrite),
                },
                Entry {
                    path: PathBuf::from(r"C:\Users\ada\AppData\Roaming\npm"),
                    wanted: Wanted::Granted(Reach::ReadOnly),
                },
                Entry {
                    path: PathBuf::from(r"C:\Users\ada\.claude\skills"),
                    wanted: Wanted::Refused,
                },
                // And the way to one of them, which is remembered beside the
                // rest for the same reason the rest are: a step is an entry on
                // a directory of the human's, and it comes off with them.
                Entry {
                    path: PathBuf::from(r"C:\Users\ada"),
                    wanted: Wanted::Stepped,
                },
            ],
            cut: vec![PathBuf::from(r"C:\Users\ada\.claude\skills")],
        }
    }

    /// Every entry comes back as it went in, which is the whole of what a record
    /// is for: the server that reads one never saw the description it came from.
    #[test]
    fn what_was_written_for_a_container_is_read_back_whole() {
        let held = tempfile::tempdir().unwrap();
        let written = remembered(7);

        wrote(held.path(), &written).expect("a record to be writable under the Data Directory");

        assert_eq!(read(held.path(), 7).as_ref(), Some(&written));
        assert_eq!(left_behind(held.path()), vec![written]);
    }

    /// And a record that has gone is nothing, which is what a sweep finds after
    /// a close.
    #[test]
    fn a_container_that_was_forgotten_is_no_longer_remembered() {
        let held = tempfile::tempdir().unwrap();

        wrote(held.path(), &remembered(7)).unwrap();
        forget(held.path(), 7);

        assert_eq!(read(held.path(), 7), None);
        assert_eq!(left_behind(held.path()), Vec::<Remembered>::new());
        assert_eq!(
            read(held.path(), 8),
            None,
            "and neither is one nobody wrote"
        );
    }

    /// A record this build cannot read is one it says nothing about — rather
    /// than one it half understands, which would be a profile deleted out from
    /// under entries still naming it.
    #[test]
    fn a_record_that_will_not_read_names_nothing_to_take_back() {
        let held = tempfile::tempdir().unwrap();

        wrote(held.path(), &remembered(7)).unwrap();
        std::fs::write(directory(held.path()).join("7"), "{ this is not a record }").unwrap();

        assert_eq!(read(held.path(), 7), None);
        assert_eq!(left_behind(held.path()), Vec::<Remembered>::new());
    }

    /// And what is in that directory under no Conversation's name is nothing
    /// this wrote, whatever is in it.
    #[test]
    fn what_is_not_a_records_name_is_no_candidate() {
        let held = tempfile::tempdir().unwrap();

        wrote(held.path(), &remembered(7)).unwrap();
        std::fs::write(
            directory(held.path()).join("not-a-conversation"),
            "somebody else's\n",
        )
        .unwrap();

        assert_eq!(
            left_behind(held.path())
                .iter()
                .map(|remembered| remembered.conversation)
                .collect::<Vec<_>>(),
            vec![7],
        );
    }
}
