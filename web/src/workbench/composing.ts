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
//! **And replayed onto whichever device of the cluster will do the work**, which
//! is [`Composed.device`]: a Conversation is made where it is going to be run, so
//! the start and every request after it are addressed to that device and go
//! through the Relay untouched (`src/reaching.ts`, and `relaying.rs` at the other
//! end). Nothing about the replay is different for it — the same endpoints in the
//! same order, and every refusal the far end's own in the words the composer says
//! them in.
//!
//! Which is also why a refusal does not undo anything. The Conversation is real
//! from the first request, so a field the server would not take leaves a draft
//! with the rest of the work in it — and the refusals travel to that draft
//! rather than dying with the page that made it, see [`refusedOnCreate`].
//!
//! **And the one outcome that is not a refusal travels whole**, which is a take-up
//! stopped to ask what closing another conversation would discard: what answers it
//! is a press, and the press is on the draft this made. So the draft's own take-up
//! picks up the outcome and its press is the confirming one — see
//! [`stoppedOnCreate`]. A sentence would have said *press start again to go ahead*
//! about a press that had no way of knowing what had been asked.
//!
//! **The files are not held the same way**, because a `File` is a handle the
//! browser gave this page rather than text a device can write down: they are
//! held in the page (`src/holding.ts`), a reload loses them, and the replay
//! sends them once the Conversation exists — before the work is kicked off,
//! because the Brief freezes when it starts and a file arriving after that
//! would be refused.
//!
//! **And the MCP servers picked at that same control go up with them**, held in
//! the page as they are and attached by name once there is a Conversation to
//! attach them to — before the kickoff for the files' own reason, the servers
//! freezing when the Brief does. See [`Compose`](./Compose.tsx) for why a name
//! is held in the page rather than written to the device.
//!
//! **A roadmap loaded into the page is held the same way and creates the other
//! kind of Conversation.** Picking one out of the Adopt dropdown writes it into
//! what this device is holding and nothing else — see [`Adopting`] — so it
//! survives a reload as everything else here does, and the press is still the
//! first thing that reaches the server. What that press starts is an adoption
//! rather than a draft against a Repo, and what kicks it off at the end is the
//! adopt endpoint rather than the grill one.
//!
//! **And the Target field is filled from the box as it is typed** — see
//! [`written`], which is the rule the server keeps for a saved Brief kept here,
//! this page having no record to keep it for.
//!
//! **And the replay is run once more against a draft that already exists**, which
//! is a saved draft being moved onto another device — see [`moveTo`]. The Brief,
//! the branch name, the base and the files are read off the draft rather than out
//! of what this page is holding, and the draft they came off is closed at the end
//! of it;
//! everything between is the same sequence through the same endpoints, because a
//! move is this page's press made about work that was already composed.

import { createSignal } from "solid-js";

