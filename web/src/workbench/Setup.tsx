//! A Conversation's setup: what has to be settled before anything will run it,
//! drawn along the bottom edge of the box the Brief is written in.
//!
//! The branch the work will be done on, the branch it will come off, the other
//! repos it works alongside, and the pairings its sessions run under. Every
//! one of them is a fact about the Conversation rather than about any one Event,
//! and every one of them is the human's to change for as long as it is still
//! drafting.
//!
//! **A row of options rather than a form under the Brief.** Setting a
//! Conversation up and kicking it off are one act, and the act is written in
//! one box — so the whole of the setup is four dropdowns inside that box's
//! bottom edge, each a dimmed label over its value, and what a reader takes off
//! them at a glance is the sentence *this repo, these three accounts*. The
//! panel behind the first of them is where the rest of it lives: the branch,
//! the base and the companion repos are all answers to *which code*, and one
//! trigger for the four of them is what keeps the row down to what it says. See
//! [`Composer`](./Composer.tsx) for the box, and [`SetupNotes`] for what the
//! setup has to say that is not a control.
//!
//! Once grilling starts none of this is drawn at all: the server freezes every
//! one of them at that moment, so nothing taken away was still actionable, and
//! a Brief past drafting opens the record of what it was configured with
//! instead.
//!
//! **Every control here is drawn twice**, because the setup is asked in two
//! places: of a Conversation, where each field saves itself the moment it is
//! touched, and on the compose page, where none of it exists anywhere until a
//! press creates something (see `Compose.tsx`). So each of them is split — the
//! control and what it looks like are exported from here, and what a pick
//! *does* and what is said under it belong to whoever draws it. That is the
//! same seam [`BasePicker`] has always stood on, widened to the rest of the
//! row: two pages that asked these questions apart would come to word them
//! differently.
//!
//! The three pairings are separate choices because they are genuinely separate
//! accounts — grill on fable, implement on opus, review on whatever did not
//! build it — and because the implementation session cannot simply carry the
//! grilling one on. Two of the pickers carry one row that is not an account at
//! all: a conversation can be built without being grilled and wrapped up
//! without being reviewed.

import { A } from "@solidjs/router";
import { useMutation, useQueryClient } from "@tanstack/solid-query";
import {
  For,
  Match,
  Show,
  Switch,
  createSignal,
  type Accessor,
  type JSX,
} from "solid-js";

import { Menu } from "../Menu";
import { Switch as Toggle } from "../Switch";
import type { AgentType } from "../agents";
import {
  addCompanion,
  chooseGrillingPairing,
  chooseImplementationPairing,
  chooseReviewPairing,
  listBranches,
  listProfiles,
  listRepos,
  removeCompanion,
  renameBranch,
  renameCompanionBranch,
  setBaseBranch,
  setCompanionBase,
  setCompanionMode,
  switchRepo,
} from "../api/client";
import type {
  BaseRecorded,
  BranchRenamed,
  CompanionAdded,
  CompanionBaseRecorded,
  CompanionBranchRenamed,
  CompanionMode,
  CompanionModeChosen,
  CompanionRemoved,
  CompanionView,
  ConversationView,
  PairingView,
  ProfileChosen,
  ProfileEntry,
  RepoEntry,
  RepoSwitched,
} from "../api/types";
import { useReading } from "../freshness";
import { Empty, ErrorLine, Note } from "../notices";
import * as pairing from "../pairing";
import { Listbox, Picker } from "../picking";
import { BROKEN } from "../profiles/ProfileList";
import { AUTOMATIC, chosen } from "./naming";
import styles from "./Setup.module.css";
import { keeping } from "./settling";

/// What each way of being refused a branch name says.
export const BRANCH_REFUSAL: Record<BranchRenamed, string> = {
  Renamed: "",
  NoSuchConversation: "This conversation is gone.",
  NotDrafting:
    "The branch exists by now, so its name is not a text field any more.",
  NotABranchName: "Git will not take that as a branch name.",
};

/// And moving the work onto another repo.
export const REPO_SWITCH_REFUSAL: Record<RepoSwitched, string> = {
  Switched: "",
  NoSuchConversation: "This conversation is gone.",
  NotDrafting:
    "The branch exists by now, so which repo the work is in is settled.",
  NoSuchRepo: "That repo is not registered any more.",
};

/// And a base branch.
export const BASE_REFUSAL: Record<BaseRecorded, string> = {
  Recorded: "",
  NoSuchConversation: "This conversation is gone.",
  NotDrafting: "The base commit was captured when grilling started.",
  NoSuchBranch: "That repo has no branch by that name any more.",
};

/// And adding a repo to work alongside.
export const COMPANION_REFUSAL: Record<CompanionAdded, string> = {
  Added: "",
  NoSuchConversation: "This conversation is gone.",
  NotDrafting:
    "The grilling has started, so what this conversation works alongside is settled.",
  NoSuchRepo: "That repo is not registered any more.",
  OwnRepo: "That is this conversation's own repo — the work is in it already.",
  AlreadyAdded: "That repo is on this conversation already.",
};

/// And taking one away again.
export const COMPANION_REMOVAL_REFUSAL: Record<CompanionRemoved, string> = {
  Removed: "",
  NoSuchConversation: "This conversation is gone.",
  NotDrafting:
    "The grilling has started, so what this conversation works alongside is settled.",
};


/// And how far into one of them the work may reach.
export const COMPANION_MODE_REFUSAL: Record<CompanionModeChosen, string> = {
  Chosen: "",
  NoSuchConversation: "This conversation is gone.",
  NotDrafting:
    "The grilling has started, so what this conversation works alongside is settled.",
  NoSuchCompanion: "That repo is not on this conversation any more.",
};

/// And the branch its checkout comes off.
export const COMPANION_BASE_REFUSAL: Record<CompanionBaseRecorded, string> = {
  Recorded: "",
  NoSuchConversation: "This conversation is gone.",
  NotDrafting: "The base commit was captured when grilling started.",
  NoSuchCompanion: "That repo is not on this conversation any more.",
  NoSuchBranch: "That repo has no branch by that name any more.",
};

/// And what its own branch is called.
export const COMPANION_BRANCH_REFUSAL: Record<CompanionBranchRenamed, string> = {
  Renamed: "",
  NoSuchConversation: "This conversation is gone.",
  NotDrafting:
    "The branch exists by now, so its name is not a text field any more.",
  NoSuchCompanion: "That repo is not on this conversation any more.",
  NotABranchName: "Git will not take that as a branch name.",
};

/// And a pairing choice.
export const CHOICE_REFUSAL: Record<ProfileChosen, string> = {
  Chosen: "",
  NoSuchConversation: "This conversation is gone.",
  NoSuchProfile: "That profile has been removed.",
  NoSuchModel: "That profile no longer lists that model.",
  NotDrafting: "The work has started, so who runs this conversation is settled.",
};

