//! Talking to `/api/ui/` — the viewer's own half of the server.
//!
//! Every payload's type comes from [`./types`], which `cargo test` writes out
//! of the Rust the server fills them in from. Nothing here declares a shape of
//! its own: a hand-written interface is a second opinion about the wire, and
//! the whole point of generating them is that there is only ever one.
//!
//! **And every call says which device it is for.** A Conversation may live on a
//! member of this device's cluster, and it is reached through this one: the
//! browser asks the origin it already has, this device puts the call to the
//! member, and the answer comes back untouched (ADR-0020, *The opened device
//! relays*). So everything a member serves takes a `Device` as its first
//! argument — `null` being this device itself — and [`on`] below is the one
//! place a path learns about it, so that no caller anywhere composes one.

import type {
  AbandonedRepo,
  Adopted,
  AnswerAttached,
  AnswerAttachmentRemoved,
  ApiError,
  AskingDevice,
  Attached,
  AttachmentRemoved,
  BacklogPane,
  Locked,
  BaseRecorded,
  BranchRenamed,
  BriefSaved,
  Capture,
  CommitPane,
  CompanionAdded,
  CompanionBaseRecorded,
  CompanionBranchRenamed,
  CompanionMode,
  CompanionModeChosen,
  CompanionRemoved,
  ConversationArchived,
  ConversationClosed,
  ConversationEntry,
  ConversationSteered,
  ConversationStopped,
  ConversationUnarchived,
  ConversationView,
  Created,
  Dependency,
  DevicesView,
  DirectoryListing,
  DiscoveredDevice,
  FileDeleted,
  FileDeleting,
  FileListsView,
  FileMade,
  FileMaking,
  FileReading,
  FileRenamed,
  FileRenaming,
  FileRootsView,
  FileStatusView,
  FileWrite,
  FileWritten,
  FolderListing,
  GrillingStarted,
  OnboardingView,
  OpenPullRequestRepo,
  PrefillView,
  ProfileChoice,
  ProfileChosen,
  ProfileDeleted,
  ProfileEdit,
  ProfileEntry,
  ProfileSaved,
  PullRequestDetails,
  PushKey,
  Registered,
  RemoteBanner,
  RemoteView,
  RepoEntry,
  RepoPairingsView,
  RepoRemoved,
  RepoSwitched,
  Resolved,
  Response as Decided,
  Resumed,
  RoleChoice,
  RoadmapPane,
  Screen,
  ServeEdit,
  ServePress,
  SetReading,
  SettingsEdit,
  SettingsSaved,
  SettingsView,
  ShareCommented,
  SharePublished,
  ShowingArchived,
  Started,
  SteerCancelled,
  SteerForm,
  SteerOpened,
  SteerSaved,
  SteerSubmission,
  Submitted,
  Subscribed,
  Subscription,
  TakenUp,
  TerminalClosed,
  TerminalOpened,
  TerminalsView,
  TranscriptView,
  UpdateNotice,
} from "./types";
import type { Device } from "../reaching";

/// The prefix a call for a member stands under, and the namespace it takes the
/// place of. The server's own two constants — see `relaying.rs`, which is the
/// device in the middle — said again here because this is the end that writes
/// them.
const MEMBERS = "/api/ui/members/";
const NAMESPACE = "/api/ui";

/// Where a call stands: the path itself on this device, and the same path under
/// the member's prefix for another's.
///
/// **The one place `/api/ui/…` becomes `/api/ui/members/{device}/…`.** The
/// prefix takes the place of `/api/ui`, so the far end sees the path the browser
/// would have written locally, and nothing of the hop shows at either end of it.
///
/// Every path in this module starts at that namespace, which is what makes the
/// swap a swap rather than a join: there is no call of the viewer's that is not
/// one a member could serve.
function on(device: Device, path: string): string {
  return device === null
    ? path
    : `${MEMBERS}${encodeURIComponent(device)}${path.slice(NAMESPACE.length)}`;
}

/// Whether a path is one this device puts to a member rather than answers
/// itself — which is the one thing [`retrying`] needs of it, and is a fact
/// about the path [`on`] wrote.
function relayed(path: string): boolean {
  return path.startsWith(MEMBERS);
}

/// A refusal from the server, in the shape both halves refuse in.
///
/// Carries the server's own wording rather than a status code, because that
/// wording is what the page has to show the human — see the `error` field on
/// `ApiError`.
export class RefusedError extends Error {
  readonly status: number;
  readonly violations: NonNullable<ApiError["violations"]>;

  /// And whether it came back from a call put to a member, which is what
  /// [`retrying`] reads: a refusal made on the way to another machine costs
  /// something to make again that a local one does not.
  readonly relayed: boolean;

  constructor(status: number, refusal: ApiError, relayed = false) {
    super(refusal.error);
    this.name = "RefusedError";
    this.status = status;
    this.violations = refusal.violations ?? [];
    this.relayed = relayed;
  }
}

/// Whether a read failed because it ran past a deadline of its own.
///
/// `AbortSignal.timeout` aborts with a `TimeoutError`, which is how a read that
/// gave up is told apart from one the server refused.
export function timedOut(error: unknown): boolean {
  return error instanceof DOMException && error.name === "TimeoutError";
}

/// How many times a read that failed is made again — the count every query in
/// the app is retried by, said here because the rule beside it is.
const RETRIES = 3;

/// The statuses a relayed call is refused with that are a verdict rather than
/// a bad moment — see [`retrying`], which is where they are read.
///
/// `502` is the hop's own and nothing else's: it is what this device answers
/// when a member answered at none of the addresses it advertised, and no route
/// in the namespace at the far end ever mints one. `400` and `404` are the two
/// a Device Id earns before anything is dialled — this device's own id, and an
/// id that is no member's — and are the far end's own refusals besides, a
/// Conversation it has no record of among them. None of the three is worth a
/// second go: the first is a machine that is not there, and the other two are
/// answers.
const FINAL_FROM_A_MEMBER = [400, 404, 502];

/// Whether a read that failed is worth making again. The app's retry rule, set
/// as the query client's default in `App.tsx`.
///
/// The ordinary three attempts, minus the two cases where trying again is worse
/// than not.
///
/// **A read that gave up on its own deadline.** Retrying that is thirty seconds
/// of nothing, three more times, before the page is allowed to say anything —
/// and what it would say is what it already knew. The error is the answer, and
/// on the Conversation pane the error is what draws the way out.
///
/// **And a member refusing by name** (ADR-0020, *The opened device relays*). A
/// call put to a member walks every address that device advertised, at two
/// seconds apiece, and marks its row unreachable when none of them answers — so
/// three more goes at a machine that is switched off is half a minute of blank
/// page and four passes down the same dead list, to arrive at the sentence the
/// first refusal already carried. A local refusal is still retried: it costs a
/// round trip on the loopback, and the rule here is about what a second attempt
/// costs rather than about how likely it is to help.
export function retrying(attempts: number, error: unknown): boolean {
  if (timedOut(error)) {
    return false;
  }

  if (
    error instanceof RefusedError &&
    error.relayed &&
    FINAL_FROM_A_MEMBER.includes(error.status)
  ) {
    return false;
  }

  return attempts < RETRIES;
}

/// One Set, rendered, with where it stands — or the stored body where this
/// build cannot read it, which is a page to draw rather than a failure.
///
/// The id is whatever the URL held, unparsed: one that is not a number cannot
/// name a Set, and the server answers for that the same way it answers for one
/// that names no Set — a 404, which the page reads as "there isn't one".
export function loadSet(device: Device, id: string): Promise<SetReading> {
  return get<SetReading>(on(device, `/api/ui/sets/${encodeURIComponent(id)}`));
}

/// Answer a Set, which ends the wait the agent is holding on it.
///
/// The outcome is the answer's body rather than its status: every one of them —
/// taken, already answered, locked, refused by the grammar — is something the
/// page has to say in words, and only a server that could not answer at all
/// throws.
export function submitResponse(
  device: Device,
  id: number,
  response: Decided,
): Promise<Submitted> {
  return post<Submitted>(on(device, `/api/ui/sets/${id}/response`), response);
}

/// Close a Set unanswered: the human declaring that nobody is ever going to
/// answer it. There is nothing to send but the Set's own id, which is in the
/// path.
export function lockSet(device: Device, id: number): Promise<Locked> {
  return post<Locked>(on(device, `/api/ui/sets/${id}/lock`));
}

/// The Repos Verkstead has been told about, by name.
export function listRepos(device: Device): Promise<RepoEntry[]> {
  return get<RepoEntry[]>(on(device, "/api/ui/repos"));
}

