//! Talking to `/api/ui/` — the viewer's own half of the server.
//!
//! Every payload's type comes from [`./types`], which `cargo test` writes out
//! of the Rust the server fills them in from. Nothing here declares a shape of
//! its own: a hand-written interface is a second opinion about the wire, and
//! the whole point of generating them is that there is only ever one.

import type {
  ApiError,
  ArchiveEntry,
  Archived,
  BaseRecorded,
  BranchRenamed,
  BriefSaved,
  ConversationEntry,
  ConversationView,
  PendingEntry,
  ProfileChosen,
  ProfileDeleted,
  ProfileEdit,
  ProfileEntry,
  ProfileSaved,
  PushKey,
  Registered,
  RepoEntry,
  Response as Decided,
  SetView,
  Started,
  Submitted,
  Subscribed,
  Subscription,
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

/// The Sets still waiting on the human, newest first.
export function listPending(): Promise<PendingEntry[]> {
  return get<PendingEntry[]>("/api/ui/pending");
}

/// The Sets that have been settled, newest first — answered or closed
/// unanswered, which is the whole of the Archive.
export function listArchive(): Promise<ArchiveEntry[]> {
  return get<ArchiveEntry[]>("/api/ui/archive");
}

/// One Set, rendered, with where it stands.
///
/// The id is whatever the URL held, unparsed: one that is not a number cannot
/// name a Set, and the server answers for that the same way it answers for one
/// that names no Set — a 404, which the page reads as "there isn't one".
export function loadSet(id: string): Promise<SetView> {
  return get<SetView>(`/api/ui/sets/${encodeURIComponent(id)}`);
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

/// Ask Verkstead to take on the repository at an absolute path.
///
/// Like answering a Set, the outcome is the answer's body rather than its
/// status: a path outside the Watched Paths is the boundary doing its job and
/// not an error, and every refusal is a different sentence to put in front of
/// the human.
export function registerRepo(path: string): Promise<Registered> {
  return post<Registered>("/api/ui/repos", { path });
}

/// The Conversations in the sidebar, newest first.
export function listConversations(): Promise<ConversationEntry[]> {
  return get<ConversationEntry[]>("/api/ui/conversations");
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

/// Override the commit the work branches from, or pass `null` to put the
/// Conversation back on the default-branch rule.
export function setBaseCommit(
  id: number,
  commit: string | null,
): Promise<BaseRecorded> {
  return post<BaseRecorded>(`/api/ui/conversations/${id}/base`, { commit });
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
export function chooseGrillingProfile(
  id: number,
  profileId: number,
): Promise<ProfileChosen> {
  return post<ProfileChosen>(`/api/ui/conversations/${id}/grilling-profile`, {
    profile_id: profileId,
  });
}

/// And the one its implementation runs under, which is a separate choice: the
/// implementation session cannot simply carry the grilling one on.
export function chooseImplementationProfile(
  id: number,
  profileId: number,
): Promise<ProfileChosen> {
  return post<ProfileChosen>(
    `/api/ui/conversations/${id}/implementation-profile`,
    { profile_id: profileId },
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
/// the one endpoint that answers with no body to read.
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
