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
//! It is also where the two switches that are about this device and this server
//! rather than about any one Conversation now live — whether the phone is told
//! about a Question Set, and whether a newer Verkstead has been released. They
//! were on the pending list, because that was the page open often enough to be
//! noticed on; with that page retired they belong beside the other thing settled
//! once and then left alone.

import { A } from "@solidjs/router";
import { useMutation, useQuery, useQueryClient } from "@tanstack/solid-query";
import { For, Match, Show, Switch, createSignal } from "solid-js";

import { listRepos, registerRepo } from "../api/client";
import type { Registered, RepoEntry } from "../api/types";
import { Notifications } from "../push/Notifications";
import { UpdateNotice } from "../update/UpdateNotice";

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

  // Read once when the page opens, like the Archive and unlike the pending
  // list: nothing here changes on its own, and what does change is this page's
  // own doing.
  const repos = useQuery(() => ({
    queryKey: ["repos"],
    queryFn: listRepos,
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
    <div class="list-page">
      {/* The way back to the workbench, in the slot every page keeps for where
          else there is to go — this list is reached from its sidebar, which is
          what a Conversation is started against. */}
      <A class="back" href="/">
        ← Workbench
      </A>
      {/* On the title's line rather than buried in a settings page of its own:
          whether this device is told about a Question Set is settled once and
          then left alone, which is the same kind of thing as which repositories
          Verkstead may touch — and a switch is small enough to live in the space
          the heading was leaving empty anyway. */}
      <div class="page-head">
        <h1>Repos</h1>
        <Notifications />
      </div>
      {/* Above the list and under the heading: it is about the server the whole
          page came from rather than about anything on it, and it asks for
          nothing — so it is read on the way past, once, and then it is the
          list's page again. Drawn only when there is a release waiting; this is
          nothing at all the rest of the time. */}
      <UpdateNotice />

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
    </div>
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
