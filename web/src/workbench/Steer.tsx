//! Steering a conversation: the modal the menu row opens, and what it settles.
//!
//! The click has already happened by the time any of this is drawn — see the
//! menu row in [`Timeline`](./Timeline.tsx), which posts it and opens this on
//! what comes back. That press stopped the drive, so nothing new is launching
//! while the human composes, and **cancel** is no press at all: the conversation
//! stays where the click left it, stopped, with resume drawn on it.
//!
//! What the modal is, therefore, is a form over one question — where does this
//! go? — with whatever that target needs under it. **Done** needs nothing: there
//! is nothing to drive in done, so no pairing is picked and no payload is
//! carried, and the submit is the move alone. **Wrapping** needs no payload
//! either — the wrap-up's watchers work out for themselves what is left to do —
//! but it does need a pairing, because sessions run there. **Grilling** carries
//! a payload: a new brief, and a choice about how much of the last interview the
//! session is primed with. **Implementing** carries another: an instruction,
//! which is what the session it starts is sent off to do. **Follow-up** carries
//! the third: the brief the session that follows the pull request up is opened
//! on.
//!
//! **The first two are required exactly where nothing else answers for them.**
//! Writing nothing under grilling means grill the brief that is already written,
//! so it is required where none is — a grilling starts from a brief, and the one
//! a steered round lands with is frozen where it lands. Writing nothing under
//! implementing means carry on what the branch already holds, so it is required
//! where it holds nothing. **The third is required always**: nothing on the
//! branch stands in for a follow-up, a follow-up being a thing the human wanted
//! rather than a step of the run. Either way the submit is held shut rather than
//! offered and then refused.
//!
//! **The pairing is the conversation's, not the session's.** It is prefilled
//! from what the conversation already runs the work under and what is picked is
//! recorded as the conversation's own: steering re-settles what runs the work.
//! A steered draft has none fixed yet, which is why the pick is part of the form
//! rather than an error path. Which role the picker settles follows the target:
//! a grilling runs under the grilling one, and everything that builds runs
//! under the implementation one. Wrapping up reaches the review role from the
//! same pick, a wrap-up both building and reviewing — but only to fill one
//! nothing was picked for: the picker is prefilled with what builds, so a human
//! who changes nothing has said nothing about the review, and an account they
//! chose to be a fresh set of eyes is not quietly replaced by whatever built
//! the work.
//!
//! **Interrupt current task** is the one thing here that is about the world
//! rather than about the move. The click left whatever was running exactly where
//! it was; ticking the box ends it where it stands, and the step is left however
//! far it had got.
//!
//! Left alone it is seen out, and what *out* means follows the target. One
//! worktree holds one agent, so a target something runs in ends it by starting:
//! the session this steer launches takes the worktree over, at once where it can
//! and once the session in front of it has finished where it cannot — a review
//! waiting on an ask. **Done** launches nothing, so there it runs to its own end
//! and the box is the only thing that would stop it. The box is drawn only where
//! the click found a session running — there is otherwise nothing to interrupt.

import { useMutation, useQueryClient } from "@tanstack/solid-query";
import { For, Show, createMemo, createSignal, type JSX } from "solid-js";

import { listProfiles, listRepos, steer } from "../api/client";
import type {
  CompanionAddition,
  CompanionMode,
  CompanionUpgrade,
  CompanionView,
  ConversationSteered,
  ConversationView,
  RepoEntry,
  SteerCompanionRefusal,
  SteerTarget,
} from "../api/types";
import { useReading } from "../freshness";
import { Empty, ErrorLine, Note } from "../notices";
import { Modal } from "../Modal";
import * as pairing from "../pairing";
import { Listbox } from "../picking";
import { Switch as Toggle } from "../Switch";
import { chosen } from "./naming";
import { BasePicker, RULE } from "./Setup";
import styles from "./Steer.module.css";

/// Each way of being refused a steer, for the conversation's own repo.
///
/// Nothing here is about the state the conversation was in: every state is
/// somewhere to steer *from*, so what is left to be wrong about is the target —
/// a state whose work cannot be set going from what the record holds.
export const STEER_REFUSAL: Record<
  Exclude<ConversationSteered, { Companion: unknown }>,
  string
> = {
  Steered: "",
  NoSuchConversation: "This conversation is gone.",
  NoPullRequest:
    "This work is on no pull request, so there is no wrap-up to steer it into.",
  NoInstruction:
    "There is nothing on this branch to carry on — no backlog with work left in it, and no roadmap it has written — so write what to do.",
  NoFollowUpBrief:
    "A follow-up is whatever you want taken up about this pull request, so write what that is.",
  EmptyBrief:
    "There is no brief to grill: this conversation has none written yet, so write the one this round is about.",
  NoPairing: "Pick the account and model the work runs under from here.",
  NoSuchProfile: "That profile has been removed.",
  NoSuchModel: "That profile no longer lists that model.",
  NoBaseCommit:
    "Nothing in the repository answers to what this branch would come off. Fix the base branch and steer it again.",
  WorktreeRefused:
    "Its worktree is not one any more, and git would not make it again from the branch.",
  // Both halves of the companion section can reach this — a row ticked to go
  // in, and a row ticked to be opened up — so it says which repo was picked
  // rather than what was being done with it. It is also the one companion
  // refusal with no name in it: an unregistered repo is a row Verkstead knows
  // nothing about but the id the page sent.
  NoSuchCompanionRepo:
    "One of the repos you picked to work alongside is not registered any more, so nothing was steered.",
};

