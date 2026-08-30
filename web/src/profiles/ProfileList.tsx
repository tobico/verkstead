//! The Agent Profiles Verkstead can run a session under, as the cards that read
//! them and the pane that adds or rewrites one.
//!
//! Typed rather than picked out of a browser, like a repo's path and for the
//! same reason: the watched paths are a security boundary and nothing here scans
//! the filesystem to offer choices from it. What the form is for is naming a
//! pair, and what the server does about it is the only thing that decides
//! whether it is taken.
//!
//! One form does both saving and rewriting. A profile is four fields with
//! nothing built from them yet, so editing one is filling the same form in with
//! what it already says — a second form for the same four fields would be a
//! second opinion about what a profile is.
//!
//! Two halves in two panes, which is what the settings page is now. Each saved
//! Profile is a [`CardButton`](../CardButton.tsx) in the middle pane and the
//! form is the details pane beside it, at `/settings/profiles/:id` — or at
//! `/settings/profiles/new`, which is the same pane about a Profile that does
//! not exist yet. The modal the form was drawn over the page in is gone, and so
//! are the rows it stood over: a card that is pressed to open the pane beside it
//! is what everything else on this page is.
//!
//! What is on the card is what a list is scanned for — the name, the models, and
//! the warning where the pair has gone. The mounted paths and the agent type
//! come off it and into the pane, which has room for them: the paths are the
//! form's own two fields, and the agent type is said beside them.
//!
//! Removing is in that pane too, under the form. It was a second control on
//! every row, which put a destructive press beside a list somebody was only
//! reading; under the form it is beside the Profile it is about, and its
//! refusals are said where the press was made.
//!
//! Both halves read the one query. They are two views of the same list, and a
//! read apiece would be two reads of it — the cache is what makes the second
//! caller free.
//!
//! The models are one of those four fields, and a profile carries the whole list
//! of them: a profile reaches one account with one configuration, so what it can
//! launch is its own rather than a list every profile shares. They are typed as
//! free text a line apiece, because a list of the models there are goes stale
//! the week another one ships.
//!
//! The agent type is not offered. There is one, and a select with a single
//! option is theatre; the discriminator is real because it is on the record, so
//! the pane says what a Profile's is rather than asking for it, and the picker
//! arrives when there is something to pick between.
//!
//! A section of the settings page rather than a page of its own: which accounts
//! a session may be run under is settled once and then left alone, which is the
//! same kind of thing as everything else on it.

import { faPlus } from "@fortawesome/free-solid-svg-icons";
import { useMutation, useQueryClient } from "@tanstack/solid-query";
import { For, Match, Show, Switch, createSignal, type JSX } from "solid-js";

import { CardButton } from "../CardButton";
import { IconButton } from "../IconButton";
import { PaneSticky } from "../Panes";
import {
  createProfile,
  deleteProfile,
  editProfile,
  listProfiles,
} from "../api/client";
import type {
  Broken,
  ProfileDeleted,
  ProfileEdit,
  ProfileEntry,
  ProfileSaved,
} from "../api/types";
import { useReading } from "../freshness";
import { Empty, ErrorLine } from "../notices";
import { PaneHead } from "../workbench/PaneHead";
import app from "../App.module.css";
import styles from "./ProfileList.module.css";

/// What each way of being refused a save says, once, wherever it is met.
///
/// `Saved` is here for completeness of the mapping and never drawn: nothing is
/// said about a save that worked, because the profile appearing on the list
/// behind the pane is what says it.
export const PROFILE_REFUSAL: Record<ProfileSaved, string> = {
  Saved: "",
  NoSuchProfile: "That profile is gone.",
  Nameless: "Give the profile a name — it is what you pick it by.",
  Modelless:
    "Give the profile at least one model — a session has to know what it runs on.",
  NameTaken: "Another profile is called that already.",
  DirNotAbsolute:
    "Give the claude directory's absolute path, starting with a slash.",
  DirMissing: "There is nothing at the claude directory's path.",
  DirOutsideWatchedPaths:
    "That claude directory is outside the watched paths, so Verkstead will not touch it.",
  NotADirectory: "That is not a directory — `~/.claude` is mounted from one.",
  ConfigNotAbsolute:
    "Give the config file's absolute path, starting with a slash.",
  ConfigMissing: "There is nothing at the config file's path.",
  ConfigOutsideWatchedPaths:
    "That config file is outside the watched paths, so Verkstead will not touch it.",
  NotAFile: "That is not a file — `~/.claude.json` is mounted from one.",
};

