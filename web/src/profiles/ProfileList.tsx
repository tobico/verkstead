//! The Agent Profiles Verkstead can run a session under, as the cards that read
//! them and the pane that adds or rewrites one.
//!
//! Typed rather than picked out of a browser, like a repo's path and for the
//! same reason: the watched paths are a security boundary and nothing here scans
//! the filesystem to offer choices from it. What the form is for is naming a
//! pair, and what the server does about it is the only thing that decides
//! whether it is taken.
//!
//! One form does both saving and rewriting. A profile is a handful of fields
//! with nothing built from them yet, so editing one is filling the same form in
//! with what it already says — a second form for the same fields would be a
//! second opinion about what a profile is.
//!
//! Which fields those are comes off the profile's agent type rather than being
//! written into the form: an account is that type's own shape — Claude's pair of
//! paths here — so [`ACCOUNT_FIELDS`] holds a row per type, and the stage that
//! adds a backend adds a row rather than restructuring anything.
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
//! the warning where the account has gone. The mounted paths and the agent type
//! come off it and into the pane, which has room for them: the paths are the
//! account's own fields, and the agent type is said beside them.
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
//! launch is its own rather than a list every profile shares. They are picked
//! off the models this build knows, with a field beside the picks for an id it
//! does not — the list goes stale the week another model ships, which is why it
//! is the ordinary way in rather than the only one. The line-a-piece textarea
//! this replaces was the whole of it, and made every profile a spelling test.
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
  ProfileAccount,
  ProfileDeleted,
  ProfileEdit,
  ProfileEntry,
  ProfileSaved,
} from "../api/types";
import { useReading } from "../freshness";
import { KNOWN_MODELS, known, prettify } from "../models";
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
  HomeNotAbsolute: "Give the home's absolute path, starting with a slash.",
  HomeMissing: "There is nothing at the home's path.",
  HomeOutsideWatchedPaths:
    "That home is outside the watched paths, so Verkstead will not touch it.",
  HomeNotADirectory:
    "That is not a directory — an account's home is mounted from one.",
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
  HomeMissing: "The home it kept its account under is gone.",
  OutsideWatchedPaths: "Its account now points outside the watched paths.",
};

/// Which agent a profile runs, which is the discriminator its account is shaped
/// by.
type AgentType = ProfileAccount["agent_type"];

/// One path an account of some agent type is: the key it is held under, what the
/// label over it says, and an example to type into it.
type AccountField = {
  key: string;
  label: JSX.Element;
  placeholder: string;
};

/// What a profile of each agent type is asked for.
///
/// The whole of the per-type half of the form: a row apiece, and the stage that
/// lands a backend adds one, which is the point of the account being a shape
/// rather than a pair every profile is assumed to have.
///
/// A row here is what a saved profile of that type is *drawn* with, and it says
/// nothing about whether one can be written: the form offers no choice of type
/// — see [`BLANK`] — so a codex profile is one saved over the API, and this is
/// what it reads back as.
const ACCOUNT_FIELDS: Record<AgentType, AccountField[]> = {
  Claude: [
    {
      key: "claude_dir",
      label: (
        <>
          Claude directory, mounted at <code>~/.claude</code>
        </>
      ),
      placeholder: "/home/you/accounts/work/.claude",
    },
    {
      key: "config_file",
      label: (
        <>
          Config file, mounted at <code>~/.claude.json</code>
        </>
      ),
      placeholder: "/home/you/accounts/work/.claude.json",
    },
  ],
  Codex: [
    {
      key: "home",
      label: (
        <>
          Home directory, mounted at <code>~/.codex</code>
        </>
      ),
      placeholder: "/home/you/accounts/work/.codex",
    },
  ],
};

/// An empty form: what "add a profile" starts from.
///
/// A Claude account, because the form offers no choice of type — a type that
/// cannot launch the real binary yet would be a lie in a picker, and the stage
/// that makes one launch is the stage that offers it. What this is not is a
/// hard-coded pair: the fields drawn under it come off the type this names.
const BLANK: ProfileEdit = {
  name: "",
  account: { agent_type: "Claude", claude_dir: "", config_file: "" },
  models: [],
};

/// One of an account's paths, by the key the table above named it with.
///
/// Every account is an agent type and the paths that type keeps, so reading one
/// by key is reading a string. What the shapes really are is `ProfileAccount`,
/// and the server is what holds them to it — [`path`] and [`written`] are the
/// two places the form looks past the type, and both look past it only to reach
/// keys the table for that type gave them.
const path = (account: ProfileAccount, key: string): string =>
  (account as Record<string, string>)[key] ?? "";