/// And each way one companion could not be put into the sandbox, which says the
/// same kinds of thing about a different repository.
const STEER_COMPANION_REFUSAL: Record<SteerCompanionRefusal, string> = {
  OwnRepo:
    "This conversation's own repo is already the work's, so it cannot go in beside itself.",
  AlreadyAdded: "It is already on this conversation.",
  NotACompanion: "It is not on this conversation, so there is nothing to open.",
  AlreadyReadWrite:
    "It is read-write on this conversation already, which is as open as a repo gets.",
  FetchFailed:
    "Git could not fetch from its remote, so nothing was steered. The server log says why.",
  NoBaseCommit: "It has nothing to check out any more.",
  BranchExists:
    "The branch already exists there, and Verkstead did not make it.",
  WorktreeRefused: "Git would not make its worktree. The server log says why.",
};

/// What to say about a steer that was refused.
///
/// A companion's refusal names the repository, because that is the whole of what
/// makes it different from the same failing on the conversation's own: the thing
/// to go and look at is one of several repos rather than the obvious one. The
/// grill start's own refusals are drawn the same way — see `grillRefusal` in
/// [`Timeline`](./Timeline.tsx).
export function steerRefusal(outcome: ConversationSteered): string {
  if (typeof outcome === "object") {
    return `${outcome.Companion.repo}: ${STEER_COMPANION_REFUSAL[outcome.Companion.why]}`;
  }

  return STEER_REFUSAL[outcome];
}

/// Where a steer can send a conversation, and what each target means.
///
/// Draft and Closed are not here and never will be: each has a way in of its
/// own; follow-up is here because a steer is the only way into it at all.
/// Wrapping up and follow-up are the two that are not always offered, which is
/// what `offered` below draws them out by: a conversation whose work is on no
/// pull request has no wrap-up to be steered into and nothing to follow up, and
/// following up is for work the pipeline has seen through rather than work still
/// being built.
///
/// `runs` is whether work goes on in that state, which is the one question the
/// rest of the form follows from: a target something runs in needs a pairing
/// settled, and one nothing runs in needs none. `role` is which pairing that
/// is, there being one for the interviewing, one for everything that builds and
/// one for the review — and wrapping up settles the review one alongside the
/// building one from the same pick, a wrap-up doing both.
///
/// In the order the work goes through them, because that is the order the human
/// reads the pipeline in everywhere else.
const TARGETS: {
  target: SteerTarget;
  label: string;
  note: string;
  runs: boolean;
  role?: "grilling" | "implementation";
}[] = [
  {
    target: "Grilling",
    label: "Grilling",
    note: "A new round: the work interviewed again, from a fresh brief if you write one. Whatever is missing is made — the branch for a draft, the worktree for a conversation that has been closed.",
    runs: true,
    role: "grilling",
  },
  {
    target: "Implementing",
    label: "Implementing",
    note: "The work built: what you write here done first, and then whatever the branch holds — the next task of its backlog, or the pull request wrapped up again.",
    runs: true,
    role: "implementation",
  },
  {
    target: "Wrapping",
    label: "Wrapping up",
    note: "The branch looked at again: the checks watched, the review run, the comments answered. What you pick runs the fixes, and the review too where nothing was picked for it. The fix attempts start over.",
    runs: true,
    role: "implementation",
  },
  {
    target: "FollowUp",
    label: "Follow-up",
    note: "The pull request followed up on: a session that answers what you ask, does what you want done about it, and keeps asking what else there is until you are finished.",
    runs: true,
    role: "implementation",
  },
  {
    target: "Done",
    label: "Done",
    note: "Finished with. Nothing runs, so there is nothing to pick and nothing to write.",
    runs: false,
  },
];

/// Whether this conversation's work is on a pull request.
///
/// What decides whether wrapping up is offered at all, and what decides whether
/// the actions menu offers sharing to one — see `Actions.tsx`, which asks the
/// same question of the same cards rather than keeping a second answer to it.
///
/// A wrapping conversation is defined by the pull request under it — the record
/// holds the move and the pull request as one act — so a steer there is a move
/// onto one that is already there rather than a way of opening one. Read off the
/// pinned events, which is where the record's own pull request is drawn from.
export function onAPullRequest(conversation: ConversationView): boolean {
  return conversation.pinned.some((pinned) => "PullRequest" in pinned);
}

