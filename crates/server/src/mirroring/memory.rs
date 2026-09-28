//! The **memory sync**: what the home account remembers of this Repo carried to
//! the device a session is running on, and what the session wrote carried back
//! (ADR-0020, *Shared Profiles*).
//!
//! **The memory switch holds away from home.** A Profile whose switch is on is
//! given the account's memory store at home; away from home there is no account
//! to join, so what stands in its place is this: before launch, the store's
//! entries for *this* Repo and this Worktree come over the link into the account
//! mirror the root is built from, and as the session ends what is in them goes
//! back. That is what makes a session run on B readable on A afterwards, and what
//! a harness's own resume will later stand on. **Switched off, nothing syncs** —
//! the session starts on an empty store exactly as it would at home, and neither
//! direction carries anything.
//!
//! **A label apiece, and each machine names its own path.** A memory store is
//! keyed by the path a session ran in wherever it is keyed by anything, and every
//! one of those paths is a different string on every machine. So nothing here
//! sends a path for the other end to write at: what crosses the link is the part
//! of the store — the Repo's entry, the Worktree's, the sessions, the memory
//! files, the data directory — and the files under it, and each device joins that
//! word onto a directory of its own. Which is the whole of the path rewrite, done
//! twice rather than sent: the name is computed by the harness's own encoding on
//! both ends, and a name computed any other way is a second entry rather than the
//! same memory. See [`crate::sandbox::root::Root::synced`], which is the list of
//! parts per harness, and [`crate::peer::memory`], which is the answering half.
//!
//! **Which path each part is named off is two different answers.** The Repo's
//! goes through the match across devices — which of the home device's Repos is
//! this repository, asked of git on both ends (see [`crate::matching`]) — and
//! **no match is an answer**: there is nothing of that Repo to pull, the session
//! starts without it, and the Timeline says so rather than the launch being
//! refused. The Worktree's is named off the Data Directory instead, because a
//! Worktree lives under one rather than under the Repo: the home device joins
//! **its own** worktrees directory onto the same stem this Worktree was named
//! with, which is the path it would have used for this work and is what leaves
//! the transcript findable when the work comes home.
//!
//! **Only what changed travels back**, which is the account mirror's own rule for
//! the login: what came down is remembered by a SHA-256 apiece, and a file still
//! hashing to what arrived is not sent. So a session that read the Repo's memory
//! and wrote none of it writes nothing home, and the rest of the account's store
//! is left exactly as it was. Nothing is deleted at home either: a file the
//! session took away is not a file it asked the account to lose.
//!
//! **Best effort rather than a gate.** A home that will not answer the memory is
//! a session that starts without it and says so on the Timeline — unlike the
//! account, which is refused, because a session with no login comes up logged out
//! and a session with no memory comes up new. The same at the end: what could not
//! be written home is a sentence rather than a retry.

use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, Read};
use std::path::{Path, PathBuf};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use verkstead_render::{MemoryFile, MemoryLeft, MemoryWanted};

use crate::device::Devices;
use crate::peer::memory::{LEFT, MEMORY};
use crate::relaying::{self, Call, Refusal, Streamed, as_json};
use crate::sandbox::root::{self, Part, Root, Whose};
use crate::store;

/// The most a memory store may be before it is left where it is: **sixty-four
/// megabytes**.
///
/// Generous against what the unit actually is — one Repo's memory, one
/// Conversation's transcripts, an account's memory files — and small against the
/// thing a bound is really in front of, which is the machine on the far end that
/// answers and then writes without stopping. The one store carried whole is
/// OpenCode's data directory, and a human whose has grown past this is one whose
/// sessions away from home start on an empty one with a line saying why. The
/// mirrors' own bound, for its reason — see [`super::MOST_THE_PROFILES_ARE`].
pub(crate) const MOST_A_STORE_IS: usize = 64 * 1024 * 1024;

/// How much of a rollout is read to say whose session wrote it.
///
/// Generous against the eighteen kilobytes codex 0.149.0 opens one with — the
/// session's whole system prompt is in there — and small against a log that runs
/// to megabytes. [`crate::transcript`]'s own bound, for its reason.
const FIRST_LINE: u64 = 256 * 1024;

/// What a file that came down is remembered by, so that the ending can tell what
/// the session changed: the SHA-256 of it as it travelled.
///
/// The account mirror's reading of the login, over a store's worth of files —
/// see [`super::account`]. A hash rather than the bytes, because a Profile lent
/// out should not cost its memory held in this process for the hours a session
/// runs; and a hash rather than a length or a modification time, because those
/// can miss a change.
type Fingerprint = [u8; 32];

/// A memory store synced here for a session away from home, and what its ending
/// has to carry back: which machine the account is at home on, where the mirror
/// is, which parts of the store this Conversation's sync is, what the home device
/// has to be asked again to put them back, and what came down in each file.
#[derive(Debug, Clone)]
pub struct Syncing {
    pool: SqlitePool,
    devices: Option<Devices>,

    /// The device the account is at home on, and the Profile's id there — which
    /// is what the write-back is addressed by, the way every press over a mirror
    /// is.
    at: store::Mirror,

    /// The mirror's own account directory on this device, which every part is
    /// said from.
    account: PathBuf,

    /// The parts as **this** machine names them.
    parts: Vec<Part>,

    /// And what the home device is asked to name its own by, which is the same
    /// question the pull was made with: a write-back that asked differently would
    /// be writing into a different part of the store.
    wanted: MemoryWanted,

