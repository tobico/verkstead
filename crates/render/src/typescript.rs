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
    AbandonedRepo, Adopted, AnswerAttached, AnswerAttachmentRemoved, Attached, AttachmentRemoved,
    BacklogPane, BaseBranchChoice, BaseRecorded, BranchRename, BranchRenamed, BriefEdit,
    BriefSaved, Capture, CommitPane, CompanionAdded, CompanionBaseRecorded, CompanionBranchRenamed,
    CompanionModeChoice, CompanionModeChosen, CompanionRemoved, ConversationArchived,
    ConversationClosed, ConversationEntry, ConversationSteered, ConversationStopped,
    ConversationUnarchived, ConversationView, Created, Creation, DirectoryListing, FileDeleted,
    FileDeleting, FileListsView, FileMade, FileMaking, FileReading, FileRenamed, FileRenaming,
    FileRootsView, FileStatusView, FileWrite, FileWritten, FolderListing, GrillingStarted,
    InstallPress, Locked, NewAdoption, NewCompanion, NewConversation, NewOrder,
    NewPullRequestAdoption, OnboardingView, OpenPullRequestRepo, PrefillView, ProfileChoice,
    ProfileChosen, ProfileDeleted, ProfileEdit, ProfileEntry, ProfileSaved, PullRequestDetails,
    PushKey, Registered, Registration, RemoteBanner, RemoteView, RepoChoice, RepoEntry,
    RepoPairingsView, RepoRemoved, RepoSwitched, RepoView, Resolved, Resumed, RoadmapPane,
    RoleChoice, Screen, ServeEdit, ServePress, SetReading, SettingsEdit, SettingsSaved,
    SettingsView, ShareCommented, SharePublished, SharedConversation, ShowArchived,
    ShowingArchived, Shown, Started, SteerCancelled, SteerForm, SteerOpened, SteerSaved,
    SteerSubmission, Submitted, Subscribed, Subscription, TakenUp, TerminalClosed, TerminalOpened,
    TerminalsView, TranscriptView, Unsubscribe, UpdateNotice, Watching,
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

    // And making one, which is the other way a Repo arrives: a parent and a name
    // in, and the whole opened Repo or the reason there is none back.
    Creation::export_all(&config).unwrap();
    Created::export_all(&config).unwrap();

    // The workbench: the sidebar, one Conversation with its Timeline, and the
    // three things the human changes about a drafting one. Each edit brings its
    // own request shape and its own named outcome, because each of them is a
    // different sentence to put in front of the human.
    ConversationEntry::export_all(&config).unwrap();

    // And what is offered beside the sidebar: the Repos holding roadmaps
    // nothing is driving, which writes the roadmap inside it.
    AbandonedRepo::export_all(&config).unwrap();

    // And the other thing offered there: the open pull requests Verkstead did
    // not open, grouped by the Repo each was read in, which writes the pull
    // request inside it.
    OpenPullRequestRepo::export_all(&config).unwrap();
    ConversationView::export_all(&config).unwrap();
    NewConversation::export_all(&config).unwrap();

    // And the same Conversation as a share carries it: the curated record, and
    // when the share was taken. The share is drawn by the workbench's own
    // components, so what a shared file boots from is this around a
    // `ConversationView` — see [`crate::shared`].
    SharedConversation::export_all(&config).unwrap();

    // And what became of publishing one somewhere a link reaches — see
    // [`crate::SharePublished`], which is the one press on this page that writes
    // to GitHub and so the one with refusals about a token.
    SharePublished::export_all(&config).unwrap();

    // And what became of the press that does the lot: publishes one and leaves the
    // link on every pull request the Conversation holds — see
    // [`crate::ShareCommented`], which carries the publish's own refusals rather
    // than saying them over again.
    ShareCommented::export_all(&config).unwrap();

    // And the order the human dragged that sidebar into, which is the one thing
    // they say about the list itself rather than about anything on it.
    NewOrder::export_all(&config).unwrap();

    // And the other: whether what has been put away is drawn among them, which
    // is read back with whether there is anything put away at all and written
    // back as the position alone.
    ShowingArchived::export_all(&config).unwrap();
    ShowArchived::export_all(&config).unwrap();

    // And starting one to adopt a roadmap with, which is the other way in — the
    // Conversation it starts comes back inside the view above.
    NewAdoption::export_all(&config).unwrap();

    // And starting one to wrap a pull request up with, which is the same way in
    // over the other kind of thing to take up.
    NewPullRequestAdoption::export_all(&config).unwrap();
    Started::export_all(&config).unwrap();
    BriefEdit::export_all(&config).unwrap();
    BriefSaved::export_all(&config).unwrap();
    BranchRename::export_all(&config).unwrap();
    BranchRenamed::export_all(&config).unwrap();
    BaseBranchChoice::export_all(&config).unwrap();
    BaseRecorded::export_all(&config).unwrap();

    // And which Repo the work is on at all, which is the first of the four and
    // the one the other three are facts about — the human's to change for as
    // long as nothing has been checked out.
    RepoChoice::export_all(&config).unwrap();
    RepoSwitched::export_all(&config).unwrap();

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

    // And the files the human handed it, which are attached and taken off on the
    // same box. What one *is* comes back inside the view above — and inside the
    // attaching's own outcome, which is the one press here that answers with the
    // record it just made rather than with a word.
    Attached::export_all(&config).unwrap();
    AttachmentRemoved::export_all(&config).unwrap();

    // And the same two presses on the answer sheet, which put a file on an
    // Answer instead: the same record back, and refusals of their own — what
    // fixes an Answer's files is the Set settling rather than the Brief.
    AnswerAttached::export_all(&config).unwrap();
    AnswerAttachmentRemoved::export_all(&config).unwrap();

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

    // And the press beside it that takes up a pull request, which is the same
    // press over the other kind of thing a Draft holds — and refused for
    // reasons of its own, a branch that is already there being the point rather
    // than the trouble.
    TakenUp::export_all(&config).unwrap();

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
    Watching::export_all(&config).unwrap();

    // And the terminals a Conversation holds of its own: which of them are
    // live and whether anything is running in each, what became of asking for
    // another, and what became of closing one (ADR 0013, and ADR 0019 for the
    // busy flag). A shell in the Conversation's Sandbox is not a record, so a
    // number and that flag are the whole of what there is to send — what is
    // *on* one arrives down the socket above, in the same two shapes a Screen
    // is watched in.
    TerminalsView::export_all(&config).unwrap();
    TerminalOpened::export_all(&config).unwrap();
    TerminalClosed::export_all(&config).unwrap();

    // And the other half of Code: the Worktrees its tree stands on, and one
    // folder of one of them. The roots are what bounds the files API — the
    // Conversation's own checkout and each companion's, a read-only one marked
    // — and a folder is read when it is expanded and never walked (ADR 0019,
    // *The tree*). The listing writes its entries with it.
    FileRootsView::export_all(&config).unwrap();
    FolderListing::export_all(&config).unwrap();

    // And every root's files at once, which is what the quick-open palette
    // matches over: git's own list of what each root holds, read afresh every
    // time the palette opens and capped per root (ADR 0019, *The tree*). The
    // lists write the root each of them belongs to with them.
    FileListsView::export_all(&config).unwrap();

    // And git's account of every one of those roots, folded into the marks the
    // tree draws on its rows: one status read per root, a folder wearing the
    // strongest mark of anything under it (ADR 0019, *The tree*). The marks and
    // the two words a mark can be are written with it.
    FileStatusView::export_all(&config).unwrap();

    // And one file of one of those folders, opened: what kind of thing it
    // turned out to be, the version a write will name itself as being over, and
    // the same refusals said about a file (ADR 0019, *Versioned reads, and a
    // stale write is refused*).
    FileReading::export_all(&config).unwrap();

    // And one written back: the text over the version the read handed over, and
    // what became of it. A write over a version that has moved is refused with
    // the version the disk has now, which is what draws the Reload / Keep mine
    // bar in front of the human.
    FileWrite::export_all(&config).unwrap();
    FileWritten::export_all(&config).unwrap();

    // And one made out of a row's own menu: an empty file or a folder, named in
    // full under a folder of a root (ADR 0019, *The tree*). The write's
    // refusals said about a path that is not there yet, with the one that is a
    // making's alone — a name already taken.
    FileMaking::export_all(&config).unwrap();
    FileMade::export_all(&config).unwrap();

    // And one renamed out of that same menu: the path it is at and the name it
    // is to have — a name rather than a path, which is what keeps the move
    // inside the root it started in. The making's refusals said about something
    // that is there, with the one that is a rename's alone — a root, which is a
    // Worktree rather than anything in one.
    FileRenaming::export_all(&config).unwrap();
    FileRenamed::export_all(&config).unwrap();

    // And one taken away out of it: the path and no more, a folder going with
    // everything under it. The rename's refusals less the one about a name,
    // there being no name in the request — and the confirm in front of the
    // press is the viewer's, the way the app asks about anything that cannot
    // be taken back.
    FileDeleting::export_all(&config).unwrap();
    FileDeleted::export_all(&config).unwrap();

    // And what a session committed: the summary drawn out, and the diff, both of
    // which are this payload's alone — the Timeline's own card is the subject
    // and the counts. The diff goes through the same renderer an attached Diff
    // does, which is why this writes no new Diff types.
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

    // And the press that sends a finished Conversation back to a wrap-up over a
    // pull request that has stopped merging. No request shape either: which
    // Conversation it is is the whole of what it says, and what comes back is
    // whether a resolution is going.
    Resolved::export_all(&config).unwrap();

    // And the two presses that stop it, which take no request shape either and
    // answer with one outcome between them: the run is stopping, or the named
    // reason there was nothing to stop.
    ConversationStopped::export_all(&config).unwrap();

    // And the four presses that steer it. The first takes no request shape —
    // it stops the drive, writes the pending steer and reports what it found
    // running — and the last carries what the form settled; cancelling takes
    // none either, the pending steer being the whole of what it names. The
    // Steer's own Event rides on the `ConversationView` above, beside the move
    // it wrote, and so does the pending steer it became.
    SteerOpened::export_all(&config).unwrap();
    SteerCancelled::export_all(&config).unwrap();
    SteerSubmission::export_all(&config).unwrap();
    ConversationSteered::export_all(&config).unwrap();

    // And the save the form makes of itself as it is typed, which is the
    // fourth: the whole form in one body, as the `ConversationView` above hands
    // it back to be prefilled from.
    SteerForm::export_all(&config).unwrap();
    SteerSaved::export_all(&config).unwrap();

    // The Agent Profiles a session can be run under, the one shape saving and
    // rewriting one both take, and the choices a Conversation makes of them —
    // a Pairing, or the row that runs no session where the role offers one.
    // A `ProfileEntry` writes the account it carries — the agent type and that
    // type's own fields, flat — and the broken-ness beside it.
    ProfileEntry::export_all(&config).unwrap();
    ProfileEdit::export_all(&config).unwrap();
    ProfileSaved::export_all(&config).unwrap();
    ProfileDeleted::export_all(&config).unwrap();
    ProfileChoice::export_all(&config).unwrap();
    ProfileChosen::export_all(&config).unwrap();
    RoleChoice::export_all(&config).unwrap();

    // And what a Repo remembers of the three, which is what the compose page
    // fills its own pickers from before there is a Conversation for the
    // server to have prefilled.
    RepoPairingsView::export_all(&config).unwrap();

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

    // And what a path field browses with: one directory of the filesystem,
    // asked for by path and nothing else. The listing writes the entries and
    // their kinds with it.
    DirectoryListing::export_all(&config).unwrap();

    // Whether there is a newer Verkstead than the one serving the page.
    UpdateNotice::export_all(&config).unwrap();

    // And what this machine's Tailscale is doing, which is the Remote access
    // section's whole read: nothing stored, and nothing this page saves — the
    // serve state it carries writes the state a phone is reached through with
    // it.
    RemoteView::export_all(&config).unwrap();

    // And whether the human is done with the banner that points at that
    // section, which is the one thing about it that is stored: read back off
    // the server on every load, so a dismissal made anywhere holds everywhere.
    RemoteBanner::export_all(&config).unwrap();

    // And the one thing on that section that is pressed rather than read: the
    // serve switch. What a press takes in, and the three answers it comes back
    // with — the machine read again, the operator grant it wants first, or what
    // went wrong in the machine's own words.
    ServeEdit::export_all(&config).unwrap();
    ServePress::export_all(&config).unwrap();

    // And whether a fresh Verkstead can do anything yet: the mode the wizard
    // runs in, the machine it is standing on, and what is missing from it. It
    // writes the rows and the three steps' met-ness with it, and the install run
    // where one is going.
    OnboardingView::export_all(&config).unwrap();

    // And the one thing that step is pressed for: the rows that were ticked,
    // which is what the run installs. The reading above already carries what
    // comes back of it.
    InstallPress::export_all(&config).unwrap();

    // And what that machine can offer the wizard's last step, which is a read
    // of its own: the git author it commits as and a GitHub token it is
    // already holding, each labelled with where it was found.
    PrefillView::export_all(&config).unwrap();

    // How every one of them refuses. The same shape the agents' half refuses in,
    // so the viewer has one thing to read whichever half answered.
    verkstead_schema::ApiError::export_all(&config).unwrap();

    // And the one thing the viewer is told rather than asked for over HTTP: what
    // moved, which is what decides which of the reads above is worth making
    // again. It hands over nothing itself — see `Nudge`.
    verkstead_schema::Nudge::export_all(&config).unwrap();
}
