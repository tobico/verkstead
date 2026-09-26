//! What the compose page is holding, where it is kept while nobody has pressed
//! anything, and what pressing something does with it.
//!
//! The composer beside this one serves a Conversation, so every field on it
//! saves itself the moment it is touched — there is a record to save into. The
//! compose page has none: it is the composer before there is anything for it to
//! be about, and what it holds is held on the device the way an answer sheet's
//! draft is (`src/set/filling.ts`), so closing the tab or reloading the page loses
//! nothing.
//!
//! Nothing reaches the server until a button, and when one is pressed the whole
//! of what was held is **replayed through the endpoints that already exist** —
//! the Conversation started against its Repo, and then a request per field the
//! human touched. No batched create and no second set of validation rules: what
//! refuses a branch name here is what refuses it on the composer, said in the
//! same words.
//!
//! Which is also why a refusal does not undo anything. The Conversation is real
//! from the first request, so a field the server would not take leaves a draft
//! with the rest of the work in it — and the refusals travel to that draft
//! rather than dying with the page that made it, see [`refusedOnCreate`].
//!
//! **The files are not held the same way**, because a `File` is a handle the
//! browser gave this page rather than text a device can write down: they are
//! held in the page (`src/holding.ts`), a reload loses them, and the replay
//! sends them once the Conversation exists — before the work is kicked off,
//! because the Brief freezes when it starts and a file arriving after that
//! would be refused.
//!
//! **A roadmap loaded into the page is held the same way and creates the other
//! kind of Conversation.** Picking one out of the Adopt dropdown writes it into
//! what this device is holding and nothing else — see [`Adopting`] — so it
//! survives a reload as everything else here does, and the press is still the
//! first thing that reaches the server. What that press starts is an adoption
//! rather than a draft against a Repo, and what kicks it off at the end is the
//! adopt endpoint rather than the grill one.
//!
//! **A pull request loaded into it is held the same way and creates the third
//! kind** — see [`AdoptingPullRequest`]. What is different is what it does to
//! the box: a roadmap locks a card over it, where a pull request *fills* it with
//! the title as a heading and the description under it, and what is left there
//! is the Brief. So the text that was being written is stowed on the record that
//! displaced it and given back when it is cleared, and the replay saves the box
//! as it would for any brief the human wrote — the two the pull request answers
//! for itself being the branch and the base, both recorded at the take-up.

import { createSignal } from "solid-js";

import {
  addCompanion,
  adoptRoadmap,
  chooseGrillingPairing,
  chooseImplementationPairing,
  chooseReviewPairing,
  pickProcess,
  renameBranch,
  renameCompanionBranch,
  saveBrief,
  setBaseBranch,
  setCompanionBase,
  setCompanionMode,
  startAdoption,
  startConversation,
  startGrilling,
  startPullRequestAdoption,
  takeUpPullRequest,
} from "../api/client";
import type { CompanionMode, Process, Started } from "../api/types";
import { forget, read, write } from "../device";
import type { Holding } from "../holding";
import * as pairing from "../pairing";
import { adoptRefusal } from "./Adoption";
import { ATTACH_REFUSAL } from "./Composer";
import { PROCESS } from "./processes";
import {
  BASE_REFUSAL,
  BRANCH_REFUSAL,
  COMPANION_BASE_REFUSAL,
  COMPANION_BRANCH_REFUSAL,
  COMPANION_MODE_REFUSAL,
  COMPANION_REFUSAL,
  CHOICE_REFUSAL,
  PROCESS_REFUSAL,
  RULE,
} from "./Setup";
import { takeUpRefusal } from "./TakeUp";
import { BRIEF_REFUSAL, grillRefusal } from "./Timeline";

/// One repo the work would run alongside, as the compose page holds it: which
/// repository, and the three things a companion row settles about it.
///
/// The same three the setup card's own row asks — how far in, off which branch,
/// under what name — because they are the same questions asked before there is
/// a Conversation to ask them of.
export type Alongside = {
  repo_id: number;
  mode: CompanionMode;
  /// The branch its checkout comes off, as the picker writes it: the empty
  /// string is the rule, that repo's default branch as it stands when the
  /// checkout is made.
  base: string;
  /// What a read-write one's branch is called, empty being *mirroring*: the
  /// conversation's own branch name.
  branch: string;
};

