//! The files the human puts on an Answer: the upload the answer sheet's
//! paperclip makes, and the removal its × makes.
//!
//! The Brief's own pair said again from the other page — see
//! [`crate::conversations::attach`], which is where the same two acts are made
//! for the Brief. What differs is three things and nothing else.
//!
//! **What they are addressed by.** A Set rather than a Conversation: the sheet
//! is a page about one Set, and the human answering it may never have seen the
//! Conversation it was asked from. Which Conversation that is is read here, off
//! the Set, because the file itself has only one place to go — the one flat
//! directory of the Conversation's own, with the Brief's files beside it, the
//! same cap over it and the same counting-up of a name already taken. A file
//! named like one of the Brief's becomes `name-2.ext`, which is the directory
//! answering rather than anything here deciding.
//!
//! **What they are refused by.** The Brief's freeze is the work starting; this
//! one is the Set settling. A Set that has been answered or locked unanswered
//! is the record of what the human sent, and a Closed Conversation is one
//! nothing is going to act on — so each of the three is refused by name, and
//! everything else about a Set is beside the point. A Deferred Ask takes files
//! like any other: nobody is idling on it, and the human answering it is doing
//! exactly what they would be doing on any other.
//!
//! **What a label has to be.** A file put on an Answer names the Question it
//! answers, and the Question has to be one the Set asks — see
//! [`verkstead_schema::QuestionSet::asks`], which is the reading a Response is
//! checked against. A Heading asks nothing of its own, a Set does not carry
//! every label somebody could type, and a Set this build cannot read asks
//! nothing anybody here can name: all three are one refusal, because all three
//! are a file put where no Answer would ever draw it.

use std::path::PathBuf;

use anyhow::Result;
use sqlx::SqlitePool;
use verkstead_render::{AnswerAttached, AnswerAttachmentRemoved, AttachmentView};
use verkstead_schema::Response;

use crate::attachments::{self, Attachments};
use crate::platform::Platform;
use crate::{AppState, store};

/// Put a file on one of a Set's Answers.
///
/// The two writes [`crate::conversations::attach`] makes, in the order it makes
/// them: the bytes into the Conversation's own directory, and then the row that
/// says they are there and which Answer they are on. A file on disk with no row
/// is a stray the Cleanup takes with the directory; a row naming a file nobody
/// wrote is a pill that opens nothing.
///
/// The name comes back as it stands on disk rather than as it was sent, for
/// that same reason: a name already taken is counted up rather than overwritten,
/// and what the sheet draws is what the record says.
pub(crate) async fn attach(
    state: &AppState,
    set_id: i64,
    label: &str,
    name: &str,
    body: &[u8],
) -> Result<AnswerAttached> {
    if body.len() > attachments::MAX_BYTES {
        return Ok(AnswerAttached::TooLarge);
    }

    if !attachments::plain(name) {
        return Ok(AnswerAttached::NotAName);
    }

    let conversation = match standing(&state.pool, set_id).await? {
        Standing::Waiting(conversation) => conversation,
        Standing::Fixed(refusal) => return Ok(refusal.attaching()),
    };

    if !asks(&state.pool, set_id, label).await? {
        return Ok(AnswerAttached::NoSuchLabel);
    }

    let directories = Attachments::under(&state.data_dir);

    // Blocking work, off the runtime's threads: a 32 MB write is not something
    // to do in the middle of an async task other requests are waiting behind.
    let kept = {
        let name = name.to_owned();
        let body = body.to_vec();
        tokio::task::spawn_blocking(move || directories.keep(conversation, &name, &body)).await??
    };

    let attachment = store::attach(
        &state.pool,
        conversation,
        store::Origin::Answer {
            set: set_id,
            label: label.trim().to_owned(),
        },
        &kept,
        body.len() as i64,
    )
    .await?;

    Ok(AnswerAttached::Attached {
        attachment: attachments::view(attachment),
    })
}

/// And take one off again, file and row together.
///
/// The row is read first for the name — it is the only thing that says which
/// file in the directory this is — and written last, for [`attach`]'s reason the
/// other way round: a row taken away before the file would leave a file nothing
/// names.
///
/// Scoped to the Set the path names, so a row put on another Set is not this
/// press's to take — not even another Set of the same Conversation. One that is
/// not there at all is [`AnswerAttachmentRemoved::Removed`] rather than a
/// refusal, the way the Brief's removal is: what the press asked for is that it
/// be gone.
pub(crate) async fn detach(
    state: &AppState,
    set_id: i64,
    attachment: i64,
) -> Result<AnswerAttachmentRemoved> {
    let conversation = match standing(&state.pool, set_id).await? {
        Standing::Waiting(conversation) => conversation,
        Standing::Fixed(refusal) => return Ok(refusal.removing()),
    };

    let Some(found) = store::set_attachment(&state.pool, set_id, attachment).await? else {
        return Ok(AnswerAttachmentRemoved::Removed);
    };

    let directories = Attachments::under(&state.data_dir);
    tokio::task::spawn_blocking(move || directories.drop_file(conversation, &found.name)).await??;

    store::detach_from_set(&state.pool, set_id, attachment).await?;

    Ok(AnswerAttachmentRemoved::Removed)
}

