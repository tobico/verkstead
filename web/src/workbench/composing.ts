//! What the compose page is holding, where it is kept while nobody has pressed
//! anything, and what pressing something does with it.
//!
//! The composer beside this one serves a Conversation, so every field on it
//! saves itself the moment it is touched — there is a record to save into. The
//! compose page has none: it is the composer before there is anything for it to
//! be about, and what it holds is held on the device the way an answer sheet's
//! draft is (`src/set/sheet.ts`), so closing the tab or reloading the page loses
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

import { createSignal } from "solid-js";

import {
  addCompanion,
  chooseGrillingPairing,
  chooseImplementationPairing,
  chooseReviewPairing,
  renameBranch,
  renameCompanionBranch,
  saveBrief,
  setBaseBranch,
  setCompanionBase,
  setCompanionMode,
  startConversation,
  startGrilling,
} from "../api/client";
import type { CompanionMode } from "../api/types";
import { forget, read, write } from "../device";
import * as pairing from "../pairing";
import {
  BASE_REFUSAL,
  BRANCH_REFUSAL,
  COMPANION_BASE_REFUSAL,
  COMPANION_BRANCH_REFUSAL,
  COMPANION_MODE_REFUSAL,
  COMPANION_REFUSAL,
  CHOICE_REFUSAL,
  RULE,
} from "./Setup";
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

/// The whole of a compose page, as it sits on the device between visits.
///
/// Three of the fields are `null` where they are **untouched** rather than
/// empty, which is a distinction the replay lives on: a role nobody picked is
/// left for the server's own prefill to fill in, and a role picked away is a
/// choice like any other. The repo is `null` for the same reason and one more —
/// nothing at all can be created without one.
export type Composed = {
  repo: number | null;
  brief: string;
  /// The branch the work will be done on, empty being the name the server
  /// invents when the Conversation is started.
  branch: string;
  /// And the branch it comes off, `null` being that repo's default-branch rule.
  base: string | null;
  companions: Alongside[];
  /// Who runs each of the three roles, as a picker writes it — `null` for a
  /// role nobody has touched. See `src/pairing.ts`.
  grilling: string | null;
  implementation: string | null;
  review: string | null;
};

/// A compose page nobody has touched.
export function blank(): Composed {
  return {
    repo: null,
    brief: "",
    branch: "",
    base: null,
    companions: [],
    grilling: null,
    implementation: null,
    review: null,
  };
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
    state.grilling === null &&
    state.implementation === null &&
    state.review === null
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
export async function create(
  state: Composed,
  work: boolean,
): Promise<Created> {
  if (state.repo === null) {
    return "NoSuchRepo";
  }

  const started = await startConversation(state.repo);
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

  if (state.brief.trim() !== "") {
    const outcome = await saveBrief(id, state.brief);
    said(
      outcome === "Saved",
      `The brief could not be saved: ${BRIEF_REFUSAL[outcome]}`,
    );
  }

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

  for (const alongside of state.companions) {
    await put(id, alongside, said);
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

  if (work && refused.length === 0) {
    const outcome = await startGrilling(id);
    said(
      outcome === "Started",
      `The work could not be started: ${grillRefusal(outcome)}`,
    );
  }

  return { conversation: id, refused };
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
    !picked(held.grilling) ||
    !picked(held.implementation) ||
    !picked(held.review)
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
    grilling: held.grilling,
    implementation: held.implementation,
    review: held.review,
  };
}

/// Whether this is a repo id or the absence of one.
function whole(value: unknown): value is number | null {
  return value === null || typeof value === "number";
}

/// And whether this is a role's choice or the absence of one.
function picked(value: unknown): value is string | null {
  return value === null || typeof value === "string";
}
