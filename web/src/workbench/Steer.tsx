//! Steering a conversation: the form the pending steer's item opens, and what
//! it settles.
//!
//! The press has already happened by the time any of this is drawn — see the
//! menu row in [`Actions`](./Actions.tsx), which posts it and navigates here on
//! what comes back. That press stopped the drive, so nothing new is launching
//! while the human composes, and it wrote the **pending steer** this form is
//! drawn on: the item at the end of the timeline, whose details pane is this.
//!
//! **A pane rather than a window over the page**, which is the whole of why it
//! is here. A steer is often a great deal of text, and a form that blocks the
//! workbench while it is written is the wrong place to write it: the human
//! wants to read the timeline, look at another conversation, and finish the
//! form when they are ready. So the item stays at the end of the timeline until
//! they decide, and the form is somewhere to come back to.
//!
//! **Cancel is a press now.** It takes the pending steer away and leaves the
//! conversation stopped, with resume drawn on it — the press is what froze it,
//! and unfreezing is a press of its own. Nothing lands on the timeline: a steer
//! that decided nothing is no event.
//!
//! **It is drawn against the live conversation** rather than one frozen at the
//! press. The item may sit open for an afternoon, so what the form reads —
//! whether a session is still running, what the branch has to carry on, which
//! repos are alongside — is what is true now.
//!
//! **And it saves itself as it is typed**, which is what makes an afternoon
//! something the form can survive: every field is kept on the pending steer, so
//! the human can leave the item, the conversation or the device and find the
//! form as they left it. The draft brief's shape, down to the pause it keeps —
//! see [`settling`](./settling.ts) — and every field goes through the one
//! keeper, the ticks and the companion rows included, because a save carries
//! the whole form: a row holding the target of one keystroke beside the
//! instruction of another would be a form that was never on anybody's screen.
//!
//! Each field follows the record until the first keystroke and itself after it,
//! so a read of the conversation landing mid-sentence cannot take the sentence
//! with it — and a second device sees what the first saved on its next read. A
//! save the server refuses stops the form for good and is said under it: what a
//! refusal means is that there is no pending steer left to save into, which is
//! somebody submitting or cancelling this same form from another device.
//!
//! What the form is, therefore, is one question — where does this go? — with
//! whatever that target needs under it. **Done** needs nothing: there
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
//! rather than about the move. The press left whatever was running exactly
//! where it was; ticking the box ends it where it stands, and the step is left
//! however far it had got.
//!
//! Left alone it is seen out, and what *out* means follows the target. One
//! worktree holds one agent, so a target something runs in ends it by starting:
//! the session this steer launches takes the worktree over, at once where it can
//! and once the session in front of it has finished where it cannot — a review
//! waiting on an ask. **Done** launches nothing, so there it runs to its own end
//! and the box is the only thing that would stop it. The box is drawn only where
//! a session is running *now* — the item may have sat open for hours, and there
//! is otherwise nothing to interrupt.
//!
//! **And the same form once it has happened**, which is [`Frozen`] at the foot
//! of this file: the pane every Steer event opens. What a submit lands is the
//! whole of what was filled in — the target and the body on the event itself,
//! and the ticks, the pairing and the companion rows on the record beside it —
//! so the record reads as the form the human filled rather than as a sentence
//! somebody wrote about it. Here rather than in a file of its own because the
//! two are one form read at two moments, and a label the pending half changed
//! without the frozen half would be the record saying the human answered a
//! question they were never asked.

import { useMutation, useQueryClient } from "@tanstack/solid-query";
import { For, Show, createMemo, createSignal, type JSX } from "solid-js";

import {
  cancelSteer,
  listProfiles,
  listRepos,
  saveSteer,
  steer,
} from "../api/client";
import type {
  CompanionAddition,
  CompanionMode,
  CompanionUpgrade,
  CompanionView,
  ConversationSteered,
  ConversationView,
  Lifecycle,
  PairingView,
  RepoEntry,
  SteerCompanionRefusal,
  SteerEvent,
  SteerForm,
  SteerPairingView,
  SteerRecordView,
  SteerSaved,
  SteerTarget,
  TimelineEvent,
} from "../api/types";
import { useReading } from "../freshness";
import { Empty, ErrorLine, Note } from "../notices";
import { PaneSticky } from "../Panes";
import * as pairing from "../pairing";
import { Listbox } from "../picking";
import { keyOf, useDevice } from "../reaching";
import { Switch as Toggle } from "../Switch";
import { chosen } from "./naming";
import { PaneHead } from "./PaneHead";
import { keeping, type Keeping } from "./settling";
import { STATE } from "./states";
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