/// Every branch of one registered Repo, local and remote-tracking both — which
/// is what a drafting Conversation picks the one it comes off out of.
///
/// Read out of git by the server every time it is asked: branches move without
/// Verkstead hearing about it, so there is nothing here that could be kept.
export function listBranches(
  device: Device,
  repoId: number,
): Promise<string[]> {
  return get<string[]>(on(device, `/api/ui/repos/${repoId}/branches`));
}

/// What one registered Repo was last grilled with, judged as something to fill
/// a picker with: the three roles a Conversation started on it would arrive
/// showing. A Repo nothing has grilled answers too, with what the server
/// prefills in place of a memory — the last start anywhere, then the platform
/// default.
///
/// For the page that asks those three questions before there is a Conversation
/// to read the answers off. Read again whenever the repo changes, because the
/// memory is the repo's: another repo is another answer.
export function loadRepoPairings(
  device: Device,
  repoId: number,
): Promise<RepoPairingsView> {
  return get<RepoPairingsView>(on(device, `/api/ui/repos/${repoId}/pairings`));
}

/// What one directory holds, for the dropdown a path field browses with.
///
/// One directory per ask and never a walk: the field asks again for every level
/// somebody drills into, so a browse costs one reading of one directory however
/// much is under it.
///
/// No path at all is the field standing empty, which the server answers with its
/// own home — a starting point rather than a ceiling, the way back out of it
/// being a listing like any other. Every refusal is in the body rather than in
/// the status, the way registering a Repo refuses: a path that is relative,
/// missing, not a directory or unreadable is a line the dropdown draws where its
/// rows would be, and most of those are the ordinary state of a field halfway
/// through being typed into.
export function listDirectory(
  device: Device,
  path: string | null,
): Promise<DirectoryListing> {
  const asking = new URLSearchParams();

  if (path !== null) {
    asking.set("path", path);
  }

  return get<DirectoryListing>(on(device, `/api/ui/directories?${asking}`));
}

/// Ask Verkstead to take on the repository at an absolute path.
///
/// Like answering a Set, the outcome is the answer's body rather than its
/// status: a path that names no repository is the server reading the path
/// rather than failing to, and every refusal is a different sentence to put in
/// front of the human.
export function registerRepo(
  device: Device,
  path: string,
): Promise<Registered> {
  return post<Registered>(on(device, "/api/ui/repos"), { path });
}

/// Ask Verkstead to *make* a repository under `parent` and take it on.
///
/// The other way a Repo arrives, and the one that ends in the same
/// registration: a directory of that name, `git init` onto `main`, a `README.md`
/// committed as the configured author, and the opened Repo back.
///
/// Two fields rather than a joined path, because the two halves are answered
/// differently — the parent is browsed for and the name is typed — and joining
/// them here would be the one place a path is built out of a separator the
/// server never agreed to. Every refusal is in the body for the registration's
/// reason: each is a different sentence to put in front of the human, and none
/// of them is something to retry.
///
/// `github` is the modal's tick: the same repository on GitHub, private, with
/// `origin` written and `main` pushed. False wherever no token is saved, there
/// being nothing to make it as — and a GitHub failure after the local repository
/// exists comes back as the Repo *and* the reason rather than as either.
export function createRepo(
  device: Device,
  parent: string,
  name: string,
  github: boolean,
): Promise<Created> {
  return post<Created>(
    on(device, "/api/ui/repos/new"),
    { parent, name, github },
  );
}

/// Take one off the registry, which is an unregistering rather than a delete:
/// every Conversation ever started on it goes on naming it, and what changes is
/// what is offered for new work. There is nothing to send but its own id, which
/// is in the path.
///
/// The outcome is the answer's body for the reason a registration's is: a Repo
/// with live work on it is refused for a reason worth saying, and not a failure.
export function removeRepo(id: number): Promise<RepoRemoved> {
  return post<RepoRemoved>(`/api/ui/repos/${id}/remove`);
}

/// The registered Repos holding roadmaps nothing is driving.
///
/// Read again every time the sidebar is, because the server reads it again
/// every time it is asked: the boxes and the branches are the repositories'
/// own answer, and a roadmap somebody has picked up since simply stops being
/// on the list.
export function listAbandonedRoadmaps(): Promise<AbandonedRepo[]> {
  return get<AbandonedRepo[]>("/api/ui/abandoned-roadmaps");
}

/// And the pull requests open in those Repos, each saying which Conversation
/// already holds it.
///
/// One request for every registered Repo, because the server asks them in
/// parallel and the browser waiting on six of them in turn would be six round
/// trips to say what one can.
///
/// Slower than everything else this page reads — a `gh` per Repo, each of them
/// a call to GitHub — so whatever draws it has something to show while it is on
/// its way. A Repo that could not be asked is simply not in the answer: this
/// never refuses, and an empty list means *nothing to wrap up here* rather than
/// *something went wrong*.
export function listOpenPullRequests(): Promise<OpenPullRequestRepo[]> {
  return get<OpenPullRequestRepo[]>("/api/ui/open-pull-requests");
}

/// Start a Conversation to adopt one of those roadmaps with.
///
/// What clicking a roadmap in the notice does. The stage is not sent: which one
/// is next is the roadmap's own answer at whatever commit the Conversation ends
/// up branching from, and the page reads it back there.
export function startAdoption(
  repoId: number,
  roadmap: string,
  base: string,
): Promise<Started> {
  return post<Started>("/api/ui/adoptions", {
    repo_id: repoId,
    roadmap,
    // The branch the roadmap was found on, so the new Conversation starts
    // fixed to it. Empty is the default branch, which is what no base means.
    base: base || null,
  });
}

/// And one to wrap one of those pull requests up with.
///
/// The whole row goes rather than its number, unlike the roadmap above: a
/// roadmap is a document in the Conversation's own repository and is read back
/// off it, where a pull request is GitHub's — and a server handed only a number
/// would have to make a `gh` call of its own to name what the human is already
/// looking at.
///
/// Nothing is checked out by this. It records and opens, and the take-up on the
/// page it lands on is what touches git.
export function startPullRequestAdoption(
  repoId: number,
  pull: {
    number: number;
    title: string;
    url: string;
    head: string;
    base: string;
  },
): Promise<Started> {
  return post<Started>("/api/ui/pull-request-adoptions", {
    repo_id: repoId,
    number: pull.number,
    title: pull.title,
    url: pull.url,
    head: pull.head,
    base: pull.base,
  });
}

/// The Conversations in the sidebar, in the order the human put them in.
export function listConversations(): Promise<ConversationEntry[]> {
  return get<ConversationEntry[]>("/api/ui/conversations");
}

/// Say where the whole list goes, which is what letting go of a dragged row
/// does.
///
/// The whole order rather than the row that moved: the list the human is
/// looking at is what they meant, and a move replayed against a list the server
/// has added to since would not be it.
///
/// Answered with nothing at all — there is no outcome to read. An id naming a
/// Conversation that has gone is passed over on the other side, which is what a
/// list drawn a moment ago is allowed to carry.
export async function placeConversations(order: number[]): Promise<void> {
  const at = "/api/ui/conversations/order";

  await refused(at, await sent(at, { order }));
}

/// Whether the sidebar is drawing what has been archived, and whether there is
/// anything archived for it to draw.
///
/// The server's answer rather than this device's, because the choice is the
/// human's rather than the browser's: a toggle kept here would be one they had
/// to find again on their phone.
///
/// Both halves rather than the position alone: the list is filtered by the
/// switch on the server, so a page looking at an empty one cannot tell nothing
/// archived from everything archived and hidden. See `workbench/zero.ts`, which
/// is what wants the difference.
export function showingArchived(): Promise<ShowingArchived> {
  return get<ShowingArchived>("/api/ui/conversations/archived");
}

/// And put that switch where they have just put it.
///
/// The position rather than a flip, so what is sent is what the human is
/// looking at. Answered with nothing at all, as the order is.
export async function showArchived(showing: boolean): Promise<void> {
  const at = "/api/ui/conversations/archived";

  await refused(at, await sent(at, { showing }));
}

/// How long this read is given before the browser gives up on it.
///
/// The one read here with a deadline, because it is the one that can hang. The
/// server does a good deal behind it — a Timeline, two Pairings, a look at
/// every checkout on disk, and on an adopting Conversation a `git fetch` — and
/// any of it stalling leaves the pane at *Loading…* for ever, which is worse
/// than any answer: a page that never resolves is one the human cannot even
/// reach the menu on to close the Conversation.
///
/// The server has a deadline of its own on the fetch, which is the hang anybody
/// has actually met. This is the net under the ones nobody has met yet, so it
/// is generous: thirty seconds is far past a read that is merely slow, and a
/// long way short of for ever. What it turns a hang into is an error, and the
/// error is what draws the escape hatch — see `workbench/Hatch.tsx`.
const READING_A_CONVERSATION = 30_000;

