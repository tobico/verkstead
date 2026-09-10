//! What was written for a Conversation, written down: the one thing a server
//! that did not write an entry has to have before it can take that entry away.
//!
//! **Because an access-control entry outlives the process that wrote it.** A
//! grant is a change to a directory on the human's own disk, and the identity it
//! names is a local account on their machine — neither goes when Verkstead does.
//! So a server that died holding a Conversation's entries left a boundary
//! standing that nothing in memory can describe any more, and the next server
//! has to be able to say, of a directory it never granted, *this entry is one of
//! mine and here is what it names*. This is where that is said (ADR-0014).

//!
//! **One record per Conversation**, under the Data Directory, named by the
//! Conversation's id — which is what the sweep at startup reads back, one
//! candidate per file, the way the orphaned worktrees are one candidate per
//! directory (see [`crate::boundaries`]).

//!
//! **What is in it is the account and every entry written for it.** The
//! account's name, so that a server which did not resolve it can still say
//! which one its entries are for; the SID, because that is what an entry names
//! and what a grant is taken back for; and the entries themselves, because a
//! description cannot be worked out again after the fact — a closed
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
//! guesses: a half-understood record would be half a boundary taken back, with
//! entries left standing on somebody's directories that nothing will look at
//! again — see [`read`].

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
///
/// **Still called what it was called when a Conversation's boundary was an
/// AppContainer**, which is this module's own rule about spellings read the
/// other way round: what is on the disk is written by one build and read by the
/// next, and a directory renamed here is every record the build before it wrote
/// left where nothing will ever look at it again. The entries in one of those
/// are entries on the human's directories whichever identity they name, and the
/// sweep takes them off either way.
pub(crate) fn directory(data_dir: &Path) -> PathBuf {
    data_dir.join("containers")
}

/// One Conversation's record: the account its sessions run as, and every entry
/// written on a real directory for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Remembered {
    /// Whose it is, which is the file's own name.
    pub(crate) conversation: i64,

    /// What the account is called on this machine, which is what a server that
    /// did not resolve it says in the log as it takes the entries back.
    pub(crate) account: String,

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
/// describes its own surface and its entries remember both descriptions
/// together — see [`super::super::entries::Entries::wrote`] — so what is
/// written here is that whole list each time, and a record is never half of
/// one.
///
/// **Refusing matters**, which is why this hands back an error at all: entries
/// nothing wrote down are entries no later server can take away, and they would
/// sit on the human's own directories for good. So the caller that cannot write
/// one refuses the session rather than granting anything (ADR-0014, Q18).
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
        account: remembered.account.clone(),
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
///
/// A record the AppContainer build wrote reads as one of these: it names a
/// profile's SID rather than an account's, and the entries it carries come off
/// the human's directories exactly the same way. See [`Written::account`].
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
/// The last thing that happens to a Conversation's boundary: the entries have
/// come off, so what is left is a record of one that is no longer anywhere. A
/// record that will not go is named in the log and nothing else — the next
/// sweep reads it and takes off entries that are not there any more, which is
/// the same nothing this failed to do.
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

/// What the machine's own record is called, beside the Conversations' in the
/// same directory.
///
/// **A name no Conversation can have**, which is what keeps it out of the
/// sweep: a candidate there is a file whose name reads as an id — see
/// [`left_behind`], and the test that says so — and this one does not. Which
/// is the whole of how a record that must never be swept sits next to the ones
/// that must.
const STANDING: &str = "standing";

/// Where that is.
fn standing_record(data_dir: &Path) -> PathBuf {
    directory(data_dir).join(STANDING)
}

/// The entries that stand for the installation rather than for one
/// Conversation — see [`super::written_down`], which is what decides between
/// them.
///
/// **Why they are written down at all, when nothing sweeps them.** Precisely
/// because nothing does: an entry no boundary takes off is one that would
/// otherwise outlive every record there is, and the account it names is
/// removable by a verb of Verkstead's own. So the machine keeps one list of
/// what it has standing, and [`super::super::account::machine::remove`] reads
/// it — the one moment anything takes a standing entry off, and the last
/// moment there is a SID to name it by.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Standing {
    /// What the account is called, for the log as the entries come off.
    pub(crate) account: String,

    /// And the identity every entry below names.
    pub(crate) sid: String,

    /// Those entries, in the order they were first written.
    pub(crate) entries: Vec<Entry>,
}