/// And the same account with one of those paths changed.
const written = (
  account: ProfileAccount,
  key: string,
  value: string,
): ProfileAccount => ({ ...account, [key]: value }) as ProfileAccount;

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
  // What is in the field beside the picks: an id this build has not learned,
  // while it is being typed. Its own signal rather than one of the form's
  // fields, because it is not part of the profile until it is added.
  const [typing, setTyping] = createSignal("");
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
          account: { ...profile.account },
          models: [...profile.models],
        };
  };

  /// The fields this profile's type is asked for, which is what the form draws
  /// between the models and the buttons.
  const fields = (): AccountField[] => ACCOUNT_FIELDS[form().account.agent_type];

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

  /// The name, typed into.
  const typedName = (value: string) => {
    setEdited({ ...form(), name: value });
    setRefused(null);
  };

  /// And one of the account's paths, whichever of them its type has.
  const typedPath = (key: string) => (value: string) => {
    setEdited({ ...form(), account: written(form().account, key, value) });
    setRefused(null);
  };

  /// Every model the form draws a tick for: the ones this build knows, and any
  /// this profile carries that it does not — one saved while the field was free
  /// text, or one added by hand since.
  ///
  /// The unknown ones after the known ones rather than in the profile's own
  /// order, because the known list stands in the same order on every profile and
  /// one that shuffled itself per profile would be a list nobody could scan.
  const offered = (): string[] => [
    ...KNOWN_MODELS.map((model) => model.id),
    ...form().models.filter((model) => !known(model)),
  ];

  /// A model ticked or unticked.
  ///
  /// A tick appends and nothing else moves, so a profile opened and saved with
  /// nothing touched sends back exactly the list it arrived with — order and
  /// all, whatever was typed into the field this replaces.
  const pick = (model: string, on: boolean) => {
    const models = form().models;
    setEdited({
      ...form(),
      models: on
        ? models.includes(model)
          ? models
          : [...models, model]
        : models.filter((listed) => listed !== model),
    });
    setRefused(null);
  };

  /// And an id typed into the field beside them, added as a pick of its own.
  ///
  /// Trimmed, this one being typed: a stray space would make an id no agent can
  /// be launched with. An empty field adds nothing.
  const add = () => {
    const model = typing().trim();
    if (model !== "") {
      pick(model, true);
      setTyping("");
    }
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
              onInput={(ev) => typedName(ev.currentTarget.value)}
            />

            {/* Ticked rather than typed, and no default among them: the list
                says what this account can launch, and which of them a session
                runs is picked when the session is set up.

                A fieldset with a legend rather than a label, because this is one
                question with several answers and a `<label for>` names one
                control. */}
            <fieldset class={styles.models}>
              <legend>Models</legend>

              <ul class={styles.picks}>
                <For each={offered()}>
                  {(model) => (
                    <li>
                      <label>
                        <input
                          type="checkbox"
                          checked={form().models.includes(model)}
                          onChange={(ev) =>
                            pick(model, ev.currentTarget.checked)
                          }
                        />
                        {prettify(model)}
                      </label>
                      {/* The id beside the name, for whoever is checking: the
                          name is this viewer's word and the id is what the
                          session is launched with. Outside the label so it is
                          not read out as part of the tick's name, and not drawn
                          at all beside one the list does not know — there the id
                          is the name already. */}
                      <Show when={known(model)}>
                        <code>{model}</code>
                      </Show>
                    </li>
                  )}
                </For>
              </ul>

              {/* The way past the list. It goes stale the week another model
                  ships, and a profile that could not be given the new one until
                  Verkstead was rebuilt would be a form standing in front of the
                  work. What is added here is a pick like any other: it joins the
                  ticks above, ticked, and unticking it takes it away again. */}
              <label for="profile-model">Another model id</label>
              <div class={styles.byHand}>
                <input
                  id="profile-model"
                  type="text"
                  autocapitalize="off"
                  autocorrect="off"
                  spellcheck={false}
                  placeholder="claude-opus-6"
                  value={typing()}
                  onInput={(ev) => setTyping(ev.currentTarget.value)}
                  // Return adds the id rather than saving the profile, which is
                  // what a return in a text field does to a form left alone.
                  onKeyDown={(ev) => {
                    if (ev.key === "Enter") {
                      ev.preventDefault();
                      add();
                    }
                  }}
                />
                <button
                  type="button"
                  disabled={typing().trim() === ""}
                  onClick={add}
                >
                  Add
                </button>
              </div>
            </fieldset>

            {/* The account, in whatever shape its agent type keeps one: the
                fields come off the type rather than being written here, so a
                backend arriving is a row in `ACCOUNT_FIELDS` and nothing
                else. */}
            <For each={fields()}>
              {(field) => (
                <>
                  <label for={`profile-${field.key}`}>{field.label}</label>
                  <input
                    id={`profile-${field.key}`}
                    type="text"
                    inputmode="url"
                    autocapitalize="off"
                    autocorrect="off"
                    spellcheck={false}
                    placeholder={field.placeholder}
                    value={path(form().account, field.key)}
                    onInput={(ev) =>
                      typedPath(field.key)(ev.currentTarget.value)
                    }
                  />
                </>
              )}
            </For>

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
                  Runs a <code>{profile().account.agent_type}</code> agent.
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
