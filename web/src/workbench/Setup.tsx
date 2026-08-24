//! A Conversation's setup: what has to be settled before anything will run it,
//! drawn under the Brief it belongs to.
//!
//! The branch the work will be done on, the branch it will come off, and the
//! two pairings its sessions run under. All four are facts about the
//! Conversation rather than about any one Event, and all four are the human's to
//! change for as long as it is still drafting.
//!
//! Under the Brief rather than in a pane of its own, because setting a
//! Conversation up and kicking it off are one act and both belong where the work
//! is read: the Brief is the headline and the setup follows it. Once grilling
//! starts none of this is drawn at all — the server freezes every one of them at
//! that moment, so nothing taken away was still actionable, and the card goes
//! back to being the Brief alone.
//!
//! The two pairings are separate choices because they are genuinely separate
//! accounts — grill on fable, implement on opus — and because the implementation
//! session cannot simply carry the grilling one on.

import { A } from "@solidjs/router";
import { useMutation, useQueryClient } from "@tanstack/solid-query";
import {
  Match,
  Show,
  Switch,
  createSignal,
  onCleanup,
  type JSX,
} from "solid-js";

import {
  chooseGrillingPairing,
  chooseImplementationPairing,
  listBranches,
  listProfiles,
  renameBranch,
  setBaseBranch,
} from "../api/client";
import type {
  BaseRecorded,
  BranchRenamed,
  ConversationView,
  PairingView,
  ProfileChoice,
  ProfileChosen,
  ProfileEntry,
} from "../api/types";
import { useReading } from "../freshness";
import * as pairing from "../pairing";
import { Picker } from "../picking";
import { BROKEN } from "../profiles/ProfileList";
import { SETTLE } from "./settling";

/// What each way of being refused a branch name says.
export const BRANCH_REFUSAL: Record<BranchRenamed, string> = {
  Renamed: "",
  NoSuchConversation: "This conversation is gone.",
  NotDrafting:
    "The branch exists by now, so its name is not a text field any more.",
  NotABranchName: "Git will not take that as a branch name.",
};

/// And a base branch.
export const BASE_REFUSAL: Record<BaseRecorded, string> = {
  Recorded: "",
  NoSuchConversation: "This conversation is gone.",
  NotDrafting: "The base commit was captured when grilling started.",
  NoSuchBranch: "That repo has no branch by that name any more.",
};

/// And a pairing choice.
export const CHOICE_REFUSAL: Record<ProfileChosen, string> = {
  Chosen: "",
  NoSuchConversation: "This conversation is gone.",
  NoSuchProfile: "That profile has been removed.",
  NoSuchModel: "That profile no longer lists that model.",
  NotDrafting:
    "The grilling has started, so who runs this conversation is settled.",
};

export function Setup(props: {
  conversation: ConversationView;
}): JSX.Element {
  return (
    <section class="conversation-setup" aria-label="Setup">
      {/* No branch field where the conversation is adopting a roadmap: a stage
          is worked on its own slug, so the name invented when the row was made
          is discarded when the stage is adopted, and naming it here would be a
          field with nothing behind it. */}
      <Show when={!props.conversation.adopting}>
        <BranchName conversation={props.conversation} />
      </Show>
      <BaseBranch conversation={props.conversation} />

      <Profiles conversation={props.conversation} />
    </section>
  );
}

