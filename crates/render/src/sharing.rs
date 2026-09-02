//! What a share carries: one Conversation's record, curated, as the file a
//! colleague opens.
//!
//! A share is read rather than worked in, and it is read by somebody who is not
//! sitting at this Verkstead — so the curation happens here, on the way out,
//! rather than in the page that draws it. Two halves to it, and both are about
//! the same thing: what a reader outside the workbench has any business with.
//!
//! **Which Events board.** The Brief, the Question Sets, the commits, the
//! steers, the moves and the Manual Tasks a Verkstead of before set going. Not
//! what a session printed, not the Notices Verkstead wrote itself, not the
//! handoff, not a Set this build could not read, and none of the pinned cards.
//! Silently: no placeholder marks the gap, because a share is a curated record
//! rather than a record with holes cut in it — a row saying *something was here*
//! would be an invitation to ask for what was deliberately left out.
//!
//! **And what is left of the Conversation around them.** Every field the
//! workbench reads to decide what may be *done* is put back to the value that
//! says *nothing* — no run to resume, no run to stop, no grilling to start,
//! nothing being adopted. The page the share is drawn with is the workbench's
//! own, so this is what makes it read-only at the source: a share cannot express
//! an action, whatever a component reused to draw it would otherwise offer.
//!
//! **And every field that is about the machine rather than the work.** Where the
//! checkouts sit on somebody's disk, and which account and model each kind of
//! session ran under, are facts about this Verkstead — and the reader of a share
//! has none of it, out of a file that is emailed about and attached to pull
//! requests. So the paths and the Pairings come off here as well, and the Brief's
//! pane draws neither of them in a share: see `Brief.tsx`, which is the other
//! half of this and the reason the values left behind are never read.
//!
//! What the reader does have is the record and the way around it, which is the
//! whole point: the Timeline on one side, and whatever it opens on the other.

use serde::{Deserialize, Serialize};
#[cfg(feature = "typescript")]
use ts_rs::TS;

use crate::conversations::{CommitPane, CompanionView, ConversationView, TimelineEvent};
use crate::profiles::PickedView;
use crate::repos::RepoEntry;
use crate::view::SetView;

/// One Conversation as a share carries it, which is what the shared file boots
/// from.
///
/// The Conversation whole rather than a shape of its own: the share is drawn by
/// the workbench's own components, so what they are handed has to be what they
/// are always handed. What differs is that this one has been through
/// [`shared`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct SharedConversation {
    /// The record, curated — see [`shared`].
    pub conversation: ConversationView,

    /// The sheet of every Question Set on that Timeline, in its order.
    ///
    /// Carried rather than fetched, which is the difference between a share and
    /// the workbench: the live viewer asks for a Set when somebody opens one,
    /// and a share has nothing to ask. So the whole of every Set the record
    /// holds — the Preface, every Option of every Question, the Diff it was
    /// asked over and what was decided — rides in the file, rendered by the
    /// endpoint the workbench reads a Set through, so that a colleague's sheet
    /// and the human's are one rendering of one decision.
    ///
    /// Read-only regardless of how a Set stood when the share was taken: what
    /// makes it so is the sheet being drawn as a record — see the share's
    /// details pane — because a Set still waiting on somebody is part of the
    /// record too, and a reader with no server behind them cannot answer it.
    pub sets: Vec<SetView>,

    /// The pane behind every commit on that Timeline, in its order.
    ///
    /// Carried for the reason the sheets are: the workbench fetches a commit
    /// when somebody opens one, and a share has nothing to fetch with. So the
    /// whole of every one of them rides in the file — the Commit Summary
    /// rendered, and the diff parsed, highlighted and folded per file.
    ///
    /// No cap on any of it, and nothing summarised on the way out. What a
    /// colleague is being shown is the work, and a patch cut off at a size is
    /// a different document from the one the human reviewed.
    pub commits: Vec<SharedCommit>,

    /// When the share was taken, RFC 3339.
    ///
    /// A share is a snapshot of a moment rather than a window onto a
    /// Conversation that goes on moving, so the moment is on the file: the
    /// reader is owed the date of the thing in their hands, and sharing again
    /// makes another one rather than freshening this.
    pub exported_at: String,
}

