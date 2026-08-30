//! Writing the viewer's TypeScript out of the Rust it is generated from.
//!
//! Every type on the viewer's side of the wire is described once, in Rust, and
//! `web/src/api/types.ts` is written from those descriptions — so the two
//! languages cannot come to disagree about a field. The file is committed, and
//! rewritten by every `cargo test`: what says whether anything moved is the diff.
//!
//! The roots are listed here rather than each type carrying its own `#[ts(export)]`
//! for two reasons. One is that the list *is* the viewer's wire surface, and a
//! list is a thing that can be read; a type that no endpoint hands over has no
//! business in the bindings. The other is mechanical: `#[ts(export)]` generates a
//! test in the crate the type is defined in, and two crates' test binaries are
//! two processes, which would race to create the one file and each write half of
//! it. Exporting from here writes it once, in one process — dependencies and all,
//! including the Set types over in `verkstead-schema`.

use ts_rs::TS;

use crate::{
    AbandonedRepo, Adopted, BacklogPane, BaseBranchChoice, BaseRecorded, BranchRename,
    BranchRenamed, BriefEdit, BriefSaved, Capture, CommitPane, CompanionAdded,
    CompanionBaseRecorded, CompanionBranchRenamed, CompanionModeChoice, CompanionModeChosen,
    CompanionRemoved, ConversationArchived, ConversationClosed, ConversationEntry,
    ConversationSteered, ConversationStopped, ConversationUnarchived, ConversationView,
    GrillingStarted, Locked, NewAdoption, NewCompanion, NewConversation, NewOrder, ProfileChoice,
    ProfileChosen, ProfileDeleted, ProfileEdit, ProfileEntry, ProfileSaved, PullRequestDetails,
    PushKey, Registered, Registration, RepoEntry, RepoRemoved, RepoView, Resumed, RoadmapPane,
    RoleChoice, Screen, SetReading, SettingsEdit, SettingsSaved, SettingsView, ShowingArchived,
    Shown, Started, SteerOpened, SteerSubmission, Submitted, Subscribed, Subscription,
    TranscriptView, Unsubscribe, UpdateNotice, Watching,
};