/// The roadmap a compose page is loaded with, as the row that loaded it worded
/// it: which repository it is in, which roadmap, and the stage that would be
/// adopted.
///
/// Everything here was read off the abandoned-roadmaps list, and nothing is read
/// again to draw it — the card in the box is this record rather than a request.
/// The stage's own brief text is never on this device at all: it is the
/// repository's, and it becomes the Conversation's Brief at the moment the stage
/// is adopted.
export type Adopting = {
  repo_id: number;
  /// What the Repo is called, for the card: the list of Repos is read beside
  /// this and may not have landed, and a card naming no repository would be a
  /// card missing the thing that tells two `mvp`s apart.
  repo: string;
  /// Its directory name under `docs/roadmaps/` — `mvp`.
  roadmap: string;
  /// What the roadmap calls itself in its heading, or empty where it has none.
  title: string;
  /// The next stage as the roadmap writes it, and what it is called.
  stage: string;
  stage_title: string;
  /// The branch the roadmap was read off, empty being the repo's default branch
  /// — which is the base the adopting Conversation is started fixed to, a
  /// roadmap on an unmerged branch being only on that branch.
  base: string;
};

/// The pull request a compose page is loaded with, as the row that loaded it
/// worded it: which repository it is in, which pull request, and the two
/// branches it sits between.
///
/// Everything here was read off the open-pull-requests list, and nothing is read
/// again to draw it — the card over the box is this record rather than a
/// request. Which is what a card over a *pull request* has to be: the list it
/// came off is a `gh` per registered Repo, so a page that re-read it to draw one
/// row would be calling out to GitHub to name what it is already holding.
export type AdoptingPullRequest = {
  repo_id: number;
  /// What the Repo is called, for the card — [`Adopting.repo`]'s reason, and
  /// one more: a number says nothing without the repository it is in.
  repo: string;
  number: number;
  title: string;
  url: string;
  /// The branch the work is on, which is the branch the take-up checks out.
  head: string;
  /// And the branch it goes into.
  base: string;

  /// What was in the box when this was loaded over it, given back when it is
  /// cleared.
  ///
  /// Held here rather than left in `brief` because a pull request *fills* the
  /// box — the title as a heading and the description under it — where a roadmap
  /// locks a card over it. So the brief that was being written has to go
  /// somewhere, and it goes with the thing that displaced it: clearing gives it
  /// back and drops this record in the one act.
  stowed: string;
};

/// The whole of a compose page, as it sits on the device between visits.
///
/// Four of the fields are `null` where they are **untouched** rather than
/// empty, which is a distinction the replay lives on: a role nobody picked is
/// left for the server's own prefill to fill in, a Process nobody picked is left
/// for the server's own reading, and a role picked away is a choice like any
/// other. The repo is `null` for the same reason and one more — nothing at all
/// can be created without one.
export type Composed = {
  repo: number | null;
  brief: string;
  /// The branch the work will be done on, empty being the name the server
  /// invents when the Conversation is started.
  branch: string;
  /// And the branch it comes off, `null` being that repo's default-branch rule.
  base: string | null;
  companions: Alongside[];
  /// What kind of work it is, `null` being a picker nobody has touched — which
  /// is what leaves the server's own reading standing, exactly as an untouched
  /// role does.
  ///
  /// Not remembered per Repo, unlike the three roles under it: the pairings are
  /// remembered because they are the same answer most of the time, and a
  /// Process is the one thing about a Conversation likeliest to differ from the
  /// last. So the control stands on Develop for every repo, and this is `null`
  /// until somebody says otherwise.
  process: Process | null;
  /// Who runs each of the three roles, as a picker writes it — `null` for a
  /// role nobody has touched. See `src/pairing.ts`.
  grilling: string | null;
  implementation: string | null;
  review: string | null;
  /// The roadmap this page is loaded with, or `null` where it is composing a
  /// piece of work of its own.
  ///
  /// Loaded rather than merged into the fields around it: the repo, the base and
  /// the branch are the roadmap's own while it is held, and the brief, the repo
  /// and the base under it are left exactly where they were — which is what
  /// clearing it restores.
  adopting: Adopting | null;

  /// And the pull request it is loaded with, or `null` where it is composing a
  /// piece of work of its own.
  ///
  /// Never `Some` alongside [`Self.adopting`]: the menu that loads either is
  /// drawn only while nothing is loaded, so there is one thing over the box at
  /// a time and the way to another is to clear the one that is there.
  pull: AdoptingPullRequest | null;
};