/// One commit as a share carries it: the pane the workbench would have fetched,
/// beside the Event whose card opens it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct SharedCommit {
    /// Which Timeline Event this is the pane of.
    ///
    /// The Event rather than the hash, because that is what the card opening it
    /// is known by — and a Conversation works in more than one repository, so a
    /// hash is not a name for one commit here either.
    pub id: i64,

    /// What the workbench's details pane draws: the Commit Summary rendered,
    /// and the diff with every fold and every colour already in it.
    ///
    /// The endpoint's own rendering rather than a second one, so that a
    /// colleague reading a patch and the human who reviewed it are reading one
    /// drawing of it.
    pub pane: CommitPane,

    /// Whether the repository still had the commit when the share was taken.
    ///
    /// `false` is a commit git can no longer show — rebased away, collected, or
    /// in a repository that has moved out from under Verkstead — and the pane
    /// says the diff is not in the file rather than that the commit changed
    /// nothing. The workbench answers that case with a 404, which a share
    /// cannot: one commit nobody can read is no reason to refuse the export of
    /// everything around it, and what the Timeline says about it — the subject,
    /// the hash, how much it moved — is on the card either way.
    pub held: bool,
}

/// One commit as a share carries it, rendered.
///
/// `patch` is what the repository said when it was asked for one, and `None` is
/// a repository that would not say — which is the whole of what
/// [`SharedCommit::held`] records.
///
/// The summary is rendered either way. It was kept by the sweep that recorded
/// the commit rather than read back out of git, so a commit that has gone still
/// has its own account of itself to read.
pub fn shared_commit(id: i64, summary: Option<&str>, patch: Option<&str>) -> SharedCommit {
    SharedCommit {
        id,
        // No patch renders as no diff, which is also what an empty commit and a
        // merge that resolved nothing render as — the flag beside it is what
        // tells the reader which of the two kinds of nothing they are looking
        // at.
        pane: crate::conversations::commit_pane(summary, patch.unwrap_or_default()),
        held: patch.is_some(),
    }
}

/// Curate a Conversation for sharing: the Events that board, the sheets and the
/// diffs behind the ones that open, and a record with nothing left on it to act
/// on.
///
/// `sets` and `commits` are every Question Set and every commit the caller
/// rendered. What comes back holds the ones still on the curated Timeline and
/// no others: which Events board is this module's rule, and a bundle carrying
/// the sheet of a Set whose row was taken off would be carrying what the reader
/// was not meant to have.
pub fn shared(
    conversation: ConversationView,
    sets: Vec<SetView>,
    commits: Vec<SharedCommit>,
    exported_at: String,
) -> SharedConversation {
    let timeline: Vec<TimelineEvent> = conversation
        .timeline
        .into_iter()
        .filter(boards)
        .map(frozen)
        .collect();

    let boarded: Vec<i64> = timeline.iter().filter_map(asked).collect();
    let landed: Vec<i64> = timeline.iter().filter_map(committed).collect();

    // The two that are nested rather than fields of their own, said here because
    // a functional update cannot reach inside one. A repository keeps its name
    // and loses where it is on the disk — the name is what the record calls the
    // work's repository and the path is where this machine happens to keep it —
    // and a companion loses its checkout for the reason the Conversation's own
    // does below.
    let repo = RepoEntry {
        path: String::new(),
        ..conversation.repo
    };

    let companions: Vec<CompanionView> = conversation
        .companions
        .into_iter()
        .map(|companion| CompanionView {
            repo: RepoEntry {
                path: String::new(),
                ..companion.repo
            },
            worktree: None,
            ..companion
        })
        .collect();

    SharedConversation {
        sets: sets
            .into_iter()
            .filter(|set| boarded.contains(&set.id))
            .collect(),
        commits: commits
            .into_iter()
            .filter(|commit| landed.contains(&commit.id))
            .collect(),
        conversation: ConversationView {
            timeline,

            // Nothing is pinned in a share. Each pinned card is the current
            // state of something the work is against — a backlog read off a
            // worktree, a pull request as GitHub has it — and a share is
            // neither of those: it is a moment, and the reader has no worktree
            // to read and nothing to open a pull request with.
            pinned: Vec::new(),

            // Every field the workbench decides an action by, said as nothing.
            // The record is what is being shared; what could be *done* about it
            // belongs to whoever has the workbench.
            ready_to_grill: false,
            ready_to_resume: false,
            ready_to_stop: false,
            stop_asked: false,
            ready_to_continue: false,
            compiles_uncached: false,

            // And what is being adopted, which is the one other thing that puts
            // a control on the record: an adopting Conversation draws the Adopt
            // press and the setup card under its Brief.
            adopting: None,

            // What is happening right now, which is nothing here: a share is a
            // file, and no session is running in it. Both of these are read as
            // of the moment a page is drawn, so a share that carried them would
            // be saying something true of a moment that has passed.
            working: false,
            driven: false,

            // And the marks that point at a stop. The Notice a stop is read
            // through does not board, so a badge pointing at one would point at
            // nothing — and *blocked on you* said to somebody who cannot act is
            // a mark asking the wrong person.
            blocked_on: None,
            stopped_by_hand: false,
            waiting_on_checks: false,
            resets: None,

            // And where a share of this Conversation was last published, which
            // is the workbench's fact about the record rather than part of it:
            // a reader already holds a share, and one carrying the link to
            // another would be handing on a URL nobody meant to give them.
            shared: None,

            // And the machine this was worked on, said as nothing. Where a
            // checkout is, and which account and model wrote the work, are the
            // human's own arrangements rather than anything a colleague reading
            // the record is owed — and a share is a file that leaves the
            // tailnet, so what it does not carry is the only thing it cannot
            // give away.
            repo,
            companions,
            worktree: None,
            grilling_pairing: PickedView::Nothing,
            implementation_pairing: None,
            review_pairing: PickedView::Nothing,

            ..conversation
        },
        exported_at,
    }
}