/// Whether there is a brief for a steer into grilling to start a round on.
///
/// The newest one on the timeline, which is the round's own — a conversation
/// gets a brief per round, and a steered one adds a second beside the first
/// rather than editing it. A grilling starts from a brief, so where this is
/// false the modal's brief field is what the target *is*, and the server
/// refuses a submit without one by name.
///
/// Empty is the ordinary draft: every conversation is created with a brief
/// nobody has written into yet.
function briefStands(conversation: ConversationView): boolean {
  const briefs = conversation.timeline.flatMap((event) =>
    "Brief" in event ? [event.Brief] : [],
  );

  return (briefs[briefs.length - 1]?.markdown.trim() ?? "") !== "";
}

/// What one row of the companion section is holding while it is filled in.
///
/// The three things the setup card's row settles about a companion, kept here
/// rather than saved a press at a time: the card writes each of them as it is
/// touched because a drafting conversation is there to be edited, and this is
/// part of one submit that either lands whole or does not happen.
type Addition = {
  mode: CompanionMode;
  /// The branch its checkout comes off, as the picker writes it: the empty
  /// string is the rule — that repo's default branch as origin holds it.
  base: string;
  /// What a read-write one's branch is called, empty being *mirroring*: the
  /// conversation's own branch name.
  branch: string;
};

/// A row the human has just ticked, before they have said anything else about
/// it: read-only, off the default branch, with no branch of its own.
///
/// The defaults the setup card's *Add companion repo* uses, because they are the
/// same defaults for the same reason — the least a human has to say to put a
/// repository in.
const PLAINEST: Addition = { mode: "ReadOnly", base: RULE, branch: "" };

/// And what one row of the set already there is holding while it is opened up.
///
/// One field rather than three, and the missing two are the point. There is no
/// mode, because there is one direction — read-write, or the row is left alone.
/// And there is no base: what the new branch comes off is the base already on
/// the row, re-resolved at the steer because the repo is joining the work now.
type Upgrade = {
  /// What the branch cut in it is called, empty being *mirroring*: the
  /// conversation's own branch name.
  branch: string;
};

/// A row the human has just ticked up, before they have named anything: the
/// branch mirrors the conversation's, which is what a setup row starts on.
const MIRRORING: Upgrade = { branch: "" };

/// The repos this conversation works alongside, and the ones it could.
///
/// **Sandbox setup rather than a property of one state**, which is why it is
/// drawn under every target work goes on in rather than under one of them: what
/// it settles is the world the sessions to come run in. Under done there is
/// nothing running and so nothing a companion could be for, and the section is
/// not drawn at all.
///
/// **The set already there is something to read, and a read-only row of it is
/// something to open.** The setup rows that configured it went when the card
/// froze, and this is the one other moment those questions can be asked — of a
/// repository joining now, and of one that came in read-only and is joining the
/// work properly now. Nothing here offers removal and no switch offers
/// read-only: the frozen set only widens and a row only opens further, which is
/// what keeps the sandbox story simple.
///
/// **A repository already on the conversation is not offered below**, unlike the
/// setup card's own menu, which offers everything and refuses by name. The set
/// is drawn directly above these rows, so a second row for a repo that is
/// already listed would be the same list disagreeing with itself.
function Companions(props: {
  conversation: ConversationView;
  /// What has been ticked so far, by the Repo's id.
  added: Record<number, Addition>;
  /// A row ticked, changed, or unticked — `null` takes it off again.
  settle: (repo: number, addition: Addition | null) => void;
  /// And which of the ones already there have been ticked up, by the same id.
  upgraded: Record<number, Upgrade>;
  /// One of those ticked up, renamed, or put back — `null` leaves it read-only.
  open: (repo: number, upgrade: Upgrade | null) => void;
  disabled: boolean;
}): JSX.Element {
  const repos = useReading(() => ({
    queryKey: ["repos"],
    queryFn: listRepos,

    // Merged by the id each row carries flat: a rebuilt row is a new element,
    // and a nudge landing while the human is filling one in would take what
    // they had typed with it.
    freshness: { reconcile: "id" },
  }));

  /// Everything registered that is not on this conversation already — and not
  /// its own repo, which is the work's repository rather than something beside
  /// it.
  const offered = createMemo(() => {
    const already = new Set(
      props.conversation.companions.map((companion) => companion.repo.id),
    );

    already.add(props.conversation.repo.id);

    return (repos.data ?? []).filter((repo) => !already.has(repo.id));
  });

  return (
    <fieldset class={styles.steerCompanions}>
      <legend>Repos alongside</legend>

      <Show when={props.conversation.companions.length}>
        <ul class={styles.steerAlongside} aria-label="Repos already alongside">
          <For each={props.conversation.companions}>
            {(companion) => (
              <Alongside
                conversation={props.conversation}
                companion={companion}
                upgrade={props.upgraded[companion.repo.id]}
                open={(upgrade) => props.open(companion.repo.id, upgrade)}
                disabled={props.disabled}
              />
            )}
          </For>
        </ul>
      </Show>

      <Show
        when={!repos.isError}
        fallback={
          <ErrorLine class={styles.failure}>
            Could not read the repos: {repos.error?.message}
          </ErrorLine>
        }
      >
        <Show
          when={offered().length}
          fallback={
            <Empty class={styles.nothing}>
              {props.conversation.companions.length
                ? "Every registered repo is already alongside this one."
                : "No other repo is registered to work alongside."}
            </Empty>
          }
        >
          <ul class={styles.steerAdding} aria-label="Repos to add">
            <For each={offered()}>
              {(repo) => (
                <Adding
                  conversation={props.conversation}
                  repo={repo}
                  addition={props.added[repo.id]}
                  settle={(addition) => props.settle(repo.id, addition)}
                  disabled={props.disabled}
                />
              )}
            </For>
          </ul>
        </Show>
      </Show>

      <Note class={styles.fieldNote}>
        What goes in is checked out as the steer lands and stays for the rest of
        this conversation, and a repo opened up is cut a branch off its base as
        that stands now. Nothing here takes a repo away or closes one back down:
        what a session has been given is not taken back.
      </Note>
    </fieldset>
  );
}