export function Setup(props: {
  conversation: ConversationView;
}): JSX.Element {
  return (
    <section class={styles.options} aria-label="Setup">
      {/* The repository first, because it is what everything after it is a fact
          about — and one dropdown for the whole of it: the branch, the branch
          it comes off, and the repos the work runs alongside are all answers to
          *which code*, and a row of four separate triggers for them would be a
          row about one repository read as four things. */}
      <RepoOption conversation={props.conversation} />

      {/* And the three accounts, one trigger each. */}
      <Profiles conversation={props.conversation} />
    </section>
  );
}

/// What the setup has to say that is not a control: drawn under the box rather
/// than inside its edge, because the row along that edge is what there is to
/// change and these are what there is to know.
export function SetupNotes(props: {
  conversation: ConversationView;
}): JSX.Element {
  return (
    <>
      {/* Nothing is said here about being ready: readiness is the business of
          the button it gates, which is enabled or else explains what is
          missing. Said up here as well it would be the same verdict twice.

          An adopting conversation never grills at all, and why every pairing is
          asked for all the same is worth a line. */}
      <Show when={props.conversation.adopting}>
        <Note class={styles.aside}>
          All three pairings are fixed before adopting: the implementation one
          is what the work runs under, the review one is what looks at it, and
          the grilling one is carried, because the stages after this one inherit
          all of them from it.
        </Note>
      </Show>

      {/* And the last thing read before the work is started, because it is
          about what the work will be like rather than about anything above:
          this repository builds Rust, and its dependencies will be compiled
          from scratch every session. */}
      <UncachedCompiles conversation={props.conversation} />
    </>
  );
}

/// The Repo, and everything that is a fact about it: which repository the work
/// is in, the branch it will be done on, the branch it comes off, and the repos
/// it runs alongside.
///
/// A label over a value like every other option in the row — the repository's
/// name, and `+1`, `+2` for the companions it is working beside — and what it
/// opens is one flat panel rather than a menu of levels: what is in there is a
/// picker, a field, a second picker and the rows they configure, which is a
/// form, and a form walked one level at a time would be a form nobody could read
/// at once.
///
/// Drawn whatever state the round is in, unlike everything below the repo
/// picker inside it. Once the branch is cut the server refuses the branch, the
/// base and every companion press, so those go — but which repository the work
/// is in is still a fact worth reading, and the picker says for itself that it
/// is settled by being disabled.
function RepoOption(props: { conversation: ConversationView }): JSX.Element {
  /// How many other repos the work runs alongside, for the `+N` after the name.
  const alongside = () => props.conversation.companions.length;

  /// Whether the branch this work is done on has been made already, which is
  /// what a worktree says: one is made with the branch and forgotten only by
  /// closing.
  ///
  /// A drafting conversation with one has had its branch cut — a later round,
  /// steered onto work that is already built. The repo, the branch and the base
  /// are settled for good by then and the server refuses all three, so what a
  /// control cannot do it does not draw, the picker excepted. The pairings are
  /// outside this panel and stay, being re-settled every time work starts under
  /// them.
  const branched = () => props.conversation.worktree !== null;

  return (
    <RepoOptions name={props.conversation.repo.name} alongside={alongside()}>
      {() => (
        <>
          {/* Which repository, first: everything under it is a fact about the
              one this picks. */}
          <RepoPicker conversation={props.conversation} disabled={branched()} />

          <Show when={!branched()}>
            {/* No branch field where the conversation is adopting a roadmap: a
                stage is worked on its own slug, so the name invented when the
                row was made is discarded when the stage is adopted, and naming
                it here would be a field with nothing behind it. */}
            <Show when={!props.conversation.adopting}>
              <BranchName conversation={props.conversation} />
            </Show>
            <BaseBranch conversation={props.conversation} />
            <AddCompanion conversation={props.conversation} />

            {/* And the ones already added, under the control they were added
                from — they belong to the branch and the base rather than to the
                pairings, and they go with them when the round's branch is
                cut. */}
            <Companions conversation={props.conversation} />
          </Show>
        </>
      )}
    </RepoOptions>
  );
}

/// The option itself: the trigger standing in the row, and the panel that comes
/// down behind it.
///
/// Presentational, because two composers draw the same option over different
/// things — a Conversation, where everything inside the panel saves itself as
/// it is touched, and the compose page, where none of it exists anywhere until
/// something is created. What they share is the shape of the option: a dimmed
/// label over a value, the companions counted after the name, and one flat card
/// holding the whole of *which code*.
export function RepoOptions(props: {
  /// What the value line reads — the repository's name, or the invitation to
  /// pick one where nothing is picked yet.
  name: string;
  /// How many other repos the work runs alongside, for the `+N` after it.
  alongside: number;
  children: () => JSX.Element;
}): JSX.Element {
  return (
    <Menu
      panel
      class={styles.repoOption!}
      name="Repo setup"
      trigger={
        <>
          <span class={styles.optionLabel}>Repo</span>
          <span class={styles.optionLine}>
            <span class={styles.optionValue}>
              {props.name}
              {/* The companions counted rather than named: the row is one line
                  and the names are inside the panel, where they can be read
                  beside what the work will do with them. */}
              <Show when={props.alongside}>{(many) => <> +{many()}</>}</Show>
            </span>
            <span class={styles.optionArrow} aria-hidden="true">
              ▾
            </span>
          </span>
        </>
      }
    >
      {props.children}
    </Menu>
  );
}

/// Which Repo the work is in at all: the first thing in the Repo panel, and the
/// one the branch, the base and the companions under it are facts about.
///
/// A drafting Conversation can be moved onto another registered Repo, and three
/// things follow from the move — the base goes back to the new repo's default,
/// a companion that has just become the Conversation's own Repo goes away, and
/// the branch name and the pairings stay exactly where they were. None of that
/// is asked for here: the server does it, and this panel reads the result off
/// the Conversation it re-reads.
///
/// Disabled once the branch has been cut, which is the one control in this
/// panel that is drawn in that state rather than taken away. A checkout is of
/// one repository, so what it says then is a fact about the work — *this is the
/// repo, and it is settled* — and a fact is worth reading where a refused field
/// is not.
///
/// **Nothing is filtered out of the list**, for [`AddCompanion`]'s reason: the
/// repo the Conversation is already on is in it, and picking it is a switch
/// onto where it already is, which changes nothing but the base.
function RepoPicker(props: {
  conversation: ConversationView;
  disabled: boolean;
}): JSX.Element {
  const queries = useQueryClient();

  const [refused, setRefused] = createSignal<RepoSwitched | null>(null);

  const move = useMutation(() => ({
    mutationFn: (repoId: number) => switchRepo(props.conversation.id, repoId),
    onSuccess: (outcome: RepoSwitched) => {
      if (outcome !== "Switched") {
        setRefused(outcome);
        // Refused about one of the two lists this control was drawn over: the
        // registry it picked out of, or the Conversation the pick was about.
        void queries.invalidateQueries({ queryKey: ["repos"] });
        void queries.invalidateQueries({ queryKey: ["conversation"] });
        return;
      }

      setRefused(null);
      // The whole panel is about the repo that has just changed — and so is the
      // sidebar row and every pane head, which read the same record.
      void queries.invalidateQueries({ queryKey: ["conversation"] });
      void queries.invalidateQueries({ queryKey: ["conversations"] });
    },
  }));

  return (
    <RepoChoice
      chosen={String(props.conversation.repo.id)}
      also={props.conversation.repo}
      disabled={props.disabled || move.isPending}
      pick={(repoId) => move.mutate(repoId)}
    >
      {/* What moving the work would take with it, said before it is done rather
          than after: the base is the one thing here that a switch resets. */}
      <Show when={!props.disabled}>
        <Note class={styles.aside}>
          Moving this onto another repo puts its base back on that repo's
          default branch. Its branch name, its pairings and the repos it works
          alongside are kept.
        </Note>
      </Show>

      <Show when={refused()}>
        {(outcome) => (
          <ErrorLine class={styles.failure}>
            {REPO_SWITCH_REFUSAL[outcome()]}
          </ErrorLine>
        )}
      </Show>
      <Show when={move.isError}>
        <ErrorLine class={styles.failure}>
          The repo could not be switched: {move.error?.message}
        </ErrorLine>
      </Show>
    </RepoChoice>
  );
}