/// Whether one Event boards.
///
/// The rule is a list rather than a judgement: what a share is for is showing
/// what was asked, answered and built, and the kinds left out are the ones that
/// are either nobody else's to read — a session's own output, the handoff
/// between two of them — or Verkstead talking to itself.
fn boards(event: &TimelineEvent) -> bool {
    match event {
        TimelineEvent::Brief(_)
        | TimelineEvent::QuestionSet(_)
        | TimelineEvent::Commit(_)
        | TimelineEvent::Steer(_)
        | TimelineEvent::ResolveConflicts(_)
        | TimelineEvent::Moved(_)
        | TimelineEvent::ManualTask(_) => true,

        TimelineEvent::AgentOutput(_)
        | TimelineEvent::UnreadableSet(_)
        | TimelineEvent::Handoff(_)
        | TimelineEvent::Notice(_)
        | TimelineEvent::PullRequest(_)
        | TimelineEvent::TaskList(_)
        | TimelineEvent::StageList(_) => false,
    }
}

/// Which Set an Event on the curated Timeline is about, on the one kind that is
/// about one.
fn asked(event: &TimelineEvent) -> Option<i64> {
    match event {
        TimelineEvent::QuestionSet(asked) => Some(asked.set_id),
        _ => None,
    }
}

/// And which Event a commit's pane belongs to, on the one kind that has one.
fn committed(event: &TimelineEvent) -> Option<i64> {
    match event {
        TimelineEvent::Commit(commit) => Some(commit.id),
        _ => None,
    }
}

/// And the one Event a share has to say something else about: the Brief, which
/// is frozen here whatever it was.
///
/// A Brief that has not frozen is a field the human types into, with the
/// Conversation's setup under it. Frozen, it is the document it will be read as
/// for the rest of the record's life — which is what a share of a Draft should
/// show, and the only thing a reader with no server behind them could do
/// anything with.
fn frozen(event: TimelineEvent) -> TimelineEvent {
    match event {
        TimelineEvent::Brief(brief) => TimelineEvent::Brief(crate::conversations::BriefEvent {
            frozen: true,
            ..brief
        }),
        other => other,
    }
}

