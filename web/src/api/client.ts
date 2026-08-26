//! Talking to `/api/ui/` — the viewer's own half of the server.
//!
//! Every payload's type comes from [`./types`], which `cargo test` writes out
//! of the Rust the server fills them in from. Nothing here declares a shape of
//! its own: a hand-written interface is a second opinion about the wire, and
//! the whole point of generating them is that there is only ever one.

import type {
  AbandonedRepo,
  Adopted,
  ApiError,
  Archived,
  BaseRecorded,
  BranchRenamed,
  BriefSaved,
  Capture,
  CommitPane,
  ConversationClosed,
  ConversationEntry,
  ConversationSteered,
  ConversationStopped,
  ConversationView,
  GrillingStarted,
  ProfileChoice,
  ProfileChosen,
  ProfileDeleted,
  ProfileEdit,
  ProfileEntry,
  ProfileSaved,
  PullRequestDetails,
  PushKey,
  Registered,
  RepoEntry,
  Response as Decided,
  Resumed,
  Screen,
  SetReading,
  SettingsEdit,
  SettingsSaved,
  SettingsView,
  Started,
  SteerOpened,
  SteerSubmission,
  Submitted,
  Subscribed,
  Subscription,
  TranscriptView,
  UpdateNotice,
} from "./types";

/// A refusal from the server, in the shape both halves refuse in.
///
/// Carries the server's own wording rather than a status code, because that
/// wording is what the page has to show the human — see the `error` field on
/// `ApiError`.
export class RefusedError extends Error {
  readonly status: number;
  readonly violations: NonNullable<ApiError["violations"]>;

  constructor(status: number, refusal: ApiError) {
    super(refusal.error);
    this.name = "RefusedError";
    this.status = status;
    this.violations = refusal.violations ?? [];
  }
}

/// One Set, rendered, with where it stands — or the stored body where this
/// build cannot read it, which is a page to draw rather than a failure.
///
/// The id is whatever the URL held, unparsed: one that is not a number cannot
/// name a Set, and the server answers for that the same way it answers for one
/// that names no Set — a 404, which the page reads as "there isn't one".
export function loadSet(id: string): Promise<SetReading> {
  return get<SetReading>(`/api/ui/sets/${encodeURIComponent(id)}`);
}

/// Answer a Set, which ends the wait the agent is holding on it.
///
/// The outcome is the answer's body rather than its status: every one of them —
/// taken, already answered, archived, refused by the grammar — is something the
/// page has to say in words, and only a server that could not answer at all
/// throws.
export function submitResponse(
  id: number,
  response: Decided,
): Promise<Submitted> {
  return post<Submitted>(`/api/ui/sets/${id}/response`, response);
}

/// Close a Set unanswered: the human declaring that nobody is ever going to
/// answer it. There is nothing to send but the Set's own id, which is in the
/// path.
export function archiveSet(id: number): Promise<Archived> {
  return post<Archived>(`/api/ui/sets/${id}/archive`);
}

/// The Repos Verkstead has been told about, by name.
export function listRepos(): Promise<RepoEntry[]> {
  return get<RepoEntry[]>("/api/ui/repos");
}

/// Every branch of one registered Repo, local and remote-tracking both — which
/// is what a drafting Conversation picks the one it comes off out of.
///
/// Read out of git by the server every time it is asked: branches move without
/// Verkstead hearing about it, so there is nothing here that could be kept.
export function listBranches(repoId: number): Promise<string[]> {
  return get<string[]>(`/api/ui/repos/${repoId}/branches`);
}