/// The two pairings the work will run under, and whether everything grilling
/// needs is settled.
///
/// The profile list is read here rather than passed down, so the pickers are
/// whole wherever they are drawn — the sidebar does the same with the repos. The
/// pairings are made of it here: a row per profile-and-model combination, which
/// is what a picker offers.
function Profiles(props: { conversation: ConversationView }): JSX.Element {
  const profiles = useReading(() => ({
    queryKey: ["profiles"],
    queryFn: listProfiles,

    // Merged by the id each row carries flat, for the pickers below: a rebuilt
    // `<option>` is a new element in a `<select>` the human may have open, and
    // a list re-read while they were choosing would take the choice with it.
    freshness: { reconcile: "id" },
  }));

  return (
    <section class="conversation-profiles" aria-label="Agent profiles">
      <h3>Agent profiles</h3>

      <Switch>
        <Match when={profiles.isError}>
          <p class="error">
            Could not read the agent profiles: {profiles.error?.message}
          </p>
        </Match>
        <Match when={profiles.data?.length === 0}>
          {/* Nothing to choose, so the only thing to offer is the page that
              fixes that. */}
          <p class="empty">
            No agent profiles are saved yet —{" "}
            <A href="/settings">add one</A> to run a session under.
          </p>
        </Match>
        <Match when={profiles.data}>
          {(saved) => (
            <>
              <PairingPicker
                conversation={props.conversation}
                saved={saved()}
                role="grilling"
                label="Grilling"
                chosen={props.conversation.grilling_pairing}
                choose={chooseGrillingPairing}
              />
              <PairingPicker
                conversation={props.conversation}
                saved={saved()}
                role="implementation"
                label="Implementation"
                chosen={props.conversation.implementation_pairing}
                choose={chooseImplementationPairing}
              />
            </>
          )}
        </Match>
      </Switch>

      {/* Whether this conversation will grill, which is the server's rule and
          not a count of the two fields above: a profile whose pair has gone is
          not one to launch a session under, and there is more to being ready
          than the pairings. Said here because this is where the pairings are
          fixed; the button it gates is at the end of the record below.

          An adopting conversation never grills, so that verdict is not the one
          to draw for it — it would read as needing a brief nobody here writes.
          What stands instead is why both pairings are fixed all the same. */}
      <Show
        when={!props.conversation.adopting}
        fallback={
          <p class="note">
            Both pairings are fixed before adopting: the implementation one is
            what the work runs under, and the grilling one is carried, because
            the stages after this one inherit both from it.
          </p>
        }
      >
        <p
          class="note readiness"
          classList={{ ready: props.conversation.ready_to_grill }}
        >
          <Show
            when={props.conversation.ready_to_grill}
            fallback={
              <>
                Not ready to grill: this needs a brief, and both pairings chosen
                and working.
              </>
            }
          >
            Ready to grill.
          </Show>
        </p>
      </Show>
    </section>
  );
}

/// One of the two choices: which profile-and-model pairing fills this role.
///
/// A select rather than a list of buttons, because the pairings are a short list
/// that barely changes and the choice is one of them — the same control the
/// sidebar picks a repo with. One flat row per pairing rather than a profile
/// picker with a model picker after it: the counts stay small, and two stages
/// would cost a tap every time.
function PairingPicker(props: {
  conversation: ConversationView;
  saved: ProfileEntry[];
  role: string;
  label: string;
  chosen: PairingView | null;
  choose: (id: number, choice: ProfileChoice) => Promise<ProfileChosen>;
}): JSX.Element {
  const queries = useQueryClient();

  const [refused, setRefused] = createSignal<ProfileChosen | null>(null);

  const choose = useMutation(() => ({
    mutationFn: (choice: ProfileChoice) =>
      props.choose(props.conversation.id, choice),
    onSuccess: (outcome: ProfileChosen) => {
      if (outcome !== "Chosen") {
        setRefused(outcome);
        // Chosen from a list this card read a moment ago: reading it again is
        // both the correction and the explanation.
        void queries.invalidateQueries({ queryKey: ["profiles"] });
        return;
      }

      setRefused(null);
      void queries.invalidateQueries({ queryKey: ["conversation"] });
    },
  }));

  return (
    <div class="profile-choice">
      <label for={`${props.role}-pairing`}>{props.label}</label>
      {/* A [`Picker`] rather than a `<select>`, so this cannot come to show one
          pairing while the mutation below would choose another — see
          `src/picking.tsx`.

          The empty value is the state of having chosen nothing, and it is not
          an option to go back to: a conversation with no pairing is one that
          will not grill, so the placeholder disappears once one is picked. It
          comes back if the profile that was picked is deleted, or if it stopped
          listing the model it was paired with, which is the honest reading of
          it — and nothing is said upwards about that, the choice being the
          server's record rather than this card's to clear. */}
      <Picker
        id={`${props.role}-pairing`}
        options={pairing.pairings(props.saved)}
        value={pairing.value}
        label={pairing.label}
        chosen={pairing.chosen(props.chosen)}
        pick={(picked) => choose.mutate(pairing.choice(picked))}
        disabled={choose.isPending}
      />

      {/* A profile chosen before models were paired with them: half a choice,
          which the picker draws as none. Said in words rather than left as a
          bare placeholder, because the conversation does have a profile. */}
      <Show when={props.chosen && !props.chosen.model}>
        <p class="note unpaired">
          {props.chosen?.profile.name} was chosen before models were picked
          beside them. Pick one to pair.
        </p>
      </Show>

      {/* What is wrong with the one that is chosen, said where it is chosen. */}
      <Show when={props.chosen?.profile.broken}>
        {(broken) => <p class="error broken">{BROKEN[broken()]}</p>}
      </Show>
      <Show when={refused()}>
        {(outcome) => <p class="error">{CHOICE_REFUSAL[outcome()]}</p>}
      </Show>
      <Show when={choose.isError}>
        <p class="error">
          The profile could not be chosen: {choose.error?.message}
        </p>
      </Show>
    </div>
  );
}