/// A compose page nobody has touched.
export function blank(): Composed {
  return {
    repo: null,
    brief: "",
    branch: "",
    base: null,
    companions: [],
    process: null,
    grilling: null,
    implementation: null,
    review: null,
    adopting: null,
    pull: null,
  };
}

/// Which Repo the work would be in: the roadmap's own where one is loaded, and
/// whatever was picked where none is.
///
/// The one reading everything about the repository is drawn off — the trigger's
/// name, the companions an add is refused for, the pairings the Repo is
/// remembered to have been grilled with. A loaded pull request settles it the
/// way a loaded roadmap does: `#41` is a fact about one repository.
export function on(state: Composed): number | null {
  return state.adopting?.repo_id ?? state.pull?.repo_id ?? state.repo;
}

/// Whether there is nothing in it worth coming back to. An untouched page is
/// not worth storing, and whitespace is no more a brief here than it is at the
/// moment work is started.
export function empty(state: Composed): boolean {
  return (
    state.repo === null &&
    state.brief.trim() === "" &&
    state.branch === "" &&
    state.base === null &&
    state.companions.length === 0 &&
    state.process === null &&
    state.grilling === null &&
    state.implementation === null &&
    state.review === null &&
    state.adopting === null &&
    state.pull === null
  );
}

/// Where the compose page's draft lives. One of them, because there is one
/// compose page — and namespaced like everything else this app leaves in a
/// browser.
export const COMPOSING = "verkstead.composing";

/// What this device was last composing, or a blank page where it was composing
/// nothing. A body that will not parse, or is not the shape of one of these, is
/// dropped on the way past: it will be no more use on the next visit.
export function stored(): Composed {
  const body = read(COMPOSING);
  if (body === null) {
    return blank();
  }

  const held = parsed(body);
  if (held === null) {
    forget(COMPOSING);
    return blank();
  }

  return held;
}

/// Write it out, replacing whatever was under the key — or drop it, where there
/// is nothing left in it: a draft of nothing would only ever restore as nothing.
export function keep(state: Composed): void {
  if (empty(state)) {
    forget(COMPOSING);
  } else {
    write(COMPOSING, JSON.stringify(state));
  }
}

/// And drop it, for a page whose work has been created: what was held is on the
/// server by then, and a device that offered it again would be offering to make
/// the same Conversation twice.
export function clear(): void {
  forget(COMPOSING);
}

/// What became of a press: the Conversation it made and whatever the replay
/// could not do to it, or the one refusal that leaves nothing at all.
export type Created =
  | { conversation: number; refused: string[] }
  | "NoSuchRepo";