/// What became of publishing a share: where it went, or why it did not go.
///
/// A publish is Verkstead's own write to GitHub rather than a session's, and
/// every way it can fail is something for the human to go and do — which is why
/// each is named rather than folded into one refusal. Two of the three are about
/// the token on the settings page, and the page is where they are answered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum SharePublished {
    /// It is up, at this link and as of this moment.
    Published {
        share: crate::conversations::ShareView,
    },

    /// No token is configured, so there is nobody to publish as.
    ///
    /// A refusal rather than a fallback to whatever login the host's `gh` has,
    /// which is the one place Verkstead's reach into GitHub differs between
    /// reading and writing: a pull request read as the host is a question asked
    /// twice, and a gist *written* as the host is a file in an account nobody
    /// chose, under a login the human may not even be able to find it in.
    NoToken,

    /// There is one, and gists are not among what GitHub will let it do — the
    /// `gist` scope, which a token issued for reading repositories does not
    /// carry. Fixed by re-issuing it with that ticked and saving it again.
    NoGistScope,

    /// Something else, in `gh`'s or git's own words.
    Refused { why: String },
}

/// What became of sharing a Conversation to the pull requests its work is on.
///
/// One press is three acts — the file, the publish, and a comment on every pull
/// request the Conversation holds — so what comes back has to say how far it
/// got. The two ways it stops before saying anything anywhere are the publish's
/// own, carried in [`SharePublished`]'s words rather than said again here: a
/// share nobody could publish is a comment with no link in it, and there is
/// nothing to leave on a pull request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum ShareCommented {
    /// It is published, and this is where the comments went.
    ///
    /// `missed` is empty on the ordinary press, and what it holds when it is not
    /// is named rather than swallowed: a pull request that has gone, or one the
    /// token may not write on, is reported beside the ones that worked. The
    /// share is up either way — the link is on the record, and the human can
    /// paste it themselves wherever a comment could not go.
    Commented {
        share: crate::conversations::ShareView,
        on: Vec<CommentedOn>,
        missed: Vec<MissedOut>,
    },

    /// The publish never happened, in the publish's own words, so nothing was
    /// said on any pull request either.
    ///
    /// Every refusal a publish has is a refusal this press has — it is the same
    /// write to GitHub under the same token — and the workbench reads it with
    /// the same sentences. [`SharePublished::Published`] never arrives here:
    /// this is the shape of a press that got no further than the publish.
    NotPublished { why: SharePublished },

    /// This Conversation is on no pull request, so there was nowhere to comment
    /// and nothing was published.
    ///
    /// The press is offered only where the record holds one, so this is a page
    /// drawn against a Conversation that has since moved — and a share made for
    /// nobody would be a gist in somebody's account for nothing.
    NoPullRequest,
}

/// One pull request the comment landed on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct CommentedOn {
    /// The number GitHub gave it, which is what a human calls it by.
    pub number: i64,

    /// Which repository that number is in, where it is not the Conversation's
    /// own — the same label a pull request's card draws, and `null` for the
    /// same reason: an unlabeled one means the repo the work is in.
    pub repo: Option<String>,

    /// The comment itself, as GitHub gave it back, so the human can go and read
    /// what was left in their name.
    pub url: String,
}

/// And one it did not.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct MissedOut {
    pub number: i64,

    pub repo: Option<String>,

    /// What `gh` said about it, in its own words. A pull request that has gone
    /// and one the token may not write on are two afternoons apart, and neither
    /// is anything Verkstead can put right on the human's behalf.
    pub why: String,
}

/// What says a comment is the one a share left, rather than something somebody
/// wants addressed.
///
/// An HTML comment, so GitHub draws nothing where it sits: a reader of the pull
/// request sees the link and the itemization and no machinery at all. It goes on
/// a line of its own at the end of the body, and whatever reads it looks for it
/// at the *start* of a line — a human quote-replying to the share gets every
/// line of it prefixed with `>`, and their reply is somebody talking rather than
/// Verkstead's own comment coming back around.
///
/// Verkstead's own name is in it because a pull request is a public place: a
/// marker somebody might plausibly write themselves is one that would silence
/// their comment by accident.
pub const SHARE_MARKER: &str = "<!-- verkstead:shared-conversation -->";

