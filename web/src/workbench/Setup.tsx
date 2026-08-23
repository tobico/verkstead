//! A Conversation's setup: what has to be settled before anything will run it,
//! drawn under the Brief it belongs to.
//!
//! The branch the work will be done on, the commit it will branch from, and the
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
import { Match, Show, Switch, createSignal, type JSX } from "solid-js";

import {
  chooseGrillingPairing,
  chooseImplementationPairing,
  listProfiles,
  renameBranch,
  setBaseCommit,
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

/// What each way of being refused a branch name says.
export const BRANCH_REFUSAL: Record<BranchRenamed, string> = {
  Renamed: "",
  NoSuchConversation: "This conversation is gone.",
  NotDrafting:
    "The branch exists by now, so its name is not a text field any more.",
  NotABranchName: "Git will not take that as a branch name.",
};

/// And a base commit.
export const BASE_REFUSAL: Record<BaseRecorded, string> = {
  Recorded: "",
  NoSuchConversation: "This conversation is gone.",
  NotDrafting: "The base commit was captured when grilling started.",
  NoSuchCommit: "That repo has nothing by that name.",
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
      <BaseCommit conversation={props.conversation} />

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
function BranchName(props: { conversation: ConversationView }): JSX.Element {
  const queries = useQueryClient();

  const [named, setNamed] = createSignal<string | null>(null);
  const [refused, setRefused] = createSignal<BranchRenamed | null>(null);

  /// What is in the field: what has been typed, or the name as it stands.
  const branch = () => named() ?? props.conversation.branch;

  const rename = useMutation(() => ({
    mutationFn: (branch: string) =>
      renameBranch(props.conversation.id, branch),
    onSuccess: (outcome: BranchRenamed) => {
      if (outcome !== "Renamed") {
        setRefused(outcome);
        return;
      }

      // The field goes back to following the Conversation, which is about to
      // come back saying what this asked for.
      setRefused(null);
      setNamed(null);
      void queries.invalidateQueries({ queryKey: ["conversation"] });
      void queries.invalidateQueries({ queryKey: ["conversations"] });
    },
  }));

  return (
    <form
      class="branch-name"
      onSubmit={(ev) => {
        ev.preventDefault();
        rename.mutate(branch());
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
            setRefused(null);
          }}
        />
        <button
          type="submit"
          disabled={rename.isPending || branch() === props.conversation.branch}
        >
          Rename
        </button>
      </div>
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

/// The commit the work branches from.
///
/// Empty is not a missing value: it is the rule — the default branch's tip as it
/// stands when grilling starts — which is why the field says which branch that
/// is rather than leaving the human to guess what nothing means.
function BaseCommit(props: { conversation: ConversationView }): JSX.Element {
  const queries = useQueryClient();

  const [typed, setTyped] = createSignal<string | null>(null);
  const [refused, setRefused] = createSignal<BaseRecorded | null>(null);

  const commit = () => typed() ?? props.conversation.base_commit ?? "";

  const record = useMutation(() => ({
    mutationFn: (commit: string) =>
      // Emptied is the override taken away rather than a commit called nothing.
      setBaseCommit(props.conversation.id, commit.trim() === "" ? null : commit),
    onSuccess: (outcome: BaseRecorded) => {
      if (outcome !== "Recorded") {
        setRefused(outcome);
        return;
      }

      setRefused(null);
      setTyped(null);
      void queries.invalidateQueries({ queryKey: ["conversation"] });
    },
  }));

  return (
    <form
      class="base-commit"
      onSubmit={(ev) => {
        ev.preventDefault();
        record.mutate(commit());
      }}
    >
      <label for="base-commit">Base commit</label>
      <div class="field-line">
        <input
          id="base-commit"
          type="text"
          autocapitalize="off"
          autocorrect="off"
          spellcheck={false}
          placeholder={`${props.conversation.repo.default_branch} at grill start`}
          value={commit()}
          onInput={(ev) => {
            setTyped(ev.currentTarget.value);
            setRefused(null);
          }}
        />
        <button
          type="submit"
          disabled={
            record.isPending ||
            commit() === (props.conversation.base_commit ?? "")
          }
        >
          Record
        </button>
      </div>
      <p class="note">
        <Show
          when={props.conversation.base_commit}
          fallback={
            <>
              Left empty, the work branches from{" "}
              <span class="default-branch">
                {props.conversation.repo.default_branch}
              </span>{" "}
              as it stands when grilling starts.
            </>
          }
        >
          Pinned. Empty the field to go back to branching from{" "}
          <span class="default-branch">
            {props.conversation.repo.default_branch}
          </span>
          .
        </Show>
      </p>
      <Show when={refused()}>
        {(outcome) => <p class="error">{BASE_REFUSAL[outcome()]}</p>}
      </Show>
      <Show when={record.isError}>
        <p class="error">
          The base commit could not be recorded: {record.error?.message}
        </p>
      </Show>
    </form>
  );
}
