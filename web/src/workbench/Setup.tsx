//! A Conversation's setup: what has to be settled before anything will run it,
//! drawn under the Brief it belongs to.
//!
//! The branch the work will be done on, the branch it will come off, the other
//! repos it works alongside, and the two pairings its sessions run under. Every
//! one of them is a fact about the Conversation rather than about any one Event,
//! and every one of them is the human's to change for as long as it is still
//! drafting.
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
  For,
  Match,
  Show,
  Switch,
  createSignal,
  type JSX,
} from "solid-js";

import { Menu, Nested } from "../Menu";
import {
  addCompanion,
  chooseGrillingPairing,
  chooseImplementationPairing,
  listBranches,
  listProfiles,
  listRepos,
  removeCompanion,
  renameBranch,
  setBaseBranch,
} from "../api/client";
import type {
  BaseRecorded,
  BranchRenamed,
  CompanionAdded,
  CompanionRemoved,
  CompanionView,
  ConversationView,
  PairingView,
  ProfileChoice,
  ProfileChosen,
  ProfileEntry,
} from "../api/types";
import { useReading } from "../freshness";
import { Empty, ErrorLine, Note } from "../notices";
import * as pairing from "../pairing";
import { Picker } from "../picking";
import { BROKEN } from "../profiles/ProfileList";
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
  /// Whether the branch this work is done on has been made already, which is
  /// what a worktree says: one is made with the branch and forgotten only by
  /// closing.
  ///
  /// A drafting conversation with one has had its branch cut already. The branch
  /// and the base commit are settled for good by then and the server refuses
  /// both, so the fields go: a field whose save comes back refused is worse than
  /// no field. The pairings stay, because they are re-settled every time work
  /// starts under them.
  const branched = () => props.conversation.worktree !== null;

  return (
    <section class={styles.conversationSetup} aria-label="Setup">
      {/* The branch and the branch it comes off, side by side wherever there
          is room for two and stacked where there is not — the same row the
          pairings below are laid out in, because they are the same kind of
          pair: two short choices about the one thing.

          No branch field where the conversation is adopting a roadmap: a stage
          is worked on its own slug, so the name invented when the row was made
          is discarded when the stage is adopted, and naming it here would be a
          field with nothing behind it. */}
      <Show when={!branched()}>
        <div class={styles.branches}>
          <Show when={!props.conversation.adopting}>
            <BranchName conversation={props.conversation} />
          </Show>
          <BaseBranch conversation={props.conversation} />
          {/* What is beyond the two fields: the other repositories this
              conversation may work alongside. Behind a ⋯ rather than in the
              row, because most work is one repository and a permanent control
              for the exception would be a control most conversations never
              press. */}
          <AddCompanion conversation={props.conversation} />
        </div>

        {/* And the ones already added, under the row they were added from —
            they belong to the branch and the base rather than to the pairings,
            and they go with them when the card freezes. */}
        <Companions conversation={props.conversation} />
      </Show>

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
    <section class={styles.conversationProfiles} aria-label="Agent profiles">
      <h3>Agent profiles</h3>

      <Switch>
        <Match when={profiles.isError}>
          <ErrorLine class={styles.failure}>
            Could not read the agent profiles: {profiles.error?.message}
          </ErrorLine>
        </Match>
        <Match when={profiles.data?.length === 0}>
          {/* Nothing to choose, so the only thing to offer is the page that
              fixes that. */}
          <Empty>
            No agent profiles are saved yet —{" "}
            <A href="/settings">add one</A> to run a session under.
          </Empty>
        </Match>
        <Match when={profiles.data}>
          {(saved) => (
            /* Side by side wherever there is room for two, stacked where there
               is not. The wrap is the pane's own width rather than the
               window's, because this card is drawn in a pane the human can
               narrow. */
            <div class={styles.pairings}>
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
            </div>
          )}
        </Match>
      </Switch>

      {/* Nothing is said here about being ready: readiness is the business of
          the button it gates, at the end of the record below, which is enabled
          or else explains what is missing. Said up here as well it would be the
          same verdict twice.

          An adopting conversation never grills at all, and why both pairings
          are fixed for it all the same is worth a line. */}
      <Show when={props.conversation.adopting}>
        <Note>
          Both pairings are fixed before adopting: the implementation one is
          what the work runs under, and the grilling one is carried, because
          the stages after this one inherit both from it.
        </Note>
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
    <div class={styles.profileChoice}>
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
        <Note class={styles.unpaired}>
          {props.chosen?.profile.name} was chosen before models were picked
          beside them. Pick one to pair.
        </Note>
      </Show>

      {/* What is wrong with the one that is chosen, said where it is chosen. */}
      <Show when={props.chosen?.profile.broken}>
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
    <form
      class={styles.branchName}
      onSubmit={(ev) => {
        // Nothing to press, so this is Enter in the field: the same save the
        // pause was about to make, made now.
        ev.preventDefault();
        keeper.keep();
      }}
    >
      <label for="branch">Branch</label>
      <div class={styles.fieldLine}>
        <input
          id="branch"
          type="text"
          autocapitalize="off"
          autocorrect="off"
          spellcheck={false}
          value={branch()}
          onInput={(ev) => {
            setNamed(ev.currentTarget.value);
            keeper.settle();
          }}
          onBlur={() => keeper.keep()}
        />
      </div>
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
    <div class={styles.baseBranch}>
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
      {/* The list is what there is to pick out of, so a read that failed is
          said rather than drawn as a repository with one branch. */}
      <Show when={branches.isError}>
        <ErrorLine class={styles.failure}>
          Could not read the repo's branches: {branches.error?.message}
        </ErrorLine>
      </Show>
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
    </div>
  );
}

