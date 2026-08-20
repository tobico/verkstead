//! The conversations sidebar: what there is to work on, and the one way to add
//! to it.
//!
//! A Conversation is started against a registered Repo and nothing else — the
//! branch name is the server's to prefill and the brief is written afterwards,
//! in the Timeline, which is where it lives. So the whole of starting one is
//! picking which repository the work is in.
//!
//! The row's name is the branch. A Conversation has no title of its own, and of
//! what it does have the branch is the short line the human chose — and the one
//! they can change while it is still drafting.
//!
//! The sidebar is also where the rest of Verkstead is reached from, because the
//! workbench has the root now: the pending list and the Repos are a line at the
//! bottom of it rather than a page of their own to find.

import { A } from "@solidjs/router";
import { useMutation, useQuery, useQueryClient } from "@tanstack/solid-query";
import { For, Match, Show, Switch, createSignal, type JSX } from "solid-js";

import { listConversations, listRepos, startConversation } from "../api/client";
import type { ConversationEntry, Started } from "../api/types";

export function Conversations(props: {
  selected: string;
  open: (id: number) => void;
}): JSX.Element {
  const queries = useQueryClient();

  const conversations = useQuery(() => ({
    queryKey: ["conversations"],
    queryFn: listConversations,
  }));

  // The Repos are the sidebar's business only because starting a Conversation
  // needs one picked. Read here rather than passed down, so the picker is whole
  // wherever it is drawn.
  const repos = useQuery(() => ({
    queryKey: ["repos"],
    queryFn: listRepos,
  }));

  // Which Repo the next Conversation is against. Empty until the list has
  // arrived and the first row can stand as the choice already made.
  const [against, setAgainst] = createSignal("");

  const chosen = () => against() || String(repos.data?.[0]?.id ?? "");

  const start = useMutation(() => ({
    mutationFn: (repoId: number) => startConversation(repoId),
    onSuccess: (outcome: Started) => {
      if (typeof outcome === "string") {
        // `NoSuchRepo`, from a list this page read a moment ago: the Repo was
        // there and is not now. Reading the list again is both the correction
        // and the explanation.
        void queries.invalidateQueries({ queryKey: ["repos"] });
        return;
      }

      // Straight into it: what the human does next is write the brief, and the
      // Conversation appearing in the sidebar is the confirmation on the way.
      void queries.invalidateQueries({ queryKey: ["conversations"] });
      props.open(outcome.Started.id);
    },
  }));

  return (
    <>
      <div class="pane-head">
        <h1>Conversations</h1>
      </div>

      <Switch>
        <Match when={repos.data?.length === 0}>
          {/* Nothing to attach a Conversation to, so the only thing to offer is
              the page that fixes that. */}
          <p class="empty">
            No repos are registered yet — <A href="/repos">register one</A> to
            start a conversation.
          </p>
        </Match>
        <Match when={repos.data}>
          {(registered) => (
            <form
              class="start-conversation"
              onSubmit={(ev) => {
                ev.preventDefault();
                const repoId = Number(chosen());
                if (repoId) {
                  start.mutate(repoId);
                }
              }}
            >
              <label for="against">New conversation in</label>
              <div class="start-conversation-line">
                <select
                  id="against"
                  value={chosen()}
                  onChange={(ev) => setAgainst(ev.currentTarget.value)}
                >
                  <For each={registered()}>
                    {(repo) => <option value={repo.id}>{repo.name}</option>}
                  </For>
                </select>
                <button type="submit" disabled={start.isPending}>
                  Start
                </button>
              </div>
              {/* A server that could not answer at all, which is the one thing
                  here that is an error rather than an outcome. */}
              <Show when={start.isError}>
                <p class="error">
                  The conversation could not be started: {start.error?.message}
                </p>
              </Show>
            </form>
          )}
        </Match>
      </Switch>

      <Switch>
        <Match when={conversations.isPending}>
          <p class="empty">Loading…</p>
        </Match>
        <Match when={conversations.isError}>
          <p class="error">
            Could not read the conversations: {conversations.error?.message}
          </p>
        </Match>
        <Match when={conversations.data?.length === 0}>
          <p class="empty">Nothing is being worked on yet.</p>
        </Match>
        <Match when={conversations.data}>
          {(rows) => (
            <ul class="conversation-list">
              <For each={rows()}>
                {(entry) => (
                  <ConversationRow
                    entry={entry}
                    selected={String(entry.id) === props.selected}
                    open={props.open}
                  />
                )}
              </For>
            </ul>
          )}
        </Match>
      </Switch>

      {/* The rest of Verkstead. The pending list is where the phone answers, and
          it is a page of its own for as long as Question Sets are not yet on a
          Timeline — see the workbench roadmap. */}
      <nav class="elsewhere">
        <A class="to-pending" href="/pending">
          Pending →
        </A>
        <A class="to-repos" href="/repos">
          Repos →
        </A>
      </nav>
    </>
  );
}

/// One Conversation: the branch it will be done on, the Repo it is in, and where
/// it has got to.
///
/// A button rather than a link, because the whole workbench is one page: opening
/// a Conversation moves the panes rather than going somewhere, and the URL that
/// follows is a record of what is open rather than a document to fetch.
function ConversationRow(props: {
  entry: ConversationEntry;
  selected: boolean;
  open: (id: number) => void;
}): JSX.Element {
  return (
    <li class="conversation-row" classList={{ selected: props.selected }}>
      <button
        type="button"
        aria-current={props.selected ? "true" : undefined}
        onClick={() => props.open(props.entry.id)}
      >
        <span class="title">{props.entry.branch}</span>
        <span class="meta">
          <span class="repo">{props.entry.repo}</span>
          <span class="state">{props.entry.state}</span>
        </span>
      </button>
    </li>
  );
}