/// What stands for the installation, added to what already stood.
///
/// **Added rather than replaced**, which is the opposite of [`wrote`] and is
/// what the thing being remembered is: a Conversation describes its whole
/// surface every time, so its record is that whole list; the installation's is
/// built up a description at a time — a session's `PATH` names one directory,
/// the Compile Server's another, a Worktree on another drive brings a step
/// nothing had stepped through before — and each of them is an entry standing
/// on the machine from the moment it is written.
///
/// **Except where the identity has changed**, in which case what is there is
/// somebody else's and is replaced whole. Entries naming an account this
/// machine no longer has are not this record's to carry: they grant nobody
/// anything, and a list that mixed two identities would have
/// [`super::super::account::machine::remove`] taking off entries for a SID it
/// is not removing.
pub(crate) fn standing_wrote(
    data_dir: &Path,
    account: &str,
    sid: &str,
    entries: &[Entry],
) -> io::Result<()> {
    let directory = directory(data_dir);

    std::fs::create_dir_all(&directory)?;

    let mut standing = match standing_read(data_dir) {
        Some(standing) if standing.sid == sid => standing,
        _ => Standing {
            account: account.to_owned(),
            sid: sid.to_owned(),
            entries: Vec::new(),
        },
    };

    for entry in entries {
        if !standing.entries.contains(entry) {
            standing.entries.push(entry.clone());
        }
    }

    let written = Written {
        account: standing.account,
        sid: standing.sid,
        entries: standing.entries.iter().map(Line::of).collect(),

        // Nothing a refusal cut is ever in here: what stands is a grant or a
        // step, and a refusal is a Conversation's own by definition — it
        // covers the account's own skills from that Conversation's session.
        cut: Vec::new(),
    };

    let body = serde_json::to_string_pretty(&written).map_err(io::Error::other)?;

    crate::settings::write_atomically(
        &standing_record(data_dir),
        &body,
        crate::settings::ORDINARY_MODE,
    )
}

/// What this machine has standing, or nothing where it has never written one.
pub(crate) fn standing_read(data_dir: &Path) -> Option<Standing> {
    let path = standing_record(data_dir);
    let remembered = at(&path, 0)?;

    Some(Standing {
        account: remembered.account,
        sid: remembered.sid,
        entries: remembered.entries,
    })
}

/// And the record itself, gone — which is what removing the account does after
/// it has taken every entry in it off.
pub(crate) fn standing_forget(data_dir: &Path) {
    let path = standing_record(data_dir);

    if let Err(error) = std::fs::remove_file(&path)
        && error.kind() != io::ErrorKind::NotFound
    {
        tracing::warn!(
            error = ?error,
            path = %path.display(),
            "the record of what this installation had standing could not be taken away, so a \
             later removal will read a list of entries that have already come off",
        );
    }
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
        account: written.account,
        sid: written.sid,

        entries: written.entries.iter().map(Line::entry).collect(),
        cut: written.cut,
    })
}

/// A record as it is written down — see this module's own documentation for why
/// it is a shape of its own rather than the types it is made from.
#[derive(Debug, Serialize, Deserialize)]
struct Written {
    /// What the account these entries were written for is called.
    ///
    /// Read under its old spelling as well, which is what a record the
    /// AppContainer build wrote carries: `name` was the profile's name there,
    /// and it is a name for the identity either way. A record whose entries
    /// name a profile that has since been deleted is still a record whose
    /// entries are on the human's directories, and the sweep takes them off.
    #[serde(alias = "name")]
    account: String,

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
    /// [`super::super::entries::Entries::wrote`] — so what is remembered
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
            account: "vk-0123456789ab".to_owned(),
            sid: "S-1-5-21-1234567890-1234567890-1234567890-1001".to_owned(),

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
    fn what_was_written_for_a_conversation_is_read_back_whole() {
        let held = tempfile::tempdir().unwrap();
        let written = remembered(7);

        wrote(held.path(), &written).expect("a record to be writable under the Data Directory");

        assert_eq!(read(held.path(), 7).as_ref(), Some(&written));
        assert_eq!(left_behind(held.path()), vec![written]);
    }