/// The control itself: every registered Repo, and whatever is chosen wherever
/// the list does not hold it.
///
/// Presentational and shared, for [`RepoOptions`]'s reason. What a pick *does*
/// is the caller's — a move on a saved Conversation, a field of the compose
/// state — and so is everything said under it.
export function RepoChoice(props: {
  /// What is chosen, as the picker writes it: the Repo's id, or the empty
  /// string where nothing is picked yet, which is only ever the compose page.
  chosen: string;
  /// The Repo to offer wherever the list does not hold it — the list still on
  /// its way, or one unregistered since the work was started on it. Drawn
  /// either way, because a picker showing one repo while the record held
  /// another would be the panel disagreeing with its own trigger.
  also?: RepoEntry;
  disabled?: boolean;
  pick: (repoId: number) => void;
  /// What the caller has to say under it. The refusals above all: what a pick
  /// is refused for is a fact about the caller's record rather than about this
  /// control.
  children?: JSX.Element;
}): JSX.Element {
  const repos = useReading(() => ({
    queryKey: ["repos"],
    queryFn: listRepos,

    // Merged by the id each row carries flat, for [`CompanionChoice`]'s reason:
    // a Nudge landing while the human has the dropdown open must not take their
    // choice with it.
    freshness: { reconcile: "id" },
  }));

  const options = (): RepoEntry[] => {
    const listed = repos.data ?? [];
    const held = props.also;

    return held === undefined || listed.some((repo) => repo.id === held.id)
      ? listed
      : [held, ...listed];
  };

  return (
    <div class={styles.repoPick}>
      <label for="conversation-repo">Repo</label>
      <Switch>
        <Match when={repos.isError}>
          <ErrorLine class={styles.failure}>
            Could not read the repos: {repos.error?.message}
          </ErrorLine>
        </Match>
        <Match when={true}>
          {/* A [`Picker`] rather than a `<select>`, so this cannot come to show
              one repo while the mutation behind it would record another — the
              same reason the base picker under it is one. A Conversation is on
              a repo from the moment it exists, so the placeholder is the
              compose page's alone. */}
          <Picker
            id="conversation-repo"
            options={options()}
            value={(repo) => String(repo.id)}
            label={(repo) => repo.name}
            chosen={props.chosen}
            disabled={props.disabled}
            pick={(repo) => props.pick(Number(repo))}
          />
        </Match>
      </Switch>

      {props.children}
    </div>
  );
}
/// What a Rust repository loses on a server with no sccache: the compiling.
///
/// Drawn only where all three hold — the repository is a Cargo workspace, the
/// shared build cache is switched on, and the server found no sccache — which
/// is the one boolean the server sends rather than three for this to combine.
///
/// A note rather than a refusal, and it gates nothing. The work runs perfectly
/// well: the crate downloads are still shared, and what is lost is time. It is
/// here because this is where somebody is about to spend that time, and because
/// what fixes it is on the server rather than in this browser.
function UncachedCompiles(props: {
  conversation: ConversationView;
}): JSX.Element {
  return (
    <Show when={props.conversation.compiles_uncached}>
      <p class={styles.uncached}>
        No sccache is installed where the server can see it, so this
        repository's dependency compiles will not be cached — only its crate
        downloads. Install sccache on the server to cache the compiling too.
      </p>
    </Show>
  );
}
/// The three pairings the work will run under — two of which may be picked
/// away instead — one option of the row each, the role as the label and the
/// pairing as the value.
///
/// The profile list is read here rather than passed down, so the pickers are
/// whole wherever they are drawn — the sidebar does the same with the repos. The
/// pairings are made of it here: a row per profile-and-model combination, which
/// is what a picker offers.
///
/// The three stand in the row rather than in a section of their own, and there
/// is no heading over them: the role is written on each one, so a word above
/// all three would be the row saying what its labels already say.
function Profiles(props: { conversation: ConversationView }): JSX.Element {
  return (
    <ProfileChoices>
      {(saved) => (
        <>
          {/* One of the two pickers with a row that is not an account: a
              brief can go straight to the work, with no interview between
              the two. */}
          <PairingPicker
            conversation={props.conversation}
            saved={saved()}
            role="grilling"
            label="Grilling"
            away="No grilling"
            chosen={pairing.settled(props.conversation.grilling_pairing)}
            pairing={pairing.under(props.conversation.grilling_pairing)}
            choose={(id, picked) =>
              chooseGrillingPairing(id, pairing.role(picked))
            }
          />
          <PairingPicker
            conversation={props.conversation}
            saved={saved()}
            role="implementation"
            label="Implementation"
            chosen={pairing.chosen(props.conversation.implementation_pairing)}
            pairing={props.conversation.implementation_pairing}
            choose={(id, picked) =>
              chooseImplementationPairing(id, pairing.choice(picked))
            }
          />
          {/* And the other: a conversation can be wrapped up without being
              reviewed at all, and that is picked here rather than anywhere
              else. */}
          <PairingPicker
            conversation={props.conversation}
            saved={saved()}
            role="review"
            label="Review"
            away="No review"
            chosen={pairing.settled(props.conversation.review_pairing)}
            pairing={pairing.under(props.conversation.review_pairing)}
            choose={(id, picked) =>
              chooseReviewPairing(id, pairing.role(picked))
            }
          />
        </>
      )}
    </ProfileChoices>
  );
}

