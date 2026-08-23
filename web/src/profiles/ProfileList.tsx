//! The Agent Profiles Verkstead can run a session under, and the form that adds
//! or rewrites one.
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
//! The agent type is not offered. There is one, and a select with a single
//! option is theatre; the discriminator is real because it is on the record, and
//! the picker arrives when there is something to pick between.
//!
//! A section of the settings page rather than a page of its own: which accounts
//! a session may be run under is settled once and then left alone, which is the
//! same kind of thing as everything else on it.

import { useMutation, useQuery, useQueryClient } from "@tanstack/solid-query";
import { For, Match, Show, Switch, createSignal, type JSX } from "solid-js";

import { createProfile, deleteProfile, editProfile, listProfiles } from "../api/client";
import type {
  Broken,
  ProfileDeleted,
  ProfileEdit,
  ProfileEntry,
  ProfileSaved,
} from "../api/types";

/// What each way of being refused a save says, once, wherever it is met.
///
/// `Saved` is here for completeness of the mapping and never drawn: nothing is
/// said about a save that worked, because the profile appearing on the list
/// underneath is what says it.
export const PROFILE_REFUSAL: Record<ProfileSaved, string> = {
  Saved: "",
  NoSuchProfile: "That profile is gone.",
  Nameless: "Give the profile a name — it is what you pick it by.",
  Modelless: "Give the profile a model — a session has to know what it runs on.",
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
  model: "",
};