/// And each way a save of the form is refused, both of which stop it for good.
///
/// Neither can come right by being asked again: a conversation that is gone
/// does not come back, and a pending steer that was submitted or cancelled is
/// not reopened. So the field says what happened and stops rather than asking
/// on every pause for as long as the human goes on writing — and each says that
/// what is on the screen is now the only copy of it, because it is.
export const STEER_SAVE_REFUSAL: Record<
  Exclude<SteerSaved, "Saved">,
  string
> = {
  NoSuchConversation:
    "This conversation is gone, so nothing more can be saved here. What you have written is only on this device now.",
  NoPendingSteer:
    "This steer was submitted or cancelled somewhere else, so there is nothing left to save into. What you have written is only on this device now.",
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
/// false the form's brief field is what the target *is*, and the server
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

/// The form a press opens on: nothing picked, nothing written, nothing ticked.
///
/// What the pane falls back to where the conversation carries no pending steer
/// — which it does for the instant between a cancel or a submit landing and the
/// pane letting go of the address.
const UNANSWERED: SteerForm = {
  target: null,
  brief: null,
  digest: false,
  instruction: null,
  follow_up: null,
  pairing: null,
  interrupt: false,
  added: [],
  upgraded: [],
};

/// The saved companion rows, back into the rows the section fills in.
///
/// Keyed by the Repo's id, which is what the section holds them by: the empty
/// base is the rule, exactly as it is on the wire, and the empty branch is
/// mirroring.
function ticked(added: CompanionAddition[]): Record<number, Addition> {
  return Object.fromEntries(
    added.map((row) => [
      row.repo_id,
      { mode: row.mode, base: row.base_ref ?? RULE, branch: row.branch },
    ]),
  );
}

/// And the same for the rows ticked up, which carry one field apiece.
function opened(upgraded: CompanionUpgrade[]): Record<number, Upgrade> {
  return Object.fromEntries(
    upgraded.map((row) => [row.repo_id, { branch: row.branch }]),
  );
}

/// One form as one string, which is what says whether there is anything to
/// save.
///
/// Written out field by field rather than handed to `JSON.stringify` whole,
/// because one of the two being compared came off the wire and the other was
/// built here: two objects that say the same thing in a different order are not
/// the same string. Nothing and the empty string are the same thing on every
/// field that has both — a textarea somebody emptied is a field with nothing in
/// it, which is what the record holds for one nobody ever typed in.
///
/// The companion rows are in the id's order on both sides: the record reads
/// them back ordered, and the section holds them by an id, which is the order a
/// JavaScript object with numbers for keys is read in.
function reading(form: SteerForm): string {
  return JSON.stringify([
    form.target,
    form.brief ?? "",
    form.digest,
    form.instruction ?? "",
    form.follow_up ?? "",
    form.pairing && pairing.spelled(form.pairing),
    form.interrupt,
    form.added.map((row) => [row.repo_id, row.mode, row.base_ref, row.branch]),
    form.upgraded.map((row) => [row.repo_id, row.branch]),
  ]);
}

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
  /// The one keeper the whole form saves itself through, so that a tick down
  /// here goes out the way a keystroke up there does: a press saves at once and
  /// a branch being typed keeps the pause. See [`keeping`](./settling.ts).
  keeper: Keeping;
  disabled: boolean;
}): JSX.Element {
  const device = useDevice();

  const repos = useReading(() => ({
    queryKey: keyOf(device(), "repos"),
    queryFn: () => listRepos(device()),

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
                keeper={props.keeper}
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
                  keeper={props.keeper}
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
  keeper: Keeping;
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
            onChange={(event) => {
              props.open(event.currentTarget.checked ? MIRRORING : null);
              props.keeper.keep();
            }}
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
                onInput={(event) => {
                  props.open({ branch: event.currentTarget.value });
                  props.keeper.settle();
                }}
                onBlur={() => props.keeper.keep()}
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
  keeper: Keeping;
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
          onChange={(event) => {
            props.settle(event.currentTarget.checked ? PLAINEST : null);
            props.keeper.keep();
          }}
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
              flip={(on) => {
                props.settle({
                  ...addition(),
                  mode: on ? "ReadWrite" : "ReadOnly",
                  // A branch name left behind on a row flipped back to
                  // read-only would be a name for a branch nobody will cut: a
                  // read-only checkout is detached and holds none.
                  branch: on ? addition().branch : "",
                });
                props.keeper.keep();
              }}
            />

            <BasePicker
              id={`steer-companion-${props.repo.id}-base`}
              label={<>Base for {props.repo.name}</>}
              repo={props.repo}
              chosen={addition().base}
              disabled={props.disabled}
              pick={(picked) => {
                props.settle({ ...addition(), base: picked ?? RULE });
                props.keeper.keep();
              }}
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
                  onInput={(event) => {
                    props.settle({
                      ...addition(),
                      branch: event.currentTarget.value,
                    });
                    props.keeper.settle();
                  }}
                  onBlur={() => props.keeper.keep()}
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

/// The pending steer's details pane: the form, and everything it settles before
/// the move.
export function Steer(props: {
  conversation: ConversationView;
  /// The way off this pane, which a narrow window walks out through — the same
  /// way out every other details pane carries. Cancel and submit go further
  /// than this: what they leave behind is a Conversation with no pending steer
  /// on it, so the page has to let go of the address as well.
  back: () => void;
  /// Said when there is no pending steer left to draw — cancelled or submitted.
  /// The page goes back to where the timeline was open before, which after a
  /// submit is the record the steer just wrote.
  done: () => void;
}): JSX.Element {
  const queries = useQueryClient();
  const device = useDevice();

  /// The targets this conversation can actually be sent to. Wrapping up is
  /// drawn out where the work is on no pull request: a target that would be
  /// refused by name is worse than one that was never offered.
  ///
  /// Implementing is not drawn out anywhere, however little the branch holds —
  /// an instruction can always be written, and what that instruction says is
  /// the one thing about this form nothing can work out in advance.
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

  /// The form as the record holds it, which is what every field follows until
  /// it is typed into — the prefill on the way in, and what a second device's
  /// save arrives as on the next read.
  ///
  /// The unanswered form where the conversation carries no pending steer, which
  /// is the instant between a cancel or a submit landing and the pane letting
  /// go of the address.
  const held = createMemo(
    () => props.conversation.pending_steer?.form ?? UNANSWERED,
  );

  /// And what the record has as far as this pane knows: what came down with the
  /// conversation, until a save of its own puts something else there.
  ///
  /// Held rather than read off [`held`] alone, because a save is answered
  /// before the read that follows it lands: a field that went on comparing
  /// itself to the older reading would save the same sentence over and over
  /// until it arrived.
  const [kept, setKept] = createSignal<SteerForm | null>(null);
  const recorded = () => kept() ?? held();

  // Where it goes, where the human has said it here. Null until they do, and
  // what the picker shows meanwhile is the record's or the first target
  // offered: a picker with nothing picked would be a form the human has to
  // answer twice.
  //
  // What is *saved* is never this — it is [`going`], the target the picker is
  // on. The radio for the first target offered opens already checked, so
  // pressing it fires nothing and this would stay null for every steer into
  // grilling there has ever been: the card at the end of the timeline would
  // read *Steer* for the life of the form, and the pairing below would have no
  // target to be read back against.
  const [target, setTarget] = createSignal<SteerTarget | null>(null);

  /// What the picker is on: what has been picked here, what the record holds,
  /// or the first target offered.
  ///
  /// The list is never empty — done is offered on every conversation there is —
  /// and the fallback is that same target said twice rather than a state this
  /// can be in. A target the record holds that is not offered any more falls
  /// back with it: the form is drawn against the live conversation, so a
  /// wrap-up saved while there was a pull request is not a wrap-up now there is
  /// none.
  const going = createMemo<SteerTarget>(() => {
    const said = target() ?? held().target;

    return said && offered().some((one) => one.target === said)
      ? said
      : (offered()[0]?.target ?? "Done");
  });

  /// Whether the target picked is one work goes on in, which is what draws the
  /// pairing picker under it.
  const runs = createMemo(
    () => offered().find((one) => one.target === going())?.runs ?? false,
  );

  /// And which role that picker settles — the one the target's sessions run
  /// under, and for wrapping up the review role beside it.
  const role = createMemo(
    () => offered().find((one) => one.target === going())?.role,
  );

  // The profile list is read here rather than passed in, so the picker is whole
  // wherever the form is opened from — the setup pane does the same.
  const profiles = useReading(() => ({
    queryKey: keyOf(device(), "profiles"),
    queryFn: () => listProfiles(device()),

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
  const [grilling, setGrilling] = createSignal<string | null>(null);
  const [implementation, setImplementation] = createSignal<string | null>(null);

  /// Which role the record's own pairing answers for, which is the role its
  /// target runs under: the row holds one pairing because a submit sends one.
  const answered = createMemo(
    () => TARGETS.find((one) => one.target === held().target)?.role,
  );

  /// The one the target picked runs under, and nothing where nothing runs.
  ///
  /// What has been picked here, then the record's where its own target runs
  /// under this same role, then what the conversation already runs the work
  /// under. Which is also what the save carries: a form whose target moves to a
  /// role of the other kind saves that role's pairing, because that is what its
  /// submit would send.
  const picked = createMemo(() => {
    const settling = role();

    if (!settling) {
      return "";
    }

    const own = settling === "grilling" ? grilling() : implementation();

    if (own !== null) {
      return own;
    }

    const on = held().pairing;

    if (on && answered() === settling) {
      return pairing.spelled(on);
    }

    return settling === "grilling"
      ? pairing.chosen(pairing.under(props.conversation.grilling_pairing))
      : pairing.chosen(props.conversation.implementation_pairing);
  });

  const pick = (chosen: string) => {
    if (role() === "grilling") {
      setGrilling(chosen);
    } else {
      setImplementation(chosen);
    }

    keeper.keep();
  };

  /// The new round's brief, for a steer into grilling.
  ///
  /// Optional where a brief already stands, and empty is the ordinary case
  /// there: the round starts on the one that is already written. Required where
  /// none does — a draft nobody has written into — because a grilling starts
  /// from a brief and there would otherwise be nothing to interview about.
  /// What is typed here lands as a brief of its own, frozen the moment it does.
  ///
  /// Null until the first keystroke, which is the rule every field here keeps:
  /// it follows the record until then and follows itself after it, so a read of
  /// the conversation landing mid-sentence cannot take the sentence with it.
  const [brief, setBrief] = createSignal<string | null>(null);
  const written = () => brief() ?? held().brief ?? "";

  /// Whether one is already written, which is what makes the field optional.
  const stands = createMemo(() => briefStands(props.conversation));

  /// And whether the session is primed with everything already answered.
  ///
  /// Off to begin with, because the steer is usually a change of direction: a
  /// fresh brief primed with the whole of the last interview would be steering
  /// into the argument that has just been left behind.
  const [digest, setDigest] = createSignal<boolean | null>(null);
  const priming = () => digest() ?? held().digest;

  /// The hand-written work, for a steer into implementing.
  ///
  /// Required where the branch holds nothing to carry on and optional where it
  /// does — the server’s own reading of the branch rather than anything worked
  /// out from the pinned backlog here: what stands includes the finish step a
  /// list of ticked tasks still has to run, which no reading of the entries
  /// could see.
  const [instruction, setInstruction] = createSignal<string | null>(null);
  const doing = () => instruction() ?? held().instruction ?? "";

  /// And the brief, for a steer into follow-up.
  ///
  /// Always required, unlike the two above it: an empty instruction carries the
  /// branch on and an empty brief grills the one already written, and there is
  /// nothing a follow-up could fall back on — it is a thing the human wants
  /// rather than a step of the run.
  const [followUp, setFollowUp] = createSignal<string | null>(null);
  const following = () => followUp() ?? held().follow_up ?? "";

  /// Whether the submit would be refused for want of one, which is what holds
  /// the button shut rather than a message after the press.
  const needsInstruction = createMemo(
    () =>
      going() === "Implementing" &&
      !props.conversation.ready_to_continue &&
      !doing().trim(),
  );

  /// And the same for the brief, which is the same rule on the other target: a
  /// round has to be about something, and where nothing is written down yet the
  /// pane is the only place it can be said.
  const needsBrief = createMemo(
    () => going() === "Grilling" && !stands() && !written().trim(),
  );

  /// The repos to put in the sandbox, by the id of each, as their rows are
  /// filled in.
  ///
  /// Kept whatever the target is rather than cleared when the human moves
  /// between them, exactly as the two payloads above are: what is sent follows
  /// the target, and a row emptied by a change of mind about where the work goes
  /// would be the form answering a question they had not been asked.
  ///
  /// One signal for the whole set rather than one per row, so the first tick is
  /// what takes the section off the record: a row arriving from another device
  /// while this one is being filled in would be the two of them editing one
  /// list between them.
  const [added, setAdded] = createSignal<Record<number, Addition> | null>(null);
  const adding = createMemo(() => added() ?? ticked(held().added));

  const settle = (repo: number, addition: Addition | null) =>
    setAdded(() => {
      const { [repo]: gone, ...rest } = adding();

      return addition ? { ...rest, [repo]: addition } : rest;
    });

  /// What that comes to on the wire: one entry per ticked row, with the empty
  /// string on either field meaning what it means everywhere else — the base is
  /// the default-branch rule, and the branch is mirroring.
  const additions = createMemo<CompanionAddition[]>(() =>
    Object.entries(adding()).map(([repo, addition]) => ({
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
  const [upgraded, setUpgraded] = createSignal<Record<number, Upgrade> | null>(
    null,
  );
  const opening = createMemo(() => upgraded() ?? opened(held().upgraded));

  const open = (repo: number, upgrade: Upgrade | null) =>
    setUpgraded(() => {
      const { [repo]: gone, ...rest } = opening();

      return upgrade ? { ...rest, [repo]: upgrade } : rest;
    });

  /// What those come to on the wire: one entry per row ticked up, with no mode
  /// on any of them — read-write is the one direction, and a row that could
  /// carry read-only would be a row that could take back what a session was
  /// given.
  const upgrades = createMemo<CompanionUpgrade[]>(() =>
    Object.entries(opening()).map(([repo, upgrade]) => ({
      repo_id: Number(repo),
      branch: upgrade.branch,
    })),
  );

  /// And the same again on the one payload that is required whatever the record
  /// holds.
  const needsFollowUp = createMemo(
    () => going() === "FollowUp" && !following().trim(),
  );

  const [interrupt, setInterrupt] = createSignal<boolean | null>(null);
  const ending = () => interrupt() ?? held().interrupt;

  /// The form as it stands, which is what a save carries: the whole of it, so
  /// the row is never the target of one keystroke beside the instruction of
  /// another.
  ///
  /// Everything the form holds rather than only what this target would submit:
  /// the payloads and the companion rows are kept across a change of mind about
  /// where the work goes, so the record keeps them too.
  ///
  /// The target is what the picker is *on* rather than what was pressed, which
  /// is the same rule as the pairing's a line below: what a save carries is
  /// what this form would submit. A radio that opens already checked is never
  /// pressed, so saving only what was pressed would leave every steer into the
  /// first target offered holding no target at all — the card reading *Steer*
  /// for the life of the form, and [`answered`] with nothing to read the saved
  /// pairing back against.
  const form = createMemo<SteerForm>(() => ({
    target: going(),
    brief: written() || null,
    digest: priming(),
    instruction: doing() || null,
    follow_up: following() || null,
    pairing: picked() ? pairing.choice(picked()) : null,
    interrupt: ending(),
    added: additions(),
    upgraded: upgrades(),
  }));

  /// What the server would not take, which stops the form saving for good: both
  /// of them are permanent, and the commonest by far is a submit or a cancel
  /// from another device landing mid-edit.
  const [unkept, setUnkept] = createSignal<Exclude<SteerSaved, "Saved"> | null>(
    null,
  );

  const saving = useMutation(() => ({
    mutationFn: (form: SteerForm) =>
      saveSteer(device(), props.conversation.id, form),
    onSuccess: (outcome: SteerSaved, sent: SteerForm) => {
      if (outcome !== "Saved") {
        // What is on the screen stands: it is the only copy of it there is, and
        // the human is owed the chance to take it somewhere else.
        setUnkept(outcome);
        return;
      }

      const moved = sent.target !== held().target;

      setUnkept(null);
      setKept(sent);

      // The item at the end of the timeline says where the steer is going, so
      // the conversation is read again when that moves — and only then. A steer
      // is a great deal of text, and a read of the whole record a sentence
      // would be the pane asking for the timeline over and over to redraw one
      // line that has not changed.
      if (moved) {
        void queries.invalidateQueries({
          queryKey: keyOf(device(), "conversation"),
        });
      }
    },
    // Whatever became of it, the form may have been typed into while it was in
    // flight — so the moment one save is done the next is considered.
    onSettled: () => keeper.done(),
  }));

  /// The one keeper the whole form saves itself through: the pause after a
  /// keystroke, one save in the air at a time, and what was typed meanwhile
  /// sent on the back of the answer.
  const keeper = keeping({
    unsaved: () => reading(form()) !== reading(recorded()),
    settled: () => unkept() !== null,
    save: () => saving.mutate(form()),
  });

  const [refused, setRefused] = createSignal<ConversationSteered | null>(null);
  const submit = useMutation(() => ({
    // The form's own state rather than the row's: a keystroke inside the pause
    // has not been saved yet, and a press that asked the server to freeze what
    // it had would lose it. What the pane shows is what goes.
    mutationFn: () =>
      steer(device(), props.conversation.id, {
        target: going(),
        interrupt: ending(),
        // Sent only where the target runs something. A target nothing runs in
        // settles no pairing, and a null there would be the form arguing with
        // itself about what it had picked.
        pairing: runs() && picked() ? pairing.choice(picked()) : null,
        // And the payload of the one target that has one, for the same reason:
        // a brief under a wrap-up would be a document about nothing, and a
        // digest is what primes a grilling and nothing else.
        brief: going() === "Grilling" && written().trim() ? written() : null,
        digest: going() === "Grilling" && priming(),
        // And the sandbox the sessions to come run in, which every target work
        // goes on in carries: it is setup rather than a payload of one state.
        // Into done nothing runs, so there is nothing for a companion to be for.
        added: runs() ? additions() : [],
        upgraded: runs() ? upgrades() : [],
        instruction:
          going() === "Implementing" && doing().trim() ? doing() : null,
        follow_up:
          going() === "FollowUp" && following().trim() ? following() : null,
      }),
    onSuccess: (outcome: ConversationSteered) => {
      // The page it was submitted from is out of date either way: the work has
      // moved, or the world had moved under the form. Reading it again is both
      // the correction and, where it was refused, the explanation.
      void queries.invalidateQueries({
        queryKey: keyOf(device(), "conversation"),
      });
      void queries.invalidateQueries({ queryKey: ["conversations"] });

      // The pending steer went with the record it became, so there is nothing
      // at this address any more: the page follows the record it wrote.
      if (outcome === "Steered") {
        props.done();
        return;
      }

      // A refused submit leaves the pending steer exactly where it was, so the
      // form stays open and says why. A pairing refused is a profile list this
      // pane read a moment ago, so that is re-read too.
      void queries.invalidateQueries({ queryKey: keyOf(device(), "profiles") });
      setRefused(outcome);
    },
  }));

  /// And the other way out, which is a press rather than a dismissal: the
  /// pending steer goes and the conversation is left stopped, with resume on
  /// offer.
  ///
  /// Nothing is confirmed first. What it throws away is the form, and the
  /// conversation it was written about is exactly where the press that opened
  /// it left the work.
  const cancelling = useMutation(() => ({
    mutationFn: () => cancelSteer(device(), props.conversation.id),
    onSuccess: () => {
      void queries.invalidateQueries({
        queryKey: keyOf(device(), "conversation"),
      });
      void queries.invalidateQueries({ queryKey: ["conversations"] });

      // Either way there is no pending steer at this address: a conversation
      // that is gone has no form to go back to, and a cancel that landed took
      // the form with it.
      props.done();
    },
  }));

  return (
    <>
      <PaneSticky>
        <PaneHead
          back={{ to: "Timeline", go: props.back }}
          title="Steer this conversation"
        />
      </PaneSticky>

      <form
        class={styles.steerConversation}
        onSubmit={(event) => {
          event.preventDefault();
          submit.mutate();
        }}
      >
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
                    checked={going() === offered.target}
                    onChange={() => {
                      setTarget(offered.target);
                      keeper.keep();
                    }}
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
        <Show when={going() === "Grilling"}>
          <div class={styles.steerBrief}>
            <label for="steer-brief">A brief for the new round</label>
            <textarea
              id="steer-brief"
              rows="6"
              value={written()}
              onInput={(event) => {
                setBrief(event.currentTarget.value);
                keeper.settle();
              }}
              onBlur={() => keeper.keep()}
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
                checked={priming()}
                onChange={(event) => {
                  setDigest(event.currentTarget.checked);
                  keeper.keep();
                }}
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
        <Show when={going() === "Implementing"}>
          <div>
            <label for="steer-instruction">What to do first</label>
            <textarea
              id="steer-instruction"
              rows="6"
              value={doing()}
              onInput={(event) => {
                setInstruction(event.currentTarget.value);
                keeper.settle();
              }}
              onBlur={() => keeper.keep()}
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
        <Show when={going() === "FollowUp"}>
          <div>
            <label for="steer-follow-up">What to follow up on</label>
            <textarea
              id="steer-follow-up"
              rows="6"
              value={following()}
              onInput={(event) => {
                setFollowUp(event.currentTarget.value);
                keeper.settle();
              }}
              onBlur={() => keeper.keep()}
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
                control showing nothing, and the form reads the profiles when
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
            added={adding()}
            settle={settle}
            upgraded={opening()}
            open={open}
            keeper={keeper}
            disabled={submit.isPending}
          />
        </Show>

        {/* Only where there is one to interrupt. With nothing running the box
            would promise something about a session that is not there. */}
        <Show when={props.conversation.working}>
          <div class={styles.steerInterrupt}>
            <label>
              <input
                type="checkbox"
                checked={ending()}
                onChange={(event) => {
                  setInterrupt(event.currentTarget.checked);
                  keeper.keep();
                }}
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
              cancelling.isPending ||
              (runs() && !picked()) ||
              needsInstruction() ||
              needsBrief() ||
              needsFollowUp()
            }
          >
            {submit.isPending ? "Steering…" : "Steer"}
          </button>
          {/* A press of its own now rather than a way of dismissing a window:
              what it throws away is the pending steer, and the conversation is
              left stopped with resume on offer. */}
          <button
            type="button"
            class={`${styles.cancel} secondary`}
            disabled={submit.isPending || cancelling.isPending}
            onClick={() => cancelling.mutate()}
          >
            {cancelling.isPending ? "Cancelling…" : "Cancel"}
          </button>
        </div>

        <Show when={refused()}>
          {(outcome) => (
            <ErrorLine class={styles.failure}>
              {steerRefusal(outcome())}
            </ErrorLine>
          )}
        </Show>
        {/* And a save the server would not take, which is a different thing
            said in the same place: the form stays exactly as it is and stops
            keeping itself, so the line has to say that what is on the screen is
            the only copy of it now. */}
        <Show when={unkept()}>
          {(outcome) => (
            <ErrorLine class={styles.failure}>
              {STEER_SAVE_REFUSAL[outcome()]}
            </ErrorLine>
          )}
        </Show>
        <Show when={saving.isError}>
          <ErrorLine class={styles.failure}>
            The form could not be saved: {saving.error?.message}
          </ErrorLine>
        </Show>
        {/* A server that could not answer at all, which is the one thing here
            that is an error rather than an outcome. */}
        <Show when={submit.isError}>
          <ErrorLine class={styles.failure}>
            The conversation could not be steered: {submit.error?.message}
          </ErrorLine>
        </Show>
        <Show when={cancelling.isError}>
          <ErrorLine class={styles.failure}>
            The steer could not be cancelled: {cancelling.error?.message}
          </ErrorLine>
        </Show>
      </form>
    </>
  );
}

/// What a steer wrote under each target, as the form asked for it.
///
/// The form's own labels rather than a heading invented here, because that is
/// what the record is being drawn as: the human filled in a field called *What
/// to do first*, and reading it back under any other name would be the record
/// answering a question they were never asked. Only the two targets that carry
/// a body are here — the rest say nothing but the state.
const WROTE: Partial<Record<Lifecycle, string>> = {
  Implementing: "What to do first",
  FollowUp: "What to follow up on",
};

/// Whether the steer at `at` on this record wrote the brief its round opened
/// on.
///
/// Read off the timeline rather than off the steer's own row, because that is
/// where the brief is: what a steer into grilling writes lands as a Brief event
/// of its own, under the move — a round starts from a brief, and a steered
/// round's is a second brief beside the first rather than an edit of it. So the
/// frozen form says a brief was written and points at where it stands rather
/// than drawing it twice.
///
/// What may come between the two is what the one transaction writes between
/// them: the notice saying what came into the sandbox, and the move itself.
/// Anything else means this steer wrote none and the brief further down belongs
/// to something later.
function wroteABrief(timeline: readonly TimelineEvent[], at: number): boolean {
  for (const event of timeline.slice(at + 1)) {
    if ("Brief" in event) {
      return true;
    }

    if (!("Moved" in event) && !("Notice" in event)) {
      return false;
    }
  }

  return false;
}

/// A steer that happened: the form the human filled, frozen.
///
/// **The record is the whole form**, which is what this pane is for. The event
/// carries where the work went and whatever was written to send it there; the
/// row beside it carries the rest — the digest tick, the pairing picked, the
/// interrupt tick and the companion rows asked for — so what the human reads
/// back is the form they filled rather than a sentence somebody wrote about it.
///
/// **Read-only throughout, and in the form's own order**: where it went, what
/// was written under it, what it runs under, which repos were asked for, and
/// what became of the session that was running. Nothing here is a control —
/// the press is over — so the ticks are drawn as ticks that cannot be moved and
/// every field is a line.
///
/// **A steer recorded before any of that was kept draws the fields it has**,
/// which is ADR-0006's rule: the record is read as it was written. Its pane is
/// the target and the body, and the rest is not drawn as a row of empty boxes
/// it never had.
///
/// **The share draws this same pane.** It carries no profiles to read, which is
/// what the missing second half of every pairing reading here is: the account's
/// name is always said, and saying it is never wrong — see
/// [`../agents`](../agents.ts). One reading for one record, wherever it is
/// read.
export function Frozen(props: {
  conversation: ConversationView;
  steer: SteerEvent;
  /// The way off this pane, which a narrow window walks out through — the same
  /// way out every other details pane carries.
  back: () => void;
}): JSX.Element {
  /// Where this steer stands on the record, which is what says whether the
  /// brief under it is the one it wrote.
  const at = createMemo(() =>
    props.conversation.timeline.findIndex(
      (event) => "Steer" in event && event.Steer.id === props.steer.id,
    ),
  );

  /// Whether it opened a round on a brief of its own — a steer into grilling
  /// that wrote one, which is the one field of the form that is not on the row
  /// beside the event.
  const opened = createMemo(
    () =>
      props.steer.target === "Grilling" &&
      at() !== -1 &&
      wroteABrief(props.conversation.timeline, at()),
  );

  return (
    <>
      <PaneSticky>
        <PaneHead back={{ to: "Timeline", go: props.back }} title="Steer" />
      </PaneSticky>

      <div class={styles.steered}>
        <p class={styles.steeredInto}>
          You steered this into {STATE[props.steer.target]}
        </p>

        {/* What was written to steer it with, under the name the field had when
            it was written in. The whole of it rather than the three lines the
            card shows, this being the pane that card opens. */}
        <Show when={props.steer.html}>
          {(html) => (
            <div class={styles.steeredField}>
              <p class={styles.steeredLabel}>
                {WROTE[props.steer.target] ?? "What was written"}
              </p>
              <div class="markdown" innerHTML={html()} />
            </div>
          )}
        </Show>

        {/* And the brief a steer into grilling wrote, said rather than drawn:
            it is a brief event of its own directly under the move, so drawing
            it here would be the same document on the pane twice. */}
        <Show when={opened()}>
          <div class={styles.steeredField}>
            <p class={styles.steeredLabel}>A brief for the new round</p>
            <Note class={styles.steeredNote}>
              It landed on the timeline under the move, frozen where it landed.
            </Note>
          </div>
        </Show>

        {/* And the rest of the form, where the record kept it. A steer from
            before it was kept has none of this, and is drawn as the two things
            above and nothing after them. */}
        <Show when={props.steer.record}>
          {(record) => (
            <Recorded target={props.steer.target} record={record()} />
          )}
        </Show>
      </div>
    </>
  );
}

/// The Pairing a steer recorded, where there is one left to name.
///
/// Both of the other two answer `null` and the pane tells them apart for
/// itself: nothing picked draws no row at all, and a profile removed since
/// draws the row with the fact where the name would be.
function ran(pairing: SteerPairingView): PairingView | null {
  return typeof pairing === "object" ? pairing.Under : null;
}

/// The half of a frozen steer that comes off the row beside the event: the
/// ticks, the pairing and the companion rows.
///
/// Its own component because it is the half that may not be there, and a record
/// without it is not a record with the boxes unticked: what it means is a steer
/// from before any of this was written down.
function Recorded(props: {
  target: Lifecycle;
  record: SteerRecordView;
}): JSX.Element {
  return (
    <>
      {/* The digest under grilling alone, the tick meaning nothing anywhere
          else — it is what primes an interview, and there is no interview
          under another target. */}
      <Show when={props.target === "Grilling"}>
        <Ticked on={props.record.digest}>
          Prime it with everything you have already answered
        </Ticked>
      </Show>

      {/* What the work runs under from here, drawn only where the steer picked
          something: a steer into done picks nothing, there being nothing to
          run. */}
      <Show when={props.record.pairing !== "Nothing"}>
        <div class={styles.steeredField}>
          <p class={styles.steeredLabel}>Run it under</p>
          <p class={styles.steeredValue}>
            {/* And where the account has been removed since, the fact rather
                than a name: the human picked something, and what the record
                can still say about it is that it is gone. */}
            <Show
              when={ran(props.record.pairing)}
              fallback="A profile since removed"
            >
              {(picked) => pairing.label(picked(), undefined)}
            </Show>
          </p>
        </div>
      </Show>

      {/* And the sandbox it asked for: the repos it put in and the ones it
          opened up. Neither is drawn where the steer asked for none, which is
          most steers. */}
      <Show when={props.record.added.length}>
        <div class={styles.steeredField}>
          <p class={styles.steeredLabel}>Repos it put in</p>
          <ul class={styles.steeredRepos} aria-label="Repos it put in">
            <For each={props.record.added}>
              {(repo) => (
                <li class={styles.steeredRepo}>
                  <span class={styles.steeredRepoName}>{repo.repo}</span>
                  <span>
                    {repo.mode === "ReadWrite" ? "read-write" : "read-only"}
                  </span>
                  {/* The empty base is the rule it was checked out under, which
                      is that repository's default branch — and the record does
                      not hold what that was called, so the rule is what is
                      said. */}
                  <span>off {repo.base_ref ?? "its default branch"}</span>
                  {/* And only where there was a branch to name. A read-only
                      companion is checked out detached and holds none. */}
                  <Show when={repo.mode === "ReadWrite"}>
                    <span>
                      as {repo.branch || "this conversation's own branch"}
                    </span>
                  </Show>
                </li>
              )}
            </For>
          </ul>
        </div>
      </Show>

      <Show when={props.record.upgraded.length}>
        <div class={styles.steeredField}>
          <p class={styles.steeredLabel}>Repos it opened up</p>
          <ul class={styles.steeredRepos} aria-label="Repos it opened up">
            <For each={props.record.upgraded}>
              {(repo) => (
                <li class={styles.steeredRepo}>
                  <span class={styles.steeredRepoName}>{repo.repo}</span>
                  <span>read-write</span>
                  <span>
                    as {repo.branch || "this conversation's own branch"}
                  </span>
                </li>
              )}
            </For>
          </ul>
        </div>
      </Show>

      {/* And what became of whatever was running, which is the one thing on the
          form that was about the world rather than about the move. */}
      <Ticked on={props.record.interrupt}>Interrupt current task</Ticked>
    </>
  );
}

/// One of the form's ticks, as the record has it: the box, and what it was
/// against.
///
/// A disabled checkbox rather than a word, because that is what it was — the
/// human read this line with a box in front of it and left it or ticked it, and
/// a record drawn as *Digest: no* would be the same answer in a shape they
/// never saw.
function Ticked(props: { on: boolean; children: JSX.Element }): JSX.Element {
  return (
    <label class={styles.steeredTick}>
      <input type="checkbox" checked={props.on} disabled />
      {props.children}
    </label>
  );
}