/// The saved profiles, read once for whatever asks who runs a session — and
/// the two things that can stand in place of the pickers: a read that failed,
/// and a workbench with no profile saved in it yet.
///
/// Shared for [`RepoOptions`]'s reason: the compose page asks the same three
/// questions of the same list, and a second read of it here would be the same
/// list twice.
export function ProfileChoices(props: {
  children: (saved: Accessor<ProfileEntry[]>) => JSX.Element;
}): JSX.Element {
  const profiles = useReading(() => ({
    queryKey: ["profiles"],
    queryFn: listProfiles,

    // Merged by the id each row carries flat, for the pickers below: a rebuilt
    // `<option>` is a new element in a `<select>` the human may have open, and
    // a list re-read while they were choosing would take the choice with it.
    freshness: { reconcile: "id" },
  }));

  return (
    <Switch>
      <Match when={profiles.isError}>
        <ErrorLine class={styles.failure}>
          Could not read the agent profiles: {profiles.error?.message}
        </ErrorLine>
      </Match>
      <Match when={profiles.data?.length === 0}>
        {/* Nothing to choose, so the only thing to offer is the page that
            fixes that. */}
        <Empty class={styles.nothing}>
          No agent profiles are saved yet — <A href="/settings">add one</A> to
          run a session under.
        </Empty>
      </Match>
      <Match when={profiles.data}>{props.children}</Match>
    </Switch>
  );
}

/// One of the three choices: which profile-and-model pairing fills this role —
/// or, where the role can be picked away, that it runs nothing.
///
/// A dropdown rather than a list of buttons, because the pairings are a short
/// list that barely changes and the choice is one of them. One flat row per
/// pairing rather than a profile picker with a model picker after it: the counts
/// stay small, and two stages would cost a tap every time.
///
/// The app's own listbox rather than a `<select>`, because every row carries the
/// mark of the harness it runs and an `<option>` holds nothing but text — the
/// mark is what makes a column of accounts scannable, which is the whole reason
/// these three rows are worth drawing by hand.
///
/// `away` is the row a role that can run nothing offers above the pairings,
/// where it offers one. In the same flat list rather than beside it as a switch,
/// because it is the same decision: what runs this, and one of the answers is
/// nobody.
function PairingPicker(props: {
  conversation: ConversationView;
  saved: ProfileEntry[];
  role: string;
  label: string;
  away?: string;
  chosen: string;
  pairing: PairingView | null;
  choose: (id: number, picked: string) => Promise<ProfileChosen>;
}): JSX.Element {
  const queries = useQueryClient();

  const [refused, setRefused] = createSignal<ProfileChosen | null>(null);

  const choose = useMutation(() => ({
    mutationFn: (picked: string) => props.choose(props.conversation.id, picked),
    onSuccess: (outcome: ProfileChosen) => {
      if (outcome !== "Chosen") {
        setRefused(outcome);
        // Chosen from a list this option read a moment ago: reading it again
        // is both the correction and the explanation.
        void queries.invalidateQueries({ queryKey: ["profiles"] });
        return;
      }

      setRefused(null);
      void queries.invalidateQueries({ queryKey: ["conversation"] });
    },
  }));

  return (
    <RolePicker
      saved={props.saved}
      role={props.role}
      label={props.label}
      away={props.away}
      chosen={props.chosen}
      pick={(picked) => choose.mutate(picked)}
      disabled={choose.isPending}
    >
      {/* A profile chosen before models were paired with them: half a choice,
          which the picker draws as none. Said in words rather than left as a
          bare placeholder, because the conversation does have a profile. */}
      <Show when={props.pairing && !props.pairing.model}>
        <Note class={styles.unpaired}>
          {props.pairing?.profile.name} was chosen before models were picked
          beside them. Pick one to pair.
        </Note>
      </Show>

      {/* What is wrong with the one that is chosen, said where it is chosen. */}
      <Show when={props.pairing?.profile.broken}>
        {(broken) => <ErrorLine class={styles.broken}>{BROKEN[broken()]}</ErrorLine>}
      </Show>
      <Show when={refused()}>
        {(outcome) => (
          <ErrorLine class={styles.failure}>{CHOICE_REFUSAL[outcome()]}</ErrorLine>
        )}
      </Show>
      <Show when={choose.isError}>
        <ErrorLine class={styles.failure}>
          The profile could not be chosen: {choose.error?.message}
        </ErrorLine>
      </Show>
    </RolePicker>
  );
}

/// The control itself: one option of the row, the role as its label and the
/// pairing as its value.
///
/// Presentational and shared, for [`RepoOptions`]'s reason — the compose page
/// asks the same three questions before there is a Conversation for an answer
/// to be about. What a pick *does* is the caller's, and so is everything said
/// under it.
export function RolePicker(props: {
  saved: ProfileEntry[];
  /// What this control is called in the document, for the `<label>` over it.
  role: string;
  label: string;
  /// The row a role that can run nothing offers above the pairings, where it
  /// offers one.
  away?: string;
  chosen: string;
  pick: (picked: string) => void;
  disabled?: boolean;
  children?: JSX.Element;
}): JSX.Element {
  /// Every row the control offers: the pairings, and the row that runs nothing
  /// above them where this role has one. Above rather than below, because it is
  /// the choice that says *skip this* and a reader scanning accounts should meet
  /// it before the accounts.
  const rows = (): Row[] => [
    ...(props.away
      ? // No mark: the row is not an account, so there is no harness for one to
        // be of — see [`Row`].
        [{ value: pairing.NONE, label: props.away, mark: null }]
      : []),
    ...pairing.pairings(props.saved).map((row) => ({
      value: pairing.value(row),
      // The whole list beside each row, because how one reads depends on the
      // rest of it: the profile's name is said after the model only where its
      // backend has more than one account saved.
      label: pairing.label(row, props.saved),
      mark: row.profile.account.agent_type,
    })),
  ];

  return (
    <div class={styles.profileChoice}>
      <label class={styles.optionLabel} for={`${props.role}-pairing`}>
        {props.label}
      </label>
      {/* A [`Listbox`] rather than a `<select>`, so this cannot come to show one
          pairing while the mutation below would choose another — and so that
          every row can carry its harness's mark, which an `<option>` cannot
          hold. See `src/picking.tsx`.

          The empty value is the state of having chosen nothing, and it is not
          an option to go back to: a conversation with no pairing is one that
          will not grill, so the placeholder disappears once one is picked. It
          comes back if the profile that was picked is deleted, or if it stopped
          listing the model it was paired with, which is the honest reading of
          it — and nothing is said upwards about that, the choice being the
          server's record rather than this option's to clear.

          The row that runs nothing is not that state and never sends the empty
          string: it is a choice like the pairings, and the placeholder stands
          above it until one of them is made. */}
      <Listbox
        id={`${props.role}-pairing`}
        class={styles.optionPick}
        options={rows()}
        value={(row) => row.value}
        label={(row) => row.label}
        mark={(row) => row.mark}
        chosen={props.chosen}
        pick={(picked) => props.pick(picked)}
        disabled={props.disabled}
      />

      {props.children}
    </div>
  );
}

