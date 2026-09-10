//! A Conversation's **entries**: every access-control entry written on a real
//! directory for one Conversation's sessions, held for as long as that
//! Conversation is working and taken off the machine when it stops.
//!
//! **The boundary on this platform is a set of entries and nothing else.** A
//! session runs as a local account of Verkstead's own — see [`super::account`],
//! which is the identity and is one account for the whole installation — so
//! what one Conversation's session may reach that the next one's may not is the
//! entries alone. There is no profile to create here and nothing per
//! Conversation to make: the identity is resolved rather than made, and this
//! module is what a Conversation *adds* to it and takes away again (ADR-0014,
//! *Amended: the Sandbox is an account*).
//!
//! **Which is what this module used to be, and is no longer named for.** It
//! held an AppContainer of each Conversation's own, and everything in it that
//! was really about containers has gone with them: the profile, the capability,
//! the pipe being told about a new identity, and the two questions only a
//! container could be asked — what a process is inside, and what its token
//! holds. What is left is what was never about containers at all, and it is the
//! whole of the boundary now.
//!
//! **What a session may reach that is not its own** is therefore every live
//! Conversation's Worktree, which is the cost this stage accepted: making an
//! account needs elevation, so one per Conversation would need elevation per
//! Conversation. The human's machine, their own profile, repositories outside
//! the Watched Paths and the account's own skills are all still refused, which
//! is the boundary the Sandbox exists for — and a Conversation's entries coming
//! off with its Worktree is what bounds the rest.
//!
//! **Its life is the Conversation's, rather than a session's** (ADR-0014, Q14).
//! A Conversation can have a session and a Conversation Terminal going at once
//! and both reach what the same entries say, and the session after those
//! reaches it too: so what [`Entries::of_conversation`] hands out is shared, and
//! what holds it is this module rather than whoever is running inside. Written
//! at the first session, and let go of at exactly two moments — the Conversation
//! being closed, which takes its Worktree in the same breath, and the sweep a
//! server starts with. Both arrive here as [`taken_back`].
//!
//! **Which is why what was written is also written down.** An entry is a fact
//! about the human's disk, so a server that died is one that left a boundary
//! standing that nothing in memory describes any more. Every Conversation's
//! entries therefore keep a record under the Data Directory — see
//! [`super::granting::remembering`] — written before anything is granted, and it
//! is that record the sweep takes a crashed-over Conversation's entries back by.
//! Entries that cannot be written down are refused: what cannot be taken away
//! afterwards should not be written now.

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex};

use super::granting::{Entry, remembering, writing};

/// Every Conversation's entries this process is holding, by the name they are
/// held under.
///
/// **A Conversation's boundary is one machine-wide thing and this is what keeps
/// it one.** Two things run behind a Conversation's entries — a grilling
/// session and the Conversation Terminal beside it — and each of them asks for
/// them as it starts. What is handed out is therefore shared, and this is where
/// the sharing is done.
///
/// **And the hold is a strong one, which is the Conversation's lifetime said in
/// code.** The entries outlive every session that runs behind them — the next
/// one is behind the same entries, and they are what makes the Worktree
/// reachable at all — so what lets go of them is not a session ending but
/// [`taken_back`], which is the close and the sweep. Held weakly, a
/// Conversation's boundary would be written and taken down around every session
/// it runs, which is a directory tree walked at each start for entries that were
/// already there.
static HELD: LazyLock<Mutex<HashMap<String, Arc<Entries>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// One Conversation's entries, held.
#[derive(Debug)]
pub struct Entries {
    /// What the account they are written for is called on this machine.
    ///
    /// Nothing here does anything with it: it is carried so that the record
    /// says which account its entries name, and so a server that did not
    /// resolve that account can still say which one it is taking entries back
    /// for. See [`super::account`], where the name is arithmetic over the Data
    /// Directory and the SID is what the machine answered when it was asked
    /// about the name.
    account: String,

    /// And the identity itself, as a SID is written down: what every entry
    /// below names, and what each of them is taken back for.
    sid: String,