/// Ask Verkstead to take on the repository at an absolute path.
///
/// Like answering a Set, the outcome is the answer's body rather than its
/// status: a path outside the Watched Paths is the boundary doing its job and
/// not an error, and every refusal is a different sentence to put in front of
/// the human.
export function registerRepo(path: string): Promise<Registered> {
  return post<Registered>("/api/ui/repos", { path });
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

/// Start a Conversation to adopt one of those roadmaps with.
///
/// What clicking a roadmap in the notice does. The stage is not sent: which one
/// is next is the roadmap's own answer at whatever commit the Conversation ends
/// up branching from, and the page reads it back there.
export function startAdoption(
  repoId: number,
  roadmap: string,
): Promise<Started> {
  return post<Started>("/api/ui/adoptions", {
    repo_id: repoId,
    roadmap,
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
  await refused(await sent("/api/ui/conversations/order", { order }));
}

/// One Conversation with its Timeline.
///
/// The id is whatever the URL held, unparsed, as a Set's is: one that is not a
/// number cannot name a Conversation, and the server answers for that the way it
/// answers for one that names none.
export function loadConversation(id: string): Promise<ConversationView> {
  return get<ConversationView>(
    `/api/ui/conversations/${encodeURIComponent(id)}`,
  );
}

/// What one session printed, whole.
///
/// Fetched by the pane that shows it rather than carried by the Conversation: a
/// session prints megabytes over an hour, and the Timeline is read again every
/// time this page hears the world moved.
export function loadCapture(id: number, event: number): Promise<Capture> {
  return get<Capture>(`/api/ui/conversations/${id}/capture/${event}`);
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
  id: number,
  event: number,
  after?: string,
): Promise<TranscriptView> {
  const from = after === undefined ? "" : `?after=${encodeURIComponent(after)}`;

  return get<TranscriptView>(
    `/api/ui/conversations/${id}/transcript/${event}${from}`,
  );
}

/// And how it looked, which is the same Event read the third way.
///
/// The grid those bytes leave on a terminal, as the escape sequences that would
/// paint it — the server holds the terminal that decided them, and this is a
/// repaint to feed the one in the pane (ADR 0007). A session that has ended
/// repaints to the screen it last stood on.
export function loadScreen(id: number, event: number): Promise<Screen> {
  return get<Screen>(`/api/ui/conversations/${id}/screen/${event}`);
}

/// And where to watch a session that is still running: the one socket in the
/// app, and the one place the viewer is sent something rather than fetching it.
///
/// A repaint on connect and what the session prints after it, with the size of
/// the window it is being watched in — and whatever is typed into it — going
/// back the other way. Everything else here stays on SSE and a refetch — a
/// terminal being drawn is the one thing neither of those is any good for.
///
/// Built off the page's own origin, so the socket goes wherever the page came
/// from: the dev server proxying `/api`, or the one binary serving both.
export function screenSocket(id: number, event: number): string {
  const at = new URL(
    `/api/ui/conversations/${id}/screen/${event}/attach`,
    window.location.href,
  );

  at.protocol = at.protocol === "https:" ? "wss:" : "ws:";

  return at.href;
}

/// One commit, rendered: what it said about itself, and its diff.
///
/// Fetched by the pane that shows it for the Capture's reason. The diff is read
/// out of the repository by the server rather than out of its database — the
/// commit is in git, which is what a commit is — where the summary was kept by
/// the sweep that recorded the commit.
export function loadCommitPane(
  id: number,
  event: number,
): Promise<CommitPane> {
  return get<CommitPane>(`/api/ui/conversations/${id}/commit/${event}`);
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
  id: number,
  event: number,
): Promise<PullRequestDetails> {
  return get<PullRequestDetails>(
    `/api/ui/conversations/${id}/pull-request/${event}`,
  );
}

/// Start a Conversation against a registered Repo.
///
/// The branch name is not sent: it is prefilled by the server, because the
/// record is the server's from the moment it exists.
export function startConversation(repoId: number): Promise<Started> {
  return post<Started>("/api/ui/conversations", { repo_id: repoId });
}

/// Save what the human has written into a Brief.
export function saveBrief(id: number, markdown: string): Promise<BriefSaved> {
  return post<BriefSaved>(`/api/ui/conversations/${id}/brief`, { markdown });
}

/// Name the branch the work will be done on. Whether git would take the name is
/// the server's to say, so this is another outcome to read rather than a status.
export function renameBranch(
  id: number,
  branch: string,
): Promise<BranchRenamed> {
  return post<BranchRenamed>(`/api/ui/conversations/${id}/branch`, { branch });
}

/// Choose the branch the work comes off, or pass `null` to put the Conversation
/// back on the default-branch rule.
///
/// The name rather than where it stands: it is resolved when grilling starts, so
/// the work comes off wherever that branch is then.
export function setBaseBranch(
  id: number,
  branch: string | null,
): Promise<BaseRecorded> {
  return post<BaseRecorded>(`/api/ui/conversations/${id}/base`, { branch });
}

/// Give a Conversation somewhere to work and set it grilling: a branch off its
/// base commit, and a worktree of its repo.
///
/// Nothing is sent. Which conversation is in the path, and there is nothing else
/// to say — everything the server needs it already has, and everything it
/// refuses for it decides itself when the button is pressed.
export function startGrilling(id: number): Promise<GrillingStarted> {
  return post<GrillingStarted>(`/api/ui/conversations/${id}/grill`, {});
}

/// Adopt the roadmap an adopting conversation was started for: its next stage
/// started, on its own branch, off this conversation's base commit.
///
/// Nothing is sent, for the reason nothing is sent to start a grilling: which
/// conversation is in the path, and which stage is the roadmap's own answer at
/// the base commit — read again by the server when the button is pressed.
export function adoptRoadmap(id: number): Promise<Adopted> {
  return post<Adopted>(`/api/ui/conversations/${id}/adopt`, {});
}

/// Stop a Conversation wherever it has got to: its worktree removed, its branch
/// left where it is.
export function closeConversation(id: number): Promise<ConversationClosed> {
  return post<ConversationClosed>(`/api/ui/conversations/${id}/close`, {});
}

/// Start driving a conversation again, from wherever the work now stands.
///
/// Nothing is sent with it. What should be running is the server's to work out
/// from the conversation's state and its branch — a press that named a step
/// would be a page deciding something it read a moment ago and cannot check.
///
/// What comes back either says driving has started or names the reason nothing
/// could: resume is never silent, and the refusals are what that means.
export function resume(id: number): Promise<Resumed> {
  return post<Resumed>(`/api/ui/conversations/${id}/resume`, {});
}

/// Stop driving a conversation after the task it is on.
///
/// Nothing new is started and nothing running is cut short: the session going
/// now runs to its own end, and the conversation stops before the next launch.
/// Nothing is sent, for the reason nothing goes with a resume — which
/// conversation it is is the whole of it.
export function stopConversation(id: number): Promise<ConversationStopped> {
  return post<ConversationStopped>(`/api/ui/conversations/${id}/stop`, {});
}

/// Click steer: stop the drive, and find out what was running when it stopped.
///
/// The click rather than the move, and a press of its own for that reason.
/// Nothing new is launched while the human composes, so the world the modal is
/// drawn against is the world the submit arrives in — and cancelling leaves the
/// conversation stopped with resume on offer, which is what the click bought.
///
/// Nothing is sent, as nothing is sent with either stop: which conversation it
/// is is the whole of it.
export function steerConversation(id: number): Promise<SteerOpened> {
  return post<SteerOpened>(`/api/ui/conversations/${id}/steer`, {});
}

/// And submit the modal it opened: where the work goes, and whether to end what
/// is running where it stands.
///
/// Into done there is nothing to start, so this is the move alone — the
/// conversation is finished with, the steer is on the timeline beside the move
/// it wrote, and the stop the click left is taken away.
export function steer(
  id: number,
  submission: SteerSubmission,
): Promise<ConversationSteered> {
  return post<ConversationSteered>(
    `/api/ui/conversations/${id}/steer/submit`,
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
  id: number,
): Promise<ConversationStopped> {
  return post<ConversationStopped>(
    `/api/ui/conversations/${id}/force-stop`,
    {},
  );
}

/// The Agent Profiles a session can be run under, by name.
///
/// Each says whether its pair is still where it was left, which the server
/// answers on every read: a directory can be moved after it was saved, and only
/// the side that can look at the filesystem knows.
export function listProfiles(): Promise<ProfileEntry[]> {
  return get<ProfileEntry[]>("/api/ui/profiles");
}

/// Take on an account, named by the pair that is mounted for it. Like
/// registering a Repo, every refusal is a named outcome rather than a status —
/// a pair outside the watched paths is the boundary doing its job.
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
export function chooseGrillingPairing(
  id: number,
  pairing: ProfileChoice,
): Promise<ProfileChosen> {
  return post<ProfileChosen>(
    `/api/ui/conversations/${id}/grilling-pairing`,
    pairing,
  );
}

/// And the one its implementation runs under, which is a separate choice: the
/// implementation session cannot simply carry the grilling one on.
export function chooseImplementationPairing(
  id: number,
  pairing: ProfileChoice,
): Promise<ProfileChosen> {
  return post<ProfileChosen>(
    `/api/ui/conversations/${id}/implementation-pairing`,
    pairing,
  );
}

/// Whether a newer Verkstead has been released than the one serving this page.
///
/// The server is the side that asks GitHub, once a day, and this hands over
/// whatever it last concluded — so the answer costs it nothing and the browser
/// never waits on GitHub being reachable.
export function updateNotice(): Promise<UpdateNotice> {
  return get<UpdateNotice>("/api/ui/update");
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
export function subscribePush(
  subscription: Subscription,
): Promise<Subscribed> {
  return post<Subscribed>("/api/ui/push/subscribe", subscription);
}

/// Ask the server to forget this device, named by the endpoint that is the only
/// name a subscription has.
///
/// Answered with nothing at all — there is no outcome to read, because an
/// endpoint the server never stored leaves what was asked for holding either
/// way.
export async function unsubscribePush(endpoint: string): Promise<void> {
  await refused(await sent("/api/ui/push/unsubscribe", { endpoint }));
}

async function get<T>(path: string): Promise<T> {
  return taken(
    await fetch(path, {
      headers: { accept: "application/json" },
    }),
  );
}

async function post<T>(path: string, body?: unknown): Promise<T> {
  return taken(await sent(path, body));
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

async function taken<T>(response: Response): Promise<T> {
  await refused(response);

  return (await response.json()) as T;
}

/// Throw if the server refused, in its own words. Split out from [`taken`] for
/// the endpoints that answer with no body to read.
async function refused(response: Response): Promise<void> {
  if (!response.ok) {
    throw new RefusedError(response.status, await refusal(response));
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