import {
  addCompanion,
  adoptRoadmap,
    attachServer, attachFile,
  chooseGrillingPairing,
  chooseImplementationPairing,
  chooseReviewPairing,
  draftMovedTo,
  nameTarget,
  pickProcess,
  readAttachment,
  renameBranch,
  renameCompanionBranch,
  saveBrief,
  setBaseBranch,
  setCompanionBase,
  setCompanionMode,
  startAdoption,
  startConversation,
  startGrilling,
  takeUpPullRequest,
} from "../api/client";
import type {
  AdoptedStage,
  AttachmentView,
  CompanionMode,
  ConversationView,
  Process,
  Started,
  TakenUp,
} from "../api/types";
import type { Holding } from "../holding";
import * as pairing from "../pairing";
import { rowKey, type Device } from "../reaching";
import { draftingOn, forget, read, write } from "../remembered";
import { adoptRefusal } from "./Adoption";
import { ATTACH_REFUSAL, SERVER_REFUSAL } from "./Composer";
import { PROCESS, targeted } from "./processes";
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
  TARGET_REFUSAL,
} from "./Setup";
import { chosen } from "./naming";
import { takeUpRefusal } from "./TakeUp";
import { pullRequestIn } from "./targets";
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
  /// And the stages the press would start beside it, where the roadmap declares
  /// and has more than one ready — the first is the conversation this page is
  /// about, and these are started as conversations of their own.
  ///
  /// Optional because a body this device wrote down before there were any has
  /// none, which is no roadmap ready beside the one it named rather than a card
  /// with a gap in it — see [`loaded`].
  beside?: AdoptedStage[];
  /// The branch the roadmap was read off, empty being the repo's default branch
  /// — which is the base the adopting Conversation is started fixed to, a
  /// roadmap on an unmerged branch being only on that branch.
  base: string;
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
  /// Which device of the cluster the work will be done on — `null` for this one,
  /// which is what every Verkstead outside a cluster is and what a browser that
  /// has picked nothing reads as.
  ///
  /// Held here rather than beside what is held, because the three ids under it
  /// are only ids *on this device*: a Repo id, a companion's Repo id and a
  /// Pairing's Profile id are each one Verkstead's own, so what says which
  /// machine they name is part of the same record. A draft read back with the
  /// device left off it would be a draft whose repository is a number on no
  /// particular machine.
  ///
  /// The pick itself is remembered apart from this, in the browser — see
  /// `draftingOn` in `src/remembered.ts`: the draft is dropped the moment a
  /// Conversation is made out of it, and a draft of nothing is never written
  /// down at all, so a pick kept only in here would be forgotten by the very
  /// press it was made for.
  device: Device;

  repo: number | null;
  brief: string;
  /// The branch the work will be done on, empty being the name the server
  /// invents when the Conversation is started.
  branch: string;
  /// And what the work is pointed at, empty being nothing named: a pull
  /// request URL, a `#number` or a branch, for the Processes that take one.
  ///
  /// Filled from the box by the same reading the server does when a Brief is
  /// saved — see [`written`], where the fill happens. Never over what was typed
  /// here, which is what [`Composed.filled`] beside it is for.
  target: string;
  /// What that fill last wrote into [`Composed.target`], empty until it has
  /// written anything.
  ///
  /// **Because filling once is not enough here.** The server fills a saved
  /// Brief, which is a whole document; this page fills as the box is typed
  /// into, so the first fill lands on a half-typed name — `#4` on the way to
  /// `#41` — and a field that would not correct itself would stand there
  /// naming a pull request nobody meant. So the fill owns what it wrote and
  /// goes on correcting it, and stops for good the moment the field says
  /// something else. Which is the human having typed in it, that being the one
  /// other thing that writes to the field.
  filled: string;
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
};

/// The box written into, with the Target filled out of it.
///
/// The same rule the server keeps for a Conversation's Brief, kept here
/// because this page has no Conversation to keep it for: a pull request URL or
/// a `#number` in the prose names the work, so the field shows what a press
/// would take up rather than standing empty over it — and never over what
/// somebody typed into the field itself.
///
/// **Read on every keystroke, so the fill has to be able to correct itself.**
/// A name is typed a character at a time, and `#4` is a whole name on the way
/// to `#41`: a fill that stopped at the first one would leave the field naming
/// pull request 4 over a Brief that says 41, which is the field lying about
/// what the press would find — the very thing it is drawn from the Brief to
/// stop. So it goes on reading while the field still holds what it last wrote,
/// and lets go the moment the field says anything else. See
/// [`Composed.filled`].
///
/// Only ever a fill. A Brief that names nothing leaves whatever is in the field
/// standing, because emptying the field is the human's act and not a Brief's —
/// the server's own `fill_target` never clears one either.
///
/// The reading is `targets.ts`'s, which is the server's own expression written
/// again on this side. What it decides is what is *drawn*; what the target
/// turns out to be is the server's at Start.
export function written(state: Composed, brief: string): Composed {
  const named = state.target === state.filled ? pullRequestIn(brief) : null;

  if (named === null) {
    return { ...state, brief };
  }

  return { ...state, brief, target: named, filled: named };
}
/// A compose page nobody has touched — on the device this browser last drafted
/// onto, which is this one until it has picked another.
///
/// The one field of it that is not simply empty, and for the reason the pick is
/// kept in the browser at all: the laptop that drives the desktop picks once and
/// goes on drafting there, so the page a create leaves behind is a fresh page for
/// that same machine rather than one pointed back here.
export function blank(): Composed {
  return {
    device: draftingOn(),
    repo: null,
    brief: "",
    branch: "",
    target: "",
    filled: "",
    base: null,
    companions: [],
    process: null,
    grilling: null,
    implementation: null,
    review: null,
    adopting: null,
  };
}