    /// Every access-control entry written on a real directory for that
    /// identity, so that every one of them can be taken back.
    ///
    /// The whole list rather than the last session's: a Conversation's second
    /// session writes the entries its own description says, and what has to
    /// come off at the end is all of them.
    granted: Mutex<Vec<Entry>>,

    /// And which of the paths those entries refuse were taking entries from
    /// above when the first of them was written.
    ///
    /// Kept beside them for their own reason and read the same way: a refusal
    /// cuts the inheritance on the directory it refuses, so this is the one
    /// thing about that directory nothing can read off it once the boundary
    /// exists — see [`super::granting::writing::inheriting`], and
    /// [`super::granting::remembering`], which is where a later server reads it
    /// instead.
    cut: Mutex<Vec<PathBuf>>,

    /// And where these are written down: the Data Directory the record goes
    /// under, and whose record it is.
    ///
    /// The pair rather than the composed path, because both halves are wanted
    /// separately — the record is read and written by Conversation, and the
    /// sweep reads every one there is under the same directory.
    kept: Kept,
}

/// Where a Conversation's entries are written down.
#[derive(Debug, Clone)]
struct Kept {
    data_dir: PathBuf,
    conversation: i64,
}

impl Entries {
    /// The entries the sessions of `conversation` run behind, under the Data
    /// Directory at `data_dir` — begun at the first of them and shared with
    /// every one after it.
    ///
    /// `account` and `sid` are the local account this installation's sessions
    /// run as, resolved by the caller. There is one of it, so it is read rather
    /// than made, and it is read where the process is started rather than here:
    /// what starts a session needs the password too, and a second reading would
    /// be a second answer to *which account is this*.
    ///
    /// **It is written down before it is handed to anybody.** Entries nothing
    /// wrote down are entries no later server could take away — see this
    /// module's own documentation — so a record that will not write refuses
    /// them, and nothing has been granted yet to leave behind.
    ///
    /// The map is held for the whole of this, which is what makes two callers
    /// arriving at once one boundary rather than two.
    pub fn of_conversation(
        data_dir: &Path,
        conversation: i64,
        account: &str,
        sid: &str,
    ) -> io::Result<Arc<Entries>> {
        let name = held_under(data_dir, conversation);

        let mut holding = HELD.lock().unwrap_or_else(|held| held.into_inner());

        if let Some(entries) = holding.get(&name) {
            return Ok(entries.clone());
        }

        // With nothing granted yet, which is the whole point of the order:
        // what a record is for is a *later* server taking this away, and the
        // moment there is anything to take away is the moment after this one.
        remembering::wrote(
            data_dir,
            &remembering::Remembered {
                conversation,
                account: account.to_owned(),
                sid: sid.to_owned(),
                entries: Vec::new(),
                cut: Vec::new(),
            },
        )?;

        let entries = Arc::new(Entries {
            account: account.to_owned(),
            sid: sid.to_owned(),
            granted: Mutex::new(Vec::new()),
            cut: Mutex::new(Vec::new()),
            kept: Kept {
                data_dir: data_dir.to_owned(),
                conversation,
            },
        });

        holding.insert(name, entries.clone());

        Ok(entries)
    }

    /// The entries written for this identity, remembered so that they can be
    /// taken back when the Conversation stops — here and on the disk both.
    ///
    /// Added to rather than replaced: a Conversation's second session describes
    /// its own surface, and an entry written by the first is still an entry on
    /// somebody's directory.
    ///
    /// **Called before the entries are written on the machine**, and it can
    /// refuse. What this hands back a failure for is a record that would not
    /// write, which is a boundary no later server could take back: so the
    /// caller refuses the session, and nothing has been granted yet to leave
    /// behind. Remembering an entry the write below then failed on is the safe
    /// side of the same order — taking back an entry that is not there is
    /// nothing at all.
    ///
    /// `cut` is which of those entries' refused paths were taking entries from
    /// above when this description was read, which is remembered with them for
    /// the reason the field is.
    pub(crate) fn wrote(&self, entries: Vec<Entry>, cut: Vec<PathBuf>) -> io::Result<()> {
        {
            let mut granted = self.granted.lock().unwrap_or_else(|held| held.into_inner());

            for entry in entries {
                if !granted.contains(&entry) {
                    granted.push(entry);
                }
            }
        }

        {
            // Added and never taken away, which is what makes the first
            // session's answer the one that stands: by the second session the
            // directory has had its inheritance cut and reads as one that was
            // never inheriting at all — see
            // [`super::granting::writing::inheriting`], which is why this is
            // read before anything is written and why a later reading of it
            // says less than the first.
            let mut was = self.cut.lock().unwrap_or_else(|held| held.into_inner());

            for path in cut {
                if !was.contains(&path) {
                    was.push(path);
                }
            }
        }

        self.remember()
    }