/// Everything `/api/ui/` hands over or takes in, as TypeScript.
///
/// Each of these brings its own dependencies with it — a `SetView` writes the
/// Diff, the Questions, the Options and the Response it is made of — so what is
/// named here is the endpoints' own payloads and nothing more.
#[test]
fn the_viewers_types_are_written_from_these() {
    // The base directory and how an `i64` is spelled come from the environment —
    // see `.cargo/config.toml`, which is where both are said once.
    let config = ts_rs::Config::from_env();

    // One Set, whole — which is what the details pane of the Timeline it
    // landed on is drawn from. The whole reading rather than the `SetView`
    // inside it: what comes back says whether this build could read the stored
    // body at all, and the pane has to be able to draw either answer.
    SetReading::export_all(&config).unwrap();

    // Answering a Set, and closing it unanswered. What goes *in* to the first of
    // them is a Response, which the Set already brought along: it is what an
    // answered Set is read back with.
    Submitted::export_all(&config).unwrap();
    Locked::export_all(&config).unwrap();

    // The Repos Verkstead has been told about, adding one by path, one of them
    // opened — which brings the roadmaps waiting in it along with it — and taking
    // one off the registry again.
    RepoEntry::export_all(&config).unwrap();
    RepoView::export_all(&config).unwrap();
    Registration::export_all(&config).unwrap();
    Registered::export_all(&config).unwrap();
    RepoRemoved::export_all(&config).unwrap();

    // The workbench: the sidebar, one Conversation with its Timeline, and the
    // three things the human changes about a drafting one. Each edit brings its
    // own request shape and its own named outcome, because each of them is a
    // different sentence to put in front of the human.
    ConversationEntry::export_all(&config).unwrap();

    // And what is offered beside the sidebar: the Repos holding roadmaps
    // nothing is driving, which writes the roadmap inside it.
    AbandonedRepo::export_all(&config).unwrap();
    ConversationView::export_all(&config).unwrap();
    NewConversation::export_all(&config).unwrap();

    // And the order the human dragged that sidebar into, which is the one thing
    // they say about the list itself rather than about anything on it.
    NewOrder::export_all(&config).unwrap();

    // And starting one to adopt a roadmap with, which is the other way in — the
    // Conversation it starts comes back inside the view above.
    NewAdoption::export_all(&config).unwrap();
    Started::export_all(&config).unwrap();
    BriefEdit::export_all(&config).unwrap();
    BriefSaved::export_all(&config).unwrap();
    BranchRename::export_all(&config).unwrap();
    BranchRenamed::export_all(&config).unwrap();
    BaseBranchChoice::export_all(&config).unwrap();
    BaseRecorded::export_all(&config).unwrap();

    // And the other registered Repos it works alongside, which are added and
    // taken away on the same card. What one *is* comes back inside the view
    // above; these are the press that adds one and the press that does not.
    NewCompanion::export_all(&config).unwrap();
    CompanionAdded::export_all(&config).unwrap();
    CompanionRemoved::export_all(&config).unwrap();

    // And what the row it draws is configured with: the mode switch, the base
    // picker and the branch field, each with a named outcome of its own for the
    // reason the Conversation's own three have one.
    CompanionModeChoice::export_all(&config).unwrap();
    CompanionModeChosen::export_all(&config).unwrap();
    CompanionBaseRecorded::export_all(&config).unwrap();
    CompanionBranchRenamed::export_all(&config).unwrap();

    // And the two actions that make and unmake what a Conversation works in.
    // Neither takes a request shape — which Conversation is in the path, and
    // there is nothing else to say about either — so it is the outcomes alone.
    GrillingStarted::export_all(&config).unwrap();
    ConversationClosed::export_all(&config).unwrap();

    // And the press that puts a closed one away, which takes no request shape
    // either: which Conversation it is is the whole of what it says.
    ConversationArchived::export_all(&config).unwrap();
    ConversationUnarchived::export_all(&config).unwrap();

    // And the press that starts an adopted stage, which is the grilling start's
    // sibling: one Conversation, one branch, and every way of being refused
    // named separately. It takes no request shape either.
    Adopted::export_all(&config).unwrap();

    // Nothing here for how the work gets built: the recommendation and its
    // reasoning ride on the `SetView` above, the pick goes back as a field of
    // the Response, and neither is a payload of its own.

    // What a session printed. The summary rides on the Timeline; the Capture
    // is its own payload, because it is fetched by the one pane that shows it.
    // A Question Set on the Timeline is the same arrangement, and its full self
    // is the `SetView` above.
    Capture::export_all(&config).unwrap();

    // And what it said, which is the same pane's other payload and the one it
    // draws wherever there is one. Its turns are already rendered: the reading
    // of the log and the rendering of what is in it both happen on this side of
    // the wire, so what the viewer receives is HTML to put in the page.
    TranscriptView::export_all(&config).unwrap();

    // And how it looked, which is the same pane read the third way: the grid
    // those bytes leave on a terminal, as the escape sequences that would paint
    // it. The parsing that decided them happened here — the browser is handed a
    // repaint to feed a terminal, not a Capture to make sense of.
    Screen::export_all(&config).unwrap();

    // And the two directions of watching one that is still running, which is the
    // one thing the viewer is sent rather than fetching: what the server says
    // down the socket — the repaint above, then what the session prints — and
    // what a watcher says back up it.
    Shown::export_all(&config).unwrap();
    ShowingArchived::export_all(&config).unwrap();
    Watching::export_all(&config).unwrap();

    // And what a session committed. The snippet rides on the Timeline too; the
    // diff is its own payload, rendered by the same renderer an attached Diff
    // goes through — which is why this writes no new Diff types.
    CommitPane::export_all(&config).unwrap();

    // And the backlog opened, which is every task document of it: the entries
    // ride on the Conversation as the pinned task list, and the documents they
    // name are their own payload, read off the Worktree when somebody opens the
    // card.
    BacklogPane::export_all(&config).unwrap();

    // And the roadmap opened, which is the same arrangement one level up: the
    // stages ride on the Conversation as the pinned stage list, and the briefs
    // they name are their own payload — named by the roadmap, a Worktree being
    // allowed more than one of those.
    RoadmapPane::export_all(&config).unwrap();

    // And what the finish step opened. The PR itself rides on the Conversation
    // as a pinned Event — a number, a title and a URL; what is *on* it is its
    // own payload, because reading that is asking GitHub over the network.
    PullRequestDetails::export_all(&config).unwrap();

    // The press that gets a stopped Conversation going again, which takes no
    // request shape at all: what to start again is recomputed from the
    // lifecycle and the branch, so all there is to send is which Conversation.
    // What comes back is the outcome — a start, or the named reason there was
    // nothing to start.
    Resumed::export_all(&config).unwrap();

    // And the two presses that stop it, which take no request shape either and
    // answer with one outcome between them: the run is stopping, or the named
    // reason there was nothing to stop.
    ConversationStopped::export_all(&config).unwrap();

    // And the two presses that steer it. The first takes no request shape — the
    // click stops the drive and reports what it found running — and the second
    // carries what the modal settled. The Steer's own Event rides on the
    // `ConversationView` above, beside the move it wrote.
    SteerOpened::export_all(&config).unwrap();
    SteerSubmission::export_all(&config).unwrap();
    ConversationSteered::export_all(&config).unwrap();

    // The Agent Profiles a session can be run under, the one shape saving and
    // rewriting one both take, and the choices a Conversation makes of them —
    // a Pairing, or the row that runs no session where the role offers one.
    // A `ProfileEntry` writes the agent type and the broken-ness it carries.
    ProfileEntry::export_all(&config).unwrap();
    ProfileEdit::export_all(&config).unwrap();
    ProfileSaved::export_all(&config).unwrap();
    ProfileDeleted::export_all(&config).unwrap();
    ProfileChoice::export_all(&config).unwrap();
    ProfileChosen::export_all(&config).unwrap();
    RoleChoice::export_all(&config).unwrap();

    // Telling one device about a Set, and stopping.
    PushKey::export_all(&config).unwrap();
    Subscription::export_all(&config).unwrap();
    Subscribed::export_all(&config).unwrap();
    Unsubscribe::export_all(&config).unwrap();

    // What Verkstead has been told: the git author, and that there is a GitHub
    // token. `SettingsView` writes the author and the token's presence, and it
    // rides back inside `SettingsSaved` along with what GitHub made of a token
    // just saved — so the save's own answer writes no third type.
    SettingsView::export_all(&config).unwrap();
    SettingsEdit::export_all(&config).unwrap();
    SettingsSaved::export_all(&config).unwrap();

    // Whether there is a newer Verkstead than the one serving the page.
    UpdateNotice::export_all(&config).unwrap();

    // How every one of them refuses. The same shape the agents' half refuses in,
    // so the viewer has one thing to read whichever half answered.
    verkstead_schema::ApiError::export_all(&config).unwrap();

    // And the one thing the viewer is told rather than asked for over HTTP: what
    // moved, which is what decides which of the reads above is worth making
    // again. It hands over nothing itself — see `Nudge`.
    verkstead_schema::Nudge::export_all(&config).unwrap();
}