/// The comment a share leaves on a pull request: the link, and what is in the
/// file behind it.
///
/// Markdown, because that is what a comment is. `link` is composed by whoever
/// is posting — the share viewer with the gist's id after it, the human's own
/// viewer where they host one and Verkstead's hosted copy where they do not —
/// so nothing here has an opinion about which page a reader is being sent
/// through.
///
/// **Itemized off the share rather than off the Conversation.** What the comment
/// lists is what the file actually carries, which is the curated Timeline and
/// not the whole one: a summary naming a Question Set that was left out of the
/// share would be an invitation to open a file that has no such thing in it.
///
/// `title` is what the Conversation is called wherever it is named — the branch,
/// or *Draft* where nobody has settled one — said by the caller because a share
/// is named the same way in three places and this is not where that rule lives.
pub fn itemized(share: &SharedConversation, title: &str, link: &str) -> String {
    listed(
        &share.conversation.timeline,
        &share.exported_at,
        title,
        link,
    )
}

/// The same, off the parts of the share it reads: the curated Timeline and the
/// day the snapshot was taken.
fn listed(timeline: &[TimelineEvent], taken: &str, title: &str, link: &str) -> String {
    let mut said = format!(
        "[Read this conversation]({link}) — a read-only copy of {}, taken {}.\n",
        coded(title),
        day(taken),
    );

    // What the work was asked for, which is the one line of the Brief a reader
    // deciding whether to open the link needs. As prose rather than as it was
    // written: a first line that is a heading would be a heading in the comment.
    if let Some(brief) = timeline.iter().find_map(opening) {
        said.push_str(&format!("\n{brief}\n"));
    }

    let sets: Vec<String> = timeline.iter().filter_map(titled).collect();

    if !sets.is_empty() {
        said.push_str("\n**Question Sets**\n\n");

        for set in sets {
            said.push_str(&format!("- {set}\n"));
        }
    }

    let commits: Vec<String> = timeline.iter().filter_map(landed).collect();

    if !commits.is_empty() {
        said.push_str("\n**Commits**\n\n");

        for commit in commits {
            said.push_str(&format!("- {commit}\n"));
        }
    }

    // The marker last, on a line of its own under a blank one, so that it is a
    // comment of markdown's own rather than the tail of whatever the itemization
    // ended on. Nothing is drawn where it sits, and nothing said above it
    // changes — see [`SHARE_MARKER`].
    said.push_str(&format!("\n{SHARE_MARKER}\n"));

    said
}

/// The Brief's first line, as words.
///
/// The first line with anything on it, put through the plain rendering every
/// card uses: the marks come off, so a Brief opening with a heading or a bullet
/// reads as a sentence in among the rest of the comment rather than as markup
/// of its own.
fn opening(event: &TimelineEvent) -> Option<String> {
    let TimelineEvent::Brief(brief) = event else {
        return None;
    };

    let line = brief
        .markdown
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())?;

    let said = crate::markdown::to_plain(line);

    (!said.trim().is_empty()).then_some(said)
}

/// One Question Set, by the title the agent gave it.
fn titled(event: &TimelineEvent) -> Option<String> {
    match event {
        TimelineEvent::QuestionSet(asked) => Some(asked.title.clone()),
        _ => None,
    }
}

/// And one commit, by its subject and how much it moved — the same three counts
/// the Timeline's own card draws, in the same words and with the signs on the
/// numbers.
fn landed(event: &TimelineEvent) -> Option<String> {
    let TimelineEvent::Commit(commit) = event else {
        return None;
    };

    // The repository, where it is not the Conversation's own. Exactly the card's
    // rule: an unlabeled line means the repo the work is in, and the label earns
    // its place when a Timeline carries more than one repository's commits.
    let repo = match &commit.repo {
        Some(repo) => format!(" ({repo})"),
        None => String::new(),
    };

    Some(format!(
        "{}{repo} — {} {}, +{} −{}",
        coded(&commit.subject),
        commit.files,
        if commit.files == 1 { "file" } else { "files" },
        commit.insertions,
        commit.deletions,
    ))
}

/// The day out of an RFC 3339 stamp, which is all a comment has room to say
/// about when a snapshot was taken.
///
/// Whatever it was handed where that is not what it is looking at: a stamp this
/// cannot read is still worth printing, and a comment saying nothing about when
/// the share was taken would be the worse of the two.
fn day(taken: &str) -> &str {
    const DAY: usize = "2026-08-30".len();

    match taken.split_once('T') {
        Some((day, _)) if day.len() == DAY => day,
        _ => taken,
    }
}