/// The branch the work will be done on: prefilled with a random name when the
/// Conversation was started, and the human's to change until grilling begins.
///
/// Nothing is created by naming it. The branch itself arrives with the stage
/// that starts grilling; this is the name it will be given.
///
/// It keeps itself the way the Brief above it does — on a pause in the typing
/// and on the way out of the field — because it is the same card and a field
/// with a button beside it would be the one thing on that card asking to be
/// pressed. There is no word about saving either: what a save cannot do is
/// said, and what it did is the name in the field and in the sidebar.
function BranchName(props: { conversation: ConversationView }): JSX.Element {
  const queries = useQueryClient();

  // What has been typed, or nothing if nothing has been: the field follows the
  // Conversation until the first keystroke and follows itself after it, so a
  // read landing mid-name cannot take the name with it.
  const [named, setNamed] = createSignal<string | null>(null);
  const [refused, setRefused] = createSignal<BranchRenamed | null>(null);

  /// What is in the field: what has been typed, or the name as it stands.
  const branch = () => named() ?? props.conversation.branch;

  // The last name a save asked for, whatever became of it: where the save
  // landed it is what the record has, and where it was refused it is the name
  // the refusal was about. Either way, asking again for that same string would
  // only get the same answer back.
  const [asked, setAsked] = createSignal<string | null>(null);
  const recorded = () => asked() ?? props.conversation.branch;

  /// Whether the field has moved on since the last save, which is the whole of
  /// what there is to save.
  const unsaved = () => branch() !== recorded();

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
    // Whatever became of it, the field may have been typed into while it was
    // in flight — so the moment one save is done the next is considered.
    onSettled: () => {
      saving = false;
      keep();
    },
  }));

  /// Whether a save is in the air.
  ///
  /// Tracked here rather than read off the mutation, because the callbacks that
  /// decide what to do next run before it has been told the save is over — and
  /// what they are for is deciding whether to start another.
  let saving = false;

  // The pause: one timer, restarted by every keystroke and cancelled by
  // whatever saves before it comes round.
  let pause: ReturnType<typeof setTimeout> | undefined;

  const settle = () => {
    clearTimeout(pause);
    pause = setTimeout(keep, SETTLE);
  };

  /// Keep what is in the field, if the record does not have it already.
  ///
  /// One save at a time, and what was typed meanwhile saved when the one in
  /// flight comes back — two in the air could land in either order, and the
  /// loser would be the record.
  ///
  /// A refusal only stops it where the refusal is permanent. `NotABranchName`
  /// is not: the name is validated server-side alone, so a pause in the middle
  /// of typing one is refused for what is not there yet, and the keystroke
  /// after it may well be what fixes it. What stops that becoming a request a
  /// second is that the same string is never asked about twice.
  const keep = () => {
    clearTimeout(pause);
    if (settled() || !unsaved() || saving) return;

    const name = branch();
    saving = true;
    setAsked(name);
    rename.mutate(name);
  };

  /// Whether what came back is worth not asking about again.
  const settled = () => {
    const outcome = refused();
    return outcome === "NoSuchConversation" || outcome === "NotDrafting";
  };

  onCleanup(() => clearTimeout(pause));

  return (
    <form
      class="branch-name"
      onSubmit={(ev) => {
        // Nothing to press, so this is Enter in the field: the same save the
        // pause was about to make, made now.
        ev.preventDefault();
        keep();
      }}
    >
      <label for="branch">Branch</label>
      <div class="field-line">
        <input
          id="branch"
          type="text"
          autocapitalize="off"
          autocorrect="off"
          spellcheck={false}
          value={branch()}
          onInput={(ev) => {
            setNamed(ev.currentTarget.value);
            settle();
          }}
          onBlur={() => keep()}
        />
      </div>
      {/* A refusal stands until the next save answers, rather than clearing on
          the next keystroke: it is the only thing that says why the sidebar is
          not following the field, and one that vanished as it was read would
          leave the human nothing. */}
      <Show when={refused()}>
        {(outcome) => <p class="error">{BRANCH_REFUSAL[outcome()]}</p>}
      </Show>
      <Show when={rename.isError}>
        <p class="error">
          The branch could not be named: {rename.error?.message}
        </p>
      </Show>
    </form>
  );
}

