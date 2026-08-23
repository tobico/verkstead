//! The Repos Verkstead has been told about, and the one way to add another: an
//! absolute path, typed.
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
//! this file is where each of them is said.
//!
//! A section of the settings page rather than a page of its own: which
//! repositories Verkstead may touch is settled once and then left alone, which
//! is the same kind of thing as everything else on it.

import { useMutation, useQueryClient } from "@tanstack/solid-query";
import { For, Match, Show, Switch, createSignal } from "solid-js";

import { listRepos, registerRepo } from "../api/client";
import type { Registered, RepoEntry } from "../api/types";
import { useReading } from "../freshness";

/// What each way of being refused says, once, wherever it is met.
///
/// `Added` is here for completeness of the mapping and never drawn: nothing is
/// said about a registration that worked, because the repo appearing on the list
/// underneath is what says it.
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

/// The registered Repos, and the form that adds one.
export function RepoList() {
  const queries = useQueryClient();

  // Read once when the page opens, like the Profiles above it: nothing here
  // changes on its own, and what does change is this section's own doing.
  const repos = useReading(() => ({
    queryKey: ["repos"],
    queryFn: listRepos,

    // Merged by the id each row carries flat, and not frozen: registering one
    // reads the list again, and a frozen query is one invalidation cannot
    // reach — the new repo would never appear underneath the form that added
    // it.
    freshness: { reconcile: "id" },
  }));

  const [path, setPath] = createSignal("");

  // What the server said about the last path offered, or `null` while nothing
  // has been. Cleared as soon as the field is touched: a refusal is about the
  // path that was sent, and it stops being about the one being typed.
  const [refused, setRefused] = createSignal<Registered | null>(null);

  const register = useMutation(() => ({
    mutationFn: (asked: string) => registerRepo(asked),
    onSuccess: (outcome: Registered) => {
      if (outcome !== "Added") {
        setRefused(outcome);
        return;
      }

      // The field is spent, and the list underneath is now out of date — the
      // repo appearing on it is the whole of the confirmation.
      setRefused(null);
      setPath("");
      void queries.invalidateQueries({ queryKey: ["repos"] });
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
    <section class="repos">
      <h2>Repos</h2>

      <form class="add-repo" onSubmit={add}>
        <label for="repo-path">Absolute path of a git repository</label>
        <div class="add-repo-line">
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
          <button type="submit" disabled={register.isPending || path().trim() === ""}>
            Register
          </button>
        </div>
        <Show when={refused()}>
          {(outcome) => <p class="error">{REFUSAL[outcome()]}</p>}
        </Show>
        {/* A server that could not answer at all, which is the one thing here
            that is an error rather than an outcome. */}
        <Show when={register.isError}>
          <p class="error">
            The repo could not be registered: {register.error?.message}
          </p>
        </Show>
      </form>

      <Switch>
        <Match when={repos.isPending}>
          <p class="empty">Loading…</p>
        </Match>
        <Match when={repos.isError}>
          <p class="error">
            Could not read the registered Repos: {repos.error?.message}
          </p>
        </Match>
        <Match when={repos.data?.length === 0}>
          <p class="empty">No repos are registered yet.</p>
        </Match>
        <Match when={repos.data}>
          {(registered) => (
            <ul class="set-list">
              <For each={registered()}>{(repo) => <RepoRow repo={repo} />}</For>
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
/// Not a link: there is nowhere to go yet. Built and styled like a list row all
/// the same, because that is what it is going to be.
function RepoRow(props: { repo: RepoEntry }) {
  return (
    <li class="set-row repo-row">
      <div>
        <span class="title">{props.repo.name}</span>
        <span class="meta">
          <span class="path">{props.repo.path}</span>
          <span class="branch">{props.repo.default_branch}</span>
        </span>
      </div>
    </li>
  );
}