/// One row of a picker, as the control reads it: what it sends, what it says,
/// and whose mark goes in front of the words.
///
/// Made here rather than taken from the pairings, because the review picker's
/// list is not only pairings — see [`PairingPicker`]. Which is the whole of why
/// the mark is `null`able: the row that runs nothing is not an account, so there
/// is no harness for a mark to be of, and the control draws no element rather
/// than a gap where one would have been.
type Row = { value: string; label: string; mark: AgentType | null };

/// The branch the work will be done on: empty until the human names one, and
/// theirs to change until grilling begins.
///
/// Empty is not *no branch*. A name was invented when the Conversation was
/// started, because there has to be one to cut, and the field stands empty
/// under a placeholder rather than showing it: a name nobody chose is nothing
/// to read, and what the human does about it is either type one or leave it
/// alone. Clearing the field goes back to that, and the Conversation goes back
/// to being a Draft.
///
/// Nothing is created by naming it. The branch itself arrives with the stage
/// that starts grilling; this is the name it will be given.
///
/// It keeps itself the way the Brief above it does — on a pause in the typing
/// and on the way out of the field — because it is the same panel and a field
/// with a button beside it would be the one thing in that panel asking to be
/// pressed. There is no word about saving either: what a save cannot do is
/// said, and what it did is the name in the field and in the sidebar.
function BranchName(props: { conversation: ConversationView }): JSX.Element {
  const queries = useQueryClient();

  // What has been typed, or nothing if nothing has been: the field follows the
  // Conversation until the first keystroke and follows itself after it, so a
  // read landing mid-name cannot take the name with it.
  const [named, setNamed] = createSignal<string | null>(null);
  const [refused, setRefused] = createSignal<BranchRenamed | null>(null);

  /// What is in the field: what has been typed, or the name as it stands —
  /// which is nothing at all while the name is still Verkstead's.
  const branch = () => named() ?? chosen(props.conversation);

  // The last name a save asked for, whatever became of it: where the save
  // landed it is what the record has, and where it was refused it is the name
  // the refusal was about. Either way, asking again for that same string would
  // only get the same answer back.
  const [asked, setAsked] = createSignal<string | null>(null);
  const recorded = () => asked() ?? chosen(props.conversation);

  /// Whether the field has moved on since the last save, which is the whole of
  /// what there is to save.
  const unsaved = () => branch() !== recorded();

  /// Whether what came back is worth not asking about again.
  ///
  /// A refusal only stops the field where the refusal is permanent.
  /// `NotABranchName` is not: the name is validated server-side alone, so a
  /// pause in the middle of typing one is refused for what is not there yet,
  /// and the keystroke after it may well be what fixes it. What stops that
  /// becoming a request a second is that the same string is never asked about
  /// twice.
  const settled = () => {
    const outcome = refused();
    return outcome === "NoSuchConversation" || outcome === "NotDrafting";
  };

  const rename = useMutation(() => ({
    mutationFn: (branch: string) => renameBranch(props.conversation.id, branch),
    onSuccess: (outcome: BranchRenamed) => {
      if (outcome !== "Renamed") {
        setRefused(outcome);
        return;
      }

      setRefused(null);
      void queries.invalidateQueries({ queryKey: ["conversation"] });
      void queries.invalidateQueries({ queryKey: ["conversations"] });
    },
    // Whatever became of it, the field may have been typed into while it was in
    // flight — so the moment one save is done the next is considered.
    onSettled: () => keeper.done(),
  }));

  const keeper = keeping({
    unsaved,
    settled,
    save: () => {
      const name = branch();
      setAsked(name);
      rename.mutate(name);
    },
  });

  return (
    <BranchField
      id="branch"
      label="Branch"
      class={styles.branchName!}
      placeholder={AUTOMATIC}
      value={branch()}
      set={(name) => {
        setNamed(name);
        keeper.settle();
      }}
      leave={() => keeper.keep()}
    >
      {/* A refusal stands until the next save answers, rather than clearing on
          the next keystroke: it is the only thing that says why the sidebar is
          not following the field, and one that vanished as it was read would
          leave the human nothing. */}
      <Show when={refused()}>
        {(outcome) => (
          <ErrorLine class={styles.failure}>{BRANCH_REFUSAL[outcome()]}</ErrorLine>
        )}
      </Show>
      <Show when={rename.isError}>
        <ErrorLine class={styles.failure}>
          The branch could not be named: {rename.error?.message}
        </ErrorLine>
      </Show>
    </BranchField>
  );
}

/// A branch name being typed: the field, the label over it, and whatever the
/// caller has to say underneath.
///
/// The conversation's own name and a read-write companion's are the same field
/// asked twice, and the compose page asks both again against nothing saved — so
/// what is here is the field and the form around it, and what a name *does*
/// stays with whoever owns it.
///
/// A `<form>` because there is nothing in it to press: Enter in a field with no
/// button beside it is a save on the panels that save as they go, and nothing at
/// all on the one that does not.
export function BranchField(props: {
  id: string;
  /// What names it. Markup rather than a string, because a companion's field
  /// says which repository it belongs to in words nobody sees — see
  /// [`ForRepo`].
  label: JSX.Element;
  /// Which of the two fields this is, for the margin the panel gives it.
  class: string;
  value: string;
  placeholder?: string;
  set: (name: string) => void;
  /// What the way out of the field does, for the caller that does anything with
  /// it: the panels that save as they go keep the name here and on Enter, and
  /// the compose page has nowhere to keep it but where it already is.
  leave?: () => void;
  children?: JSX.Element;
}): JSX.Element {
  return (
    <form
      class={props.class}
      onSubmit={(ev) => {
        // Nothing to press, so this is Enter in the field: the same save the
        // pause was about to make, made now.
        ev.preventDefault();
        props.leave?.();
      }}
    >
      <label for={props.id}>{props.label}</label>
      <div class={styles.fieldLine}>
        <input
          id={props.id}
          type="text"
          autocapitalize="off"
          autocorrect="off"
          spellcheck={false}
          placeholder={props.placeholder}
          value={props.value}
          onInput={(ev) => props.set(ev.currentTarget.value)}
          onBlur={() => props.leave?.()}
        />
      </div>
      {props.children}
    </form>
  );
}

/// What the first entry of the base dropdown sends, which is nothing: the rule
/// is the override taken away, and the record holds no branch for it.
export const RULE = "";