/// Create the Conversation this page describes, and put every touched field on
/// it — kicking the work off afterwards where that is what was pressed.
///
/// A request per field rather than one that carries them all. The endpoints are
/// there, they are the ones the composer uses, and every one of them decides for
/// itself what it will take: a second path that took the whole of a setup at
/// once would be a second opinion about all of it.
///
/// Nothing is undone by a refusal and nothing is held back by one. Each field is
/// its own question, so the answer to one says nothing about the next, and what
/// is left at the end is a draft holding everything the server would take — with
/// the refusals named, for the pane it is about to be read on.
///
/// The kickoff is the one thing a refusal does stop. A setup the server would
/// not take whole is not the setup the human asked to start work under, and a
/// draft is what they can look at and fix; every other refusal on the way is
/// still worth carrying, so the list is what decides rather than the first one.
///
/// **The files this page is holding go up in the same replay**, one request
/// apiece through the route a draft's own paperclip uses — there is a
/// Conversation by then, so there is nothing else they could need. They go
/// after every field and before the kickoff: what is attached freezes with the
/// Brief, and a file arriving after the grilling started would be refused for
/// being late rather than for anything the human did. **Except where a roadmap
/// is loaded**, which sends none of them: the box is locked to a card, and what
/// is held is given back when the card is cleared.
///
/// **A page loaded with a roadmap creates the other kind of Conversation**, and
/// most of the replay is not asked of it: the Brief is the stage's, the branch
/// is the stage's slug and the base was fixed by the row that loaded it, so what
/// is left to put on is the companions and the pairings. What the press does at
/// the end of it is adopt rather than grill, which is the same act — the work
/// beginning — under the other name.
///
/// **And a page loaded with a pull request creates the third**, which keeps the
/// Brief and loses the same branch and base: what it is worked on is the head
/// branch. What the press does at the end of *it* is the take-up, which is that
/// act again at the far end of the pipeline — the work is built, so what begins
/// is its wrap-up.
export async function create(
  state: Composed,
  work: boolean,
  files: Holding,
): Promise<Created> {
  const held = state.adopting;
  const pull = state.pull;
  if (held === null && pull === null && state.repo === null) {
    return "NoSuchRepo";
  }

  const started = await opened(state, held, pull);
  if (started === "NoSuchRepo") {
    return started;
  }

  const id = started.Started.id;
  const refused: string[] = [];

  /// One field's answer, read the way the composer reads it: nothing where it
  /// landed, and the sentence the pane would have said where it did not.
  const said = (ok: boolean, sentence: string) => {
    if (!ok) refused.push(sentence);
    return ok;
  };

  // The three the roadmap answers for itself, and so are asked only of a page
  // composing work of its own: the stage's brief arrives with the adoption, the
  // stage is worked on its own slug, and the base went out with the start.
  if (held === null) {
    if (state.brief.trim() !== "") {
      const outcome = await saveBrief(id, state.brief);
      said(
        outcome === "Saved",
        `The brief could not be saved: ${BRIEF_REFUSAL[outcome]}`,
      );
    }
  }

  // And the two the pull request answers as well, which are the same two the
  // roadmap does: its branch is the head branch and its base is what GitHub
  // names, both recorded by the take-up rather than settled here. The Brief is
  // *not* one of them — a pull request prefills the box rather than locking a
  // card over it, so what is in the box is the human's and goes up above.
  if (held === null && pull === null) {
    if (state.branch !== "") {
      const outcome = await renameBranch(id, state.branch);
      said(
        outcome === "Renamed",
        `The branch could not be named: ${BRANCH_REFUSAL[outcome]}`,
      );
    }

    if (state.base !== null) {
      const outcome = await setBaseBranch(id, state.base);
      said(
        outcome === "Recorded",
        `The base branch could not be recorded: ${BASE_REFUSAL[outcome]}`,
      );
    }
  }

  for (const alongside of state.companions) {
    await put(id, alongside, said);
  }

  // What kind of work it is, and only where the human touched the picker: a
  // page left on Develop sends nothing, exactly as a role left on its prefill
  // does, and the server's own reading of a Conversation with no row of its own
  // is what stands.
  //
  // Not asked of a page holding a pull request, which answers this for itself
  // the way it answers the branch and the base: what a take-up makes is a
  // Review, and that is the reading rather than a row anybody wrote.
  if (pull === null && state.process !== null) {
    const outcome = await pickProcess(id, state.process);
    said(
      outcome === "Picked",
      `The process could not be picked: ${PROCESS_REFUSAL[outcome]}`,
    );
  }

  if (state.grilling !== null) {
    const outcome = await chooseGrillingPairing(
      id,
      pairing.role(state.grilling),
    );
    said(
      outcome === "Chosen",
      `The grilling profile could not be chosen: ${CHOICE_REFUSAL[outcome]}`,
    );
  }

  if (state.implementation !== null) {
    const outcome = await chooseImplementationPairing(
      id,
      pairing.choice(state.implementation),
    );
    said(
      outcome === "Chosen",
      `The implementation profile could not be chosen: ${CHOICE_REFUSAL[outcome]}`,
    );
  }

  if (state.review !== null) {
    const outcome = await chooseReviewPairing(id, pairing.role(state.review));
    said(
      outcome === "Chosen",
      `The review profile could not be chosen: ${CHOICE_REFUSAL[outcome]}`,
    );
  }

  // And the files, last of the fields: each is one more thing put on the
  // Conversation, and one the server would not take is one more refusal to
  // carry — which is what stops the kickoff, exactly as a refused branch name
  // does.
  //
  // None of them on a page that loaded a roadmap, for the reason the paperclip
  // is not offered on one: the box is locked to a card, so nothing was being
  // written for a file to be handed over with — and a file picked before the
  // roadmap was loaded is one picked for a box the roadmap has since taken
  // over. What is held stays held, and clearing the roadmap gives it back.
  if (held === null) {
    for (const rejected of await files.flush(id)) {
      refused.push(
        `${rejected.name} could not be attached: ${ATTACH_REFUSAL[rejected.refused]}`,
      );
    }
  }

  if (work && refused.length === 0) {
    if (held !== null) {
      const outcome = await adoptRoadmap(id);
      said(
        outcome === "Adopted",
        `The stage could not be started: ${adoptRefusal(outcome)}`,
      );
    } else if (pull !== null) {
      // The third kickoff, and the one that starts no session: the take-up puts
      // the Conversation on the pull request's head branch and moves it into
      // Wrapping, and what runs from there is the wrap-up's own watchers.
      const outcome = await takeUpPullRequest(id);
      said(
        outcome === "TakenUp",
        `The pull request could not be taken up: ${takeUpRefusal(outcome)}`,
      );
    } else {
      const outcome = await startGrilling(id);
      said(
        outcome === "Started",
        `The work could not be started: ${grillRefusal(outcome)}`,
      );
    }
  }

  return { conversation: id, refused };
}