/// One repository this conversation already works alongside: what it is called,
/// how far into it the work reaches, what its checkout came off — and, where it
/// came in read-only, the one thing about it that can still be changed.
///
/// **The mode and the base are something to read.** Both were settled while the
/// conversation drafted, and a steer widens the set and opens a row rather than
/// rewriting what is in it.
///
/// **A read-only row offers the upgrade, and a read-write one offers nothing**,
/// being already as open as a companion gets. There is no switch back either
/// way: what a session has been given is not taken back, so the control is a
/// tick that opens rather than a toggle with two ends.
function Alongside(props: {
  conversation: ConversationView;
  companion: CompanionView;
  /// What this row holds once it has been ticked up, or `undefined` where it
  /// has not been.
  upgrade: Upgrade | undefined;
  open: (upgrade: Upgrade | null) => void;
  disabled: boolean;
}): JSX.Element {
  /// Whether the work may write in it once this steer lands: what the record
  /// says, or what the tick has just asked for.
  const writing = () =>
    props.companion.mode === "ReadWrite" || props.upgrade !== undefined;

  return (
    <li class={styles.steerAlong}>
      {/* Only the name is picked out. The mode and the base read as the quiet
          half of the line, which is what the row's own rule already makes
          them — so they carry no class of their own to say it again. */}
      <span class={styles.steerAlongName}>{props.companion.repo.name}</span>
      <span>{writing() ? "read-write" : "read-only"}</span>
      <span>
        off{" "}
        {props.companion.base_ref ?? props.companion.repo.default_branch}
      </span>

      {/* Only on a read-only row. A read-write one is already as open as a
          companion gets, so there is nothing here for it to offer. */}
      <Show when={props.companion.mode === "ReadOnly"}>
        <label class={styles.steerOpenUp}>
          <input
            type="checkbox"
            checked={props.upgrade !== undefined}
            disabled={props.disabled}
            onChange={(event) =>
              props.open(event.currentTarget.checked ? MIRRORING : null)
            }
          />
          Open it up
        </label>

        <Show when={props.upgrade}>
          {(upgrade) => (
            <div class={styles.steerOpenBranch}>
              <label for={`steer-open-${props.companion.repo.id}-branch`}>
                Branch in {props.companion.repo.name}
              </label>
              {/* Filled in with what has been typed, or with the conversation's
                  own branch, which is what mirroring comes to — exactly as an
                  added row's is, so what the human reads is what they get. */}
              <input
                id={`steer-open-${props.companion.repo.id}-branch`}
                type="text"
                value={upgrade().branch || chosen(props.conversation)}
                disabled={props.disabled}
                onInput={(event) =>
                  props.open({ branch: event.currentTarget.value })
                }
              />
              <Note class={styles.fieldNote}>
                Cleared, it follows this conversation's own branch. It is cut
                from this repo's base as that stands now — the detached checkout
                it has been read through goes.
              </Note>
            </div>
          )}
        </Show>
      </Show>
    </li>
  );
}