/// Every file put on one Set's Answers, in the shape its page draws them.
///
/// Whether or not the Set has settled: the sheet draws them beside the answers
/// while it waits, and the record draws them under the decision once it has.
pub(crate) async fn attached(pool: &SqlitePool, set_id: i64) -> Result<Vec<AttachmentView>> {
    Ok(store::set_attachments(pool, set_id)
        .await?
        .into_iter()
        .map(attachments::view)
        .collect())
}

/// Put the files on the Response as the agent reads it: each Answer's own, by
/// the path the session opens each at.
///
/// **On the way out rather than in.** What the human submitted carries none of
/// these — see [`verkstead_schema::Answer::attachments`] — so the rows are the
/// truth about which Answer each file is on, and the Response is stored exactly
/// as it was sent. Which is also why a Response written before the field
/// existed needs nothing done to it: it reads as it always did, and what is
/// filled in here is filled in from the record beside it.
///
/// The path is the one the prompt's listing would name the same file at, off
/// the one reading — see [`crate::attachments::inside`] — so a session cannot
/// be told about a file twice and given two different paths for it. In the
/// order they were attached, which is the order the rows come back in.
///
/// An entry the record says nothing about is left alone, and that is nearly all
/// of them: an Answer with no files carries no list at all. One the human
/// marked `unanswered` and handed a file over with carries its file the same
/// way — the marker is theirs to mean and the file is still there to read.
pub(crate) async fn onto(
    state: &AppState,
    conversation_id: i64,
    set_id: i64,
    response: &mut Response,
) -> Result<()> {
    let files = store::set_attachments(&state.pool, set_id).await?;

    if files.is_empty() {
        return Ok(());
    }

    let inside = Attachments::under(&state.data_dir).inside(Platform::HERE, conversation_id);

    for answer in &mut response.answers {
        let label = answer.label.trim();

        answer.attachments = files
            .iter()
            .filter(|file| match &file.origin {
                store::Origin::Answer { label: on, .. } => on == label,
                store::Origin::Brief => false,
            })
            .map(|file| {
                crate::sandbox::under(&inside, &file.name)
                    .display()
                    .to_string()
            })
            .collect();
    }

    Ok(())
}

/// Where a Set stands, as far as its files are concerned.
pub(crate) enum Standing {
    /// Waiting on the human, with the Conversation whose directory its files
    /// land in — which is read here because the sheet is a page about a Set and
    /// the directory is a Conversation's.
    Waiting(i64),

    /// Fixed, and this is which way.
    Fixed(Fixed),
}

/// Why a Set's files are not the human's to change.
///
/// Each is said by name because each is a different sentence for the sheet to
/// put in front of them: an answered Set is what they sent, a locked one is
/// what they closed, and a Closed Conversation is work nothing is going to pick
/// up. The refusals of the two presses are the same list, so one reading
/// answers both — see [`Fixed::attaching`] and [`Fixed::removing`].
pub(crate) enum Fixed {
    NoSuchSet,
    Answered,
    Locked,
    Closed,
}

impl Fixed {
    fn attaching(self) -> AnswerAttached {
        match self {
            Self::NoSuchSet => AnswerAttached::NoSuchSet,
            Self::Answered => AnswerAttached::Answered,
            Self::Locked => AnswerAttached::Locked,
            Self::Closed => AnswerAttached::Closed,
        }
    }

    fn removing(self) -> AnswerAttachmentRemoved {
        match self {
            Self::NoSuchSet => AnswerAttachmentRemoved::NoSuchSet,
            Self::Answered => AnswerAttachmentRemoved::Answered,
            Self::Locked => AnswerAttachmentRemoved::Locked,
            Self::Closed => AnswerAttachmentRemoved::Closed,
        }
    }
}