/// And what each way of being refused a removal says.
export const PROFILE_REMOVAL_REFUSAL: Record<ProfileDeleted, string> = {
  Removed: "",
  NoSuchProfile: "That profile is gone already.",
  InUse:
    "A conversation is set to run under it. Change that conversation's profiles first.",
};

/// What is wrong with a profile whose pair is no longer where it was left.
export const BROKEN: Record<Broken, string> = {
  DirMissing: "Its claude directory is gone.",
  ConfigMissing: "Its config file is gone.",
  OutsideWatchedPaths: "Its pair now points outside the watched paths.",
};

/// An empty form: what "add a profile" starts from.
const BLANK: ProfileEdit = {
  name: "",
  claude_dir: "",
  config_file: "",
  models: [],
};

/// The fields of the form that are one line of text. The models are the one
/// that is not, and they are typed into through [`typedModels`] below.
type TextField = Exclude<keyof ProfileEdit, "models">;

/// The Profiles as they stand, read once for the two panes that draw them.
///
/// Read when the page opens, like the Repos beside them: nothing here changes on
/// its own, and what does change is this section's own doing.
///
/// Merged by the id each entry carries flat. The server asks the filesystem
/// about every pair on every read, so a profile whose directory has been moved
/// changes underneath a page nobody has touched — and the pane is open over the
/// Profile it is rewriting while that happens.
function useProfiles() {
  return useReading(() => ({
    queryKey: ["profiles"],
    queryFn: listProfiles,
    freshness: { reconcile: "id" },
  }));
}

/// The saved Profiles, as the cards that open them.
export function ProfileList(props: {
  /// Which Profile's pane is open — its id, `"new"` while the blank form is, or
  /// `null` where the details pane is showing something else entirely.
  opening: number | "new" | null;
  /// Open one, which is what pressing a card does.
  open: (id: number) => void;
  /// And open the blank form, which is what the plus does.
  add: () => void;
}): JSX.Element {
  const profiles = useProfiles();

  return (
    <section class={styles.profiles}>
      {/* The heading, with the one thing there is to do to the list under it on
          the other end of its line. An `IconButton` rather than the quiet text
          button it was, for the reason the gear at the head of the conversations
          is one: it is another thing standing in this pane that is selected and
          opened into the pane beside it, so it is drawn as open while the blank
          form is what is being read. */}
      <div class={app.sectionHead}>
        <h2>Agent profiles</h2>
        <IconButton
          of={faPlus}
          label="Add a profile"
          class={styles.add}
          open={props.opening === "new"}
          press={props.add}
        />
      </div>

      <Switch>
        <Match when={profiles.isPending}>
          <Empty>Loading…</Empty>
        </Match>
        <Match when={profiles.isError}>
          <ErrorLine>
            Could not read the agent profiles: {profiles.error?.message}
          </ErrorLine>
        </Match>
        <Match when={profiles.data?.length === 0}>
          <Empty>No agent profiles are saved yet.</Empty>
        </Match>
        <Match when={profiles.data}>
          {(saved) => (
            <ul class={styles.list}>
              <For each={saved()}>
                {(profile) => (
                  <ProfileCard
                    profile={profile}
                    open={props.opening === profile.id}
                    press={() => props.open(profile.id)}
                  />
                )}
              </For>
            </ul>
          )}
        </Match>
      </Switch>
    </section>
  );
}

/// One saved profile: what it is called, and what it runs.
///
/// Drawn as an `article`, the way every card holding more than a run of text is
/// — a button may not have paragraphs inside it, and `CardButton` puts the
/// press, the keyboard and the role that says what it is on the article instead.
function ProfileCard(props: {
  profile: ProfileEntry;
  /// Whether the pane beside this is the one that is open.
  open: boolean;
  /// What pressing it does, which is opening that pane.
  press: () => void;
}): JSX.Element {
  return (
    <li>
      <CardButton
        as="article"
        class={[
          styles.profile,
          props.profile.broken !== null ? styles.broken : undefined,
        ]
          .filter(Boolean)
          .join(" ")}
        open={props.open}
        press={props.press}
      >
        <span class={styles.title}>{props.profile.name}</span>
        <span class={styles.meta}>
          {/* Every model, because the list is the whole of what a profile says
              it can run and a card showing one of them would be picking. */}
          <For each={props.profile.models}>
            {(model) => <span class={styles.model}>{model}</span>}
          </For>
        </span>
        {/* Said here rather than left to be found out when a session will not
            start: the profile was checked when it was saved, and what has become
            of its pair since is the server's to report on every read. */}
        <Show when={props.profile.broken}>
          {(broken) => (
            <ErrorLine class={styles.broken}>{BROKEN[broken()]}</ErrorLine>
          )}
        </Show>
      </CardButton>
    </li>
  );
}