/// One repository that could go in, and everything to say about it if it does.
///
/// The tick is what puts it in the submit; until it is ticked the row says only
/// that the repository is registered. What opens under it is what the setup
/// card's row asks — how far in, off which branch, and under what name — because
/// this is the same question asked at the one other moment it can be.
function Adding(props: {
  conversation: ConversationView;
  repo: RepoEntry;
  /// What this row holds, or `undefined` where it has not been ticked.
  addition: Addition | undefined;
  settle: (addition: Addition | null) => void;
  disabled: boolean;
}): JSX.Element {
  /// What is in the branch field: what has been typed, or the conversation's
  /// own branch, which is what *mirroring* comes to. Drawn filled in rather than
  /// empty, exactly as the setup card's is, so what the human reads is what they
  /// will get.
  const branch = () => props.addition?.branch || chosen(props.conversation);

  return (
    <li class={styles.steerAdd}>
      <label class={styles.steerAddName}>
        <input
          type="checkbox"
          checked={props.addition !== undefined}
          disabled={props.disabled}
          onChange={(event) =>
            props.settle(event.currentTarget.checked ? PLAINEST : null)
          }
        />
        {props.repo.name}
      </label>

      <Show when={props.addition}>
        {(addition) => (
          <div class={styles.steerAddConfig}>
            <Toggle
              label={<>Read-write</>}
              on={addition().mode === "ReadWrite"}
              disabled={props.disabled}
              flip={(on) =>
                props.settle({
                  ...addition(),
                  mode: on ? "ReadWrite" : "ReadOnly",
                  // A branch name left behind on a row flipped back to
                  // read-only would be a name for a branch nobody will cut: a
                  // read-only checkout is detached and holds none.
                  branch: on ? addition().branch : "",
                })
              }
            />

            <BasePicker
              id={`steer-companion-${props.repo.id}-base`}
              label={<>Base for {props.repo.name}</>}
              repo={props.repo}
              chosen={addition().base}
              disabled={props.disabled}
              pick={(picked) =>
                props.settle({ ...addition(), base: picked ?? RULE })
              }
            />

            {/* Only where there is a branch to name. A read-only companion is
                checked out detached and takes no name in somebody else's
                repository. */}
            <Show when={addition().mode === "ReadWrite"}>
              <div class={styles.steerAddBranch}>
                <label for={`steer-companion-${props.repo.id}-branch`}>
                  Branch in {props.repo.name}
                </label>
                <input
                  id={`steer-companion-${props.repo.id}-branch`}
                  type="text"
                  value={branch()}
                  disabled={props.disabled}
                  onInput={(event) =>
                    props.settle({
                      ...addition(),
                      branch: event.currentTarget.value,
                    })
                  }
                />
                <Note class={styles.fieldNote}>
                  Cleared, it follows this conversation's own branch.
                </Note>
              </div>
            </Show>
          </div>
        )}
      </Show>
    </li>
  );
}