    /// What came down in each file of them, by part and by where it sits under
    /// it — see [`Fingerprint`].
    came_down: HashMap<(String, String), Fingerprint>,
}

/// What the home account remembers of this Repo, pulled into the mirror a session
/// away from home is about to be launched out of.
///
/// `profile` is the Profile as the launch reads it — the mirror row with this
/// device's own mirror named as its account, which is what
/// [`super::account::fetched`] answers — and `at` is where that account is at
/// home. `conversation` is what the two paths are named off: its Repo, which the
/// match across devices settles, and its Worktree, whose stem the home device
/// joins onto its own worktrees directory.
///
/// `None` is *nothing was pulled and nothing is to be written back*: a Profile
/// whose memory switch is off, a Conversation with no Worktree to run in, or a
/// home that would not answer — which is said on the Timeline rather than
/// refused. **The session starts either way.** A session with no memory is a
/// session that starts new, where a session with no login is one that comes up
/// logged out with nothing saying why; only the second is worth refusing a launch
/// over.
pub async fn pulled(
    pool: &SqlitePool,
    devices: Option<&Devices>,
    conversation: &store::Conversation,
    at: &store::Mirror,
    profile: &store::Profile,
) -> Option<Syncing> {
    // Off, and nothing of the account's store is made, joined or carried — which
    // is [`Root::synced`]'s own reading of the same switch, asked here so that
    // nothing is fetched to be thrown away.
    if !profile.memory {
        return None;
    }

    let worktree = conversation.worktree.clone()?;
    let stem = worktree.file_name()?.to_string_lossy().into_owned();

    let sessions = match store::session_ids(pool, conversation.id).await {
        Ok(sessions) => sessions,

        Err(why) => {
            tracing::error!(
                error = ?why,
                conversation_id = conversation.id,
                "the names of this Conversation's sessions could not be read, so its memory was \
                 not synced",
            );

            return None;
        }
    };

    // Which of the home device's Repos is this repository, which is what the
    // Repo's half of the store is named off there. Asked of git on both ends,
    // and **no match is an answer**: the Worktree's half still travels, and the
    // Timeline says the Repo's did not.
    let matched = match devices {
        Some(devices) => crate::matching::across(devices, &at.device, &conversation.repo).await,

        // A Verkstead linked to nothing has no mirror to launch under either, so
        // this is a shape rather than a case.
        None => Err(Refusal::ours(
            "this server holds no device identity to relay through".to_owned(),
        )),
    };

    let repo = match matched {
        Ok(matched) => {
            if matched.is_none() {
                noted(
                    pool,
                    conversation.id,
                    "This Profile's account is on another device of the cluster, and that device \
                     has no Repo that is this repository — so this session started without what \
                     that account remembers of it. Open the repository there, and the next \
                     session will have it.",
                )
                .await;
            }

            matched.map(|there| there.id)
        }

        Err(why) => {
            noted(
                pool,
                conversation.id,
                &format!(
                    "Verkstead could not settle which of the other device's Repos is this \
                     repository — {}. This session started without what that account remembers \
                     of it.",
                    why.saying,
                ),
            )
            .await;

            None
        }
    };

    let wanted = MemoryWanted {
        repo,
        worktree: Some(stem),
        sessions,
    };

    // The parts as this machine names them, which is the root a session here is
    // about to be built out of asked for its store. Blocking: one question to
    // git, and two platforms resolve a path to name an entry.
    let account = profile.account.clone();
    let parts = tokio::task::spawn_blocking({
        let wanted = wanted.clone();

        move || here(&account, &worktree, &wanted)
    })
    .await;

    let Ok(Some((account, parts))) = parts else {
        tracing::error!(
            conversation_id = conversation.id,
            "the parts of this Profile's memory store could not be named on this device, so \
             nothing was synced",
        );

        return None;
    };

    let call = match asking(at.id, MEMORY, &wanted) {
        Some(call) => call,
        None => return None,
    };

    let said: Vec<MemoryFile> =
        match relaying::word_from(devices, &at.device, call, MOST_A_STORE_IS).await {
            Ok(said) => said,

            Err(why) => {
                noted(
                    pool,
                    conversation.id,
                    &format!(
                        "Verkstead did not fetch what this Profile's account remembers of this \
                         repository — {}. This session started on an empty memory store.",
                        why.saying,
                    ),
                )
                .await;

                return None;
            }
        };

    let written = tokio::task::spawn_blocking({
        let account = account.clone();
        let parts = parts.clone();

        move || written_down(&account, &parts, &said)
    })
    .await;

    let came_down = match written {
        Ok(Ok(came_down)) => came_down,

        Ok(Err(why)) => {
            noted(
                pool,
                conversation.id,
                &format!(
                    "Verkstead could not write what this Profile's account remembers of this \
                     repository into its mirror here — {why:#}. This session started on an empty \
                     memory store.",
                ),
            )
            .await;

            return None;
        }

        Err(why) => {
            tracing::error!(
                error = ?why,
                conversation_id = conversation.id,
                "writing a member's memory store into its mirror ended badly",
            );

            return None;
        }
    };

    Some(Syncing {
        pool: pool.clone(),
        devices: devices.cloned(),
        at: at.clone(),
        account,
        parts,
        wanted,
        came_down,
    })
}