export function ProfileList(): JSX.Element {
  const queries = useQueryClient();

  // Read once when the page opens, like the Repos under it: nothing here
  // changes on its own, and what does change is this section's own doing.
  const profiles = useQuery(() => ({
    queryKey: ["profiles"],
    queryFn: listProfiles,
  }));

  // Which profile the form is about: `null` for a new one, an id for the one
  // being rewritten. The form is the same either way.
  const [rewriting, setRewriting] = createSignal<number | null>(null);
  const [form, setForm] = createSignal<ProfileEdit>(BLANK);
  const [refused, setRefused] = createSignal<ProfileSaved | null>(null);
  const [refusedRemoval, setRefusedRemoval] =
    createSignal<ProfileDeleted | null>(null);

  const done = () => {
    setRefused(null);
    setRewriting(null);
    setForm(BLANK);
    void queries.invalidateQueries({ queryKey: ["profiles"] });
    // A rewritten profile is shown on whichever conversation chose it, and
    // whether one is ready to grill turns on what its profiles are.
    void queries.invalidateQueries({ queryKey: ["conversation"] });
  };

  const save = useMutation(() => ({
    mutationFn: (profile: ProfileEdit) => {
      const id = rewriting();
      return id === null ? createProfile(profile) : editProfile(id, profile);
    },
    onSuccess: (outcome: ProfileSaved) => {
      if (outcome !== "Saved") {
        setRefused(outcome);
        return;
      }
      done();
    },
  }));

  const remove = useMutation(() => ({
    mutationFn: (id: number) => deleteProfile(id),
    onSuccess: (outcome: ProfileDeleted, id: number) => {
      if (outcome !== "Removed") {
        setRefusedRemoval(outcome);
        // A profile that was already gone is one this page's list is wrong
        // about, and reading it again is both the correction and the
        // explanation.
        void queries.invalidateQueries({ queryKey: ["profiles"] });
        return;
      }

      setRefusedRemoval(null);
      // The form was about the profile that is now gone.
      if (rewriting() === id) {
        setRewriting(null);
        setForm(BLANK);
      }
      void queries.invalidateQueries({ queryKey: ["profiles"] });
    },
  }));

  /// Fill the form in with what a profile already says, which is what editing
  /// one is.
  const rewrite = (profile: ProfileEntry) => {
    setRewriting(profile.id);
    setRefused(null);
    setRefusedRemoval(null);
    setForm({
      name: profile.name,
      claude_dir: profile.claude_dir,
      config_file: profile.config_file,
      model: profile.model,
    });
  };

  /// One field of the form, typed into.
  const typed = (field: keyof ProfileEdit) => (value: string) => {
    setForm({ ...form(), [field]: value });
    setRefused(null);
  };

  const submit = (ev: SubmitEvent) => {
    ev.preventDefault();
    save.mutate(form());
  };

  return (
    <section class="profiles">
      <h2>Agent profiles</h2>

      <form class="edit-profile" onSubmit={submit}>
        <h3>{rewriting() === null ? "Add a profile" : "Edit profile"}</h3>

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

        <label for="profile-model">Default model</label>
        <input
          id="profile-model"
          type="text"
          autocapitalize="off"
          autocorrect="off"
          spellcheck={false}
          placeholder="claude-opus-5"
          value={form().model}
          onInput={(ev) => typed("model")(ev.currentTarget.value)}
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

        <div class="edit-profile-buttons">
          <button type="submit" disabled={save.isPending}>
            {rewriting() === null ? "Save" : "Save changes"}
          </button>
          <Show when={rewriting() !== null}>
            <button
              type="button"
              class="cancel"
              onClick={() => {
                setRewriting(null);
                setForm(BLANK);
                setRefused(null);
              }}
            >
              Cancel
            </button>
          </Show>
        </div>

        <Show when={refused()}>
          {(outcome) => <p class="error">{PROFILE_REFUSAL[outcome()]}</p>}
        </Show>
        {/* A server that could not answer at all, which is the one thing here
            that is an error rather than an outcome. */}
        <Show when={save.isError}>
          <p class="error">
            The profile could not be saved: {save.error?.message}
          </p>
        </Show>
      </form>

      <Show when={refusedRemoval()}>
        {(outcome) => <p class="error">{PROFILE_REMOVAL_REFUSAL[outcome()]}</p>}
      </Show>
      <Show when={remove.isError}>
        <p class="error">
          The profile could not be removed: {remove.error?.message}
        </p>
      </Show>

      <Switch>
        <Match when={profiles.isPending}>
          <p class="empty">Loading…</p>
        </Match>
        <Match when={profiles.isError}>
          <p class="error">
            Could not read the agent profiles: {profiles.error?.message}
          </p>
        </Match>
        <Match when={profiles.data?.length === 0}>
          <p class="empty">No agent profiles are saved yet.</p>
        </Match>
        <Match when={profiles.data}>
          {(saved) => (
            <ul class="set-list">
              <For each={saved()}>
                {(profile) => (
                  <ProfileRow
                    profile={profile}
                    editing={rewriting() === profile.id}
                    edit={() => rewrite(profile)}
                    remove={() => remove.mutate(profile.id)}
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

/// One saved profile: what it is called, what it runs, and the pair that is
/// mounted for it.
///
/// The paths shown are the resolved ones the server recorded rather than
/// whatever was typed to save them — those are what will be bind-mounted, and
/// the point of showing them is that they can be checked.
function ProfileRow(props: {
  profile: ProfileEntry;
  editing: boolean;
  edit: () => void;
  remove: () => void;
}): JSX.Element {
  return (
    <li
      class="set-row profile-row"
      classList={{ editing: props.editing, broken: props.profile.broken !== null }}
    >
      <div>
        <span class="title">{props.profile.name}</span>
        <span class="meta">
          <span class="model">{props.profile.model}</span>
          <span class="agent-type">{props.profile.agent_type}</span>
        </span>
        <span class="meta">
          <span class="path">{props.profile.claude_dir}</span>
        </span>
        <span class="meta">
          <span class="path">{props.profile.config_file}</span>
        </span>
        {/* Said here rather than left to be found out when a session will not
            start: the profile was checked when it was saved, and what has become
            of its pair since is the server's to report on every read. */}
        <Show when={props.profile.broken}>
          {(broken) => <p class="error broken">{BROKEN[broken()]}</p>}
        </Show>
      </div>
      <div class="profile-actions">
        <button type="button" onClick={props.edit}>
          Edit
        </button>
        <button type="button" class="remove" onClick={props.remove}>
          Remove
        </button>
      </div>
    </li>
  );
}