/// The branch a checkout comes off, out of the repository's own branches.
///
/// One control for the conversation's own base, for each companion's, and for
/// each row of the steer modal's companion section, because it is one choice
/// asked in three places: the rule — that repository's default branch as it
/// stands when the checkout is made — as the first entry, then its branches.
/// There is nothing to type: a sha or a tag would pin the work to a moment, and
/// what this asks is which line of work to come off.
///
/// A picked branch is stored by name and resolved when grilling starts, so the
/// checkout comes off wherever that branch stands then — which is the point of
/// picking one over its tip.
///
/// The list is read under the *Repo's* key rather than the conversation's,
/// because that is what it belongs to: two conversations against one repository
/// are reading the same list, and so is every companion row that names it.
export function BasePicker(props: {
  /// The control's own id, for the `<label>` that names it — one per repo on a
  /// panel that may draw several of these.
  id: string;
  /// What names it. Markup rather than a string, because a companion's
  /// controls say which repository they belong to in words nobody sees —
  /// see [`ForRepo`].
  label: JSX.Element;
  repo: RepoEntry;
  /// What is chosen, as the dropdown writes it: the empty string is the rule.
  chosen: string;
  disabled?: boolean;
  /// What was picked, with `null` for the rule — the override taken away rather
  /// than a branch called nothing.
  pick: (branch: string | null) => void;
  /// What the caller has to say under it. The refusals above all: what a pick
  /// is refused for is a fact about the caller's record rather than about this
  /// control.
  children?: JSX.Element;
}): JSX.Element {
  const branches = useReading(() => ({
    queryKey: ["repos", props.repo.id, "branches"],
    queryFn: () => listBranches(props.repo.id),

    // Merged by position, there being no key on a string: a branch that is
    // still there is the same string, so the option drawn for it survives a
    // re-read whether or not the ones around it did.
    freshness: { reconcile: "id" },
  }));

  /// What is offered: the rule, then the repository's branches — and what is
  /// chosen wherever the list does not hold it, which is the list still on its
  /// way and a branch taken away since it was picked. Drawn either way, because
  /// falling quietly to the rule would show one base while the record held
  /// another.
  const options = (): string[] => {
    const listed = branches.data ?? [];

    return props.chosen === "" || listed.includes(props.chosen)
      ? [RULE, ...listed]
      : [RULE, props.chosen, ...listed];
  };

  return (
    <div class={styles.baseBranch}>
      <label for={props.id}>{props.label}</label>
      {/* A [`Picker`] rather than a `<select>`, so this cannot come to show one
          branch while the mutation behind it would record another — see
          `src/picking.tsx`. The rule is an option of its own rather than the
          picker's placeholder: it is a choice to go back to, and the
          placeholder is a state there is no way out of. */}
      <Picker
        id={props.id}
        options={options()}
        value={(branch) => branch}
        label={(branch) =>
          branch === RULE
            ? `Default branch (${props.repo.default_branch})`
            : branch
        }
        chosen={props.chosen}
        pick={(picked) => props.pick(picked === RULE ? null : picked)}
        disabled={props.disabled}
      />
      {/* The list is what there is to pick out of, so a read that failed is
          said rather than drawn as a repository with one branch. */}
      <Show when={branches.isError}>
        <ErrorLine class={styles.failure}>
          Could not read the repo's branches: {branches.error?.message}
        </ErrorLine>
      </Show>
      {props.children}
    </div>
  );
}

/// The branch the work itself comes off.
function BaseBranch(props: { conversation: ConversationView }): JSX.Element {
  const queries = useQueryClient();

  const [refused, setRefused] = createSignal<BaseRecorded | null>(null);

  const record = useMutation(() => ({
    mutationFn: (branch: string | null) =>
      setBaseBranch(props.conversation.id, branch),
    onSuccess: (outcome: BaseRecorded) => {
      if (outcome !== "Recorded") {
        setRefused(outcome);
        // Picked out of a list this panel read a moment ago: reading it again
        // is both the correction and the explanation.
        void queries.invalidateQueries({
          queryKey: ["repos", props.conversation.repo.id, "branches"],
        });
        return;
      }

      setRefused(null);
      void queries.invalidateQueries({ queryKey: ["conversation"] });
    },
  }));

  return (
    <BasePicker
      id="base-branch"
      label="Base branch"
      repo={props.conversation.repo}
      chosen={props.conversation.base_commit ?? RULE}
      disabled={record.isPending}
      pick={(branch) => record.mutate(branch)}
    >
      <Show when={refused()}>
        {(outcome) => (
          <ErrorLine class={styles.failure}>{BASE_REFUSAL[outcome()]}</ErrorLine>
        )}
      </Show>
      <Show when={record.isError}>
        <ErrorLine class={styles.failure}>
          The base branch could not be recorded: {record.error?.message}
        </ErrorLine>
      </Show>
    </BasePicker>
  );
}

/// The control that puts another registered Repo on the conversation, inside
/// the Repo panel with the branch and the base it belongs beside.
///
/// A picker whose first row is the invitation rather than a choice: what is
/// picked is done rather than held, so the control goes straight back to that
/// first row and the repository appears as one of the companion rows under it.
/// It was a ⋯ menu with a level of repositories inside it, which is what a row
/// in a *card* has to be — the panel is that card by now, and a menu opened
/// inside a popover to reach a list the popover had the room for would be two
/// popovers for one press.
///
/// The Repos are read here rather than passed down, so the control is whole
/// wherever it is drawn — the sidebar's own repo menu does the same.
///
/// **Nothing is filtered out of the list.** This conversation's own Repo and
/// one already added are both in it, and both are refused by name when they are
/// picked: the server is what decides either way, and a list that quietly left
/// a repository out would leave the human hunting for one that is registered.
function AddCompanion(props: { conversation: ConversationView }): JSX.Element {
  const queries = useQueryClient();

  const [refused, setRefused] = createSignal<CompanionAdded | null>(null);

  // What was picked, for as long as the add is in the air: the control shows it
  // while it is being made, and goes back to the invitation whatever became of
  // it — an add is done rather than held, and the repository's own row under
  // the control is where it is read afterwards.
  const [picked, setPicked] = createSignal("");

  const add = useMutation(() => ({
    mutationFn: (repoId: number) => addCompanion(props.conversation.id, repoId),
    onSuccess: (outcome: CompanionAdded) => {
      if (outcome !== "Added") {
        setRefused(outcome);
        // Every refusal is about one of two lists this control was drawn over:
        // the registered Repos, or the conversation the row would hang off.
        // Reading both again is the correction and the explanation together.
        void queries.invalidateQueries({ queryKey: ["repos"] });
        void queries.invalidateQueries({ queryKey: ["conversation"] });
        return;
      }

      setRefused(null);
      void queries.invalidateQueries({ queryKey: ["conversation"] });
    },
    onSettled: () => setPicked(""),
  }));

  return (
    <CompanionChoice
      chosen={picked()}
      disabled={add.isPending}
      add={(repoId) => {
        setPicked(String(repoId));
        add.mutate(repoId);
      }}
    >
      {/* Said under the control the press was made in, which is where the human
          is still standing: the panel does not shut on a refusal. */}
      <Show when={refused()}>
        {(outcome) => (
          <ErrorLine class={styles.failure}>
            {COMPANION_REFUSAL[outcome()]}
          </ErrorLine>
        )}
      </Show>
      <Show when={add.isError}>
        <ErrorLine class={styles.failure}>
          The companion repo could not be added: {add.error?.message}
        </ErrorLine>
      </Show>
    </CompanionChoice>
  );
}