/// One Conversation with its Timeline.
///
/// The id is whatever the URL held, unparsed, as a Set's is: one that is not a
/// number cannot name a Conversation, and the server answers for that the way it
/// answers for one that names none.
export function loadConversation(
  device: Device,
  id: string,
): Promise<ConversationView> {
  return get<ConversationView>(
    on(device, `/api/ui/conversations/${encodeURIComponent(id)}`),
    READING_A_CONVERSATION,
  );
}

/// And where the same Conversation stands as a file to send somebody: the share
/// build of the viewer with this record inside it.
///
/// A path rather than a fetch, which is the one thing in this module that is
/// not a request. What the human asked for is a file in their downloads, and a
/// link is the whole of how a browser does that: the server names it and says
/// it is an attachment, so nothing here has to hold a megabyte of HTML in
/// memory to hand it straight back to the page it came from.
export function sharePath(device: Device, id: number): string {
  return on(device, `/api/ui/conversations/${id}/share`);
}

/// And the same file put where a link reaches it: one press builds the share
/// and publishes it as a secret gist.
///
/// A request rather than a path, unlike the download above, because what comes
/// back is where it went — and because it costs something: a gist is made in
/// somebody's account, and every way that can be refused has a name.
export function publishShare(
  device: Device,
  id: number,
): Promise<SharePublished> {
  return post<SharePublished>(
    on(device, `/api/ui/conversations/${id}/share/publish`), {},
  );
}

/// And the whole of it in one press: the same publish, and a comment carrying
/// the link on every pull request the conversation is on.
///
/// One request rather than a publish followed by a comment per pull request,
/// because it is one intention — the human is handing the record to whoever is
/// reviewing the work, and where that is is the server's to know. What comes
/// back says how far it got: where each comment landed, and which pull request
/// missed out.
export function shareToPullRequests(
  device: Device,
  id: number,
): Promise<ShareCommented> {
  return post<ShareCommented>(
    on(device, `/api/ui/conversations/${id}/share/comment`), {},
  );
}

/// What one session printed, whole.
///
/// Fetched by the pane that shows it rather than carried by the Conversation: a
/// session prints megabytes over an hour, and the Timeline is read again every
/// time this page hears the world moved.
export function loadCapture(
  device: Device,
  id: number,
  event: number,
): Promise<Capture> {
  return get<Capture>(
    on(device, `/api/ui/conversations/${id}/capture/${event}`),
  );
}

/// And what it said, as a conversation.
///
/// The same Event read the other way: the Capture is how the session looked and
/// this is what it was saying, parsed and rendered by the server out of the
/// lines its own backend wrote. A session that left no such record comes back
/// with nothing in it, which is what sends the pane to the Capture.
///
/// `after` is the cursor a previous reading ended at, and asks for only what the
/// session has said since — which is what an open pane wants on every one of the
/// Nudges a running session sends it, rather than the hour of talking it already
/// has. The cursor is the server's own and means nothing here: what a reader does
/// with one is hand it back. Reading without one reads the record whole, and so
/// does the server whenever it cannot carry on from the one it was given — which
/// is why what comes back says which of the two it is.
export function loadTranscript(
  device: Device,
  id: number,
  event: number,
  after?: string,
): Promise<TranscriptView> {
  const from = after === undefined ? "" : `?after=${encodeURIComponent(after)}`;

  return get<TranscriptView>(
    on(device, `/api/ui/conversations/${id}/transcript/${event}${from}`),
  );
}

/// And how it looked, which is the same Event read the third way.
///
/// The grid those bytes leave on a terminal, as the escape sequences that would
/// paint it — the server holds the terminal that decided them, and this is a
/// repaint to feed the one in the pane (ADR 0007). A session that has ended
/// repaints to the screen it last stood on.
export function loadScreen(
  device: Device,
  id: number,
  event: number,
): Promise<Screen> {
  return get<Screen>(on(device, `/api/ui/conversations/${id}/screen/${event}`));
}

/// And where to watch a session that is still running: a socket rather than a
/// fetch, which is the one shape in the app where the viewer is sent something
/// rather than asking for it — this and a conversation terminal below.
///
/// A repaint on connect and what the session prints after it, with the size of
/// the window it is being watched in — and whatever is typed into it — going
/// back the other way. Everything else here stays on SSE and a refetch — a
/// terminal being drawn is the one thing neither of those is any good for.
export function screenSocket(
  device: Device,
  id: number,
  event: number,
): string {
  return socketAt(
    on(device, `/api/ui/conversations/${id}/screen/${event}/attach`),
  );
}

/// Where one of our sockets stands, whichever of them it is — the Screen's, and
/// a conversation terminal's below.
///
/// Built off the page's own origin, so it goes wherever the page came from: the
/// dev server proxying `/api`, or the one binary serving both.
function socketAt(path: string): string {
  const at = new URL(path, window.location.href);

  at.protocol = at.protocol === "https:" ? "wss:" : "ws:";

  return at.href;
}

/// Which of a conversation's terminals are live: a human's shell inside its
/// sandbox, in the same machinery a session's Screen is watched through
/// (ADR 0013).
///
/// A number and whether anything is running in it, which is the whole of what
/// there is to say about one from out here — a terminal is memory on the server
/// rather than a record, so what is *on* each of them arrives down the socket
/// below.
export function listTerminals(
  device: Device,
  id: number,
): Promise<TerminalsView> {
  return get<TerminalsView>(
    on(device, `/api/ui/conversations/${id}/terminals`),
  );
}

/// And opening another, which answers the number it will answer to.
///
/// A request rather than a path, and named refusals rather than a status: the
/// shell is started in the conversation's sandbox, and every way that can be
/// refused is a sentence the pane has to say.
export function openTerminal(
  device: Device,
  id: number,
): Promise<TerminalOpened> {
  return post<TerminalOpened>(
    on(device, `/api/ui/conversations/${id}/terminals`),
  );
}

/// And where to watch one, which is the Screen's own socket pointed at a shell:
/// a repaint on connect, what it prints after that, and the window size and
/// whatever is typed going back the other way.
export function terminalSocket(
  device: Device,
  id: number,
  number: number,
): string {
  return socketAt(
    on(device, `/api/ui/conversations/${id}/terminals/${number}/attach`),
  );
}

/// And closing one, which is the × at the end of its tab: the shell is hung up
/// and then killed where it lingers, and the terminal comes off the server's
/// register.
///
/// Taking the thing away rather than posting about it — the one delete in the
/// app, because a terminal is something the server is holding rather than a
/// record it keeps.
///
/// **Two-part where somebody is working in it.** The server answers `Busy` and
/// leaves the shell running when its foreground is something other than the
/// shell itself, which is what the pane asks the human about; `asked` is that
/// answer coming back, and the close it carries is made whatever is running
/// (ADR 0019). Read rather than ignored for that reason: the answer is the
/// question.
export async function closeTerminal(
  device: Device,
  id: number,
  number: number,
  asked = false,
): Promise<TerminalClosed> {
  const at = on(
    device,
    `/api/ui/conversations/${id}/terminals/${number}${asked ? "?asked=true" : ""}`,
  );

  return taken<TerminalClosed>(
    at,
    await fetch(at, {
      method: "DELETE",
      headers: { accept: "application/json" },
    }),
  );
}

/// The worktrees Code's tree is drawn over: the conversation's own first, then
/// each companion's, each saying whether anything in it can be written.
///
/// What bounds the files API, rather than a list for the eye: the server reads
/// and writes the worktrees as itself, with no sandbox in front of it, and a
/// path under none of these is refused (ADR 0019).
export function listFileRoots(
  device: Device,
  id: number,
): Promise<FileRootsView> {
  return get<FileRootsView>(
    on(device, `/api/ui/conversations/${id}/files/roots`),
  );
}

/// And where a Code pane says it is drawn: a socket it holds open for as long
/// as it is, which is what runs the server's watcher over those worktrees
/// (ADR 0019, *Following the disk*).
///
/// Nothing travels either way. What the pane hears about the disk is a `files`
/// Nudge down the stream every other change comes down; what this is for is
/// being *open*, the way a terminal tab's attach is — it dies with the tab
/// whatever becomes of the browser, so a laptop shut mid-edit stops the watcher
/// without anybody having to notice.
export function filesSocket(device: Device, id: number): string {
  return socketAt(
    on(device, `/api/ui/conversations/${id}/files/attach`),
  );
}