/// What is left of a compose page when the work moves to another device — which
/// is the device it is now for, and everything that would have named the one it
/// came off taken away.
///
/// **Everything that named the machine it came off goes.** A Repo id, a
/// companion's Repo id and a Pairing's Profile id are each one Verkstead's own
/// and collide across a cluster by construction, so the repo, the base, the
/// repos alongside and the three pairings travel nowhere: they would name rows
/// on the wrong machine. The brief, the branch name and the files being held are
/// the human's words and stay exactly where they are.
///
/// That is the move a Repo switch already makes for the base and for a companion
/// that has become the work's own Repo, made one level up — see `moveTo` in
/// `Compose.tsx`. And the pairings going with it is what puts the picked device's
/// own remembered pairings in front of the human, rather than three pickers
/// standing on a memory read off somewhere else.
///
/// The roadmap or pull request a page is loaded with is not touched here. Either
/// one is this device's own, so what loading it does is put the work *back* on
/// this device — which is this move, in the other direction, made by the caller
/// that loads it.
export function elsewhere(state: Composed, device: Device): Composed {
  return {
    ...state,
    device,
    repo: null,
    base: null,
    companions: [],
    grilling: null,
    implementation: null,
    review: null,
  };
}

/// Which Repo the work would be in: the roadmap's own where one is loaded, and
/// whatever was picked where none is.
///
/// The one reading everything about the repository is drawn off — the trigger's
/// name, the companions an add is refused for, the pairings the Repo is
/// remembered to have been grilled with.
export function on(state: Composed): number | null {
  return state.adopting?.repo_id ?? state.repo;
}