impl Syncing {
    /// What the session wrote, put back into the account on the device it belongs
    /// to.
    ///
    /// **Called once the process has gone and the ending has seen to the
    /// profile**, which is where a login's write-back is called and for the same
    /// reason: what is read here is the store as the session finished with it.
    /// Before the login's, so that the login has the last word over the one file
    /// both of them can name — an OpenCode account keeps its login inside the data
    /// directory this carries whole.
    ///
    /// **Only what changed travels.** Every file is held against what came down in
    /// it, and one that still hashes the same is not sent: a session that read the
    /// Repo's memory and wrote none of it writes nothing home. A file the session
    /// took away is not sent either, and nothing is deleted at home — the rest of
    /// that account's store is somebody else's work, and this is not the place it
    /// would be taken from.
    ///
    /// **A home that has gone away by then is said rather than swallowed**, as the
    /// login's is: the mirror is left where it is, nothing retries, and the
    /// Conversation's Timeline carries a Notice naming the machine. What is lost is
    /// a transcript that stayed on this device, which is worth a line.
    pub async fn written_home(self, conversation_id: i64) {
        let Syncing {
            pool,
            devices,
            at,
            account,
            parts,
            wanted,
            came_down,
        } = self;

        let left = tokio::task::spawn_blocking({
            let account = account.clone();

            move || left_behind(&account, &parts, &came_down)
        })
        .await;

        let files = match left {
            Ok(Ok(files)) if files.is_empty() => return,
            Ok(Ok(files)) => files,

            Ok(Err(why)) => {
                tracing::error!(
                    conversation_id,
                    "what a session away from home left in its memory store could not be read, \
                     so the account it belongs to was not written: {why:#}",
                );

                return;
            }

            Err(why) => {
                tracing::error!(
                    error = ?why,
                    conversation_id,
                    "reading what a session away from home left in its memory store ended badly",
                );

                return;
            }
        };

        let carrying = files.len();

        let body = match serde_json::to_vec(&MemoryLeft { wanted, files }) {
            Ok(body) => body,

            Err(why) => {
                tracing::error!(
                    error = ?why,
                    conversation_id,
                    "what a session away from home left in its memory store could not be composed \
                     for the device its account is on",
                );

                return;
            }
        };

        let call = Call {
            method: reqwest::Method::POST,
            onwards: LEFT.replace("{profile}", &at.id.to_string()),
            headers: as_json(),
            body: Streamed::saying(body),
        };

        match relaying::put_to(devices.as_ref(), &at.device, call).await {
            Ok(_) => tracing::debug!(
                conversation_id,
                device = %at.device,
                carrying,
                "what this session wrote to its memory store was written into the account on the \
                 device it is at home on",
            ),

            Err(why) => {
                tracing::warn!(
                    conversation_id,
                    device = %at.device,
                    "what this session wrote to its memory store could not be written back: {}",
                    why.saying,
                );

                noted(
                    &pool,
                    conversation_id,
                    &format!(
                        "Verkstead did not write this session's memory and transcript back to \
                         the device this Profile's account is on — {}. They are still on this \
                         device, and the next session there will not find them.",
                        why.saying,
                    ),
                )
                .await;
            }
        }
    }
}

/// The parts of the store on **this** machine, and the account directory they are
/// said from — the mirror's own, or the account's where this is the home device.
///
/// `account` is the Profile's account as this device holds it, `worktree` the
/// directory the session runs in, and `wanted` the question both ends name their
/// parts by.
///
/// `None` is a Claude account whose Worktree has no common git directory, which
/// is a Worktree there is no session to run in either.
///
/// Blocking: one question to git, and two platforms resolve a path to name an
/// entry.
pub(crate) fn here(
    account: &store::Account,
    worktree: &Path,
    wanted: &MemoryWanted,
) -> Option<(PathBuf, Vec<Part>)> {
    let root = match account {
        store::Account::Claude { claude_dir, .. } => Root::claude(
            crate::platform::Platform::HERE,
            claude_dir,
            &crate::worktrees::common_git_dir(worktree)?,
            worktree,
        ),

        store::Account::Codex { home } => Root::codex(home),
        store::Account::Grok { home } => Root::grok(home),
        store::Account::OpenCode { home } => Root::opencode(home),
    };

    // What the question settled, which on this end is what a path could be named
    // off: the Repo's entry is named off the match across devices, and the
    // Worktree's off the directory this session is about to run in.
    let parts = carried(
        root.synced(wanted.worktree.as_deref(), &wanted.sessions),
        wanted.repo.is_some(),
        wanted.worktree.is_some(),
    );

    Some((root.account().to_owned(), parts))
}

/// The parts there is a path for on this device, out of the ones this harness
/// has.
///
/// A part there is no path for here is one nothing of travels in either
/// direction: the Repo's entry where no Repo of this device's is this
/// repository, and the Worktree's where nothing said what this Worktree is
/// called.
///
/// **What is asked is whether a path was found, rather than whether one was
/// wanted** — and on the device that answers the two are not the same question.
/// A question naming a Repo that has been unregistered since the match, or a
/// Worktree stem that does not read as a bare name, leaves that end with nothing
/// to name the entry off; and an entry named off an empty path is not a narrower
/// entry but a **wider** one — `projects/` rather than one directory under it,
/// which is every repository the human has ever run the harness in. So what
/// filters is the finding, and a part nothing could be named for is left out.
///
/// **Left out on one end is enough.** The other end keeps its own copy of that
/// part, and what it sends under the label is dropped by [`written_down`], which
/// takes only the labels this device holds a path for — so a part left out here
/// neither answers files nor takes any.
pub(crate) fn carried(parts: Vec<Part>, repo: bool, worktree: bool) -> Vec<Part> {
    parts
        .into_iter()
        .filter(|part| match part.label {
            root::REPO => repo,
            root::WORKTREE => worktree,
            _ => true,
        })
        .collect()
}