/// And what one folder of one of them holds.
///
/// One folder per ask and never a walk — the shape a path field browses with,
/// and why the tree asks again for every level somebody expands. Git-ignored
/// paths and `.git` are not in the answer.
///
/// **And why a `files` Nudge is cheap to follow.** The tree re-reads the
/// folders it has expanded and nothing else, which is one of these apiece — a
/// listing rather than a walk, and none at all for the folders nobody has
/// opened (ADR 0019, *Following the disk*).
///
/// Every refusal is in the body rather than in the status: a path outside every
/// root, a path under `.git`, a worktree that has gone and a folder that has are
/// four different sentences to draw where the rows would be.
export function listFolder(
  device: Device,
  id: number,
  path: string,
): Promise<FolderListing> {
  const asking = new URLSearchParams({ path });

  return get<FolderListing>(
    on(device, `/api/ui/conversations/${id}/files/folder?${asking}`),
  );
}

/// And one file of one of those folders, opened.
///
/// What comes back says which of four kinds of thing it read — text, an image,
/// a binary it will not send, or a file over the size cap — and text carries a
/// version, which is a hash of the bytes and is what a write names itself as
/// being over (ADR 0019).
///
/// **And what the tabs follow the disk with.** The pane reads every open file
/// again on a `files` Nudge and compares that version: where it matches — which
/// is nearly every file on nearly every Nudge — the tab is left exactly as it
/// is, and where it differs a clean buffer takes the new text and a dirty one
/// raises the bar (ADR 0019, *Following the disk*). One of these per open file
/// and none for anything else.
///
/// Refused in the body like the folder beside it: a path outside every root, a
/// path under `.git`, a worktree that has gone and a file that has are each
/// their own sentence rather than a status to retry.
export function readFile(
  device: Device,
  id: number,
  path: string,
): Promise<FileReading> {
  const asking = new URLSearchParams({ path });

  return get<FileReading>(
    on(device, `/api/ui/conversations/${id}/files/file?${asking}`),
  );
}

/// And that file written back, over the version the read handed over.
///
/// The same path the read is at, posted to rather than got: it is the same
/// file, and a read and a write of one thing are what a GET and a POST on one
/// route are for.
///
/// **The version is the whole of it.** A write over a file the agent has
/// changed since is refused rather than landing, which is what the pane draws
/// its Reload / Keep mine bar from (ADR 0019, *Versioned reads, and a stale
/// write is refused*). Last writer wins is what this is not: an agent's edit
/// silently overwritten by a human who never saw it is exactly what the version
/// exists to surface.
///
/// A write that landed answers with the version it made, so the tab need not
/// read the file again to save over it. A refused one answers with nothing to
/// go on, and the presses under the bar read the file: what the viewer knows
/// after a write that landed is what it just sent, and after one that did not
/// it knows nothing at all.
///
/// Refused in the body like the read beside it, a root that takes no writes
/// among them, because each of those is a different sentence for the human.
export function writeFile(
  device: Device,
  id: number,
  path: string,
  version: string,
  text: string,
): Promise<FileWritten> {
  return post<FileWritten>(
    on(device, `/api/ui/conversations/${id}/files/file`),
    {
      path,
      version,
      text,
    } satisfies FileWrite,
  );
}

/// And an empty file made under a folder of one of those roots, which is what a
/// name typed into a row's own field asks for.
///
/// The path in full and nothing else: what is being made is a row of the tree,
/// so what the request says is where that row goes (ADR 0019, *The tree*). The
/// file is empty, and the text that goes into it is a save through the endpoint
/// above.
///
/// Refused in the body like everything else here, with one refusal of its own —
/// a name already taken — and each of them is a sentence drawn beside the field
/// with what was typed still in it.
export function makeFile(
  device: Device,
  id: number,
  path: string,
): Promise<FileMade> {
  return post<FileMade>(
    on(device, `/api/ui/conversations/${id}/files/file/new`),
    {
      path,
    } satisfies FileMaking,
  );
}

/// And a folder made there, which is the same request about the other kind of
/// row.
///
/// Its own endpoint rather than a flag on the one above, because a file and a
/// folder are two different things to make: what a new file does afterwards is
/// open as a tab, and a new folder opens nothing.
export function makeFolder(
  device: Device,
  id: number,
  path: string,
): Promise<FileMade> {
  return post<FileMade>(
    on(device, `/api/ui/conversations/${id}/files/folder/new`),
    {
      path,
    } satisfies FileMaking,
  );
}

/// And one of them renamed: whatever is at a path, given a new name in the
/// folder it is already in.
///
/// **A name rather than a path**, which is the whole of why a rename cannot
/// cross two roots: two roots are two repositories, and a file taken out of one
/// checkout and put in another is not something this pane has a way to ask for
/// (ADR 0019, *The tree*). The field is drawn over the row, what is typed there
/// is a name, and the server joins it onto the folder the row is already in.
///
/// What comes back is where it now is, which is what every open tab of that path
/// follows — a folder renamed carrying everything under it.
///
/// Refused in the body like everything else here, with one refusal of its own
/// beyond the making's: a root, which is a Worktree rather than anything in one.
export function renamePath(
  device: Device,
  id: number,
  path: string,
  name: string,
): Promise<FileRenamed> {
  return post<FileRenamed>(
    on(device, `/api/ui/conversations/${id}/files/rename`),
    {
      path,
      name,
    } satisfies FileRenaming,
  );
}

/// And one of them taken away: whatever is at a path, a folder with everything
/// under it.
///
/// **The confirm is drawn before this is called**, which is the one thing about
/// this endpoint worth saying twice: a deletion cannot be taken back, so the
/// card the app puts up for a busy shell and a dirty tab goes up over the row
/// first, and one confirm covers a folder's whole contents (ADR 0019, *The
/// tree*).
///
/// What comes back says it landed or says why it did not, and either way the
/// folder above the row is read again by the press itself: the row goes the
/// moment the server says it has, rather than when the Nudge the deletion
/// raises comes back round.
/// The open tab of a file that has gone stays, read-only, saying so: see
/// `Code.tsx`, which is where a tab keeps its text after the file under it goes.
export function deletePath(
  device: Device,
  id: number,
  path: string,
): Promise<FileDeleted> {
  return post<FileDeleted>(
    on(device, `/api/ui/conversations/${id}/files/delete`),
    {
      path,
    } satisfies FileDeleting,
  );
}

/// And every root's files at once, which is what the quick-open palette matches
/// over.
///
/// Git's own list per root — what it tracks, plus what it does not track and
/// does not ignore — rather than a walk, and capped per root, a root that was
/// cut short saying so in the answer (ADR 0019, *The tree*).
///
/// **Read afresh every time the palette opens**, which is what makes the list
/// worth reading at all: there is no watcher until the stage after this one,
/// and a list read once would go stale the first time the agent wrote
/// anything.
///
/// Nothing here is refused: what it answers is every path there is to name, so
/// there is no path of anybody's to measure against the roots. A Conversation
/// with no Worktrees answers with no roots, and a root git will not answer
/// about answers with no files.
export function listFiles(device: Device, id: number): Promise<FileListsView> {
  return get<FileListsView>(
    on(device, `/api/ui/conversations/${id}/files/list`),
  );
}

/// And what git says about every one of those roots, folded into the marks the
/// tree draws on its rows.
///
/// One status read per root rather than a call per row, answered for the whole
/// conversation at once — and folded, so that a folder wears the strongest mark
/// of anything under it and the tree draws a row by looking its own path up
/// (ADR 0019, *The tree*).
///
/// **Read again on a `files` Nudge and on a `commit`**, which is the one reading
/// on this wire that two kinds both stand for: a commit made in a terminal
/// beside the tree clears every mark in the worktree without touching a file, so
/// nothing about any folder has moved and every mark has changed.
///
/// Nothing here is refused, for the list's reason: it is about no path anybody
/// named. A conversation with no worktrees answers with no roots, and a root git
/// will not answer about answers with no marks — which is a tree whose rows are
/// drawn unmarked rather than a tree that will not draw.
export function readFileStatus(
  device: Device,
  id: number,
): Promise<FileStatusView> {
  return get<FileStatusView>(
    on(device, `/api/ui/conversations/${id}/files/status`),
  );
}

/// One commit, rendered: what it said about itself, and its diff.
///
/// Fetched by the pane that shows it for the Capture's reason. The diff is read
/// out of the repository by the server rather than out of its database — the
/// commit is in git, which is what a commit is — where the summary was kept by
/// the sweep that recorded the commit.
export function loadCommitPane(
  device: Device,
  id: number,
  event: number,
): Promise<CommitPane> {
  return get<CommitPane>(
    on(device, `/api/ui/conversations/${id}/commit/${event}`),
  );
}