/// The control itself, shared for [`RepoOptions`]'s reason: what putting a
/// repository alongside the work looks like, wherever the work is being set up.
///
/// A picker whose first row is the invitation rather than a choice. Nothing is
/// filtered out of the list, for [`AddCompanion`]'s reason, and what an add
/// *does* — a request on a saved Conversation, a row of the compose state — is
/// the caller's.
export function CompanionChoice(props: {
  /// What the control is showing: the invitation, or the repository an add
  /// still in flight is about.
  chosen: string;
  disabled?: boolean;
  add: (repoId: number) => void;
  /// What the caller has to say under it, the refusals above all.
  children?: JSX.Element;
}): JSX.Element {
  const repos = useReading(() => ({
    queryKey: ["repos"],
    queryFn: listRepos,

    // Merged by the id each row carries flat: a rebuilt `<option>` is a new
    // element in a `<select>` the human may have open, and a Nudge landing
    // while they were choosing would take the choice with it.
    freshness: { reconcile: "id" },
  }));

  /// What is offered: the invitation, and then every registered Repo.
  ///
  /// `null` is the invitation, and it sends the empty string — which is a row
  /// of the caller's own rather than the picker's placeholder, so nothing is
  /// drawn over it and picking it does nothing. See `src/picking.tsx`.
  const options = (): (RepoEntry | null)[] => [null, ...(repos.data ?? [])];

  return (
    <div class={styles.addCompanion}>
      <label for="add-companion">Works alongside</label>
      <Switch>
        <Match when={repos.isError}>
          <ErrorLine class={styles.failure}>
            Could not read the repos: {repos.error?.message}
          </ErrorLine>
        </Match>
        <Match when={repos.data?.length === 0}>
          {/* Nothing to work alongside, so the only thing to offer is the page
              that fixes that. */}
          <Empty class={styles.nothing}>
            No repos are registered yet — <A href="/settings">register one</A>{" "}
            to work alongside.
          </Empty>
        </Match>
        <Match when={repos.data}>
          <Picker
            id="add-companion"
            options={options()}
            value={(repo) => (repo ? String(repo.id) : "")}
            label={(repo) => repo?.name ?? "Add a repo…"}
            chosen={props.chosen}
            disabled={props.disabled}
            pick={(repo) => {
              // The invitation is not a repository, so picking it is not an
              // add: the control was already showing it.
              if (!repo) return;

              props.add(Number(repo));
            }}
          />
        </Match>
      </Switch>

      {props.children}
    </div>
  );
}

/// The repos this conversation works alongside, one row each under the control
/// they were added from, inside the Repo panel.
///
/// Nothing at all where there are none, rather than an empty list with a
/// heading over it: one repository is what most work needs, and a conversation
/// with no companions has nothing here to say.
function Companions(props: { conversation: ConversationView }): JSX.Element {
  return (
    <Show when={props.conversation.companions.length}>
      <ul class={styles.companions} aria-label="Companion repos">
        <For each={props.conversation.companions}>
          {(companion) => (
            <Companion
              conversation={props.conversation}
              companion={companion}
            />
          )}
        </For>
      </ul>
    </Show>
  );
}

/// One of them: what it is called, the × that takes it away, and what the work
/// will do with it — read it or commit to it, off which branch, on a branch
/// called what.
///
/// The name and the × on a line of their own, and the configuration under
/// them: the branch field is only drawn in one of the two modes, so a place for
/// it in that line would be a hole in the other.
function Companion(props: {
  conversation: ConversationView;
  companion: CompanionView;
}): JSX.Element {
  const queries = useQueryClient();

  const [refused, setRefused] = createSignal<CompanionRemoved | null>(null);

  const forget = useMutation(() => ({
    mutationFn: () =>
      removeCompanion(props.conversation.id, props.companion.repo.id),
    onSuccess: (outcome: CompanionRemoved) => {
      setRefused(outcome === "Removed" ? null : outcome);

      // Either way: what came back is about a conversation this panel read a
      // moment ago, so reading it again is both the correction and — where the
      // row is simply gone — the whole of what there was to do.
      void queries.invalidateQueries({ queryKey: ["conversation"] });
    },
  }));

  return (
    <li class={styles.companion}>
      <div class={styles.companionLine}>
        <span class={styles.companionName}>{props.companion.repo.name}</span>
        {/* A mark rather than a word, because the row is one line and the name
            beside it is what says which repository is being taken away. The
            screen reader gets the sentence. */}
        <button
          type="button"
          class={styles.forget}
          aria-label={`Remove ${props.companion.repo.name}`}
          disabled={forget.isPending}
          onClick={() => forget.mutate()}
        >
          ×
        </button>
      </div>
      {/* Under the line the × is on, which is what it is about. */}
      <Show when={refused()}>
        {(outcome) => (
          <ErrorLine class={styles.failure}>
            {COMPANION_REMOVAL_REFUSAL[outcome()]}
          </ErrorLine>
        )}
      </Show>
      <Show when={forget.isError}>
        <ErrorLine class={styles.failure}>
          The companion repo could not be removed: {forget.error?.message}
        </ErrorLine>
      </Show>

      {/* Where its checkout comes off, and how far into it the work may reach —
          side by side wherever there is room, as the two fields above them
          are. */}
      <div class={styles.companionConfig}>
        <CompanionBase
          conversation={props.conversation}
          companion={props.companion}
        />
        <CompanionAccess
          conversation={props.conversation}
          companion={props.companion}
        />
      </div>

      {/* And, where it may be written to, what its branch is called. Nothing at
          all where it may not: a read-only companion is checked out detached,
          so there is no branch to name. */}
      <Show when={props.companion.mode === "ReadWrite"}>
        <CompanionBranch
          conversation={props.conversation}
          companion={props.companion}
        />
      </Show>
    </li>
  );
}

/// Which repository a companion's control belongs to, for whoever is not
/// reading the row it stands in.
///
/// The labels on a companion row say *Base* and *Branch* and the switch says
/// *Read-write*, because the repository's name is the line above them — and a
/// screen reader tabbing from one control to the next gets no line above. So
/// every one of them carries the name inside its own label, and none of them
/// shows it twice.
export function ForRepo(props: { repo: string }): JSX.Element {
  return <span class={styles.forRepo}> for {props.repo}</span>;
}