/// The call that asks a member's device about the store of the Profile it holds
/// under `profile`, with the question in its body.
///
/// **A press rather than a path**, because the question is three facts about this
/// device's side — which of that device's Repos this repository is, what this
/// Worktree is called, and every session id this Conversation has had — and none
/// of them is a thing to spell into a path.
fn asking(profile: i64, onwards: &str, wanted: &MemoryWanted) -> Option<Call> {
    let body = match serde_json::to_vec(wanted) {
        Ok(body) => body,

        Err(why) => {
            tracing::error!(
                error = ?why,
                "the question about a member's memory store could not be composed",
            );

            return None;
        }
    };

    Some(Call {
        method: reqwest::Method::POST,
        onwards: onwards.replace("{profile}", &profile.to_string()),
        headers: as_json(),
        body: Streamed::saying(body),
    })
}

/// Every file of `parts` under the account at `account`, as they travel.
///
/// **Which files is the part's own rule** — see [`Whose`]. A part that is not
/// there at all is no files rather than a failure, which is the ordinary case: a
/// Repo nobody has run this harness in, an account that has never written a
/// memory.
///
/// Refused whole where the store is larger than [`MOST_A_STORE_IS`], rather than
/// carried in part: half a store is a database that will not open and a
/// transcript that stops in the middle, and neither is better than starting new.
///
/// Blocking.
pub(crate) fn gathered(account: &Path, parts: &[Part]) -> io::Result<Vec<MemoryFile>> {
    let mut files = Vec::new();
    let mut held = 0usize;

    for part in parts {
        let under = account.join(&part.inside);

        match &part.whose {
            Whose::Everything => walked(&under, &under, part.label, None, &mut files, &mut held)?,

            // Directly under the store, and one level down: grok keeps a
            // directory per working directory and the session's inside it, and
            // [`crate::transcript`] looks in both for the same reason — which
            // directory a session's is under is that harness's own arithmetic
            // over a path, and reproducing it is what walking the store avoids.
            Whose::Called(ids) => {
                for child in directories_in(&under)? {
                    if called(&child, ids) {
                        walked(&under, &child, part.label, None, &mut files, &mut held)?;
                        continue;
                    }

                    for inside in directories_in(&child)? {
                        if called(&inside, ids) {
                            walked(&under, &inside, part.label, None, &mut files, &mut held)?;
                        }
                    }
                }
            }

            Whose::Naming(stem) => walked(
                &under,
                &under,
                part.label,
                Some(stem),
                &mut files,
                &mut held,
            )?,
        }
    }

    Ok(files)
}

/// And what the session left that the account at home has not got: [`gathered`]
/// with every file that still hashes to what came down in it taken out.
///
/// The account mirror's reading of the login over a store — see
/// [`super::account`]. A file that is no longer what arrived is one the session
/// wrote, and one that was never there at all is one it made; both go back. A
/// file the session took away is not here to send, and nothing is deleted at
/// home: a session that removed what it was given is not a session asking the
/// account to lose it.
///
/// Blocking.
pub(crate) fn left_behind(
    account: &Path,
    parts: &[Part],
    came_down: &HashMap<(String, String), Fingerprint>,
) -> io::Result<Vec<MemoryFile>> {
    let mut left = gathered(account, parts)?;

    left.retain(|file| {
        let key = (file.part.clone(), file.inside.clone());

        came_down.get(&key) != Some(&fingerprint(file.bytes.as_bytes()))
    });

    Ok(left)
}

/// And `files` written into the parts of the store under the account at
/// `account`, with what was written remembered by a fingerprint apiece.
///
/// **The parts are this device's own**, rather than whatever arrived: a file
/// under a label this device does not hold a part for is dropped with a line in
/// the log, and so is one whose path inside would land anywhere but under that
/// part. Which is the account mirror's rule for the same reason — nothing here
/// joins a name the far end chose onto a directory of this device's without
/// saying which names it will take.
///
/// **Written beside and renamed over**, as a mirror's own files are: a session
/// running beside this one keeps the file it was given, and what is at a path is
/// either the whole of the new file or the whole of the old one.
///
/// Blocking.
pub(crate) fn written_down(
    account: &Path,
    parts: &[Part],
    files: &[MemoryFile],
) -> io::Result<HashMap<(String, String), Fingerprint>> {
    let mut came_down = HashMap::new();

    for file in files {
        let Some(part) = parts.iter().find(|part| part.label == file.part) else {
            tracing::warn!(
                part = %file.part,
                "a device answered with a part of a memory store this one holds no path for, so \
                 it was dropped",
            );

            continue;
        };

        let Some(inside) = super::account::named(&file.inside) else {
            tracing::warn!(
                inside = %file.inside,
                "a device answered with a path that is not a file of a memory store, so it was \
                 dropped",
            );

            continue;
        };

        let Ok(bytes) = STANDARD.decode(&file.bytes) else {
            tracing::warn!(
                inside = %file.inside,
                "a file of a memory store did not arrive as bytes, so it was dropped",
            );

            continue;
        };

        let path = account.join(&part.inside).join(&inside);

        if let Some(over) = path.parent() {
            std::fs::create_dir_all(over)?;
        }

        beside(&path, &bytes)?;

        came_down.insert(
            (part.label.to_owned(), plainly(&inside)),
            fingerprint(file.bytes.as_bytes()),
        );
    }

    Ok(came_down)
}