/// The backlog opened: every task document `.tasks/` holds, rendered.
///
/// Named by the conversation alone, unlike the three panes around it. A backlog
/// is read off the worktree rather than remembered, so there is no event to
/// reach it by: there is one backlog per conversation, and this is it.
export function loadBacklogPane(
  device: Device,
  id: number,
): Promise<BacklogPane> {
  return get<BacklogPane>(on(device, `/api/ui/conversations/${id}/backlog`));
}

/// The roadmap opened: every stage brief one of them holds, rendered.
///
/// Named by the roadmap rather than by the conversation, which is where this
/// parts company with the backlog above: a worktree holds one `.tasks/` and may
/// hold any number of roadmaps, so the card that opens this says which of them
/// it is. Encoded, because that name is a directory name out of a repository.
export function loadRoadmapPane(
  device: Device,
  id: number,
  name: string,
): Promise<RoadmapPane> {
  return get<RoadmapPane>(
    on(
      device,
      `/api/ui/conversations/${id}/roadmap/${encodeURIComponent(name)}`,
    ),
  );
}

/// What is on the pull request the finish step opened: its commit list and its
/// comments.
///
/// Fetched by the pane that shows it for a stronger version of the diff's
/// reason: the server reads this by asking GitHub through the host's `gh`, so a
/// conversation that carried it would make an API call every time the page heard
/// anything at all had moved. Fetched here, it is read on a commit landing and
/// on nothing else (ADR-0009). A server that cannot ask refuses with the reason,
/// which is what the pane shows.
export function loadPullRequest(
  device: Device,
  id: number,
  event: number,
): Promise<PullRequestDetails> {
  return get<PullRequestDetails>(
    on(device, `/api/ui/conversations/${id}/pull-request/${event}`),
  );
}

/// Start a Conversation against a registered Repo.
///
/// The branch name is not sent: it is prefilled by the server, because the
/// record is the server's from the moment it exists.
export function startConversation(repoId: number): Promise<Started> {
  return post<Started>("/api/ui/conversations", { repo_id: repoId });
}

/// Put a file on a Conversation for its sessions to read.
///
/// One request per file, the raw bytes as the body and the name in the path —
/// there is one file and one name, so a multipart envelope around them would be
/// a parser on both sides for nothing. The name goes over encoded, which is
/// what carries a name with a separator in it to the server to be refused
/// rather than turning the request into a path that matches no route.
///
/// A body the server would not even read comes back as `TooLarge` like one it
/// read and refused: the route's own limit answers a 413, and the composer has
/// one sentence to say either way — see `Attached::TooLarge`, which is the same
/// refusal named.
export async function attachFile(
  device: Device,
  id: number,
  file: File,
): Promise<Attached> {
  const at = on(
    device,
    `/api/ui/conversations/${id}/attachments/${encodeURIComponent(file.name)}`,
  );

  const response = await fetch(at, {
    method: "POST",
    headers: {
      accept: "application/json",
      "content-type": "application/octet-stream",
    },
    body: file,
  });

  if (response.status === 413) {
    return "TooLarge";
  }

  return taken<Attached>(at, response);
}

/// And take one off again, by the row's own id: two files on one Conversation
/// may share a name, and neither of them is a key.
export function removeAttachment(
  device: Device,
  id: number,
  attachment: number,
): Promise<AttachmentRemoved> {
  return post<AttachmentRemoved>(
    on(device, `/api/ui/conversations/${id}/attachments/${attachment}/remove`),
  );
}

/// Put a file on one of a waiting Set's Answers, under the label of the
/// Question it answers.
///
/// The Brief's upload said from the other page: one request per file, the raw
/// bytes as the body, and the label and the name in the path — both encoded, so
/// a name with a separator in it reaches the server to be refused rather than
/// turning the request into a path that matches no route.
///
/// Addressed by the Set rather than by the Conversation, because that is what
/// the sheet is a page about: where the file goes is the server's to work out.
/// A body it would not even read comes back as `TooLarge` the way the Brief's
/// does, and for the same reason.
export async function attachToAnswer(
  device: Device,
  set: number,
  label: string,
  file: File,
): Promise<AnswerAttached> {
  const at = on(
    device,
    `/api/ui/sets/${set}/answers/${encodeURIComponent(
      label,
    )}/attachments/${encodeURIComponent(file.name)}`,
  );

  const response = await fetch(at, {
    method: "POST",
    headers: {
      accept: "application/json",
      "content-type": "application/octet-stream",
    },
    body: file,
  });

  if (response.status === 413) {
    return "TooLarge";
  }

  return taken<AnswerAttached>(at, response);
}

/// And take one off an Answer again, by the row's own id — which is what the
/// path names it by, under the Set it was put on: two files on one Set may
/// share a name, and neither of them is a key.
export function removeAnswerAttachment(
  device: Device,
  set: number,
  attachment: number,
): Promise<AnswerAttachmentRemoved> {
  return post<AnswerAttachmentRemoved>(
    on(device, `/api/ui/sets/${set}/attachments/${attachment}/remove`),
  );
}

/// Save what the human has written into a Brief.
export function saveBrief(
  device: Device,
  id: number,
  markdown: string,
): Promise<BriefSaved> {
  return post<BriefSaved>(
    on(device, `/api/ui/conversations/${id}/brief`), { markdown },
  );
}

/// Move a drafting Conversation onto another registered Repo.
///
/// Which Repo is the whole of what goes out, the way an added companion is:
/// what follows — the base back on the new repo's rule, and a companion that
/// has just become this Conversation's own Repo going away — is the server's to
/// do rather than this page's to ask for.
export function switchRepo(
  device: Device,
  id: number,
  repoId: number,
): Promise<RepoSwitched> {
  return post<RepoSwitched>(
    on(device, `/api/ui/conversations/${id}/repo`),
    {
      repo_id: repoId,
    },
  );
}
/// Name the branch the work will be done on. Whether git would take the name is
/// the server's to say, so this is another outcome to read rather than a status.
export function renameBranch(
  device: Device,
  id: number,
  branch: string,
): Promise<BranchRenamed> {
  return post<BranchRenamed>(
    on(device, `/api/ui/conversations/${id}/branch`), { branch },
  );
}

/// Choose the branch the work comes off, or pass `null` to put the Conversation
/// back on the default-branch rule.
///
/// The name rather than where it stands: it is resolved when grilling starts, so
/// the work comes off wherever that branch is then.
export function setBaseBranch(
  device: Device,
  id: number,
  branch: string | null,
): Promise<BaseRecorded> {
  return post<BaseRecorded>(
    on(device, `/api/ui/conversations/${id}/base`), { branch },
  );
}

/// Work alongside another registered Repo, read-only and off its own default
/// branch until the row says otherwise.
///
/// Which Repo is the whole of what goes out: everything else a companion holds
/// has a default worth having, and picking one out of a menu is one decision.
export function addCompanion(
  device: Device,
  id: number,
  repoId: number,
): Promise<CompanionAdded> {
  return post<CompanionAdded>(
    on(device, `/api/ui/conversations/${id}/companions`),
    {
      repo_id: repoId,
    },
  );
}

/// And stop working alongside one. Which Repo is in the path, and there is
/// nothing else to say about taking it away.
export function removeCompanion(
  device: Device,
  id: number,
  repoId: number,
): Promise<CompanionRemoved> {
  return post<CompanionRemoved>(
    on(device, `/api/ui/conversations/${id}/companions/${repoId}/remove`),
    {},
  );
}

/// Say how far into one of them the work may reach.
export function setCompanionMode(
  device: Device,
  id: number,
  repoId: number,
  mode: CompanionMode,
): Promise<CompanionModeChosen> {
  return post<CompanionModeChosen>(
    on(device, `/api/ui/conversations/${id}/companions/${repoId}/mode`),
    { mode },
  );
}

/// Choose the branch its checkout comes off, or pass `null` for that
/// repository's default-branch rule.
///
/// The branch is the companion repository's own: a conversation and a companion
/// of it are two repositories, each with a list of its own.
export function setCompanionBase(
  device: Device,
  id: number,
  repoId: number,
  branch: string | null,
): Promise<CompanionBaseRecorded> {
  return post<CompanionBaseRecorded>(
    on(device, `/api/ui/conversations/${id}/companions/${repoId}/base`),
    { branch },
  );
}

/// Name the branch a read-write companion's work is done on, or send nothing at
/// all to go back to mirroring the conversation's own.
export function renameCompanionBranch(
  device: Device,
  id: number,
  repoId: number,
  branch: string,
): Promise<CompanionBranchRenamed> {
  return post<CompanionBranchRenamed>(
    on(device, `/api/ui/conversations/${id}/companions/${repoId}/branch`),
    { branch },
  );
}