/// A line of somebody else's words as markdown may hold it: inline code, fenced
/// with one backtick more than the longest run inside it.
///
/// A commit subject is prose an agent wrote, and prose has asterisks and
/// underscores in it — so it goes in a comment as code rather than as markup to
/// be interpreted. The fence is counted rather than assumed because a subject
/// may itself carry a backtick, which is markdown's own rule for this: `` `a` ``
/// inside a longer fence is the code it looks like.
fn coded(text: &str) -> String {
    let longest = text
        .split(|character| character != '`')
        .map(str::len)
        .max()
        .unwrap_or(0);

    let fence = "`".repeat(longest + 1);

    // The spaces are markdown's own escape for code that begins or ends with a
    // backtick, and they are eaten by the renderer rather than drawn.
    match text.starts_with('`') || text.ends_with('`') {
        true => format!("{fence} {text} {fence}"),
        false => format!("{fence}{text}{fence}"),
    }
}

#[cfg(test)]
mod tests {
    use crate::conversations::{CommitEvent, QuestionSetEvent};
    use crate::view::Standing;

    use super::*;

    /// A Timeline of the three kinds a comment itemizes, in the order a
    /// Conversation reaches them: what was asked for, what was put to the human,
    /// and what was built.
    fn timeline() -> Vec<TimelineEvent> {
        vec![
            crate::conversations::brief_event(
                1,
                "2026-08-29T09:00:00Z".to_owned(),
                "# Sharing\n\nA read-only copy of a conversation to send somebody.\n".to_owned(),
                true,
            ),
            asked(2, "What a share carries"),
            committed(
                3,
                "feat: share a conversation as one file to send",
                9,
                742,
                11,
            ),
        ]
    }

    fn asked(id: i64, title: &str) -> TimelineEvent {
        TimelineEvent::QuestionSet(QuestionSetEvent {
            id,
            at: "2026-08-29T10:00:00Z".to_owned(),
            set_id: id,
            title: title.to_owned(),
            rows: Vec::new(),
            standing: Standing::LockedUnanswered("2026-08-29T11:00:00Z".to_owned()),
        })
    }

    fn committed(
        id: i64,
        subject: &str,
        files: i64,
        insertions: i64,
        deletions: i64,
    ) -> TimelineEvent {
        TimelineEvent::Commit(CommitEvent {
            id,
            at: "2026-08-29T12:00:00Z".to_owned(),
            sha: "9f1c0de".to_owned(),
            subject: subject.to_owned(),
            files,
            insertions,
            deletions,
            snippet: None,
            repo: None,
            merge: false,
        })
    }

    #[test]
    fn a_comment_leads_with_the_link_and_what_it_is() {
        let said = listed(
            &timeline(),
            "2026-08-30T01:02:03.456Z",
            "sharing",
            "https://tobico.github.io/shares/#9f1",
        );

        assert!(
            said.starts_with(
                "[Read this conversation](https://tobico.github.io/shares/#9f1) \
                 — a read-only copy of `sharing`, taken 2026-08-30.\n"
            ),
            "the first line of: {said}",
        );
    }

    /// The Brief's first line, which is what a reader deciding whether to open
    /// the link is going on — as words rather than as it was written, so a Brief
    /// that opens with a heading does not open the comment with one.
    #[test]
    fn what_the_work_was_asked_for_is_under_it() {
        let said = listed(
            &timeline(),
            "2026-08-30T01:02:03Z",
            "sharing",
            "https://x/#9f1",
        );

        assert!(said.contains("\nSharing\n"), "the Brief's line in: {said}");
    }

    #[test]
    fn the_sets_are_listed_by_title_and_the_commits_by_subject() {
        let said = listed(
            &timeline(),
            "2026-08-30T01:02:03Z",
            "sharing",
            "https://x/#9f1",
        );

        assert!(
            said.contains("**Question Sets**\n\n- What a share carries\n"),
            "{said}"
        );
        assert!(
            said.contains(
                "**Commits**\n\n\
                 - `feat: share a conversation as one file to send` — 9 files, +742 −11\n"
            ),
            "{said}",
        );
    }