    /// These entries as they are written down.
    fn remember(&self) -> io::Result<()> {
        remembering::wrote(
            &self.kept.data_dir,
            &remembering::Remembered {
                conversation: self.kept.conversation,
                account: self.account.clone(),
                sid: self.sid.clone(),
                entries: self
                    .granted
                    .lock()
                    .unwrap_or_else(|held| held.into_inner())
                    .clone(),
                cut: self
                    .cut
                    .lock()
                    .unwrap_or_else(|held| held.into_inner())
                    .clone(),
            },
        )
    }

    /// The identity every one of these entries names, which is the account a
    /// session is started as.
    pub fn sid(&self) -> &str {
        &self.sid
    }
}

impl Drop for Entries {
    /// The entries come off the human's directories, and the record of them
    /// goes after they have.
    ///
    /// **What runs this is [`taken_back`]**, ordinarily: a Conversation's
    /// entries are held by this module for as long as the Conversation is
    /// working, so the last hold to go is the close or the sweep letting go of
    /// them. Where a session is still running at that moment the hold is the
    /// session's for a little longer, and this runs when its relay lets go —
    /// which is the right order rather than a race: the boundary a running
    /// session is behind is not one to take down around it.
    fn drop(&mut self) {
        let granted =
            std::mem::take(&mut *self.granted.lock().unwrap_or_else(|held| held.into_inner()));
        let cut = std::mem::take(&mut *self.cut.lock().unwrap_or_else(|held| held.into_inner()));

        let name = held_under(&self.kept.data_dir, self.kept.conversation);
        let holding = HELD.lock().unwrap_or_else(|held| held.into_inner());

        // A live hold under this name is a boundary begun *after* this one, in
        // the moment between this one's hold being taken out of the map and
        // this running — see [`Entries::of_conversation`], which begins afresh
        // under a name nothing is holding. The entries this wrote are the same
        // identity's, so they are handed to it rather than taken back from
        // under it.
        if let Some(taken) = holding.get(&name).cloned() {
            // The record is that one's too, and rewriting it is how the entries
            // this hands over reach the disk. Nothing is done about a refusal:
            // what this is holding has already been handed on, and whoever now
            // holds it is somebody a caller could refuse for.
            if let Err(error) = taken.wrote(granted, cut) {
                tracing::warn!(
                    name,
                    error = ?error,
                    "the entries of a Conversation that has ended were handed to the boundary \
                     that took its name and could not be written down with it"
                );
            }

            return;
        }

        // All of it while the map is held, so that a caller arriving in the
        // meantime waits and then begins a boundary of its own rather than
        // finding this one part-way out. What that costs is a session start
        // held up by another Conversation's ending, for as long as it takes to
        // walk back the trees this was granted.
        given_back(
            &self.sid,
            &granted,
            &cut,
            &self.kept.data_dir,
            self.kept.conversation,
        );
    }
}

/// One Conversation's boundary taken off the machine: the entries off every
/// directory they were written on, and the record of them removed.
///
/// The one place either happens, because the order is the whole of it and there
/// are two ways in: entries this process is holding, which arrive through
/// [`Entries::drop`], and entries a server that has gone left behind, which
/// arrive off their record through [`taken_back`]. Both are the same two things
/// in the same order.
///
/// `cut` is which of the paths `granted` refuses were inheriting before any of
/// it was written — see [`super::granting::writing::inheriting`], which is what
/// a refusal cannot work out for itself once it has been made.
fn given_back(sid: &str, granted: &[Entry], cut: &[PathBuf], data_dir: &Path, conversation: i64) {
    writing::strip(granted, cut, sid);

    // And the record after them, for the reason the order inside `strip` is: a
    // record is what the next sweep would take this back by, so it goes once
    // there is nothing left to take back.
    remembering::forget(data_dir, conversation);
}