/// Give a Conversation somewhere to work and set it grilling: a branch off its
/// base commit, and a worktree of its repo.
///
/// Nothing is sent. Which conversation is in the path, and there is nothing else
/// to say — everything the server needs it already has, and everything it
/// refuses for it decides itself when the button is pressed.
export function startGrilling(
  device: Device,
  id: number,
): Promise<GrillingStarted> {
  return post<GrillingStarted>(
    on(device, `/api/ui/conversations/${id}/grill`), {},
  );
}

/// Adopt the roadmap an adopting conversation was started for: its next stage
/// started, on its own branch, off this conversation's base commit.
///
/// Nothing is sent, for the reason nothing is sent to start a grilling: which
/// conversation is in the path, and which stage is the roadmap's own answer at
/// the base commit — read again by the server when the button is pressed.
export function adoptRoadmap(device: Device, id: number): Promise<Adopted> {
  return post<Adopted>(on(device, `/api/ui/conversations/${id}/adopt`), {});
}

/// And take up the pull request a conversation is holding: its head branch
/// checked out, the pull request recorded, and the wrap-up running over it.
///
/// Nothing is sent here either, for the reason nothing is sent to adopt: which
/// conversation is in the path, and what the branch is now is the repository's
/// own answer — read when the button is pressed rather than taken from a page
/// that read it a moment ago.
export function takeUpPullRequest(
  device: Device,
  id: number,
): Promise<TakenUp> {
  return post<TakenUp>(on(device, `/api/ui/conversations/${id}/take-up`), {});
}

/// Stop a Conversation wherever it has got to: its worktree removed, its branch
/// left where it is.
export function closeConversation(
  device: Device,
  id: number,
): Promise<ConversationClosed> {
  return post<ConversationClosed>(
    on(device, `/api/ui/conversations/${id}/close`), {},
  );
}

/// And the same press with the archive already made: the Conversation ends and
/// comes off the sidebar at once.
///
/// One request rather than the two it stands for, so that a connection dropped
/// between them cannot leave a Conversation closed and still on the list. What
/// comes back is what became of the close, the archive of a Conversation just
/// closed having nothing left to refuse.
export function closeAndArchiveConversation(
  device: Device,
  id: number,
): Promise<ConversationClosed> {
  return post<ConversationClosed>(
    on(device, `/api/ui/conversations/${id}/close-and-archive`),
    {},
  );
}

/// Put a closed Conversation away: it comes off the sidebar, and nothing else
/// about it moves.
///
/// Nothing is sent with it either — which Conversation it is is the whole of
/// what the press says, and whether it is one to put away is the server's to
/// answer.
export function archiveConversation(
  device: Device,
  id: number,
): Promise<ConversationArchived> {
  return post<ConversationArchived>(
    on(device, `/api/ui/conversations/${id}/archive`), {},
  );
}

/// And take it back out: it is on the sidebar again, for good.
///
/// Archiving's mirror, sending as little as archiving does.
export function unarchiveConversation(
  device: Device,
  id: number,
): Promise<ConversationUnarchived> {
  return post<ConversationUnarchived>(
    on(device, `/api/ui/conversations/${id}/unarchive`),
    {},
  );
}

/// Say the human has looked at a Conversation, which takes the news mark off
/// its sidebar row.
///
/// A press of its own rather than something reading the Conversation does on
/// the way past: what is being recorded is a person having looked, and a read
/// that wrote it would spend the mark on a prefetch or a retry.
///
/// Answered with nothing at all, as the order and the archived switch are.
/// There is nothing to be refused for — an id naming nothing clears nothing —
/// and what the row does next arrives as a Nudge like every other change to the
/// list.
export async function seeConversation(
  device: Device,
  id: string,
): Promise<void> {
  const at = on(
    device,
    `/api/ui/conversations/${encodeURIComponent(id)}/seen`,
  );

  await refused(at, await sent(at, {}));
}

/// Start driving a conversation again, from wherever the work now stands.
///
/// Nothing is sent with it. What should be running is the server's to work out
/// from the conversation's state and its branch — a press that named a step
/// would be a page deciding something it read a moment ago and cannot check.
///
/// What comes back either says driving has started or names the reason nothing
/// could: resume is never silent, and the refusals are what that means.
export function resume(device: Device, id: number): Promise<Resumed> {
  return post<Resumed>(on(device, `/api/ui/conversations/${id}/resume`), {});
}

/// Get a finished conversation's merge conflict resolved.
///
/// The press on a done pull request's details pane, offered only while the
/// recorded fact says the branch conflicts. It sends the conversation back into
/// its wrap-up, where the resolution session is dispatched at the pull request
/// by the rules a wrap-up already resolves a conflict under — with the review's
/// settle left standing, so nothing reads the branch a second time.
///
/// Nothing is sent, for the reason nothing goes with a resume: which
/// conversation it is is the whole of it, and which of its pull requests
/// conflict is the server's to know.
export function resolveConflicts(
  device: Device,
  id: number,
): Promise<Resolved> {
  return post<Resolved>(
    on(device, `/api/ui/conversations/${id}/resolve-conflicts`), {},
  );
}

/// Stop driving a conversation after the task it is on.
///
/// Nothing new is started and nothing running is cut short: the session going
/// now runs to its own end, and the conversation stops before the next launch.
/// Nothing is sent, for the reason nothing goes with a resume — which
/// conversation it is is the whole of it.
export function stopConversation(
  device: Device,
  id: number,
): Promise<ConversationStopped> {
  return post<ConversationStopped>(
    on(device, `/api/ui/conversations/${id}/stop`), {},
  );
}

/// Press steer: stop the drive and open the pending steer the form is written
/// on.
///
/// The press rather than the move, and an act of its own for that reason.
/// Nothing new is launched while the human composes, so the world the form is
/// written against is the world the submit arrives in — and cancelling leaves
/// the conversation stopped with resume on offer, which is what the press
/// bought.
///
/// A second press writes nothing and answers the same word: the page goes to
/// the form there is either way, so which press it was is nothing to say here.
///
/// Nothing is sent, as nothing is sent with either stop: which conversation it
/// is is the whole of it.
export function steerConversation(
  device: Device,
  id: number,
): Promise<SteerOpened> {
  return post<SteerOpened>(on(device, `/api/ui/conversations/${id}/steer`), {});
}

/// Keep the form as it stands, so the item can be left and come back to.
///
/// The whole form rather than the field that moved: a row holding the target of
/// one keystroke beside the instruction of another would be a form that was
/// never on anybody's screen. Posted on a pause in the typing and on the way
/// out of a field, the way a drafting brief's saves are — see
/// `src/workbench/settling.ts`, which is the pause both of them keep.
export function saveSteer(
  device: Device,
  id: number,
  form: SteerForm,
): Promise<SteerSaved> {
  return post<SteerSaved>(
    on(device, `/api/ui/conversations/${id}/steer/save`), form,
  );
}

/// Cancel it: the pending steer goes, and the conversation is left exactly as
/// the press found it — stopped, with resume on offer.
///
/// Nothing is sent for the same reason, and nothing lands on the timeline: a
/// steer that decided nothing is no event.
export function cancelSteer(
  device: Device,
  id: number,
): Promise<SteerCancelled> {
  return post<SteerCancelled>(
    on(device, `/api/ui/conversations/${id}/steer/cancel`), {},
  );
}

/// And submit the form: where the work goes, and whether to end what is running
/// where it stands.
///
/// Into done there is nothing to start, so this is the move alone — the
/// conversation is finished with, the steer is on the timeline beside the move
/// it wrote, the stop the press left is taken away, and the pending steer goes
/// in the same transaction as the record it became.
export function steer(
  device: Device,
  id: number,
  submission: SteerSubmission,
): Promise<ConversationSteered> {
  return post<ConversationSteered>(
    on(device, `/api/ui/conversations/${id}/steer/submit`),
    submission,
  );
}

/// And stop it now: whatever is running is ended where it stands, and the stop
/// is written at once.
///
/// The step is left however far the session had got, uncommitted work and all.
/// Nothing else goes either — the worktree stays, the branch stays, and a
/// question set nobody has answered is left standing.
export function forceStopConversation(
  device: Device,
  id: number,
): Promise<ConversationStopped> {
  return post<ConversationStopped>(
    on(device, `/api/ui/conversations/${id}/force-stop`),
    {},
  );
}

/// The Agent Profiles a session can be run under, by name.
///
/// Each says whether its pair is still where it was left, which the server
/// answers on every read: a directory can be moved after it was saved, and only
/// the side that can look at the filesystem knows.
export function listProfiles(device: Device): Promise<ProfileEntry[]> {
  return get<ProfileEntry[]>(on(device, "/api/ui/profiles"));
}