/// The Conversation this page's press makes, which is one of three starts.
///
/// Three endpoints rather than one with a shape inside it, for the reason every
/// other field here goes through the endpoint that already existed: what a
/// Conversation is started *over* is a different question in each case — a Repo,
/// a roadmap in one, a pull request open in one — and each of them is refused
/// for its own reasons.
async function opened(
  state: Composed,
  held: Adopting | null,
  pull: AdoptingPullRequest | null,
): Promise<Started> {
  if (held !== null) {
    return startAdoption(held.repo_id, held.roadmap, held.base);
  }

  if (pull !== null) {
    return startPullRequestAdoption(pull.repo_id, pull);
  }

  return startConversation(state.repo!);
}

/// One companion, put on the Conversation and then configured: the add first,
/// because everything after it is about the row the add makes, and nothing after
/// it where the add was refused.
async function put(
  id: number,
  alongside: Alongside,
  said: (ok: boolean, sentence: string) => boolean,
): Promise<void> {
  const added = await addCompanion(id, alongside.repo_id);
  if (
    !said(
      added === "Added",
      `A companion repo could not be added: ${COMPANION_REFUSAL[added]}`,
    )
  ) {
    return;
  }

  // Read-only off the rule with no branch of its own is what an add already
  // leaves, so only what the human moved is sent.
  if (alongside.mode !== "ReadOnly") {
    const outcome = await setCompanionMode(
      id,
      alongside.repo_id,
      alongside.mode,
    );
    said(
      outcome === "Chosen",
      `A companion repo's mode could not be set: ${COMPANION_MODE_REFUSAL[outcome]}`,
    );
  }

  if (alongside.base !== RULE) {
    const outcome = await setCompanionBase(
      id,
      alongside.repo_id,
      alongside.base,
    );
    said(
      outcome === "Recorded",
      `A companion repo's base could not be recorded: ${COMPANION_BASE_REFUSAL[outcome]}`,
    );
  }

  // Empty is mirroring, which is what an add leaves — and a read-only companion
  // is checked out detached, so there is no branch of its own to name.
  if (alongside.mode === "ReadWrite" && alongside.branch !== "") {
    const outcome = await renameCompanionBranch(
      id,
      alongside.repo_id,
      alongside.branch,
    );
    said(
      outcome === "Renamed",
      `A companion repo's branch could not be named: ${COMPANION_BRANCH_REFUSAL[outcome]}`,
    );
  }
}

/// What a create could not do, waiting for the pane it was about.
///
/// The compose page is gone by the time the draft is on screen — it navigated
/// into it — so a refusal drawn where it happened would be a refusal nobody
/// reads. It is left here instead, against the Conversation it is about, and
/// the composer picks it up: one create's worth, because the next create
/// replaces it and a refusal about a Conversation nobody is looking at is a
/// refusal about work already done.
const [replayed, setReplayed] = createSignal<{
  conversation: number;
  refused: string[];
} | null>(null);

/// What the create that made this Conversation could not do, in the words its
/// own pane would have used — and nothing at all for every other Conversation,
/// which is all of them but the one just made.
export function refusedOnCreate(id: number): string[] {
  const left = replayed();
  return left !== null && left.conversation === id ? left.refused : [];
}

/// Leave them for that Conversation's composer, or take away what was left for
/// the one before it.
export function leaveRefusals(conversation: number, refused: string[]): void {
  setReplayed(refused.length === 0 ? null : { conversation, refused });
}