/// The entries of `conversation`, let go of: off the human's directories, and
/// their record with them.
///
/// **The two ways a Conversation's boundary ends**, and both of them come
/// through here: the close, which takes the entries in the same breath as the
/// Worktree, and the sweep a server starts with — see [`crate::boundaries`],
/// which is where both are decided.
///
/// **Entries this process is holding are let go of, and ones it is not are
/// taken back off their record.** The second is what a crash leaves: the
/// entries are on the machine and nothing in memory knows about them, so what
/// says which directories carry one is what the server that wrote them wrote
/// down — see [`super::granting::remembering`].
///
/// **Nothing is refused and nothing comes back.** What could be done about an
/// entry that will not come off is what the next sweep will do about it anyway,
/// and a close that failed for it would be a Conversation nothing can ever end.
pub fn taken_back(data_dir: &Path, conversation: i64) {
    let name = held_under(data_dir, conversation);

    // Taken out of the map under the lock and let go of outside it: the drop
    // below takes that same lock, to make sure of the name it is ending.
    let held = {
        let mut holding = HELD.lock().unwrap_or_else(|hold| hold.into_inner());

        holding.remove(&name)
    };

    if let Some(entries) = held {
        // Which is the whole of it where nothing else is behind them: the last
        // hold going is [`Entries::drop`], and that is where a boundary ends. A
        // session still running holds one too, and then this is the *second*
        // last hold and the ending is that session's — see the drop.
        drop(entries);

        return;
    }

    // Nothing held under that name, so what there is to go on is what the
    // server that wrote them wrote down. Nothing at all where there is no
    // record: a Conversation whose sessions never ran on this platform has no
    // entry anywhere naming anybody.
    let Some(remembered) = remembering::read(data_dir, conversation) else {
        return;
    };

    tracing::info!(
        account = remembered.account,
        conversation,
        entries = remembered.entries.len(),
        "the entries a server that has gone wrote for a Conversation are being taken off the \
         directories they were written on"
    );

    given_back(
        &remembered.sid,
        &remembered.entries,
        &remembered.cut,
        data_dir,
        conversation,
    );
}

/// Let go of the hold on a Conversation's entries without taking anything
/// back — which is what a server that died did, and what a suite makes a crash
/// out of.
///
/// **The one thing here that leaves a boundary standing on purpose.** Everything
/// else takes the entries off the human's directories; this leaves them exactly
/// where they are, with the record still under the Data Directory, which is the
/// state the next server's sweep has to be able to clear. Nothing in the server
/// calls it — see `crates/server/tests/sandbox_windows.rs`, which is the whole
/// of why it is here.
pub fn forgotten(data_dir: &Path, conversation: i64) {
    let name = held_under(data_dir, conversation);

    let held = {
        let mut holding = HELD.lock().unwrap_or_else(|hold| hold.into_inner());

        holding.remove(&name)
    };

    // Leaked rather than dropped, because dropping is precisely what a process
    // that died did not do: what is being made here is a machine carrying
    // entries with nothing holding them.
    if let Some(entries) = held {
        std::mem::forget(entries);
    }
}

/// What a Conversation's entries are held under in this process.
///
/// Both halves because both are needed: the Data Directory, so that two
/// Verksteads on one machine are two sets of holds, and the Conversation,
/// because a hold is one Conversation's. Through the Data Directory's
/// fingerprint rather than its path — see [`crate::pipe::bare`] — so that a
/// server pointed at one directory by two spellings holds one boundary per
/// Conversation rather than two.
fn held_under(data_dir: &Path, conversation: i64) -> String {
    format!("{}-{conversation}", crate::pipe::bare(data_dir))
}