/// Take on an account, named by the pair that is mounted for it. Like
/// registering a Repo, every refusal is a named outcome rather than a status —
/// a config field pointed at a directory is something to go and correct.
export function createProfile(profile: ProfileEdit): Promise<ProfileSaved> {
  return post<ProfileSaved>("/api/ui/profiles", profile);
}

/// Rewrite one, whole. Everything about a profile is the human's to change:
/// nothing has been built from it that a change would contradict.
export function editProfile(
  id: number,
  profile: ProfileEdit,
): Promise<ProfileSaved> {
  return post<ProfileSaved>(`/api/ui/profiles/${id}`, profile);
}

/// Remove one nobody is running under. There is nothing to send but its own id,
/// which is in the path.
export function deleteProfile(id: number): Promise<ProfileDeleted> {
  return post<ProfileDeleted>(`/api/ui/profiles/${id}/delete`);
}

/// Choose which account and model a conversation's grilling session runs under.
///
/// `null` is the picker's own "No grilling" row, which is a choice like any
/// other rather than the absence of one: the brief goes straight to an inline
/// implementation.
export function chooseGrillingPairing(
  device: Device,
  id: number,
  choice: RoleChoice,
): Promise<ProfileChosen> {
  return post<ProfileChosen>(
    on(device, `/api/ui/conversations/${id}/grilling-pairing`),
    choice,
  );
}

/// And the one its implementation runs under, which is a separate choice: the
/// implementation session cannot simply carry the grilling one on.
export function chooseImplementationPairing(
  device: Device,
  id: number,
  pairing: ProfileChoice,
): Promise<ProfileChosen> {
  return post<ProfileChosen>(
    on(device, `/api/ui/conversations/${id}/implementation-pairing`),
    pairing,
  );
}

/// And the one the wrap-up's review runs under, which is a third separate
/// choice: reviewing is a fresh set of eyes on what was built.
///
/// `null` is the picker's own "No review" row, which is a choice like any
/// other rather than the absence of one.
export function chooseReviewPairing(
  device: Device,
  id: number,
  choice: RoleChoice,
): Promise<ProfileChosen> {
  return post<ProfileChosen>(
    on(device, `/api/ui/conversations/${id}/review-pairing`),
    choice,
  );
}

/// What this machine's Tailscale is doing: whether there is one at all, whether
/// it is up, what this node is called on the tailnet and whether the tailnet
/// name is already in front of the workbench.
///
/// Read off the machine on every ask rather than out of anything saved — a
/// `tailscale up` run in a terminal shows on the next load. Never a refusal:
/// every way the reading can fail is one of the states it answers with, because
/// each of them is a different thing to say to the human.
export function loadRemote(): Promise<RemoteView> {
  return get<RemoteView>("/api/ui/remote");
}

/// What this Verkstead is: the device this machine runs, and how many others
/// are linked to it.
///
/// Read off the machine on every ask, the way the Tailscale reading above is
/// and for the same reason — the name is the hostname, the OS is the platform's
/// own word and the addresses are whatever interfaces this box has at the
/// moment it answers, none of which anybody configured.
///
/// The same answer a peer reads off this device's identity endpoint, told to
/// the browser instead: that listener is TLS on a port of its own, presenting a
/// certificate nothing but another Verkstead has a reason to trust, so the
/// workbench is where the pane reads it.
export function loadDevices(): Promise<DevicesView> {
  return get<DevicesView>("/api/ui/devices");
}

/// And the devices nobody has typed an address for: the Discovered list under
/// those rows.
///
/// A read of its own beside the one above rather than a field of it, and that is
/// what it is for: a browse hears something every few seconds, and a list
/// arriving on the same answer as the membership would be the cluster's own rows
/// replaced each time the LAN said anything. A `discovered` Nudge re-reads this
/// and nothing else.
///
/// **And asking is what holds the browse open.** The server starts browsing on
/// the first of these and stops once nothing has asked for a spell — a phone that
/// closes a tab says nothing, so the reading being read is the whole of what
/// governs it, and an open pane asks again on an interval to say it is still
/// looking. Which is why the first answer is empty or short: a cold browse has
/// heard nothing yet, and the rows arrive over the seconds after it, each with a
/// Nudge to say so.
export function loadDiscovered(): Promise<DiscoveredDevice[]> {
  return get<DiscoveredDevice[]>("/api/ui/devices/discovered");
}

/// Ask the device at an address to let this one into its cluster.
///
/// The one thing in that section that is pressed rather than read, which is the
/// same departure Unlink makes beside it and Remove on a Repo made before
/// either. A port is optional: every device answers on the peer port unless its
/// host was told another.
///
/// What comes back is the section read again, the way a serve press answers —
/// so the pending row this leaves behind arrives out of this answer rather than
/// out of a second request. A refusal is the far end's own words for what went
/// wrong, because what the human can do about a machine that is off and about a
/// Verkstead that refused are two different things.
export function addDevice(address: string): Promise<DevicesView> {
  return post<DevicesView>("/api/ui/devices/joins", { address });
}

/// And the same question asked of a device on the Discovered list, which is the
/// press on that row: nothing typed anywhere.
///
/// **The device rather than an address**, because a discovery found a list of
/// them — every address that device advertised, and the tailnet's where a probe
/// answered too — and the server dials them in the order it found them. A page
/// that picked one would be choosing between addresses it knows nothing about.
///
/// What comes back is the Devices section read again, the way the typed press
/// answers, so the pending row arrives out of it. The Discovered list is re-read
/// rather than answered here: the row the press was made on has left it, and one
/// list is not the other's to redraw.
///
/// A refusal is the words the dial put it in, naming the device: a row can be
/// stale by the time somebody presses it, the device having gone off the LAN or
/// left the tailnet since it was drawn.
export function addFound(device: string): Promise<DevicesView> {
  return post<DevicesView>(
    `/api/ui/devices/discovered/${encodeURIComponent(device)}/add`,
  );
}

/// And take that request back: Cancel on a row still waiting, Dismiss on one
/// whose ten minutes have run out.
///
/// One call for the two because they are one act at two moments — the human is
/// done with a request nobody has answered — and which of them it is is a fact
/// about the row. A second press is not a second thing happening.
export function cancelJoin(request: string): Promise<DevicesView> {
  return post<DevicesView>(
    `/api/ui/devices/joins/${encodeURIComponent(request)}/cancel`,
  );
}

/// **Unlink**: take a device out of this cluster, for everybody.
///
/// A membership rather than a set of pairs (ADR-0020), so this is not cutting
/// this device's own half of a link: every member drops the same device, and
/// the device itself is told to let go of the lot of them. Which is why it is
/// asked once before it is called — one card over the page, naming the device,
/// as Remove on a Repo is — and why nothing about it can be taken back.
///
/// A member that is not answering is unlinked exactly as one that is: it simply
/// cannot be told, and nothing waits on telling it. So what comes back is the
/// section read again, the way the two presses above answer, rather than
/// anything some third machine made of the press.
export function unlinkDevice(device: string): Promise<DevicesView> {
  return post<DevicesView>(
    `/api/ui/devices/members/${encodeURIComponent(device)}/unlink`,
  );
}

/// And the other side of a join: every device asking to be let into *this*
/// one's cluster.
///
/// Read by the shell every page sits inside rather than by a page, because the
/// question belongs to no page: a join arrives while somebody is reading a
/// Transcript, and it is theirs to answer wherever they are.
///
/// A request whose ten minutes have run out is not in the answer, which is what
/// takes the modal down when nobody pressed anything: the page reads this again
/// and the question it was holding open is not in it.
export function loadAsking(): Promise<AskingDevice[]> {
  return get<AskingDevice[]>("/api/ui/devices/asking");
}

/// **Allow**: let the device that asked into this one's cluster.
///
/// One press joins it to the whole cluster: it is recorded as a member here,
/// dialled back with this device and every member it holds, and announced to
/// each of those members over the link this one already has to them — so
/// nothing is pressed anywhere else. What comes back is the list read again, so
/// a modal that has just been answered goes out of this answer rather than out
/// of a second request.
///
/// A second press is not a second thing happening: two workbenches may both be
/// showing the modal, and the one that presses second finds the request settled
/// and the list empty.
export function allowJoin(request: string): Promise<AskingDevice[]> {
  return post<AskingDevice[]>(
    `/api/ui/devices/asking/${encodeURIComponent(request)}/allow`,
  );
}