/// Every file under `at`, said from `root`, kept where the part's rule keeps it.
///
/// `naming` is [`Whose::Naming`]'s stem where that is the rule, and nothing where
/// every file is carried.
///
/// Blocking, and recursive: a memory store is a shallow tree of small files, and
/// the one that is not — an OpenCode data directory — is bounded by what the
/// whole of it comes to rather than by how deep it goes.
fn walked(
    root: &Path,
    at: &Path,
    label: &'static str,
    naming: Option<&str>,
    files: &mut Vec<MemoryFile>,
    held: &mut usize,
) -> io::Result<()> {
    let entries = match std::fs::read_dir(at) {
        Ok(entries) => entries,
        Err(why) if why.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(why) => return Err(why),
    };

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let kind = entry.file_type()?;

        if kind.is_dir() {
            walked(root, &path, label, naming, files, held)?;
            continue;
        }

        // A symbolic link is followed by nothing here: what it points at is
        // outside the part wherever it matters, and a store is a tree of files
        // the harness wrote.
        if !kind.is_file() {
            continue;
        }

        if naming.is_some_and(|stem| !wrote_in(&path, stem)) {
            continue;
        }

        let Ok(inside) = path.strip_prefix(root) else {
            continue;
        };

        let bytes = std::fs::read(&path)?;

        *held += bytes.len();

        if *held > MOST_A_STORE_IS {
            return Err(io::Error::other(format!(
                "the memory store under {} is larger than the {MOST_A_STORE_IS} bytes one may be \
                 to cross a link",
                root.display(),
            )));
        }

        files.push(MemoryFile {
            part: label.to_owned(),
            inside: plainly(inside),
            bytes: STANDARD.encode(&bytes),
        });
    }

    Ok(())
}

/// Whether `log` is a rollout whose session was working in a Worktree called
/// `stem`.
///
/// **The name rather than the path**, which is what makes this the same reading
/// on both machines: a Worktree is at a different path on every device of a
/// cluster and is called the same thing on all of them, being named for the Repo
/// and the branch. So a rollout this Conversation's session wrote here is this
/// Conversation's over there too, whichever machine wrote it.
///
/// One line rather than the file, which is [`crate::transcript`]'s reading of the
/// same thing: a rollout opens with the session's own metadata, and in it is the
/// directory codex was launched in. Anything that is not a rollout at all is not
/// this Conversation's — codex compresses its older logs and keeps an index
/// beside them, and neither is something to carry a piece of.
///
/// Blocking.
fn wrote_in(log: &Path, stem: &str) -> bool {
    let is_rollout = log
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with("rollout-") && name.ends_with(".jsonl"));

    if !is_rollout {
        return false;
    }

    let Ok(file) = std::fs::File::open(log) else {
        return false;
    };

    let mut first = Vec::new();

    if BufReader::new(file.take(FIRST_LINE))
        .read_until(b'\n', &mut first)
        .is_err()
    {
        return false;
    }

    verkstead_render::rollout_cwd(&String::from_utf8_lossy(&first))
        .is_some_and(|cwd| Path::new(&cwd).file_name().is_some_and(|name| name == stem))
}

/// The directories directly under `under`, and nothing else that is there.
///
/// Nothing at all where `under` is not there, which is a store this harness has
/// never written.
///
/// Blocking.
fn directories_in(under: &Path) -> io::Result<Vec<PathBuf>> {
    let entries = match std::fs::read_dir(under) {
        Ok(entries) => entries,
        Err(why) if why.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(why) => return Err(why),
    };

    let mut directories = Vec::new();

    for entry in entries {
        let entry = entry?;

        if entry.file_type()?.is_dir() {
            directories.push(entry.path());
        }
    }

    Ok(directories)
}

/// Whether the directory at `path` is called one of `ids`.
fn called(path: &Path, ids: &[String]) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| ids.iter().any(|id| id == name))
}

/// A path inside a part as it travels: forward slashes, whichever machine
/// composed it.
///
/// The paths under a part are the harness's own relative names, and the device
/// that writes them down joins them onto a directory of its own — so a Windows
/// store's would otherwise arrive spelled in a way a Unix device could not take
/// apart. The account mirror's rule for the same thing — see
/// [`super::account::named`], which is what reads one back.
fn plainly(inside: &Path) -> String {
    inside.to_string_lossy().replace('\\', "/")
}

/// One file written beside its own name and renamed over it, so that what is at
/// the path is either the whole of the new file or the whole of the old one.
///
/// [`super::account`]'s own write, without the mode: what is written here is a
/// transcript and a memory rather than a login, and the directory it lands in is
/// already the account's own or the mirror's own.
///
/// The name it is written beside under is [`super::aside`]'s, which is where the
/// reason two of these must never share one is — and this is the half of a
/// mirror where the window is widest, a store being a file per entry rather than
/// one login.
///
/// Blocking.
fn beside(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let temporary = super::aside(path);

    let written =
        std::fs::write(&temporary, bytes).and_then(|()| std::fs::rename(&temporary, path));

    if written.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }

    written
}

