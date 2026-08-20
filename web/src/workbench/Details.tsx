//! The details pane: what a Conversation is, beside the Timeline of what has
//! happened to it.
//!
//! The Repo it is attached to, the branch the work will be done on, and the
//! commit it will branch from. All three are facts about the Conversation rather
//! than about any one Event, and two of them are the human's to change for as
//! long as it is still drafting — which is why the pane that is neither the list
//! nor the record is where they are settled.
//!
//! The two agent profiles are settled here too, for the same reason: which
//! account and model the grilling runs under, and which the implementation runs
//! under, are facts about the conversation rather than about any one event. They
//! are separate choices because they are genuinely separate accounts — grill on
//! fable, implement on opus — and because the implementation session cannot
//! simply carry the grilling one on.
//!
//! The Brief has no place here on purpose: it is inline in the Timeline, because
//! there is nothing of it the Timeline does not already show. What will stand
//! here is the full self of the Events that do have one — a transcript, a
//! Question Set, a diff — as each of those stages lands.

import { A } from "@solidjs/router";
import { useMutation, useQuery, useQueryClient } from "@tanstack/solid-query";
import { For, Match, Show, Switch, createSignal, type JSX } from "solid-js";

import {
  chooseGrillingProfile,
  chooseImplementationProfile,
  listProfiles,
  renameBranch,
  setBaseCommit,
} from "../api/client";
import type {
  BaseRecorded,
  BranchRenamed,
  ConversationView,
  ProfileChosen,
  ProfileEntry,
} from "../api/types";
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

/// And a profile choice.
export const CHOICE_REFUSAL: Record<ProfileChosen, string> = {
  Chosen: "",
  NoSuchConversation: "This conversation is gone.",
  NoSuchProfile: "That profile has been removed.",
};

export function Details(props: {
  conversation: ConversationView;
  back: () => void;
}): JSX.Element {
  return (
    <>
      <div class="pane-head">
        <button type="button" class="pane-back" onClick={props.back}>
          ← Timeline
        </button>
        <h1>Details</h1>
      </div>

      {/* The repository, as the Repo list shows one: what it is called, where it
          is, and what a Conversation branches from unless it says otherwise.
          Nothing here is editable — a Conversation is attached to a Repo when it
          is started, and moving it to another would be starting a different
          piece of work. */}
      <dl class="conversation-facts">
        <dt>Repo</dt>
        <dd class="repo">{props.conversation.repo.name}</dd>
        <dt>Path</dt>
        <dd class="path">{props.conversation.repo.path}</dd>
        <dt>State</dt>
        <dd class="state">{props.conversation.state}</dd>
      </dl>

      <BranchName conversation={props.conversation} />
      <BaseCommit conversation={props.conversation} />

      <Profiles conversation={props.conversation} />
    </>
  );
}

/// The two accounts the work will run under, and whether everything grilling
/// needs is settled.
///
/// The profile list is read here rather than passed down, so the pickers are
/// whole wherever they are drawn — the sidebar does the same with the repos.
function Profiles(props: { conversation: ConversationView }): JSX.Element {
  const profiles = useQuery(() => ({
    queryKey: ["profiles"],
    queryFn: listProfiles,
  }));

  return (
    <section class="conversation-profiles" aria-label="Agent profiles">
      <h2>Agent profiles</h2>

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
            <A href="/profiles">add one</A> to run a session under.
          </p>
        </Match>
        <Match when={profiles.data}>
          {(saved) => (
            <>
              <ProfilePicker
                conversation={props.conversation}
                saved={saved()}
                role="grilling"
                label="Grilling"
                chosen={props.conversation.grilling_profile}
                choose={chooseGrillingProfile}
              />
              <ProfilePicker
                conversation={props.conversation}
                saved={saved()}
                role="implementation"
                label="Implementation"
                chosen={props.conversation.implementation_profile}
                choose={chooseImplementationProfile}
              />
            </>
          )}
        </Match>
      </Switch>

      {/* Whether the next stage will grill this conversation, which is the
          server's rule and not a count of the two fields above: a profile whose
          pair has gone is not one to launch a session under. */}
      <p class="note readiness" classList={{ ready: props.conversation.ready_to_grill }}>
        <Show
          when={props.conversation.ready_to_grill}
          fallback={<>Not ready to grill: both profiles have to be chosen and working.</>}
        >
          Ready to grill.
        </Show>
      </p>
    </section>
  );
}

/// One of the two choices: which saved profile fills this role.
///
/// A select rather than a list of buttons, because the profiles are a short list
/// that barely changes and the choice is one of them — the same control the
/// sidebar picks a repo with.
function ProfilePicker(props: {
  conversation: ConversationView;
  saved: ProfileEntry[];
  role: string;
  label: string;
  chosen: ProfileEntry | null;
  choose: (id: number, profileId: number) => Promise<ProfileChosen>;
}): JSX.Element {
  const queries = useQueryClient();

  const [refused, setRefused] = createSignal<ProfileChosen | null>(null);

  const choose = useMutation(() => ({
    mutationFn: (profileId: number) =>
      props.choose(props.conversation.id, profileId),
    onSuccess: (outcome: ProfileChosen) => {
      if (outcome !== "Chosen") {
        setRefused(outcome);
        // Chosen from a list this pane read a moment ago: reading it again is
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
      <label for={`${props.role}-profile`}>{props.label}</label>
      <select
        id={`${props.role}-profile`}
        // The empty value is the state of having chosen nothing, and it is not
        // an option to go back to: a conversation with no profile is one that
        // will not grill, so the placeholder disappears once one is picked.
        value={props.chosen ? String(props.chosen.id) : ""}
        disabled={choose.isPending}
        onChange={(ev) => {
          const picked = Number(ev.currentTarget.value);
          if (picked) {
            choose.mutate(picked);
          }
        }}
      >
        <Show when={!props.chosen}>
          <option value="">Not chosen</option>
        </Show>
        <For each={props.saved}>
          {(profile) => (
            <option value={profile.id}>
              {profile.name} — {profile.model}
            </option>
          )}
        </For>
      </select>

      {/* What is wrong with the one that is chosen, said where it is chosen. */}
      <Show when={props.chosen?.broken}>
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