/// Whether there is nothing in it worth coming back to. An untouched page is
/// not worth storing, and whitespace is no more a brief here than it is at the
/// moment work is started.
///
/// The device is not one of the answers. A page holding a pick and nothing else
/// is a page holding nothing — the pick is the browser's and is kept there, so
/// the next visit comes back to that machine with or without a draft under it.
export function empty(state: Composed): boolean {
  return (
    state.repo === null &&
    state.brief.trim() === "" &&
    state.branch === "" &&
    state.target === "" &&
    state.base === null &&
    state.companions.length === 0 &&
    state.process === null &&
    state.grilling === null &&
    state.implementation === null &&
    state.review === null &&
    state.adopting === null
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
///
/// `stopped` is the one outcome of the replay that is not a refusal — a take-up
/// that stopped to ask what closing another conversation would discard. It travels
/// whole because what answers it is a press rather than anything to go and fix,
/// and the press is on the draft this made.
export type Created =
  | { conversation: number; refused: string[]; stopped: TakenUp | null }
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
/// **What this page is holding at its Attach control goes up in the same
/// replay** — the MCP servers attached by name, and the files one request
/// apiece through the route a draft's own paperclip uses. There is a
/// Conversation by then, so there is nothing else either of them could need.
/// They go after every field and before the kickoff: what is attached freezes
/// with the Brief, and either arriving after the grilling started would be
/// refused for being late rather than for anything the human did. **Except
/// where a roadmap is loaded**, which sends neither: the box is locked to a
/// card, and what is held is given back when the card is cleared.
///
/// **A page loaded with a roadmap creates the other kind of Conversation**, and
/// most of the replay is not asked of it: the Brief is the stage's, the branch
/// is the stage's slug and the base was fixed by the row that loaded it, so what
/// is left to put on is the companions and the pairings. What the press does at
/// the end of it is adopt rather than grill, which is the same act — the work
/// beginning — under the other name.
///
/// **And the whole of it is addressed to [`Composed.device`]** — the start, every
/// field, every file and the kickoff — because a Conversation is made on the
/// machine that will do its work. Which is one argument on each call rather than
/// a second path: the endpoints are the endpoints, and a member's are reached
/// through the Relay by a prefix the client writes (see `src/api/client.ts`).
export async function create(
  state: Composed,
  work: boolean,
  files: Holding,
  servers: Array<string>,
): Promise<Created> {
  const held = state.adopting;
  if (held === null && state.repo === null) {
    return "NoSuchRepo";
  }

  // The device the whole replay is put to, read once: it is the device the page
  // was drafting onto, and what it names has to be the same machine from the
  // start to the kickoff.
  const to = state.device;

  const started = await opened(to, state, held);
  if (started === "NoSuchRepo") {
    return started;
  }

  const id = started.Started.id;
  const refused: string[] = [];

  /// The take-up outcome that stopped to ask rather than refusing, where the
  /// kickoff was a take-up and that is what it came back with.
  ///
  /// Carried whole rather than as a sentence, because what answers it is the press
  /// on the draft this made — see the take-up below.
  let stopped: TakenUp | null = null;

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
      const outcome = await saveBrief(to, id, state.brief);
      said(
        outcome === "Saved",
        `The brief could not be saved: ${BRIEF_REFUSAL[outcome]}`,
      );
    }
  }

  // And the three the roadmap answers for itself besides the Brief: its branch
  // is the stage's slug, its base was fixed by the row that loaded it, and it is
  // pointed at nothing — adopting is not one of the Processes that take a target.
  if (held === null) {
    if (state.branch !== "") {
      const outcome = await renameBranch(to, id, state.branch);
      said(
        outcome === "Renamed",
        `The branch could not be named: ${BRANCH_REFUSAL[outcome]}`,
      );
    }

    // Whatever the Process: the field and the picker are settled
    // independently and the server takes the target off either, and which
    // Processes *wait* on one is decided over there.
    if (state.target !== "") {
      const outcome = await nameTarget(id, state.target);
      said(
        outcome === "Recorded",
        `The target could not be named: ${TARGET_REFUSAL[outcome]}`,
      );
    }

    if (state.base !== null) {
      const outcome = await setBaseBranch(to, id, state.base);
      said(
        outcome === "Recorded",
        `The base branch could not be recorded: ${BASE_REFUSAL[outcome]}`,
      );
    }
  }

  for (const alongside of state.companions) {
    await put(to, id, alongside, said);
  }

  // What kind of work it is, and only where the human touched the picker: a
  // page left on Develop sends nothing, exactly as a role left on its prefill
  // does, and the server's own reading of a Conversation with no row of its own
  // is what stands.
  if (state.process !== null) {
    const outcome = await pickProcess(id, state.process);
    said(
      outcome === "Picked",
      `The process could not be picked: ${PROCESS_REFUSAL[outcome]}`,
    );
  }

  if (state.grilling !== null) {
    const outcome = await chooseGrillingPairing(
      to,
      id,
      pairing.choice(state.grilling),
    );
    said(
      outcome === "Chosen",
      `The grilling profile could not be chosen: ${CHOICE_REFUSAL[outcome]}`,
    );
  }

  if (state.implementation !== null) {
    const outcome = await chooseImplementationPairing(
      to,
      id,
      pairing.choice(state.implementation),
    );
    said(
      outcome === "Chosen",
      `The implementation profile could not be chosen: ${CHOICE_REFUSAL[outcome]}`,
    );
  }

  if (state.review !== null) {
    const outcome = await chooseReviewPairing(
      to,
      id,
      pairing.role(state.review),
    );
    said(
      outcome === "Chosen",
      `The review profile could not be chosen: ${CHOICE_REFUSAL[outcome]}`,
    );
  }

  // And what was put on at the Attach control, last of the fields: the MCP
  // servers and then the files, each one more thing put on the Conversation,
  // and one the server would not take is one more refusal to carry — which is
  // what stops the kickoff, exactly as a refused branch name does.
  //
  // None of either on a page that loaded a roadmap, for the reason the control
  // is not offered on one: the box is locked to a card, so nothing was being
  // written for a file to be handed over with — and what was picked before the
  // roadmap was loaded was picked for a box the roadmap has since taken over.
  // What is held stays held, and clearing the roadmap gives it back.
  if (held === null) {
    // By name, the way the composer attaches one: what a Conversation records
    // is which declarations it has, and the declaration itself stays in the
    // settings. A name nothing is declared by any more is refused here rather
    // than dropped, the human having picked it on purpose.
    for (const name of servers) {
      const outcome = await attachServer(to, id, name);
      if (outcome !== "Attached") {
        refused.push(
          `${name} could not be attached: ${SERVER_REFUSAL[outcome]}`,
        );
      }
    }

    for (const rejected of await files.flush(to, id)) {
      refused.push(
        `${rejected.name} could not be attached: ${ATTACH_REFUSAL[rejected.refused]}`,
      );
    }
  }

  if (work && refused.length === 0) {
    if (held !== null) {
      const outcome = await adoptRoadmap(to, id);
      said(
        outcome === "Adopted",
        `The stage could not be started: ${adoptRefusal(outcome)}`,
      );
    } else if (state.process !== null && targeted(state.process)) {
      // The third kickoff, and the one that starts no session: the take-up reads
      // the Target, puts the Conversation on what it names and moves it into
      // Wrapping, and what runs from there is the wrap-up's own watchers.
      //
      // Asked of the Process's own list rather than of its name — a **Review**
      // and a **Fix Merge Issues** are both pointed at work that is already
      // somewhere else, and both reach it by this one endpoint. Nothing picked
      // is the Develop every draft defaults to, which is pointed at nothing.
      const outcome = await takeUpPullRequest(to, id);

      // And the one outcome that is not a refusal: the press stopped to ask what
      // closing another conversation would discard, and the answer is a press. So
      // it travels as the outcome rather than as a sentence about it — the draft's
      // own composer takes it up, draws it with a link per conversation, and the
      // first press there is the confirming one. Left as a sentence it would have
      // read *press start again to go ahead* over a press that could only ask the
      // same question over again.
      if (typeof outcome === "object" && "WouldDiscard" in outcome) {
        stopped = outcome;
      } else {
        said(
          outcome === "TakenUp",
          `The pull request could not be taken up: ${takeUpRefusal(outcome)}`,
        );
      }
    } else {
      const outcome = await startGrilling(to, id);
      said(
        outcome === "Started",
        `The work could not be started: ${grillRefusal(outcome)}`,
      );
    }
  }

  return { conversation: id, refused, stopped };
}