    /// What stands for the installation is added to rather than replaced, is
    /// no candidate for the sweep, and goes when the account does.
    #[test]
    fn what_this_installation_has_standing_is_built_up_and_swept_by_nothing() {
        let held = tempfile::tempdir().unwrap();
        let sid = "S-1-5-21-1234567890-1234567890-1234567890-1001";

        let step = |path: &str| Entry {
            path: PathBuf::from(path),
            wanted: Wanted::Stepped,
        };

        standing_wrote(held.path(), "vk-0123456789ab", sid, &[step(r"C:\Users")]).unwrap();

        standing_wrote(
            held.path(),
            "vk-0123456789ab",
            sid,
            &[step(r"C:\Users"), step(r"D:\")],
        )
        .unwrap();

        assert_eq!(
            standing_read(held.path()).map(|standing| standing.entries),
            Some(vec![step(r"C:\Users"), step(r"D:\")]),
            "a second description adds what it names and says the rest again \
             for nothing: the installation's list is built up a boundary at a \
             time rather than rewritten by each",
        );

        assert_eq!(
            left_behind(held.path()),
            Vec::<Remembered>::new(),
            "and no sweep ever sees it — its name is not one a Conversation \
             could have, which is the whole of how it sits beside theirs",
        );

        standing_forget(held.path());

        assert_eq!(
            standing_read(held.path()),
            None,
            "and removing the account takes the record with the entries",
        );
    }

    /// A record written for one identity is not added to for another: an
    /// account made again is a new SID, and the entries the old one left are
    /// not this one's to carry or to take off.
    #[test]
    fn what_stood_for_another_identity_is_replaced_rather_than_joined() {
        let held = tempfile::tempdir().unwrap();

        let step = |path: &str| Entry {
            path: PathBuf::from(path),
            wanted: Wanted::Stepped,
        };

        standing_wrote(
            held.path(),
            "vk-old",
            "S-1-5-21-1-1-1-1001",
            &[step(r"C:\Users")],
        )
        .unwrap();
        standing_wrote(
            held.path(),
            "vk-new",
            "S-1-5-21-1-1-1-1002",
            &[step(r"D:\")],
        )
        .unwrap();

        let standing = standing_read(held.path()).expect("a record for the account that is there");

        assert_eq!(standing.sid, "S-1-5-21-1-1-1-1002");
        assert_eq!(standing.entries, vec![step(r"D:\")]);
    }

    /// And a record that has gone is nothing, which is what a sweep finds after
    /// a close.
    #[test]
    fn a_boundary_that_was_forgotten_is_no_longer_remembered() {
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

    /// And a record the AppContainer build wrote reads back whole, naming what
    /// it named: a profile's SID and a profile's name, which are an identity
    /// and a name for it like any other.
    ///
    /// **Because the entries in it are on the human's directories either way.**
    /// A machine upgraded from that build has records under this directory that
    /// no session will ever run behind again, and a build that could not read
    /// one would leave every entry in it standing for good.
    #[test]
    fn a_record_the_appcontainer_build_wrote_still_names_what_to_take_back() {
        let held = tempfile::tempdir().unwrap();

        std::fs::create_dir_all(directory(held.path())).unwrap();
        std::fs::write(
            directory(held.path()).join("7"),
            r#"{
              "name": "verkstead-0123456789abcdef-7",
              "sid": "S-1-15-2-1234567890-1234567890-1234567890-1234567890",
              "entries": [
                { "path": "C:\\state\\worktrees\\verkstead-rate-limiting", "wanted": "read-write" }
              ]
            }"#,
        )
        .unwrap();

        let read = read(held.path(), 7).expect("a record the build before this one wrote");

        assert_eq!(read.account, "verkstead-0123456789abcdef-7");
        assert_eq!(
            read.entries,
            vec![Entry {
                path: PathBuf::from(r"C:\state\worktrees\verkstead-rate-limiting"),
                wanted: Wanted::Granted(Reach::ReadWrite),
            }],
        );
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
