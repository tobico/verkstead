//! The Repos Verkstead has been told about, as the cards that read them, each
//! one opened, and the one way to add another: an absolute path, typed.
//!
//! Typed rather than picked out of a browser: the Watched Paths are a security
//! boundary and nothing here scans the filesystem to offer choices from it, so
//! there is nothing to pick from. What the form is for is naming a path, and
//! what the server does about it is the only thing that decides whether it is
//! taken.
//!
//! Every refusal is shown as a refusal, in words, beside the field the path was
//! typed into: the human has just done something and is owed a reason it did not
//! happen. Which reason it was comes from the server as a named outcome, and
//! this file is where each of them is said — which is why the pane is what a
//! refusal keeps up rather than something the answer replaces.
//!
//! The cards are in the middle pane and the two panes they lead to are in the
//! details beside them, which is what the settings page is now. Each registered
//! Repo is a [`CardButton`](../CardButton.tsx), pressed to open it at
//! `/settings/repos/:id`, and the plus over them opens the form at
//! `/settings/repos/new`. The modal the form was drawn over the page in is gone,
//! and so are the boxed rows the list stood over: a card is what everything else
//! on this page is.
//!
//! What is on a card is what a list is scanned for, which for a Repo is all
//! three of the things the list knows: the name it is picked by, the directory
//! Verkstead will work in, and what a Conversation will branch from. Everything
//! else about it is in the pane, and is asked for when somebody opens one — the
//! branches git has, how much work is on it, and the roadmaps in it nothing is
//! driving. None of that is on the list because none of it is stored: each is a
//! git read or a count, and a list that carried them would pay for all of them
//! on every visit to this page.
//!
//! Taking one away is in the pane that opened it, and it is an unregistering
//! rather than a delete: Verkstead stops offering the repository and leaves the
//! directory where it is, so every Conversation ever worked in it goes on saying
//! which repository that was. Refused while live work is on it, in words, beside
//! the press — the same way a Profile a Conversation is set to run under is.
//!
//! The list and the form read the one query. They are two views of the same
//! list, and a read apiece would be two reads of it — the cache is what makes
//! the second caller free. The opened Repo is a read of its own, keyed by the
//! Repo, because it is not on that list at all.
//!
//! A section of the settings page rather than a page of its own: which
//! repositories Verkstead may touch is settled once and then left alone, which
//! is the same kind of thing as everything else on it.

import { faPlus } from "@fortawesome/free-solid-svg-icons";
import { useMutation, useQueryClient } from "@tanstack/solid-query";
import { For, Match, Show, Switch, createSignal, type JSX } from "solid-js";

import { CardButton } from "../CardButton";
import { IconButton } from "../IconButton";
import {
  RefusedError,
  listRepos,
  loadRepo,
  registerRepo,
  removeRepo,
} from "../api/client";
import type { Registered, RepoEntry, RepoRemoved } from "../api/types";
import { useReading } from "../freshness";
import { Empty, ErrorLine } from "../notices";
import { PaneHead } from "../workbench/PaneHead";
import app from "../App.module.css";
import styles from "./RepoList.module.css";

/// What each way of being refused says, once, wherever it is met.
///
/// `Added` is here for completeness of the mapping and never drawn: nothing is
/// said about a registration that worked, because the repo appearing on the list
/// behind the pane is what says it.
export const REFUSAL: Record<Registered, string> = {
  Added: "",
  NotAbsolute: "Give the repo's absolute path, starting with a slash.",
  Missing: "There is nothing at that path.",
  OutsideWatchedPaths:
    "That is outside the watched paths, so Verkstead will not touch it.",
  NotARepository:
    "That is not a git repository — name the repository's own directory.",
  NoDefaultBranch:
    "That repository has no branch to call its default. Check one out first.",
  AlreadyRegistered: "That repo is registered already.",
};

/// And what each way of being refused a removal says.
///
/// `Removed` is here for completeness of the mapping and never drawn: the pane
/// is spent by then, and the repo leaving the list behind it is what says the
/// removal landed.
export const REPO_REMOVAL_REFUSAL: Record<RepoRemoved, string> = {
  Removed: "",
  NoSuchRepo: "That repo is off the registry already.",
  InUse:
    "A conversation that is still going is on it. Finish or close that conversation first.",
};