/// The Conversation this page's press makes, which is one of two starts.
///
/// Two endpoints rather than one with a shape inside it, for the reason every
/// other field here goes through the endpoint that already existed: what a
/// Conversation is started *over* is a different question in each case — a Repo,
/// or a roadmap in one — and each of them is refused for its own reasons.
///
/// Both on the device the page is drafting onto: the Repo and the roadmap are
/// each read off that device's registry, so that is where the Conversation over
/// them is made.
async function opened(
  device: Device,
  state: Composed,
  held: Adopting | null,
): Promise<Started> {
  if (held !== null) {
    return startAdoption(device, held.repo_id, held.roadmap, held.base);
  }

  return startConversation(device, state.repo!);
}

/// One companion, put on the Conversation and then configured: the add first,
/// because everything after it is about the row the add makes, and nothing after
/// it where the add was refused.
async function put(
  device: Device,
  id: number,
  alongside: Alongside,
  said: (ok: boolean, sentence: string) => boolean,
): Promise<void> {
  const added = await addCompanion(device, id, alongside.repo_id);
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
      device,
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
      device,
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
      device,
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

/// The device a draft is moving onto, said the two ways a move needs it.
///
/// **Both, because they are different facts.** How this browser *reaches* a
/// device is `null` for the one it opened and the Device Id for every other
/// (see [`Device`] in `src/reaching.ts`), which is what every call in the app is
/// addressed by; what the device *is* is an id and a name, which is what the
/// words left on the old draft's Timeline say. A move to this device has both —
/// `null` to reach it by and an id like anybody else's — so one field could not
/// have carried the pair.
export type Target = {
  /// How the calls of the move are addressed: `null` for the device this
  /// browser opened.
  reaching: Device;

  /// And what the device is, off the membership this browser read — see
  /// `deviceShown` in `src/devices.ts`.
  identity: { device: string; name: string };
};

/// Move a saved draft onto another device: the same replay, run against a
/// Conversation that already exists.
///
/// **The human chose a move over a select that reads settled** (ADR-0020,
/// *Drafting on a device*): a draft started on the wrong machine is worth moving
/// rather than worth making again. So the Brief, the branch name and the files
/// are read off the draft here and written onto a new Conversation over there,
/// through the endpoints [`create`] uses and in the order it uses them — there
/// is no second way of putting a Brief on a Conversation, and no second set of
/// refusals to word.
///
/// **What cannot travel says nothing, because it was never the human's to say.**
/// A Repo id, a companion's Repo id and a Pairing's Profile id are each one
/// Verkstead's own — the rule [`elsewhere`] is written under, one Conversation
/// along. So:
///
/// - the **Repo** is picked as part of the move, by the caller, out of the
///   target's own registry;
/// - the **companions** are ids in the old device's registry and are left
///   behind;
/// - and the **Pairings** are Profile ids on the old machine, so the new draft
///   arrives showing the target's own remembered pairings — its prefill, which
///   is exactly what a draft created there would have arrived showing.
///
/// **The base travels, because it is not one of those.** An override is a branch
/// *name* picked out of that repository's own list rather than an id — see
/// `BasePicker` in `Setup.tsx` — so it means what it means on whichever machine
/// the repository is cloned on, which is the move this is nearly always: the
/// same repository, over there. A target that has not got that branch refuses it
/// by name, exactly as the branch above it is refused, and the rule is what a
/// draft with no override arrives under.
///
/// **The files travel**, because they are the human's own bytes: each is read
/// back off the device the draft is on — see `readAttachment` in
/// `src/api/client.ts`, the one read of an attachment there is — and put on the
/// new Conversation through the route a paperclip uses. One the target will not
/// take is one more refusal to carry, and so is one this device could not read.
///
/// **Nothing is undone by a refusal**, which is [`create`]'s rule again. The new
/// Conversation is real from its first request, so a field the target would not
/// take leaves the rest of the work on it — and the old draft is closed only
/// where nothing was refused at all: a move that left something behind is one
/// the human has to be able to look at both ends of. A start the target would
/// not make leaves both drafts exactly where they were, and a close that could
/// not be made is the last refusal to carry rather than a move to make again.
export async function moveTo(
  from: Device,
  draft: ConversationView,
  brief: string,
  to: Target,
  repo: number,
): Promise<Created> {
  const started = await startConversation(to.reaching, repo);
  if (started === "NoSuchRepo") {
    return started;
  }

  const id = started.Started.id;
  const refused: string[] = [];

  const said = (ok: boolean, sentence: string) => {
    if (!ok) refused.push(sentence);
    return ok;
  };

  if (brief.trim() !== "") {
    const outcome = await saveBrief(to.reaching, id, brief);
    said(
      outcome === "Saved",
      `The brief could not be saved: ${BRIEF_REFUSAL[outcome]}`,
    );
  }

  // The name the human settled on rather than whatever the record is carrying:
  // a branch nobody chose is Verkstead's own invention, and the new Conversation
  // was started under an invention of the target's — see `chosen` in
  // `naming.ts`, which is the one rule for which of the two is worth drawing.
  const branch = chosen(draft);
  if (branch !== "") {
    const outcome = await renameBranch(to.reaching, id, branch);
    said(
      outcome === "Renamed",
      `The branch could not be named: ${BRANCH_REFUSAL[outcome]}`,
    );
  }

  // And the base the human overrode the rule with, which is a branch name and so
  // is theirs rather than one machine's — the record carries it on
  // `base_commit`, and `null` there is the rule itself, which is what the new
  // Conversation already arrived under.
  if (draft.base_commit !== null) {
    const outcome = await setBaseBranch(to.reaching, id, draft.base_commit);
    said(
      outcome === "Recorded",
      `The base branch could not be recorded: ${BASE_REFUSAL[outcome]}`,
    );
  }

  for (const attachment of draft.attachments) {
    await carry(from, draft.id, to.reaching, id, attachment, refused);
  }

  // And the draft this came off, closed with the words that say where its work
  // went — last of all, and only where the whole of it landed: a move that left
  // something behind is a move the human finishes by hand, and a draft closed
  // under them would be the half that did not arrive with nowhere left to read
  // it from.
  //
  // And a close that could not be made at all is one more refusal to carry
  // rather than a move to make again: the Conversation over there is real by
  // now, so a press that answered with *the move failed* would be one the human
  // could press a second time and have two of the work. What is said instead is
  // said on the draft that arrived, which is where they are about to be looking.
  if (refused.length === 0) {
    try {
      await draftMovedTo(from, draft.id, {
        device: to.identity.device,
        name: to.identity.name,
        conversation: id,
      });
    } catch (error: unknown) {
      refused.push(
        `The work is here, but the draft it came off could not be closed: ${
          error instanceof Error ? error.message : String(error)
        }`,
      );
    }
  }

  // A move takes nothing up, so there is never a take-up stopped to ask.
  return { conversation: id, refused, stopped: null };
}

/// One file carried from the draft to the Conversation it is becoming.
///
/// Two requests rather than one — the bytes off the old device, and the same
/// bytes onto the new one — because there is no hop between two members: the
/// browser is what both of them are reached through (ADR-0020, *The opened
/// device relays*), so the file comes here and goes back out.
///
/// Either half can refuse, and the two are one sentence to the human: the file
/// did not arrive. They are worded apart all the same, because what to go and do
/// about them is different — a file the target refused is one the human can
/// attach again by hand, and one that could not be read is a file on a machine
/// that has stopped answering.
async function carry(
  from: Device,
  draft: number,
  to: Device,
  conversation: number,
  attachment: AttachmentView,
  refused: string[],
): Promise<void> {
  let file: File;

  try {
    file = await readAttachment(from, draft, attachment.id, attachment.name);
  } catch (error: unknown) {
    refused.push(
      `${attachment.name} could not be read off this conversation: ${
        error instanceof Error ? error.message : String(error)
      }`,
    );
    return;
  }

  const outcome = await attachFile(to, conversation, file);
  if (typeof outcome === "string") {
    refused.push(
      `${attachment.name} could not be attached: ${ATTACH_REFUSAL[outcome]}`,
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
///
/// **Against the Conversation *and* its device**, which is how a row of the
/// merged sidebar is named and for the same reason: ids collide by construction,
/// every Verkstead issuing a Conversation 1, so refusals left against a bare
/// number would be drawn on the composer of an unrelated draft the moment two
/// devices are in play. See `rowKey` in `src/reaching.ts`.
///
/// And the take-up that stopped to ask beside them, for the same reason and for
/// one more: it is answered by a press rather than by anything to go and fix, so
/// the draft's own take-up picks it up as the outcome it is and the press there is
/// the confirming one. See [`stoppedOnCreate`].
const [replayed, setReplayed] = createSignal<{
  row: string;
  refused: string[];
  stopped: TakenUp | null;
} | null>(null);

/// What the create that made this Conversation could not do, in the words its
/// own pane would have used — and nothing at all for every other Conversation,
/// which is all of them but the one just made.
export function refusedOnCreate(device: Device, id: number): string[] {
  const left = replayed();
  return left !== null && left.row === rowKey(device, id) ? left.refused : [];
}

/// And the take-up the create stopped over, for this Conversation's own press to
/// go on from — `null` for every other Conversation and for every create that was
/// not stopped.
///
/// Read once, when the composer's take-up starts: what it becomes is that press's
/// own state, and everything after it is the press answering itself.
export function stoppedOnCreate(device: Device, id: number): TakenUp | null {
  const left = replayed();
  return left !== null && left.row === rowKey(device, id) ? left.stopped : null;
}

/// Leave them for that Conversation's composer, or take away what was left for
/// the one before it.
export function leaveRefusals(
  device: Device,
  conversation: number,
  refused: string[],
  stopped: TakenUp | null,
): void {
  setReplayed(
    refused.length === 0 && stopped === null
      ? null
      : { row: rowKey(device, conversation), refused, stopped },
  );
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
    !onDevice(held.device) ||
    !whole(held.repo) ||
    typeof held.brief !== "string" ||
    typeof held.branch !== "string" ||
    // A body from a build before these two has neither, which is nothing named
    // and nothing filled rather than a fault — [`loaded`]'s absence, read the
    // same way. A `target` from such a body reads as the human's, which is what
    // a `filled` of nothing says: whatever put it there, the fill does not own
    // it and will not write over it.
    !(held.target === undefined || typeof held.target === "string") ||
    !(held.filled === undefined || typeof held.filled === "string") ||
    !(held.base === null || typeof held.base === "string") ||
    !Array.isArray(held.companions) ||
    !kind(held.process) ||
    !picked(held.grilling) ||
    !picked(held.implementation) ||
    !picked(held.review) ||
    !loaded(held.adopting)
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
    // A body from a build before there was a cluster names no device at all,
    // which reads as whatever this browser is drafting onto rather than as this
    // machine: a draft mid-sentence keeps the pick it was being written under,
    // that pick having been kept in the browser all along. A body that says
    // `null` *has* said — this device — and is read as what it says.
    device: held.device === undefined ? draftingOn() : held.device,
    repo: held.repo,
    brief: held.brief,
    branch: held.branch,
    target: held.target ?? "",
    filled: held.filled ?? "",
    base: held.base,
    companions,
    process: held.process ?? null,
    grilling: held.grilling,
    implementation: held.implementation,
    review: held.review,
    adopting: held.adopting ?? null,
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
    // A body from before the press started more than one stage has none, which
    // is the same absence `adopting` itself is: it reads as nothing ready
    // beside the stage the card names, and the press reads the roadmap again
    // anyway.
    (roadmap.beside === undefined || Array.isArray(roadmap.beside)) &&
    typeof roadmap.base === "string"
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

/// Whether this is a Device Id, this device, or a body that never said — the
/// last being a draft written before there was a device to say.
function onDevice(value: unknown): value is Device | undefined {
  return value === null || value === undefined || typeof value === "string";
}

/// Whether this is a repo id or the absence of one.
function whole(value: unknown): value is number | null {
  return value === null || typeof value === "number";
}

/// And whether this is a role's choice or the absence of one.
function picked(value: unknown): value is string | null {
  return value === null || typeof value === "string";
}