    /// One file is one file. The same words the Timeline's own card draws, in
    /// the same order and with the signs on the numbers.
    #[test]
    fn a_commit_that_moved_one_file_says_so() {
        let said = listed(
            &[committed(3, "fix: a typo", 1, 1, 1)],
            "2026-08-30T01:02:03Z",
            "sharing",
            "https://x/#9f1",
        );

        assert!(said.contains("- `fix: a typo` — 1 file, +1 −1\n"), "{said}");
    }

    /// A commit in a companion repository carries its name, exactly as its card
    /// does: an unlabeled line means the repository the work is in.
    #[test]
    fn a_companions_commit_says_which_repository_it_landed_in() {
        let TimelineEvent::Commit(commit) = committed(3, "fix: the footer", 1, 3, 3) else {
            unreachable!("that is what `committed` makes");
        };

        let said = listed(
            &[TimelineEvent::Commit(CommitEvent {
                repo: Some("verkstead-site".to_owned()),
                ..commit
            })],
            "2026-08-30T01:02:03Z",
            "sharing",
            "https://x/#9f1",
        );

        assert!(
            said.contains("- `fix: the footer` (verkstead-site) — 1 file, +3 −3\n"),
            "{said}",
        );
    }

    /// A Conversation with nothing built yet is a comment with no commits in it,
    /// rather than a heading over an empty list.
    #[test]
    fn a_section_with_nothing_in_it_is_not_drawn() {
        let said = listed(
            &timeline()[..2],
            "2026-08-30T01:02:03Z",
            "sharing",
            "https://x/#9f1",
        );

        assert!(said.contains("**Question Sets**"), "{said}");
        assert!(!said.contains("**Commits**"), "{said}");
    }

    /// Nothing an agent wrote can turn into markup. A subject is prose, and
    /// prose has asterisks, underscores and backticks in it.
    #[test]
    fn a_subject_is_code_rather_than_markup() {
        let said = listed(
            &[committed(3, "fix: draw `*` as a *mark*", 1, 1, 0)],
            "2026-08-30T01:02:03Z",
            "sharing",
            "https://x/#9f1",
        );

        assert!(
            said.contains("- ``fix: draw `*` as a *mark*`` — 1 file, +1 −0\n"),
            "{said}",
        );
    }

    /// The comment ends with the marker that keeps Wrapping off it, on a line
    /// of its own — an HTML comment, so a reader of the pull request sees
    /// nothing where it sits and everything above it is the comment as it was.
    #[test]
    fn a_comment_ends_with_the_marker_that_says_whose_it_is() {
        let said = listed(
            &timeline(),
            "2026-08-30T01:02:03Z",
            "sharing",
            "https://x/#9f1",
        );

        assert!(
            said.ends_with(&format!("\n{SHARE_MARKER}\n")),
            "the marker on a line of its own at the end of: {said}",
        );
        assert!(
            SHARE_MARKER.starts_with("<!--") && SHARE_MARKER.ends_with("-->"),
            "and it is an HTML comment, which draws as nothing: {SHARE_MARKER}",
        );
        assert!(
            said.contains("**Question Sets**\n\n- What a share carries\n")
                && said.contains("- `feat: share a conversation as one file to send` —"),
            "with what the comment says unchanged above it: {said}",
        );
    }

    /// A Conversation with nothing in the Timeline at all is still Verkstead's
    /// own comment, so the marker is not something the itemization carries: it
    /// is on every share comment there is.
    #[test]
    fn the_marker_is_there_whatever_the_share_holds() {
        let said = listed(&[], "2026-08-30T01:02:03Z", "sharing", "https://x/#9f1");

        assert!(
            said.lines().any(|line| line.starts_with(SHARE_MARKER)),
            "{said}",
        );
    }

    /// And a stamp this cannot read is still worth printing: a comment saying
    /// nothing about when the snapshot was taken is the worse of the two.
    #[test]
    fn a_moment_that_is_not_a_stamp_is_said_as_it_was_given() {
        assert_eq!(day("2026-08-30T01:02:03Z"), "2026-08-30");
        assert_eq!(day("whenever"), "whenever");
    }
}