/// The SHA-256 of `bytes` — see [`Fingerprint`].
fn fingerprint(bytes: &[u8]) -> Fingerprint {
    Sha256::digest(bytes).into()
}

/// One sentence on the Conversation's Timeline about what the sync did or did
/// not carry.
///
/// A Notice rather than a refusal, every time: the session starts, and what the
/// human can act on — a repository to open on the other machine, a machine to
/// wake up — is a line rather than a stop.
async fn noted(pool: &SqlitePool, conversation_id: i64, line: &str) {
    if let Err(error) = store::note(pool, conversation_id, line).await {
        tracing::error!(
            error = ?error,
            conversation_id,
            "saying on the Timeline what a memory sync carried failed",
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A store to ask about: the account's own directory, with whatever the
    /// harness keeps in it written down.
    fn store(files: &[(&str, &[u8])]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();

        for (inside, bytes) in files {
            let path = dir.path().join(inside);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, bytes).unwrap();
        }

        dir
    }

    /// What `parts` carries out of the store at `account`, as paths under their
    /// part and in order.
    fn carrying(account: &Path, parts: &[Part]) -> Vec<String> {
        let mut said: Vec<String> = gathered(account, parts)
            .unwrap()
            .into_iter()
            .map(|file| format!("{}/{}", file.part, file.inside))
            .collect();

        said.sort();
        said
    }

    /// The question a Conversation's sync is made with.
    fn wanted(worktree: &str, sessions: &[&str]) -> MemoryWanted {
        MemoryWanted {
            repo: Some(1),
            worktree: Some(worktree.to_owned()),
            sessions: sessions.iter().map(|id| (*id).to_owned()).collect(),
        }
    }

    /// A rollout as codex opens one: the session's own metadata, with the
    /// directory it was launched in.
    fn rollout(cwd: &str) -> Vec<u8> {
        format!(
            "{{\"type\":\"session_meta\",\"payload\":{{\"cwd\":\"{cwd}\"}}}}\n\
             {{\"type\":\"response_item\"}}\n",
        )
        .into_bytes()
    }

    /// Codex keeps one flat directory of rollouts for every directory it has ever
    /// run in, so what travels is the ones this Conversation's sessions wrote —
    /// named by the Worktree, which is called the same thing on every machine —
    /// and the memory files beside them. Everything else in there is the human's
    /// work in other repositories.
    #[test]
    fn a_codex_store_carries_this_conversations_rollouts_and_its_memory_files() {
        let account = store(&[
            (
                "sessions/2026/09/29/rollout-ours.jsonl",
                &rollout("/srv/worktrees/verkstead-rate-limiting"),
            ),
            (
                "sessions/2026/09/28/rollout-theirs.jsonl",
                &rollout("/home/you/src/secrets"),
            ),
            ("sessions/2026/09/29/rollout-ours.jsonl.zst", b"compressed"),
            ("memories/MEMORY.md", b"what it remembers\n"),
            ("state_5.sqlite", b"the human's own"),
        ]);

        let root = Root::codex(account.path());
        let wanted = wanted("verkstead-rate-limiting", &[]);
        let parts = carried(
            root.synced(wanted.worktree.as_deref(), &wanted.sessions),
            wanted.repo.is_some(),
            wanted.worktree.is_some(),
        );

        assert_eq!(
            carrying(account.path(), &parts),
            ["memory/MEMORY.md", "sessions/2026/09/29/rollout-ours.jsonl",],
        );
    }

    /// A rollout written on another machine is still this Conversation's: the
    /// Worktree is at a different path on every device of a cluster and is called
    /// the same thing on all of them, which is what makes the reading the same
    /// reading on both ends.
    #[test]
    fn a_rollout_this_conversation_wrote_elsewhere_is_still_its_own() {
        let account = store(&[(
            "sessions/2026/09/29/rollout-away.jsonl",
            &rollout("C:/ProgramData/Verkstead/worktrees/verkstead-rate-limiting"),
        )]);

        let root = Root::codex(account.path());
        let wanted = wanted("verkstead-rate-limiting", &[]);
        let parts = carried(
            root.synced(wanted.worktree.as_deref(), &wanted.sessions),
            wanted.repo.is_some(),
            wanted.worktree.is_some(),
        );

        assert_eq!(
            carrying(account.path(), &parts),
            ["sessions/2026/09/29/rollout-away.jsonl"],
        );
    }

    /// Grok files a session's directory under the id Verkstead named it with,
    /// inside a directory it names by encoding the working directory — so the ids
    /// are what say which sessions are this Conversation's, at either depth. Not
    /// its index of the whole store, which is the human's.
    #[test]
    fn a_grok_store_carries_the_sessions_verkstead_named_and_its_memory() {
        let account = store(&[
            (
                "sessions/-srv-worktrees-verkstead/abc123/updates.jsonl",
                b"ours\n",
            ),
            ("sessions/def456/updates.jsonl", b"ours, beside\n"),
            (
                "sessions/-home-you-src-secrets/zzz999/updates.jsonl",
                b"theirs\n",
            ),
            ("sessions/session_search.sqlite", b"the human's index"),
            ("memory/MEMORY.md", b"what it remembers\n"),
            ("memory/verkstead/index.sqlite", b"per repository\n"),
            ("skills/theirs.md", b"how the human works\n"),
        ]);

        let root = Root::grok(account.path());
        let wanted = wanted("verkstead-rate-limiting", &["abc123", "def456"]);
        let parts = carried(
            root.synced(wanted.worktree.as_deref(), &wanted.sessions),
            wanted.repo.is_some(),
            wanted.worktree.is_some(),
        );

        assert_eq!(
            carrying(account.path(), &parts),
            [
                "memory/MEMORY.md",
                "memory/verkstead/index.sqlite",
                "sessions/-srv-worktrees-verkstead/abc123/updates.jsonl",
                "sessions/def456/updates.jsonl",
            ],
        );
    }

    /// OpenCode keeps one database for every directory it has ever run in, in
    /// write-ahead-log mode: there is no unit smaller than the data directory, and
    /// a database carried without its siblings is one that will not open. So the
    /// directory travels whole.
    #[test]
    fn an_opencode_store_carries_its_data_directory_whole() {
        let account = store(&[
            (".local/share/opencode/opencode.db", b"the database"),
            (".local/share/opencode/opencode.db-wal", b"the log"),
            (".local/share/opencode/opencode.db-shm", b"the index"),
            (".local/share/opencode/auth.json", b"the login"),
            (".config/opencode/opencode.json", b"the configuration"),
        ]);

        let root = Root::opencode(account.path());
        let wanted = wanted("verkstead-rate-limiting", &[]);
        let parts = carried(
            root.synced(wanted.worktree.as_deref(), &wanted.sessions),
            wanted.repo.is_some(),
            wanted.worktree.is_some(),
        );

        assert_eq!(
            carrying(account.path(), &parts),
            [
                "data/auth.json",
                "data/opencode.db",
                "data/opencode.db-shm",
                "data/opencode.db-wal",
            ],
        );
    }

    /// And a database carried that way **opens** on the machine it lands on, which
    /// is the whole reason the directory travels whole: opencode keeps its store in
    /// write-ahead-log mode, and a file carried without its `-wal` and `-shm`
    /// siblings is a database that will not open.
    #[tokio::test]
    async fn an_opencode_database_opens_on_the_device_it_lands_on() {
        use sqlx::Executor;
        use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode};

        let account = tempfile::tempdir().unwrap();
        let database = account.path().join(".local/share/opencode/opencode.db");
        std::fs::create_dir_all(database.parent().unwrap()).unwrap();

        let writing = sqlx::SqlitePool::connect_with(
            SqliteConnectOptions::new()
                .filename(&database)
                .create_if_missing(true)
                .journal_mode(SqliteJournalMode::Wal),
        )
        .await
        .unwrap();

        writing
            .execute(
                "CREATE TABLE session (id TEXT PRIMARY KEY, directory TEXT NOT NULL);
                 INSERT INTO session VALUES ('one', '/srv/worktrees/verkstead-rate-limiting');",
            )
            .await
            .unwrap();

        // Left as a running session leaves it: the log is beside the file rather
        // than folded into it, which is the state a copy has to survive.
        let root = Root::opencode(account.path());
        let wanted = wanted("verkstead-rate-limiting", &[]);
        let parts = carried(
            root.synced(wanted.worktree.as_deref(), &wanted.sessions),
            wanted.repo.is_some(),
            wanted.worktree.is_some(),
        );

        let carrying = gathered(account.path(), &parts).unwrap();

        assert!(
            carrying.iter().any(|file| file.inside == "opencode.db-wal"),
            "the log travels beside the database: {:?}",
            carrying.iter().map(|file| &file.inside).collect::<Vec<_>>(),
        );

        let landed = tempfile::tempdir().unwrap();
        let there = Root::opencode(landed.path());
        let theirs = carried(
            there.synced(wanted.worktree.as_deref(), &wanted.sessions),
            wanted.repo.is_some(),
            wanted.worktree.is_some(),
        );

        written_down(landed.path(), &theirs, &carrying).unwrap();

        let reading = sqlx::SqlitePool::connect_with(
            SqliteConnectOptions::new()
                .filename(landed.path().join(".local/share/opencode/opencode.db"))
                .create_if_missing(false),
        )
        .await
        .expect("the database that landed opens");

        let (directory,): (String,) = sqlx::query_as("SELECT directory FROM session WHERE id = ?")
            .bind("one")
            .fetch_one(&reading)
            .await
            .expect("and what was written before it travelled is in it");

        assert_eq!(directory, "/srv/worktrees/verkstead-rate-limiting");
    }

    /// Switched off, nothing syncs: the session starts on an empty store away from
    /// home exactly as it does at home, and neither direction carries anything.
    #[test]
    fn a_profile_that_shares_no_memory_carries_nothing() {
        let account = store(&[("memories/MEMORY.md", b"what it remembers\n")]);

        let root = Root::codex(account.path()).remembering(false);
        let wanted = wanted("verkstead-rate-limiting", &[]);
        let parts = carried(
            root.synced(wanted.worktree.as_deref(), &wanted.sessions),
            wanted.repo.is_some(),
            wanted.worktree.is_some(),
        );

        assert!(parts.is_empty(), "no parts: {parts:?}");
        assert!(carrying(account.path(), &parts).is_empty());
    }

    /// And a part named off something the question never settled is left out on
    /// both ends: no Repo of the other device's is this repository, so there is
    /// nothing of the Repo's memory to name there.
    #[test]
    fn no_repo_match_leaves_the_repos_entry_out() {
        let account = tempfile::tempdir().unwrap();
        let worktree = account.path().join("worktrees/verkstead-rate-limiting");
        std::fs::create_dir_all(worktree.join(".git")).unwrap();

        let root = Root::claude(
            crate::platform::Platform::Linux,
            account.path(),
            &account.path().join("src/verkstead/.git"),
            &worktree,
        );

        let wanted = MemoryWanted {
            repo: None,
            worktree: Some("verkstead-rate-limiting".to_owned()),
            sessions: Vec::new(),
        };

        let parts = carried(
            root.synced(wanted.worktree.as_deref(), &wanted.sessions),
            wanted.repo.is_some(),
            wanted.worktree.is_some(),
        );

        assert_eq!(
            parts.iter().map(|part| part.label).collect::<Vec<_>>(),
            [root::WORKTREE],
        );
    }

    /// And a part this device could not find a path for is left out rather than
    /// **named off nothing**, which is the wider answer rather than the narrower
    /// one: a Claude entry named off an empty path is the whole of `projects/`,
    /// and what is under that is every repository the human has ever run the
    /// harness in.
    ///
    /// Which is the shape [`crate::peer::memory`] is in when the question names a
    /// Repo that has been unregistered since the match, or a Worktree stem it
    /// will not take — so the finding is what filters, not the question.
    #[test]
    fn a_part_with_no_path_is_left_out_rather_than_naming_the_whole_store() {
        let account = store(&[
            ("projects/-srv-worktrees-verkstead/ours.jsonl", b"ours\n"),
            (
                "projects/-home-you-src-secrets/theirs.jsonl",
                b"another repository's\n",
            ),
        ]);

        // The two paths the answering device is left with where it could name
        // neither: a Repo's path it has not got, with `.git` joined onto it, and
        // no Worktree stem it will take.
        let root = Root::claude(
            crate::platform::Platform::Linux,
            account.path(),
            &PathBuf::new().join(".git"),
            Path::new(""),
        );

        let asked = root.synced(Some("verkstead-rate-limiting"), &[]);

        assert!(
            carrying(account.path(), &asked)
                .iter()
                .any(|file| file.contains("secrets")),
            "an entry named off nothing reaches every repository in the store, \
             which is what there must be no path to: {:?}",
            carrying(account.path(), &asked),
        );

        assert!(
            carried(asked, false, false).is_empty(),
            "so neither part is one that travels",
        );
    }

    /// Only what the session changed goes home: a file still exactly as it came
    /// down is not sent, one it wrote is, one it made is — and one it took away is
    /// nothing to send, nothing at home being deleted from here.
    #[test]
    fn only_what_the_session_changed_goes_home() {
        let account = store(&[
            ("memories/untouched.md", b"as it came down\n"),
            ("memories/written.md", b"as the session left it\n"),
            ("memories/made.md", b"the session's own\n"),
        ]);

        let root = Root::codex(account.path());
        let wanted = wanted("verkstead-rate-limiting", &[]);
        let parts = carried(
            root.synced(wanted.worktree.as_deref(), &wanted.sessions),
            wanted.repo.is_some(),
            wanted.worktree.is_some(),
        );

        let came_down = HashMap::from([
            (
                (root::MEMORY.to_owned(), "untouched.md".to_owned()),
                fingerprint(STANDARD.encode("as it came down\n").as_bytes()),
            ),
            (
                (root::MEMORY.to_owned(), "written.md".to_owned()),
                fingerprint(STANDARD.encode("as it came down\n").as_bytes()),
            ),
            (
                (root::MEMORY.to_owned(), "taken-away.md".to_owned()),
                fingerprint(STANDARD.encode("as it came down\n").as_bytes()),
            ),
        ]);

        let mut left: Vec<String> = left_behind(account.path(), &parts, &came_down)
            .unwrap()
            .into_iter()
            .map(|file| file.inside)
            .collect();

        left.sort();

        assert_eq!(left, ["made.md", "written.md"]);
    }

    /// What is written down is written under the parts **this** device names, and
    /// nothing else: a label it holds no part for is not a directory to invent,
    /// and a path that would climb out of one is not a file of a memory store.
    #[test]
    fn only_a_part_this_device_names_is_written_down() {
        let account = tempfile::tempdir().unwrap();

        let root = Root::codex(account.path());
        let wanted = wanted("verkstead-rate-limiting", &[]);
        let parts = carried(
            root.synced(wanted.worktree.as_deref(), &wanted.sessions),
            wanted.repo.is_some(),
            wanted.worktree.is_some(),
        );

        let said = |part: &str, inside: &str| MemoryFile {
            part: part.to_owned(),
            inside: inside.to_owned(),
            bytes: STANDARD.encode("what arrived\n"),
        };

        let came_down = written_down(
            account.path(),
            &parts,
            &[
                said(root::MEMORY, "MEMORY.md"),
                said(root::REPO, "somewhere-else.md"),
                said(root::MEMORY, "../../.ssh/authorized_keys"),
                said(root::MEMORY, "/etc/passwd"),
            ],
        )
        .unwrap();

        assert_eq!(
            came_down.keys().collect::<Vec<_>>(),
            [&(root::MEMORY.to_owned(), "MEMORY.md".to_owned())],
        );
        assert_eq!(
            std::fs::read_to_string(account.path().join("memories/MEMORY.md")).unwrap(),
            "what arrived\n",
        );
        assert!(!account.path().join("repo").exists());
    }
}