/// The ⋯ at the end of the branch row: what there is to settle about this
/// conversation beyond the two fields beside it.
///
/// One row today — *Add companion repo* — and that row opens a level of the
/// same menu listing every registered Repo, because the list is as long as the
/// registry is and flattening it into the first level would put a dozen
/// repositories in front of somebody who came for something else.
///
/// The Repos are read here rather than passed down, so the menu is whole
/// wherever it is drawn — the sidebar's own repo menu does the same.
///
/// **Nothing is filtered out of the list.** This conversation's own Repo and
/// one already added are both in it, and both are refused by name when they are
/// pressed: the server is what decides either way, and a list that quietly left
/// a repository out would leave the human hunting for one that is registered.
function AddCompanion(props: { conversation: ConversationView }): JSX.Element {
  const queries = useQueryClient();

  const repos = useReading(() => ({
    queryKey: ["repos"],
    queryFn: listRepos,

    // Merged by the id each row carries flat, for this menu rather than for any
    // list: a rebuilt row is a new element, and a Nudge landing while the menu
    // is open would take the focus off the row the human had tabbed to.
    freshness: { reconcile: "id" },
  }));

  const [refused, setRefused] = createSignal<CompanionAdded | null>(null);

  // The menu's own way to shut, held out here because what closes this one is a
  // request coming back rather than the press that sent it.
  let shut = (): void => {};

  const add = useMutation(() => ({
    mutationFn: (repoId: number) => addCompanion(props.conversation.id, repoId),
    onSuccess: (outcome: CompanionAdded) => {
      if (outcome !== "Added") {
        setRefused(outcome);
        // Every refusal is about one of two lists this menu was drawn over: the
        // registered Repos, or the conversation the row would hang off.
        // Reading both again is the correction and the explanation together,
        // and the menu stays open to be read.
        void queries.invalidateQueries({ queryKey: ["repos"] });
        void queries.invalidateQueries({ queryKey: ["conversation"] });
        return;
      }

      // Straight back to the card, where the row it added is: the menu was a
      // way to say which repository, and the row appearing under the branch is
      // the confirmation.
      setRefused(null);
      shut();
      void queries.invalidateQueries({ queryKey: ["conversation"] });
    },
  }));

  return (
    <Menu
      class={styles.setupMenu!}
      label="More setup"
      name="More setup"
      trigger="⋯"
      closer={(close) => (shut = close)}
      opening={() => setRefused(null)}
    >
      {() => (
        <Nested label="Add companion repo">
          {() => (
            <>
              <Switch>
                <Match when={repos.isError}>
                  <ErrorLine class={styles.failure}>
                    Could not read the repos: {repos.error?.message}
                  </ErrorLine>
                </Match>
                <Match when={repos.data?.length === 0}>
                  {/* Nothing to work alongside, so the only thing to offer is
                      the page that fixes that. */}
                  <Empty class={styles.nothing}>
                    No repos are registered yet —{" "}
                    <A href="/settings">register one</A> to work alongside.
                  </Empty>
                </Match>
                <Match when={repos.data}>
                  {(registered) => (
                    <For each={registered()}>
                      {(repo) => (
                        <button
                          type="button"
                          role="menuitem"
                          disabled={add.isPending}
                          onClick={() => add.mutate(repo.id)}
                        >
                          {repo.name}
                        </button>
                      )}
                    </For>
                  )}
                </Match>
              </Switch>

              {/* Said in the level the press was made in, which is where the
                  human is still standing: this menu does not shut on a refusal,
                  and the level it does not shut is this one. */}
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
            </>
          )}
        </Nested>
      )}
    </Menu>
  );
}

/// The repos this conversation works alongside, one row each under the branch
/// row they were added from.
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

/// One of them: what it is called, and the × that takes it away.
///
/// The name and the × are the whole row for now. What the work will do with the
/// repository — read it or commit to it, off which branch, on a branch called
/// what — is the pickers this row gets next.
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

      // Either way: what came back is about a conversation this card read a
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
    </li>
  );
}