/// **Deny**: settle the request and record nothing.
///
/// The same answer and the same shrug at a second press. Nothing is remembered
/// about the device refused — a cluster is a membership rather than a list of
/// verdicts, and a device turned away is free to ask again.
export function denyJoin(request: string): Promise<AskingDevice[]> {
  return post<AskingDevice[]>(
    `/api/ui/devices/asking/${encodeURIComponent(request)}/deny`,
  );
}

/// Put this machine's tailnet name in front of the workbench, or take it off
/// again.
///
/// What comes back is the machine read again, so the switch settles where the
/// machine is rather than where the press meant to put it — with two answers
/// that are not a reading: the operator grant Tailscale wants before it will
/// take a serve from this user, and whatever else went wrong, in the machine's
/// own words.
export function pressServe(edit: ServeEdit): Promise<ServePress> {
  return post<ServePress>("/api/ui/remote/serve", edit);
}

/// A new Workbench Key over the old one, which logs every other device out.
///
/// What comes back is the machine read again, the way a serve press answers:
/// the login link is a field of that reading, so the QR code and the copyable
/// link redraw on the new key from this answer rather than from a second ask.
///
/// The browser that pressed it stays logged in — the answer carries the cookie
/// for the key it just made — because a reset made from the phone on the
/// tailnet is a reset made from the only device that can reach this server.
export function resetKey(): Promise<RemoteView> {
  return post<RemoteView>("/api/ui/remote/key");
}

/// Whether the human is done with the banner that points at Remote access.
///
/// The server's answer rather than this device's, for the archived switch's
/// reason said stronger: the banner is drawn at a desk and points at a phone,
/// so a dismissal that stayed in this browser would meet the human again on the
/// very device it had just sent them to.
export async function remoteBannerDismissed(): Promise<boolean> {
  return (await get<RemoteBanner>("/api/ui/remote/banner")).dismissed;
}

/// And say they are, which is the whole of the press.
///
/// Nothing to send: the banner is dismissed and never put back, so there is no
/// position for a body to carry. What comes back is the flag as it now stands,
/// so the page that pressed it holds the truth without a second ask.
export function dismissRemoteBanner(): Promise<RemoteBanner> {
  return post<RemoteBanner>("/api/ui/remote/banner");
}

/// Whether a newer Verkstead has been released than the one serving this page.
///
/// The server is the side that asks GitHub, once a day, and this hands over
/// whatever it last concluded — so the answer costs it nothing and the browser
/// never waits on GitHub being reachable.
export function updateNotice(): Promise<UpdateNotice> {
  return get<UpdateNotice>("/api/ui/update");
}

/// Whether this Verkstead can do anything yet: the mode the server settled at
/// startup, the machine it is standing on, and what is missing from it.
///
/// Everything but the mode is probed at the moment it is asked, so an install
/// that lands between two reads is ticked on the next one — which is what the
/// wizard's own interval is for. The mode itself is the startup verdict and
/// does not move while the server runs.
export function loadOnboarding(): Promise<OnboardingView> {
  return get<OnboardingView>("/api/ui/onboarding");
}

/// What the wizard's git step can fill its fields with, for whatever Verkstead
/// has not been told.
///
/// A read of its own rather than a part of the one above, and made by the step
/// that has the fields: the probes behind it are two `git config` runs and a
/// `gh`, and one of the three values is a GitHub token — none of which belongs
/// in a payload the wizard re-reads every ten seconds and the workbench asks
/// for at every start. A field Verkstead already holds a value for is absent
/// here, because the value in front of the human is then the one written down.
export function loadGitPrefill(): Promise<PrefillView> {
  return get<PrefillView>("/api/ui/onboarding/git");
}

/// The wizard's first Next: install the rows that were ticked.
///
/// It answers as soon as the run is going — the machine read again, with the
/// run on it at nothing done of however many were ticked — because what is on
/// the other side of the press is a password dialog somebody has to read and a
/// package manager that takes as long as it takes. The install screen is drawn
/// from that reading and from the ones the interval takes after it.
export function startInstall(
  dependencies: Dependency[],
): Promise<OnboardingView> {
  return post<OnboardingView>("/api/ui/onboarding/install", { dependencies });
}

/// And the press that stops it where it can be stopped: the unit under way
/// finishes and the ones after it are skipped.
///
/// Answered with the reading either way, a press about a run that ended while
/// it was in flight having changed nothing.
export function cancelInstall(): Promise<OnboardingView> {
  return post<OnboardingView>("/api/ui/onboarding/install/cancel");
}

/// The wizard's last Next: onboarding mode is off for the rest of this run.
///
/// It writes nothing — the author and the token went through the settings save
/// a moment before — and what comes back is the machine read again with the
/// mode off, which is what says the workbench is a page again. The next start
/// reaches its own verdict, off a machine that now has what it was missing.
export function finishOnboarding(): Promise<OnboardingView> {
  return post<OnboardingView>("/api/ui/onboarding/finished");
}

/// What Verkstead has been told: who a session commits as, and that there is a
/// GitHub token.
///
/// The token itself is not among it and cannot be asked for — what comes back is
/// its last four characters and when it was saved. See [`saveSettings`].
export function loadSettings(): Promise<SettingsView> {
  return get<SettingsView>("/api/ui/settings");
}

/// Write both settings files back, which the page does in one press.
///
/// The token's half of the edit is an action rather than a value — `"Keep"`,
/// `{ Set }` or `"Clear"` — because most saves are about the author, and a
/// write-only field left blank means *leave it alone* rather than *take it
/// away*.
///
/// The answer carries the settings as they now stand, read back off the files,
/// and — where a token was set — what GitHub made of it. A refusal there is part
/// of the answer rather than a failed save: the token is written down either
/// way.
export function saveSettings(edit: SettingsEdit): Promise<SettingsSaved> {
  return post<SettingsSaved>("/api/ui/settings", edit);
}

/// The public half of the server's VAPID keypair — what `PushManager.subscribe`
/// takes as its `applicationServerKey`, and the only way a browser names the
/// server it is subscribing to.
export async function pushKey(): Promise<string> {
  return (await get<PushKey>("/api/ui/push/key")).key;
}

/// Hand this device's subscription over, so a Set arriving can reach it.
export function subscribePush(subscription: Subscription): Promise<Subscribed> {
  return post<Subscribed>("/api/ui/push/subscribe", subscription);
}

/// Ask the server to forget this device, named by the endpoint that is the only
/// name a subscription has.
///
/// Answered with nothing at all — there is no outcome to read, because an
/// endpoint the server never stored leaves what was asked for holding either
/// way.
export async function unsubscribePush(endpoint: string): Promise<void> {
  const at = "/api/ui/push/unsubscribe";

  await refused(at, await sent(at, { endpoint }));
}

async function get<T>(path: string, within?: number): Promise<T> {
  return taken(
    path,
    await fetch(path, {
      headers: { accept: "application/json" },
      // Only where the caller named one. A deadline is a decision about what a
      // page does while it waits rather than a property of talking to this
      // server, so it belongs to the read that has something to draw instead —
      // see [`loadConversation`], which is the only one.
      ...(within === undefined ? {} : { signal: AbortSignal.timeout(within) }),
    }),
  );
}

async function post<T>(path: string, body?: unknown): Promise<T> {
  return taken(path, await sent(path, body));
}

function sent(path: string, body?: unknown): Promise<Response> {
  return fetch(path, {
    method: "POST",
    headers: {
      accept: "application/json",
      "content-type": "application/json",
    },
    // An empty object rather than no body at all: the routes that take one
    // want JSON, and the ones that take none are not troubled by it.
    body: JSON.stringify(body ?? {}),
  });
}

async function taken<T>(path: string, response: Response): Promise<T> {
  await refused(path, response);

  return (await response.json()) as T;
}

/// Throw if the server refused, in its own words. Split out from [`taken`] for
/// the endpoints that answer with no body to read.
///
/// The path is carried in beside the answer so that the refusal knows whether
/// it crossed a hop, which is what [`retrying`] reads — see [`relayed`]. Off
/// the path this module wrote rather than off `response.url`, because that is
/// the one account of it that is there whatever answered.
async function refused(path: string, response: Response): Promise<void> {
  if (!response.ok) {
    throw new RefusedError(
      response.status,
      await refusal(response),
      relayed(path),
    );
  }
}

/// What a refusal said, or a stand-in when it did not say anything readable —
/// a proxy in front of the server can answer where the server would have.
async function refusal(response: Response): Promise<ApiError> {
  try {
    const body: unknown = await response.json();
    if (
      typeof body === "object" &&
      body !== null &&
      typeof (body as ApiError).error === "string"
    ) {
      return body as ApiError;
    }
  } catch {
    // Not JSON at all. Falls through to the status line below.
  }

  return { error: `the server answered ${response.status}` };
}
