//! The Repos Verkstead has been told about, as the cards that read them, and the
//! one way to add another: an absolute path, typed.
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
//! Two halves in two panes, which is what the settings page is now. Each
//! registered Repo is a [`CardButton`](../CardButton.tsx) in the middle pane and
//! the form is the details pane beside it, at `/settings/repos/new`. The modal
//! the form was drawn over the page in is gone, and so are the boxed rows it
//! stood over: a card is what everything else on this page is.
//!
//! The cards are not pressed. There is nowhere for one to go until task 08 gives
//! a Repo a pane of its own, and a card that answered a press by doing nothing
//! would be a promise this build cannot keep — so `CardButton` draws them
//! without a pointer, without a tab stop and without the role that would say
//! they can be opened. What is on them is what a list is scanned for, which for
//! a Repo is all three of the things it knows: the name it is picked by, the
//! directory Verkstead will work in, and what a Conversation will branch from.
//!
//! Both halves read the one query. They are two views of the same list, and a
//! read apiece would be two reads of it — the cache is what makes the second
//! caller free.
//!
//! A section of the settings page rather than a page of its own: which
//! repositories Verkstead may touch is settled once and then left alone, which
//! is the same kind of thing as everything else on it.

import { faPlus } from "@fortawesome/free-solid-svg-icons";
import { useMutation, useQueryClient } from "@tanstack/solid-query";
import { For, Match, Show, Switch, createSignal, type JSX } from "solid-js";

import { CardButton } from "../CardButton";
import { IconButton } from "../IconButton";
import { listRepos, registerRepo } from "../api/client";
import type { Registered, RepoEntry } from "../api/types";
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

/// The registered Repos, as the cards that read them.
export function RepoList(props: {
  /// Whether the pane that registers one is what is open, which is what the plus
  /// says about itself.
  adding: boolean;
  /// And opening it, which is what the plus does.
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
          open={props.adding}
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
              <For each={registered()}>{(repo) => <RepoCard repo={repo} />}</For>
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
/// Drawn as an `article`, the way every card holding more than a run of text is,
/// and pressed by nobody: there is nowhere to go until task 08 gives a Repo a
/// pane of its own.
function RepoCard(props: { repo: RepoEntry }): JSX.Element {
  return (
    <li>
      <CardButton as="article" class={styles.repo} open={false} press={null}>
        <span class={styles.title}>{props.repo.name}</span>
        <span class={styles.meta}>
          <span class={styles.path}>{props.repo.path}</span>
          <span class={styles.branch}>{props.repo.default_branch}</span>
        </span>
      </CardButton>
    </li>
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