/// What the first entry of the base dropdown sends, which is nothing: the rule
/// is the override taken away, and the record holds no branch for it.
const RULE = "";

/// The branch the work comes off.
///
/// A dropdown of the repository's own branches, with the rule — the default
/// branch as it stands when grilling starts — as its first entry, because that
/// is what choosing nothing means and it is worth reading rather than guessing
/// at. There is nothing to type: a sha or a tag would pin the work to a moment,
/// and what this asks is which line of work to come off.
///
/// A picked branch is stored by name and resolved when grilling starts, so the
/// work branches from wherever that branch stands then — which is the point of
/// picking one over its tip.
function BaseBranch(props: { conversation: ConversationView }): JSX.Element {
  const queries = useQueryClient();

  const [refused, setRefused] = createSignal<BaseRecorded | null>(null);

  // Under the Repo's key, because that is what they belong to: two conversations
  // against one repository are reading the same list, and a repo that moved is
  // the one thing the page hears about that could have moved these.
  const branches = useReading(() => ({
    queryKey: ["repos", props.conversation.repo.id, "branches"],
    queryFn: () => listBranches(props.conversation.repo.id),

    // Merged by position, there being no key on a string: a branch that is
    // still there is the same string, so the option drawn for it survives a
    // re-read whether or not the ones around it did.
    freshness: { reconcile: "id" },
  }));

  /// What is chosen, as the dropdown writes it: the empty string is the rule.
  const chosen = (): string => props.conversation.base_commit ?? "";

  /// What is offered: the rule, then the repository's branches — and what is
  /// chosen wherever the list does not hold it, which is the list still on its
  /// way and a branch taken away since it was picked. Drawn either way, because
  /// falling quietly to the rule would show one base while the record held
  /// another.
  const options = (): string[] => {
    const listed = branches.data ?? [];
    const pinned = chosen();

    return pinned === "" || listed.includes(pinned)
      ? [RULE, ...listed]
      : [RULE, pinned, ...listed];
  };

  const record = useMutation(() => ({
    mutationFn: (branch: string) =>
      // The rule is the override taken away rather than a branch called
      // nothing.
      setBaseBranch(props.conversation.id, branch === RULE ? null : branch),
    onSuccess: (outcome: BaseRecorded) => {
      if (outcome !== "Recorded") {
        setRefused(outcome);
        // Picked out of a list this card read a moment ago: reading it again is
        // both the correction and the explanation.
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
    <div class="base-branch">
      <label for="base-branch">Base branch</label>
      {/* A [`Picker`] rather than a `<select>`, so this cannot come to show one
          branch while the mutation below would record another — see
          `src/picking.tsx`. The rule is an option of its own rather than the
          picker's placeholder: it is a choice to go back to, and the placeholder
          is a state there is no way out of. */}
      <Picker
        id="base-branch"
        options={options()}
        value={(branch) => branch}
        label={(branch) =>
          branch === RULE
            ? `Default branch (${props.conversation.repo.default_branch})`
            : branch
        }
        chosen={chosen()}
        pick={(picked) => record.mutate(picked)}
        disabled={record.isPending}
      />
      <p class="note">
        <Show
          when={props.conversation.base_commit}
          fallback={
            <>
              The work branches from{" "}
              <span class="default-branch">
                {props.conversation.repo.default_branch}
              </span>{" "}
              as it stands when grilling starts.
            </>
          }
        >
          {(branch) => (
            <>
              Pinned to{" "}
              <span class="default-branch">{branch()}</span> — the work branches
              from wherever it stands when grilling starts.
            </>
          )}
        </Show>
      </p>
      {/* The list is what there is to pick out of, so a read that failed is
          said rather than drawn as a repository with one branch. */}
      <Show when={branches.isError}>
        <p class="error">
          Could not read the repo's branches: {branches.error?.message}
        </p>
      </Show>
      <Show when={refused()}>
        {(outcome) => <p class="error">{BASE_REFUSAL[outcome()]}</p>}
      </Show>
      <Show when={record.isError}>
        <p class="error">
          The base branch could not be recorded: {record.error?.message}
        </p>
      </Show>
    </div>
  );
}