/// And the form that adds or rewrites one, which is the details pane a card —
/// or the plus above them — opens.
///
/// There is no Cancel: a details pane is left by opening something else or by
/// the way back a narrow window draws, and a button that said the same thing
/// again would be a second way out of a pane that has one.
export function ProfilePane(props: {
  /// Which Profile the form is about: its id, or `"new"` for one that does not
  /// exist yet.
  profile: number | "new";
  /// The way back to the settings, which is a change of level rather than a
  /// navigation: what is open stays open, and the URL goes on saying so.
  back: () => void;
  /// And what a save or a removal that was taken does, which is a navigation:
  /// the pane is spent, and what says the work landed is the list of cards it
  /// goes back to.
  done: () => void;
}): JSX.Element {
  const queries = useQueryClient();
  const profiles = useProfiles();

  const adding = (): boolean => props.profile === "new";

  /// The Profile this pane is about, or `undefined` while the list is still
  /// being read — and for an id the list has not got, which is a path naming a
  /// Profile that is gone.
  const saved = (): ProfileEntry | undefined =>
    profiles.data?.find((profile) => profile.id === props.profile);

  // What has been typed, or `null` while nothing has — the fields follow the
  // saved Profile until somebody touches them, the way the credentials' do. It
  // matters here because the pane is built before the read that fills it lands.
  const [edited, setEdited] = createSignal<ProfileEdit | null>(null);
  const [refused, setRefused] = createSignal<ProfileSaved | null>(null);
  const [refusedRemoval, setRefusedRemoval] =
    createSignal<ProfileDeleted | null>(null);

  /// What is in the fields: what was typed, or what the Profile says, or
  /// nothing at all for one that does not exist yet.
  ///
  /// The paths are the resolved ones the server recorded rather than whatever
  /// was typed to save them — those are what will be bind-mounted, and the point
  /// of showing them is that they can be checked.
  const form = (): ProfileEdit => {
    const typed = edited();
    if (typed !== null) {
      return typed;
    }

    const profile = saved();
    return profile === undefined
      ? BLANK
      : {
          name: profile.name,
          claude_dir: profile.claude_dir,
          config_file: profile.config_file,
          models: [...profile.models],
        };
  };

  const save = useMutation(() => ({
    mutationFn: (profile: ProfileEdit) => {
      const which = props.profile;
      return which === "new" ? createProfile(profile) : editProfile(which, profile);
    },
    onSuccess: (outcome: ProfileSaved) => {
      if (outcome !== "Saved") {
        setRefused(outcome);
        return;
      }

      void queries.invalidateQueries({ queryKey: ["profiles"] });
      // A rewritten profile is shown on whichever conversation chose it, and
      // whether one is ready to grill turns on what its profiles are.
      void queries.invalidateQueries({ queryKey: ["conversation"] });
      props.done();
    },
  }));

  const remove = useMutation(() => ({
    mutationFn: (id: number) => deleteProfile(id),
    onSuccess: (outcome: ProfileDeleted) => {
      // A profile that was already gone is one this page's list is wrong about,
      // and reading it again is both the correction and the explanation.
      void queries.invalidateQueries({ queryKey: ["profiles"] });

      if (outcome !== "Removed") {
        setRefusedRemoval(outcome);
        return;
      }

      props.done();
    },
  }));

  /// One of the form's one-line fields, typed into.
  const typed = (field: TextField) => (value: string) => {
    setEdited({ ...form(), [field]: value });
    setRefused(null);
  };

  /// And the models, which are that same typing split at its newlines.
  ///
  /// Split and nothing else: the empty line under the one being written is part
  /// of writing the next, so trimming here would fight the human. What is blank
  /// is dropped by the server at the moment of saving.
  const typedModels = (value: string) => {
    setEdited({ ...form(), models: value.split("\n") });
    setRefused(null);
  };

  const submit = (ev: SubmitEvent) => {
    ev.preventDefault();
    save.mutate(form());
  };

  return (
    <>
      <PaneSticky>
        <PaneHead
          back={{ to: "Settings", go: props.back }}
          title={adding() ? "Add a profile" : "Edit profile"}
        />
      </PaneSticky>

      {/* The blank form first, so that adding one never waits on a read it has
          no use for. Everything below it is about a Profile that is saved, and
          the fallback is what is left once the list has been read and has no
          such Profile in it. */}
      <Switch fallback={<Empty>That profile is gone.</Empty>}>
        <Match when={adding() || saved() !== undefined}>
          <form class={styles.form} onSubmit={submit}>
            <label for="profile-name">Name</label>
            <input
              id="profile-name"
              type="text"
              autocapitalize="off"
              autocorrect="off"
              spellcheck={false}
              placeholder="work"
              value={form().name}
              onInput={(ev) => typed("name")(ev.currentTarget.value)}
            />

            {/* One per line, and no default among them: the list says what this
                account can launch, and which of them a session runs is picked
                when the session is set up. */}
            <label for="profile-models">Models, one per line</label>
            <textarea
              id="profile-models"
              rows={3}
              autocapitalize="off"
              autocorrect="off"
              spellcheck={false}
              placeholder={"claude-opus-5\nclaude-fable-5"}
              value={form().models.join("\n")}
              onInput={(ev) => typedModels(ev.currentTarget.value)}
            />

            <label for="profile-dir">
              Claude directory, mounted at <code>~/.claude</code>
            </label>
            <input
              id="profile-dir"
              type="text"
              inputmode="url"
              autocapitalize="off"
              autocorrect="off"
              spellcheck={false}
              placeholder="/home/you/accounts/work/.claude"
              value={form().claude_dir}
              onInput={(ev) => typed("claude_dir")(ev.currentTarget.value)}
            />

            <label for="profile-config">
              Config file, mounted at <code>~/.claude.json</code>
            </label>
            <input
              id="profile-config"
              type="text"
              inputmode="url"
              autocapitalize="off"
              autocorrect="off"
              spellcheck={false}
              placeholder="/home/you/accounts/work/.claude.json"
              value={form().config_file}
              onInput={(ev) => typed("config_file")(ev.currentTarget.value)}
            />

            <div class={styles.buttons}>
              <button type="submit" disabled={save.isPending}>
                {adding() ? "Save" : "Save changes"}
              </button>
            </div>

            <Show when={refused()}>
              {(outcome) => (
                <ErrorLine class={styles.failure}>
                  {PROFILE_REFUSAL[outcome()]}
                </ErrorLine>
              )}
            </Show>
            {/* A server that could not answer at all, which is the one thing
                here that is an error rather than an outcome. */}
            <Show when={save.isError}>
              <ErrorLine class={styles.failure}>
                The profile could not be saved: {save.error?.message}
              </ErrorLine>
            </Show>
          </form>

          {/* What is on the record about a saved Profile but not in the form:
              the agent type it runs, and the press that takes the whole thing
              away. Neither belongs to a Profile that does not exist yet. */}
          <Show when={saved()}>
            {(profile) => (
              <section class={styles.standing}>
                {/* Said rather than offered: there is one agent type, and a
                    select with a single option is theatre — but it is on the
                    record, and this pane has the room the card did not. */}
                <p class={styles.agentType}>
                  Runs a <code>{profile().agent_type}</code> agent.
                </p>

                <div class={styles.actions}>
                  <button
                    type="button"
                    class={styles.remove}
                    disabled={remove.isPending}
                    onClick={() => remove.mutate(profile().id)}
                  >
                    Remove
                  </button>
                </div>

                {/* Refused rather than taken away from the conversation that
                    chose it, and said here because here is where the press was
                    made. */}
                <Show when={refusedRemoval()}>
                  {(outcome) => (
                    <ErrorLine class={styles.failure}>
                      {PROFILE_REMOVAL_REFUSAL[outcome()]}
                    </ErrorLine>
                  )}
                </Show>
                <Show when={remove.isError}>
                  <ErrorLine class={styles.failure}>
                    The profile could not be removed: {remove.error?.message}
                  </ErrorLine>
                </Show>
              </section>
            )}
          </Show>
        </Match>

        <Match when={profiles.isPending}>
          <Empty>Loading…</Empty>
        </Match>
        <Match when={profiles.isError}>
          <ErrorLine>
            Could not read the agent profiles: {profiles.error?.message}
          </ErrorLine>
        </Match>
      </Switch>
    </>
  );
}