/// The Repos as they stand, read once for the two panes that draw them.
///
/// Read when the page opens, like the Profiles above them: nothing here changes
/// on its own, and what does change is this section's own doing.
///
/// Merged by the id each entry carries flat, and not frozen: registering one
/// reads the list again, and a frozen query is one invalidation cannot reach —
/// the new repo would never appear behind the pane that added it.
function useRepos() {
  return useReading(() => ({
    queryKey: ["repos"],
    queryFn: listRepos,
    freshness: { reconcile: "id" },
  }));
}

/// The registered Repos, as the cards that open them.
export function RepoList(props: {
  /// Which Repo's pane is open — its id, `"new"` while the form that registers
  /// one is, or `null` where the details pane is showing something else
  /// entirely.
  opening: number | "new" | null;
  /// Open one, which is what pressing a card does.
  open: (id: number) => void;
  /// And open the form, which is what the plus does.
  add: () => void;
}): JSX.Element {
  const repos = useRepos();

  return (
    <section class={styles.repos}>
      {/* The heading, with the one thing there is to do to the list under it on
          the other end of its line. An `IconButton` rather than the quiet text
          button it was, for the reason the gear at the head of the conversations
          is one: it is another thing standing in this pane that is selected and
          opened into the pane beside it, so it is drawn as open while the form
          is what is being read. */}
      <div class={app.sectionHead}>
        <h2>Repos</h2>
        <IconButton
          of={faPlus}
          label="Add a repo"
          class={styles.add}
          open={props.opening === "new"}
          press={props.add}
        />
      </div>

      <Switch>
        <Match when={repos.isPending}>
          <Empty>Loading…</Empty>
        </Match>
        <Match when={repos.isError}>
          <ErrorLine>
            Could not read the registered Repos: {repos.error?.message}
          </ErrorLine>
        </Match>
        <Match when={repos.data?.length === 0}>
          <Empty>No repos are registered yet.</Empty>
        </Match>
        <Match when={repos.data}>
          {(registered) => (
            <ul class={styles.list}>
              <For each={registered()}>
                {(repo) => (
                  <RepoCard
                    repo={repo}
                    open={props.opening === repo.id}
                    press={() => props.open(repo.id)}
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

/// One registered Repo: what it is called, where it is, and what a Conversation
/// will branch from.
///
/// The path shown is the resolved one the server recorded rather than whatever
/// was typed to register it — that is the directory Verkstead will actually work
/// in, and the point of showing it is that it can be checked.
///
/// Drawn as an `article`, the way every card holding more than a run of text is
/// — a button may not have paragraphs inside it, and `CardButton` puts the
/// press, the keyboard and the role that says what it is on the article instead.
function RepoCard(props: {
  repo: RepoEntry;
  /// Whether the pane beside this is the one that is open.
  open: boolean;
  /// What pressing it does, which is opening that pane.
  press: () => void;
}): JSX.Element {
  return (
    <li>
      <CardButton
        as="article"
        class={styles.repo}
        open={props.open}
        press={props.press}
      >
        <span class={styles.title}>{props.repo.name}</span>
        <span class={styles.meta}>
          <span class={styles.path}>{props.repo.path}</span>
          <span class={styles.branch}>{props.repo.default_branch}</span>
        </span>
      </CardButton>
    </li>
  );
}

/// One registered Repo opened, which is the details pane a card leads to.
///
/// Everything on it but one press is the repository's own answer or the store's
/// count of what has been done in it. The press is Remove, which is the one
/// thing there is to *do* to a Repo — and it is an unregistering rather than a
/// delete: Verkstead stops offering it, the directory is left where it is, and
/// every Conversation ever worked in it goes on saying so.
///
/// That press stands under the facts and behind a rule, the way the Profile
/// pane's does: a press that undoes something, set among the things it would
/// undo, is one waiting to be made by mistake. It is refused while live work is
/// on the Repo, and the refusal is said here, because here is where it was made.
///
/// Its own read rather than the list's row, because none of what it shows is on
/// that row: the branches are a git call, the counts are a query, and the
/// roadmaps are a walk of `docs/roadmaps/` at the default branch's tip. All of
/// them are asked afresh every time the pane is opened, which is why the pane
/// waits on a read of its own even though the card that opened it already knew
/// the name.
///
/// A 404 is the repo being gone rather than a failure: somebody followed a link
/// after it was taken away, or reloaded a page they had left open. So it is said
/// in a line rather than shown as an error the human is meant to do something
/// about.
export function RepoDetails(props: {
  /// Which Repo, by the id its card carried.
  repo: number;
  /// The way back to the settings, which is a change of level rather than a
  /// navigation: what is open stays open, and the URL goes on saying so.
  back: () => void;
  /// And what a Repo taken off the registry does, which is a navigation: the
  /// pane is about something that is not registered any more, and the cards it
  /// goes back to are what say the removal landed.
  done: () => void;
}): JSX.Element {
  const queries = useQueryClient();

  // What the server said about the last removal asked for, or `null` while none
  // has been. Nothing clears it: there is one press on this pane, and what
  // answers a refusal is doing something about the conversation it named.
  const [refusedRemoval, setRefusedRemoval] = createSignal<RepoRemoved | null>(
    null,
  );

  const opened = useReading(() => ({
    // The Repo is in the key, so opening another is another query rather than
    // the same one showing the wrong repository for a moment.
    queryKey: ["repo", props.repo],
    queryFn: () => loadRepo(props.repo),
    // Merged rather than frozen: none of this is the store's alone, and a Nudge
    // is as good a moment as any to hear that a branch was pushed.
    freshness: { reconcile: "id" },
  }));

  const remove = useMutation(() => ({
    mutationFn: (id: number) => removeRepo(id),
    onSuccess: (outcome: RepoRemoved) => {
      // Whichever it was, this page's account of the Repo is older than the
      // server's now: a refusal is about work that has moved on, and a removal
      // is the list changing. The roadmaps waiting go with the Repos, because
      // they are read off whatever is registered.
      void queries.invalidateQueries({ queryKey: ["repos"] });
      void queries.invalidateQueries({ queryKey: ["abandoned-roadmaps"] });
      void queries.invalidateQueries({ queryKey: ["repo", props.repo] });

      if (outcome !== "Removed") {
        setRefusedRemoval(outcome);
        return;
      }

      props.done();
    },
  }));

  /// Whether there is simply no such Repo, which the server says with a 404 —
  /// the same shape a Set that is not there comes back in.
  const absent = (): boolean =>
    opened.error instanceof RefusedError && opened.error.status === 404;

  return (
    <>
      {/* Titled by the repository rather than by a word, because a pane about
          one thing is named by that thing. What it falls back to is a word: the
          head is drawn before the read lands, and there is nothing else to call
          it until it does. */}
      <PaneHead
        back={{ to: "Settings", go: props.back }}
        title={opened.data?.name ?? "Repo"}
      />

      <Switch>
        <Match when={opened.isPending}>
          <Empty>Loading…</Empty>
        </Match>
        <Match when={absent()}>
          <Empty>That repo is gone.</Empty>
        </Match>
        <Match when={opened.isError}>
          <ErrorLine>
            Could not read this repo: {opened.error?.message}
          </ErrorLine>
        </Match>
        <Match when={opened.data}>
          {(repo) => (
            <div class={styles.opened}>
              {/* The three facts that decide what Verkstead will do in it, and
                  what has been done in it so far. A description list, because
                  each of them is a short answer to a named question. */}
              <dl class={styles.facts}>
                <dt>Path</dt>
                <dd class={styles.path}>{repo().path}</dd>

                <dt>Default branch</dt>
                <dd class={styles.branch}>{repo().default_branch}</dd>

                {/* Counted apart because they are read for different reasons:
                    what is on this Repo now, and what has been. */}
                <dt>Conversations</dt>
                <dd class={styles.counted}>
                  <span class={styles.live}>{repo().live} live</span>
                  <span class={styles.finished}>
                    {repo().finished} finished
                  </span>
                </dd>
              </dl>

              {/* Every branch git has, local and remote-tracking both — the same
                  list a drafting Conversation picks what it comes off out of. */}
              <section class={styles.branches}>
                <h2>Branches</h2>
                <Show
                  when={repo().branches.length > 0}
                  fallback={<Empty>Git says nothing about its branches.</Empty>}
                >
                  <ul class={styles.branchList}>
                    <For each={repo().branches}>
                      {(branch) => <li class={styles.branch}>{branch}</li>}
                    </For>
                  </ul>
                </Show>
              </section>

              {/* And what it is holding that nothing is driving, which is the
                  same reading the notice under the new-conversation box makes.
                  Said here whether or not there is any: the notice is drawn only
                  where there is something to say, and this pane is an account of
                  the Repo. */}
              <section class={styles.roadmaps}>
                <h2>Roadmaps waiting</h2>
                <Show
                  when={repo().roadmaps.length > 0}
                  fallback={<Empty>Nothing is waiting to be adopted.</Empty>}
                >
                  <ul class={styles.roadmapList}>
                    <For each={repo().roadmaps}>
                      {(roadmap) => (
                        <li class={styles.roadmap}>
                          <span class={styles.title}>
                            {roadmap.title || roadmap.name}
                          </span>
                          {/* Which stage adopting it would start, which is the
                              roadmap's own order rather than anybody's choice. */}
                          <span class={styles.stage}>
                            {roadmap.stage}: {roadmap.stage_title}
                          </span>
                        </li>
                      )}
                    </For>
                  </ul>
                </Show>
              </section>

              {/* And the one press there is to make about a Repo, under
                  everything it is about. What the line says is what the press
                  does: Verkstead stops offering the repository, and nothing on
                  any Timeline moves. */}
              <section class={styles.standing}>
                <p class={styles.note}>
                  Removing it takes it off the registry. The directory is left
                  where it is, and every conversation worked in it goes on
                  saying so.
                </p>

                <div class={styles.actions}>
                  <button
                    type="button"
                    class={styles.remove}
                    disabled={remove.isPending}
                    onClick={() => remove.mutate(repo().id)}
                  >
                    Remove
                  </button>
                </div>

                {/* Refused rather than taken out from under the work going on
                    in it, and said here because here is where the press was
                    made. */}
                <Show when={refusedRemoval()}>
                  {(outcome) => (
                    <ErrorLine class={styles.failure}>
                      {REPO_REMOVAL_REFUSAL[outcome()]}
                    </ErrorLine>
                  )}
                </Show>
                <Show when={remove.isError}>
                  <ErrorLine class={styles.failure}>
                    The repo could not be removed: {remove.error?.message}
                  </ErrorLine>
                </Show>
              </section>
            </div>
          )}
        </Match>
      </Switch>
    </>
  );
}

/// And the form that registers one, which is the details pane the plus opens.
///
/// There is no Cancel: a details pane is left by opening something else or by
/// the way back a narrow window draws, and a button that said the same thing
/// again would be a second way out of a pane that has one.
export function RepoPane(props: {
  /// The way back to the settings, which is a change of level rather than a
  /// navigation: what is open stays open, and the URL goes on saying so.
  back: () => void;
  /// And what a registration that was taken does, which is a navigation: the
  /// pane is spent, and what says the work landed is the list of cards it goes
  /// back to.
  done: () => void;
}): JSX.Element {
  const queries = useQueryClient();

  // The path typed into the field.
  const [path, setPath] = createSignal("");

  // What the server said about the last path offered, or `null` while nothing
  // has been. Cleared as soon as the field is touched: a refusal is about the
  // path that was sent, and it stops being about the one being typed.
  const [refused, setRefused] = createSignal<Registered | null>(null);

  const register = useMutation(() => ({
    mutationFn: (asked: string) => registerRepo(asked),
    onSuccess: (outcome: Registered) => {
      if (outcome !== "Added") {
        // Said inside the pane, which stays up: the path that was refused is
        // the one about to be corrected.
        setRefused(outcome);
        return;
      }

      // The list behind the pane is now out of date — the repo appearing on it
      // is the whole of the confirmation.
      void queries.invalidateQueries({ queryKey: ["repos"] });
      props.done();
    },
  }));

  const add = (ev: SubmitEvent) => {
    ev.preventDefault();

    const asked = path().trim();
    if (asked === "") {
      return;
    }

    register.mutate(asked);
  };

  return (
    <>
      <PaneHead back={{ to: "Settings", go: props.back }} title="Add a repo" />

      {/* Every way it can be refused is said inside it, because a refusal is
          answered by correcting the path. */}
      <form class={styles.form} onSubmit={add}>
        <label for="repo-path">Absolute path of a git repository</label>
        <input
          id="repo-path"
          type="text"
          inputmode="url"
          autocapitalize="off"
          autocorrect="off"
          spellcheck={false}
          placeholder="/home/you/src/verkstead"
          value={path()}
          onInput={(ev) => {
            setPath(ev.currentTarget.value);
            setRefused(null);
          }}
        />

        <div class={styles.buttons}>
          <button
            type="submit"
            disabled={register.isPending || path().trim() === ""}
          >
            Register
          </button>
        </div>

        <Show when={refused()}>
          {(outcome) => (
            <ErrorLine class={styles.failure}>{REFUSAL[outcome()]}</ErrorLine>
          )}
        </Show>
        {/* A server that could not answer at all, which is the one thing here
            that is an error rather than an outcome. */}
        <Show when={register.isError}>
          <ErrorLine class={styles.failure}>
            The repo could not be registered: {register.error?.message}
          </ErrorLine>
        </Show>
      </form>
    </>
  );
}