/// A compose page out of its stored body, checked field by field.
///
/// Hand-checked because nothing on this side of the wire does it for us, and a
/// body under this key is whatever some older build of the app left there.
function parsed(body: string): Composed | null {
  let payload: unknown;
  try {
    payload = JSON.parse(body);
  } catch {
    return null;
  }

  if (typeof payload !== "object" || payload === null) {
    return null;
  }

  const held = payload as Partial<Composed>;

  if (
    !whole(held.repo) ||
    typeof held.brief !== "string" ||
    typeof held.branch !== "string" ||
    !(held.base === null || typeof held.base === "string") ||
    !Array.isArray(held.companions) ||
    !kind(held.process) ||
    !picked(held.grilling) ||
    !picked(held.implementation) ||
    !picked(held.review) ||
    !loaded(held.adopting) ||
    !taken(held.pull)
  ) {
    return null;
  }

  const companions: Alongside[] = [];
  for (const row of held.companions as unknown[]) {
    if (typeof row !== "object" || row === null) {
      return null;
    }

    const { repo_id, mode, base, branch } = row as Partial<Alongside>;
    if (
      typeof repo_id !== "number" ||
      (mode !== "ReadOnly" && mode !== "ReadWrite") ||
      typeof base !== "string" ||
      typeof branch !== "string"
    ) {
      return null;
    }

    companions.push({ repo_id, mode, base, branch });
  }

  return {
    repo: held.repo,
    brief: held.brief,
    branch: held.branch,
    base: held.base,
    companions,
    process: held.process ?? null,
    grilling: held.grilling,
    implementation: held.implementation,
    review: held.review,
    adopting: held.adopting ?? null,
    pull: held.pull ?? null,
  };
}

/// Whether this is a roadmap loaded into the page, or none at all.
///
/// Every field of one, because the card is drawn straight off it and nothing
/// reads it again: a body missing the stage would be a card with a gap in it,
/// and a body from an older build has no `adopting` at all — which is the one
/// absence that is not a fault, and reads as no roadmap loaded.
function loaded(value: unknown): value is Adopting | null | undefined {
  if (value === null || value === undefined) {
    return true;
  }

  if (typeof value !== "object") {
    return false;
  }

  const roadmap = value as Partial<Adopting>;
  return (
    typeof roadmap.repo_id === "number" &&
    typeof roadmap.repo === "string" &&
    typeof roadmap.roadmap === "string" &&
    typeof roadmap.title === "string" &&
    typeof roadmap.stage === "string" &&
    typeof roadmap.stage_title === "string" &&
    typeof roadmap.base === "string"
  );
}

/// And whether this is a pull request loaded into the page, or none at all.
///
/// Every field of one, for [`loaded`]'s reason — the card is drawn straight off
/// it — and a body from a build before this one has no `pull` at all, which
/// reads as none loaded rather than as a fault.
function taken(value: unknown): value is AdoptingPullRequest | null | undefined {
  if (value === null || value === undefined) {
    return true;
  }

  if (typeof value !== "object") {
    return false;
  }

  const pull = value as Partial<AdoptingPullRequest>;
  return (
    typeof pull.repo_id === "number" &&
    typeof pull.repo === "string" &&
    typeof pull.number === "number" &&
    typeof pull.title === "string" &&
    typeof pull.url === "string" &&
    typeof pull.head === "string" &&
    typeof pull.base === "string" &&
    typeof pull.stowed === "string"
  );
}

/// Whether this is a Process the wire knows, or the absence of one.
///
/// Checked against the words rather than merely for a string, because a Process
/// goes straight back out on the wire: a body left by some other build holding a
/// word this one has never heard of would be a request refused for a reason
/// nobody could act on. A body from before this field has no `process` at all,
/// which is untouched rather than a fault — [`loaded`]'s absence, read the same
/// way.
function kind(value: unknown): value is Process | null | undefined {
  return (
    value === null ||
    value === undefined ||
    (typeof value === "string" && Object.hasOwn(PROCESS, value))
  );
}

/// Whether this is a repo id or the absence of one.
function whole(value: unknown): value is number | null {
  return value === null || typeof value === "number";
}

/// And whether this is a role's choice or the absence of one.
function picked(value: unknown): value is string | null {
  return value === null || typeof value === "string";
}