/// Read where a Set stands: whether it has settled, and where the Conversation
/// it was asked from has got to.
///
/// Read before the writes rather than guarded inside them, for the reason the
/// Brief's own guard is read that way — there is one human at the workbench, and
/// what would be raced here is their own two tabs.
///
/// A Set on no Timeline is one nothing can be put on: the record is what says
/// which directory its files would land in, and a Set that names no Conversation
/// names no directory. It is the same answer a Set that is not there gets,
/// because from the sheet's side there is no such Set to speak of.
pub(crate) async fn standing(pool: &SqlitePool, set_id: i64) -> Result<Standing> {
    match store::settlement(pool, set_id).await? {
        Some(store::Settlement::Answered(_)) => return Ok(Standing::Fixed(Fixed::Answered)),
        Some(store::Settlement::LockedUnanswered(_)) => {
            return Ok(Standing::Fixed(Fixed::Locked));
        }
        None => {}
    }

    let Some(conversation) = store::asked_from(pool, set_id).await? else {
        return Ok(Standing::Fixed(Fixed::NoSuchSet));
    };

    Ok(match store::state(pool, conversation).await? {
        None => Standing::Fixed(Fixed::NoSuchSet),
        Some(store::Lifecycle::Closed) => Standing::Fixed(Fixed::Closed),
        Some(_) => Standing::Waiting(conversation),
    })
}

/// Whether the Set asks a Question by that label.
///
/// A Set this build cannot read asks nothing anybody here can name — the stored
/// body is kept as it stands and rendered by nothing, so a file put under one of
/// its labels would be a pill no page could ever draw.
pub(crate) async fn asks(pool: &SqlitePool, set_id: i64, label: &str) -> Result<bool> {
    let Some(stored) = store::load_set(pool, set_id).await? else {
        return Ok(false);
    };

    Ok(stored.set.set().is_some_and(|set| set.asks(label)))
}

/// Every file this Conversation's Answers carry, and where a session reads
/// them.
///
/// What a session brought up to speed on an exchange it did not see is told
/// about the files that came with it — see [`crate::exchanges::exchange`],
/// which writes them under the decision they were put on. The digest is made of
/// what the Timeline holds, and what the Timeline holds is the Response the
/// human submitted: the files are a record beside it, exactly as they are on the
/// way out to a waiting agent — see [`onto`], which fills the same paths into
/// the same Answers from the same rows.
///
/// The whole Conversation's rather than one Set's, because the two readings
/// that use it walk several Sets: a relaunched grilling is primed with
/// everything settled, and a folding carries every Deferred Ask answered since.
/// One read, and the Sets are told apart inside it.
pub(crate) struct OnAnswers {
    /// Every file the Conversation holds, in the order they were attached —
    /// the Brief's among them, which name no Answer and are therefore never
    /// found by one.
    files: Vec<store::Attachment>,

    /// Where the Conversation's directory is reached from inside the sandbox.
    inside: PathBuf,
}

impl OnAnswers {
    /// Read back for one Conversation.
    ///
    /// A read that fails is nothing named rather than a session that does not
    /// start: what the digest is for is priming a session with what was decided,
    /// and a launch lost over the files beside it would cost the human the whole
    /// exchange to save a line of it.
    pub(crate) async fn of(state: &AppState, conversation_id: i64) -> OnAnswers {
        let files = match store::attachments(&state.pool, conversation_id).await {
            Ok(files) => files,
            Err(error) => {
                tracing::error!(error = ?error, conversation_id, "reading the files put on this Conversation's Answers failed");
                Vec::new()
            }
        };

        OnAnswers::holding(
            files,
            Attachments::under(&state.data_dir).inside(Platform::HERE, conversation_id),
        )
    }

    /// Made of what the record holds and where the sandbox puts it, which is
    /// the pair every reading of these is: the rows say which file is on which
    /// Answer, and the directory says what to call it.
    pub(crate) fn holding(files: Vec<store::Attachment>, inside: PathBuf) -> OnAnswers {
        OnAnswers { files, inside }
    }

    /// The files put on one Set's Answers, which is what an exchange of that Set
    /// is written with.
    pub(crate) fn on(&self, set_id: i64) -> OnSet<'_> {
        OnSet {
            answers: self,
            set_id,
        }
    }
}

/// One Set's share of them, scoped so that an exchange cannot draw another
/// Set's files under its own decisions.
#[derive(Clone, Copy)]
pub(crate) struct OnSet<'a> {
    answers: &'a OnAnswers,
    set_id: i64,
}

impl OnSet<'_> {
    /// The paths the files put on one Question's Answer are read at, in the
    /// order they were attached.
    ///
    /// The path the prompt's own listing names the same file at, off the one
    /// reading — see [`crate::attachments::inside`] — so a session cannot be
    /// told about a file twice and given two different paths for it.
    pub(crate) fn under(&self, label: &str) -> Vec<String> {
        self.answers
            .files
            .iter()
            .filter(|file| match &file.origin {
                store::Origin::Answer { set, label: on } => *set == self.set_id && on == label,
                store::Origin::Brief => false,
            })
            .map(|file| {
                crate::sandbox::under(&self.answers.inside, &file.name)
                    .display()
                    .to_string()
            })
            .collect()
    }
}