/// The branch this companion's checkout comes off.
///
/// The same control the conversation's own base is picked with, over the
/// companion repository's own branches: the two are different repositories, and
/// what is offered here is what *this* one has.
function CompanionBase(props: {
  conversation: ConversationView;
  companion: CompanionView;
}): JSX.Element {
  const queries = useQueryClient();

  const [refused, setRefused] = createSignal<CompanionBaseRecorded | null>(null);

  const record = useMutation(() => ({
    mutationFn: (branch: string | null) =>
      setCompanionBase(props.conversation.id, props.companion.repo.id, branch),
    onSuccess: (outcome: CompanionBaseRecorded) => {
      if (outcome !== "Recorded") {
        setRefused(outcome);
        // Every refusal is about one of the two lists this row was drawn over:
        // the companion repository's branches, or the conversation the row
        // hangs off. Reading both again is the correction and the explanation.
        void queries.invalidateQueries({
          queryKey: ["repos", props.companion.repo.id, "branches"],
        });
        void queries.invalidateQueries({ queryKey: ["conversation"] });
        return;
      }

      setRefused(null);
      void queries.invalidateQueries({ queryKey: ["conversation"] });
    },
  }));

  return (
    <BasePicker
      id={`companion-${props.companion.repo.id}-base`}
      label={<>Base<ForRepo repo={props.companion.repo.name} /></>}
      repo={props.companion.repo}
      chosen={props.companion.base_ref ?? RULE}
      disabled={record.isPending}
      pick={(branch) => record.mutate(branch)}
    >
      <Show when={refused()}>
        {(outcome) => (
          <ErrorLine class={styles.failure}>
            {COMPANION_BASE_REFUSAL[outcome()]}
          </ErrorLine>
        )}
      </Show>
      <Show when={record.isError}>
        <ErrorLine class={styles.failure}>
          The base branch could not be recorded: {record.error?.message}
        </ErrorLine>
      </Show>
    </BasePicker>
  );
}

/// How far into this companion the work may reach: read it, or work in it.
///
/// A switch rather than two rows in a dropdown, because it is a state rather
/// than one of a set — and it is the one control on the row that changes what
/// the row is: flipping it on is what reveals the branch field, a branch being
/// what a repository the work may commit to needs and a read-only one has
/// none of.
function CompanionAccess(props: {
  conversation: ConversationView;
  companion: CompanionView;
}): JSX.Element {
  const queries = useQueryClient();

  const [refused, setRefused] = createSignal<CompanionModeChosen | null>(null);

  const choose = useMutation(() => ({
    mutationFn: (mode: CompanionMode) =>
      setCompanionMode(props.conversation.id, props.companion.repo.id, mode),
    onSuccess: (outcome: CompanionModeChosen) => {
      setRefused(outcome === "Chosen" ? null : outcome);

      // Either way: what came back is about a conversation this panel read a
      // moment ago, and the switch draws what the record says rather than what
      // was pressed — so reading it again is both the correction and the way
      // the flip lands.
      void queries.invalidateQueries({ queryKey: ["conversation"] });
    },
  }));

  return (
    <div class={styles.companionMode}>
      <Toggle
        label={<>Read-write<ForRepo repo={props.companion.repo.name} /></>}
        on={props.companion.mode === "ReadWrite"}
        disabled={choose.isPending}
        flip={(on) => choose.mutate(on ? "ReadWrite" : "ReadOnly")}
      />
      <Show when={refused()}>
        {(outcome) => (
          <ErrorLine class={styles.failure}>
            {COMPANION_MODE_REFUSAL[outcome()]}
          </ErrorLine>
        )}
      </Show>
      <Show when={choose.isError}>
        <ErrorLine class={styles.failure}>
          The companion repo's mode could not be set: {choose.error?.message}
        </ErrorLine>
      </Show>
    </div>
  );
}

/// What a read-write companion's branch is called.
///
/// Empty in the record is *mirroring*: the branch is whatever the conversation's
/// own is called, so renaming that one renames this with it. The field is drawn
/// prefilled with that name rather than empty, so what the human reads is what
/// they will get — and the first thing typed into it is a name of its own that
/// no longer follows. Clearing it back to empty is going back to mirroring,
/// which is why the field fills itself in again afterwards.
///
/// Where the conversation has no name of its own yet there is nothing to
/// prefill it with, so it stands empty as well. Still mirroring, and still what
/// the human will get: the name it follows is the one they have not chosen.
///
/// It keeps itself the way the branch field above it does — on a pause in the
/// typing, on the way out of the field and on Enter — because it is the same
/// panel, and a Save button here would be the one thing in it asking to be
/// pressed.
function CompanionBranch(props: {
  conversation: ConversationView;
  companion: CompanionView;
}): JSX.Element {
  const queries = useQueryClient();

  const [named, setNamed] = createSignal<string | null>(null);
  const [refused, setRefused] = createSignal<CompanionBranchRenamed | null>(
    null,
  );

  /// What the record comes to: the name it holds, or the conversation's own
  /// where it holds none.
  const mirrored = () => props.companion.branch || chosen(props.conversation);

  /// What is in the field: what has been typed, or what the record comes to.
  const branch = () => named() ?? mirrored();

  // The last name a save asked for, whatever became of it — see the branch
  // field above, which holds the same two signals for the same reasons.
  const [asked, setAsked] = createSignal<string | null>(null);
  const recorded = () => asked() ?? mirrored();

  const unsaved = () => branch() !== recorded();

  /// Whether what came back is worth not asking about again. `NotABranchName`
  /// is not: a pause in the middle of typing a name is refused for what is not
  /// there yet, and the keystroke after it may well be what fixes it.
  const settled = () => {
    const outcome = refused();
    return (
      outcome === "NoSuchConversation" ||
      outcome === "NotDrafting" ||
      outcome === "NoSuchCompanion"
    );
  };

  const rename = useMutation(() => ({
    mutationFn: (branch: string) =>
      renameCompanionBranch(
        props.conversation.id,
        props.companion.repo.id,
        branch,
      ),
    onSuccess: (outcome: CompanionBranchRenamed) => {
      if (outcome !== "Renamed") {
        setRefused(outcome);
        return;
      }

      setRefused(null);

      // A name cleared away is mirroring rather than a branch called nothing,
      // so the field goes back to following the record — which is about to say
      // the conversation's own branch, and that is what this companion's will
      // be called.
      if (asked() === "") {
        setNamed(null);
        setAsked(null);
      }

      void queries.invalidateQueries({ queryKey: ["conversation"] });
    },
    // Whatever became of it, the field may have been typed into while it was in
    // flight — so the moment one save is done the next is considered.
    onSettled: () => keeper.done(),
  }));

  const keeper = keeping({
    unsaved,
    settled,
    save: () => {
      const name = branch();
      setAsked(name);
      rename.mutate(name);
    },
  });

  return (
    <BranchField
      id={`companion-${props.companion.repo.id}-branch`}
      label={
        <>
          Branch
          <ForRepo repo={props.companion.repo.name} />
        </>
      }
      class={styles.companionBranch!}
      value={branch()}
      set={(name) => {
        setNamed(name);
        keeper.settle();
      }}
      leave={() => keeper.keep()}
    >
      <Show when={refused()}>
        {(outcome) => (
          <ErrorLine class={styles.failure}>
            {COMPANION_BRANCH_REFUSAL[outcome()]}
          </ErrorLine>
        )}
      </Show>
      <Show when={rename.isError}>
        <ErrorLine class={styles.failure}>
          The branch could not be named: {rename.error?.message}
        </ErrorLine>
      </Show>
    </BranchField>
  );
}