/// The modal, and everything it settles before the move.
export function Steer(props: {
  conversation: ConversationView;
  /// Whether the click found a session still running, which is the only thing
  /// **Interrupt current task** is offered against.
  working: boolean;
  /// Said when the modal has gone, however it went — cancelled, escaped,
  /// pressed away, or submitted.
  close: () => void;
}): JSX.Element {
  const queries = useQueryClient();

  /// The targets this conversation can actually be sent to. Wrapping up is
  /// drawn out where the work is on no pull request: a target that would be
  /// refused by name is worse than one that was never offered.
  ///
  /// Implementing is not drawn out anywhere, however little the branch holds —
  /// an instruction can always be written, and what that instruction says is
  /// the one thing about this modal nothing can work out in advance.
  const offered = createMemo(() =>
    TARGETS.filter((offered) => {
      switch (offered.target) {
        case "Wrapping":
          return onAPullRequest(props.conversation);
        // And the same pull request, plus the work having been seen through:
        // following up is for a conversation the pipeline has finished with or
        // is finishing with, and one still building has the ordinary ways of
        // saying what to do next.
        case "FollowUp":
          return (
            onAPullRequest(props.conversation) &&
            (props.conversation.state === "Done" ||
              props.conversation.state === "Wrapping")
          );
        default:
          return true;
      }
    }),
  );

  // Where it goes. Prefilled with the first target offered rather than left
  // empty: a picker with nothing picked would be a form the human has to answer
  // twice. The list is never empty — done is offered on every conversation there
  // is — and the fallback is that same target said twice rather than a state
  // this can be in.
  const [target, setTarget] = createSignal<SteerTarget>(
    offered()[0]?.target ?? "Done",
  );

  /// Whether the target picked is one work goes on in, which is what draws the
  /// pairing picker under it.
  const runs = createMemo(
    () => offered().find((one) => one.target === target())?.runs ?? false,
  );

  /// And which role that picker settles — the one the target's sessions run
  /// under, and for wrapping up the review role beside it.
  const role = createMemo(
    () => offered().find((one) => one.target === target())?.role,
  );

  // The profile list is read here rather than passed in, so the picker is whole
  // wherever the modal is opened from — the setup pane does the same.
  const profiles = useReading(() => ({
    queryKey: ["profiles"],
    queryFn: listProfiles,

    // Merged by the id each row carries flat: a rebuilt `<option>` is a new
    // element in a `<select>` the human may have open, and a list re-read while
    // they were choosing would take the choice with it.
    freshness: { reconcile: "id" },
  }));

  // What the work runs under from here, prefilled from what the conversation
  // already runs it under. The empty string is a conversation with none fixed
  // yet — a steered draft — which is a pick to make rather than an error.
  //
  // One per role rather than one shared: the two are different choices about
  // different work, and a pick made for a grilling that followed the human over
  // to wrapping up would be the form answering a question they had not been
  // asked.
  //
  // The Pairing behind the grilling pick rather than the pick itself: a
  // conversation whose human chose "No grilling" has no account to prefill this
  // with, and steering into a grilling is asking for an interview — so that row
  // is not one this picker offers, and the field opens empty for them to pick
  // who runs it.
  const [grilling, setGrilling] = createSignal(
    pairing.chosen(pairing.under(props.conversation.grilling_pairing)),
  );
  const [implementation, setImplementation] = createSignal(
    pairing.chosen(props.conversation.implementation_pairing),
  );

  /// The one the target picked runs under, and nothing where nothing runs.
  const picked = createMemo(() =>
    role() === "grilling" ? grilling() : role() ? implementation() : "",
  );

  const pick = (chosen: string) =>
    role() === "grilling" ? setGrilling(chosen) : setImplementation(chosen);

  /// The new round's brief, for a steer into grilling.
  ///
  /// Optional where a brief already stands, and empty is the ordinary case
  /// there: the round starts on the one that is already written. Required where
  /// none does — a draft nobody has written into — because a grilling starts
  /// from a brief and there would otherwise be nothing to interview about.
  /// What is typed here lands as a brief of its own, frozen the moment it does.
  const [brief, setBrief] = createSignal("");

  /// Whether one is already written, which is what makes the field optional.
  const stands = createMemo(() => briefStands(props.conversation));

  /// And whether the session is primed with everything already answered.
  ///
  /// Off to begin with, because the steer is usually a change of direction: a
  /// fresh brief primed with the whole of the last interview would be steering
  /// into the argument that has just been left behind.
  const [digest, setDigest] = createSignal(false);

  /// The hand-written work, for a steer into implementing.
  ///
  /// Required where the branch holds nothing to carry on and optional where it
  /// does — the server’s own reading of the branch rather than anything worked
  /// out from the pinned backlog here: what stands includes the finish step a
  /// list of ticked tasks still has to run, which no reading of the entries
  /// could see.
  const [instruction, setInstruction] = createSignal("");

  /// And the brief, for a steer into follow-up.
  ///
  /// Always required, unlike the two above it: an empty instruction carries the
  /// branch on and an empty brief grills the one already written, and there is
  /// nothing a follow-up could fall back on — it is a thing the human wants
  /// rather than a step of the run.
  const [followUp, setFollowUp] = createSignal("");

  /// Whether the submit would be refused for want of one, which is what holds
  /// the button shut rather than a message after the press.
  const needsInstruction = createMemo(
    () =>
      target() === "Implementing" &&
      !props.conversation.ready_to_continue &&
      !instruction().trim(),
  );

  /// And the same for the brief, which is the same rule on the other target: a
  /// round has to be about something, and where nothing is written down yet the
  /// modal is the only place it can be said.
  const needsBrief = createMemo(
    () => target() === "Grilling" && !stands() && !brief().trim(),
  );

  /// The repos to put in the sandbox, by the id of each, as their rows are
  /// filled in.
  ///
  /// Kept whatever the target is rather than cleared when the human moves
  /// between them, exactly as the two payloads above are: what is sent follows
  /// the target, and a row emptied by a change of mind about where the work goes
  /// would be the form answering a question they had not been asked.
  const [added, setAdded] = createSignal<Record<number, Addition>>({});

  const settle = (repo: number, addition: Addition | null) =>
    setAdded((added) => {
      const { [repo]: gone, ...rest } = added;

      return addition ? { ...rest, [repo]: addition } : rest;
    });

  /// What that comes to on the wire: one entry per ticked row, with the empty
  /// string on either field meaning what it means everywhere else — the base is
  /// the default-branch rule, and the branch is mirroring.
  const additions = createMemo<CompanionAddition[]>(() =>
    Object.entries(added()).map(([repo, addition]) => ({
      repo_id: Number(repo),
      mode: addition.mode,
      base_ref: addition.base || null,
      branch: addition.branch,
    })),
  );

  /// And the ones already there that are being opened up, by the id of each.
  ///
  /// Kept across a change of target for the reason everything else here is:
  /// what is sent follows the target, and a tick undone by a change of mind
  /// about where the work goes would be the form answering a question nobody
  /// asked.
  const [upgraded, setUpgraded] = createSignal<Record<number, Upgrade>>({});

  const open = (repo: number, upgrade: Upgrade | null) =>
    setUpgraded((upgraded) => {
      const { [repo]: gone, ...rest } = upgraded;

      return upgrade ? { ...rest, [repo]: upgrade } : rest;
    });

  /// What those come to on the wire: one entry per row ticked up, with no mode
  /// on any of them — read-write is the one direction, and a row that could
  /// carry read-only would be a row that could take back what a session was
  /// given.
  const upgrades = createMemo<CompanionUpgrade[]>(() =>
    Object.entries(upgraded()).map(([repo, upgrade]) => ({
      repo_id: Number(repo),
      branch: upgrade.branch,
    })),
  );

  /// And the same again on the one payload that is required whatever the record
  /// holds.
  const needsFollowUp = createMemo(
    () => target() === "FollowUp" && !followUp().trim(),
  );

  const [interrupt, setInterrupt] = createSignal(false);
  const [refused, setRefused] = createSignal<ConversationSteered | null>(null);

  const submit = useMutation(() => ({
    mutationFn: () =>
      steer(props.conversation.id, {
        target: target(),
        interrupt: interrupt(),
        // Sent only where the target runs something. A target nothing runs in
        // settles no pairing, and a null there would be the form arguing with
        // itself about what it had picked.
        pairing: runs() && picked() ? pairing.choice(picked()) : null,
        // And the payload of the one target that has one, for the same reason:
        // a brief under a wrap-up would be a document about nothing, and a
        // digest is what primes a grilling and nothing else.
        brief: target() === "Grilling" && brief().trim() ? brief() : null,
        digest: target() === "Grilling" && digest(),
        // And the sandbox the sessions to come run in, which every target work
        // goes on in carries: it is setup rather than a payload of one state.
        // Into done nothing runs, so there is nothing for a companion to be for.
        added: runs() ? additions() : [],
        upgraded: runs() ? upgrades() : [],
        instruction:
          target() === "Implementing" && instruction().trim()
            ? instruction()
            : null,
        follow_up:
          target() === "FollowUp" && followUp().trim() ? followUp() : null,
      }),
    onSuccess: (outcome: ConversationSteered) => {
      // The page it was submitted from is out of date either way: the work has
      // moved, or the world had moved under the modal. Reading it again is both
      // the correction and, where it was refused, the explanation.
      void queries.invalidateQueries({ queryKey: ["conversation"] });
      void queries.invalidateQueries({ queryKey: ["conversations"] });

      if (outcome === "Steered") {
        props.close();
        return;
      }

      // A pairing refused is a profile list this modal read a moment ago, so
      // that is re-read too.
      void queries.invalidateQueries({ queryKey: ["profiles"] });
      setRefused(outcome);
    },
  }));

  return (
    <Modal
      class={styles.steerConversation!}
      open
      close={props.close}
      labelledBy="steer-title"
    >
      <form
        onSubmit={(event) => {
          event.preventDefault();
          submit.mutate();
        }}
      >
        <h3 id="steer-title">Steer this conversation</h3>
        <Note class={styles.lead}>
          The run has stopped while you decide. Cancel leaves it stopped, with
          resume on offer.
        </Note>

        {/* One block per target, each saying what it means, for the reason the
            actions menu gives each of its presses a line: the words between two
            of these are the difference between an hour of work and none. */}
        <fieldset class={styles.steerTargets}>
          <legend>Move it into</legend>
          <For each={offered()}>
            {(offered) => (
              <div class={styles.steerTarget}>
                <label>
                  <input
                    type="radio"
                    name="steer-target"
                    value={offered.target}
                    checked={target() === offered.target}
                    onChange={() => setTarget(offered.target)}
                  />
                  {offered.label}
                </label>
                <Note class={styles.optionNote}>{offered.note}</Note>
              </div>
            )}
          </For>
        </fieldset>

        {/* Only under grilling, which is the one target that takes anything
            written. Both are optional and both default to the quietest thing
            they could mean: no brief is the round starting on the one already
            there, and no digest is the interview starting from the brief alone. */}
        <Show when={target() === "Grilling"}>
          <div class={styles.steerBrief}>
            <label for="steer-brief">A brief for the new round</label>
            <textarea
              id="steer-brief"
              rows="6"
              value={brief()}
              onInput={(event) => setBrief(event.currentTarget.value)}
              disabled={submit.isPending}
              placeholder={
                stands()
                  ? "Leave it empty to grill the brief that is already there."
                  : "Nothing is written down yet, so say what this round is about."
              }
            />
            <Note class={styles.fieldNote}>
              What you write lands as a brief of its own, frozen at once. The
              brief the earlier round was built from stays on the timeline.
            </Note>

            <label class={styles.steerDigest}>
              <input
                type="checkbox"
                checked={digest()}
                onChange={(event) => setDigest(event.currentTarget.checked)}
              />
              Prime it with everything you have already answered
            </label>
            <Note class={styles.fieldNote}>
              Every answered question set of this conversation, in the order it
              was asked. Leave it off to start the interview fresh.
            </Note>
          </div>
        </Show>

        {/* And under implementing, the other payload. Empty is carrying on from
            what the branch holds, which is only something it can mean where
            there is something there — so where there is not, the field is what
            the target is, and the submit is held shut until it says something. */}
        <Show when={target() === "Implementing"}>
          <div>
            <label for="steer-instruction">What to do first</label>
            <textarea
              id="steer-instruction"
              rows="6"
              value={instruction()}
              onInput={(event) => setInstruction(event.currentTarget.value)}
              disabled={submit.isPending}
              placeholder={
                props.conversation.ready_to_continue
                  ? "Leave it empty to carry on with what the branch already holds."
                  : "There is nothing on this branch to carry on, so say what to do."
              }
            />
            <Note>
              A session does what you write and commits it. What follows is
              Verkstead’s: the next task of the backlog, or the pull request
              wrapped up again.
            </Note>
          </div>
        </Show>

        {/* And under follow-up, the one payload with nothing it could mean
            empty: there is no follow-up to start without something to follow
            up on, so the field is the target and the submit is held shut until
            it says something. */}
        <Show when={target() === "FollowUp"}>
          <div>
            <label for="steer-follow-up">What to follow up on</label>
            <textarea
              id="steer-follow-up"
              rows="6"
              value={followUp()}
              onInput={(event) => setFollowUp(event.currentTarget.value)}
              disabled={submit.isPending}
              placeholder="Ask about this pull request, or say what you want done to it."
            />
            <Note>
              A session answers what you ask and does what you want done, then
              asks you what else there is. It goes on until you are finished
              with it.
            </Note>
          </div>
        </Show>

        {/* Only where something runs in the state picked. What is settled here
            is the conversation's own pairing rather than one session's, which is
            what the line under it says: steering re-settles what runs the work.

            A [`Listbox`] rather than a `<select>`, so this cannot come to show
            one pairing while the submit would send another — and so that every
            row carries the mark of the harness it runs, which an `<option>`
            cannot hold. See `src/picking.tsx`. */}
        <Show when={runs()}>
          <div class={styles.steerPairing}>
            <label for="steer-pairing">Run it under</label>
            {/* Drawn only once the list is here, the way the setup's pickers
                are: a control whose choice is set before its rows exist is a
                control showing nothing, and the modal reads the profiles when
                it opens rather than finding them already read. */}
            <Show
              when={profiles.data}
              fallback={
                <Note class={styles.fieldNote}>
                  {profiles.isError
                    ? `Could not read the agent profiles: ${profiles.error?.message}`
                    : "Reading the agent profiles…"}
                </Note>
              }
            >
              {(saved) => (
                <Listbox
                  id="steer-pairing"
                  options={pairing.pairings(saved())}
                  value={pairing.value}
                  // The whole list beside each row: the profile's name is said
                  // after the model only where its backend has more than one
                  // account saved.
                  label={(row) => pairing.label(row, saved())}
                  mark={(row) => row.profile.account.agent_type}
                  chosen={picked()}
                  pick={pick}
                  gone={() => pick("")}
                  disabled={submit.isPending}
                />
              )}
            </Show>
            <Note class={styles.fieldNote}>
              What the work runs under from here. This is recorded as the
              conversation's, not just this run's.
            </Note>
          </div>
        </Show>

        {/* And the sandbox the sessions to come run in, under every target work
            goes on in: it is setup rather than a property of one state. Into
            done nothing runs, so there is nothing a companion could be for. */}
        <Show when={runs()}>
          <Companions
            conversation={props.conversation}
            added={added()}
            settle={settle}
            upgraded={upgraded()}
            open={open}
            disabled={submit.isPending}
          />
        </Show>

        {/* Only where there is one to interrupt. With nothing running the box
            would promise something about a session that is not there. */}
        <Show when={props.working}>
          <div class={styles.steerInterrupt}>
            <label>
              <input
                type="checkbox"
                checked={interrupt()}
                onChange={(event) => setInterrupt(event.currentTarget.checked)}
              />
              Interrupt current task
            </label>
            <Note class={styles.optionNote}>
              End the session running now where it stands, leaving the step
              however far it had got. Left alone it keeps the worktree — to its
              own end into done, where nothing is started, and otherwise until
              the session this steer starts is ready to take it over.
            </Note>
          </div>
        </Show>

        <div class={styles.steerButtons}>
          {/* Held shut where the target runs something and nothing is picked to
              run it: the server refuses that by name, and a press that could
              only be refused is one the human should not have to make. */}
          <button
            type="submit"
            class={styles.steer}
            disabled={
              submit.isPending ||
              (runs() && !picked()) ||
              needsInstruction() ||
              needsBrief() ||
              needsFollowUp()
            }
          >
            {submit.isPending ? "Steering…" : "Steer"}
          </button>
          {/* Drawn as well as the ways out the modal already has: escape and a
              press on the backdrop are for a keyboard and a cursor, and this is
              the one a thumb has. */}
          <button type="button" class={styles.cancel} onClick={props.close}>
            Cancel
          </button>
        </div>

        <Show when={refused()}>
          {(outcome) => (
            <ErrorLine class={styles.failure}>{steerRefusal(outcome())}</ErrorLine>
          )}
        </Show>
        {/* A server that could not answer at all, which is the one thing here
            that is an error rather than an outcome. */}
        <Show when={submit.isError}>
          <ErrorLine class={styles.failure}>
            The conversation could not be steered: {submit.error?.message}
          </ErrorLine>
        </Show>
      </form>
    </Modal>
  );
}
