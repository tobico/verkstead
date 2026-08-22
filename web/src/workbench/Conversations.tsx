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
//! Under that box are the roadmaps nothing is driving, one notice per Repo —
//! see [`Abandoned`]. Read there because that is what they are about: work
//! somebody staged before Verkstead was driving anything, with a stage waiting
//! to be started.
//!
//! Clicking one of those roadmaps starts a conversation to adopt it with: a
//! draft on a page shaped for adopting, which is the other way work gets into
//! the pipeline.
//!
//! The sidebar is also where the rest of Verkstead is reached from, because the
//! workbench has the root: the Repos and the Agent Profiles are a line at the
//! bottom of it rather than a page of their own to find.

import { A } from "@solidjs/router";
import { useMutation, useQuery, useQueryClient } from "@tanstack/solid-query";
import { For, Match, Show, Switch, createSignal, type JSX } from "solid-js";

import {
  listAbandonedRoadmaps,
  listConversations,
  listRepos,
  startAdoption,
  startConversation,
} from "../api/client";
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

      <Abandoned open={props.open} />

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

      {/* The rest of Verkstead: the two things a Conversation is settled
          against, and nothing else. What is waiting on the human is not among
          them — a Question Set is reached through the Conversation it was asked
          from, which is the list above. */}
      <nav class="elsewhere">
        <A class="to-repos" href="/repos">
          Repos →
        </A>
        <A class="to-profiles" href="/profiles">
          Profiles →
        </A>
      </nav>
    </>
  );
}

/// The Repos holding roadmaps nothing is driving, one notice each.
///
/// Under the new-conversation box because that is what it is: another way to
/// start work, on a roadmap somebody wrote before Verkstead was driving
/// anything. What each one names is the roadmap and the stage that would be
/// started, which is the whole of the decision.
///
/// Each roadmap is a button, and pressing it starts a conversation to adopt
/// that roadmap with — a draft, on a page shaped for adopting rather than for
/// grilling. Nothing is adopted by pressing it: both profiles have to be fixed
/// first, and there is a press on that page for the adopting itself.
///
/// There is no way to dismiss one, now or later. The repository is the source
/// of truth for its own roadmaps everywhere else, so a notice that is true and
/// unwanted is silenced in the repository — tick the box, or annotate the
/// stage. A dismissal Verkstead stored would be a second opinion about a
/// roadmap the repository says has work left.
function Abandoned(props: { open: (id: number) => void }): JSX.Element {
  const queries = useQueryClient();

  const abandoned = useQuery(() => ({
    queryKey: ["abandoned-roadmaps"],
    queryFn: listAbandonedRoadmaps,
  }));

  const adopt = useMutation(() => ({
    mutationFn: ({ repoId, roadmap }: { repoId: number; roadmap: string }) =>
      startAdoption(repoId, roadmap),
    onSuccess: (outcome: Started) => {
      if (typeof outcome === "string") {
        // `NoSuchRepo`, against a notice read a moment ago: the Repo was there
        // and is not now. Reading both lists again is the correction and the
        // explanation together.
        void queries.invalidateQueries({ queryKey: ["repos"] });
        void queries.invalidateQueries({ queryKey: ["abandoned-roadmaps"] });
        return;
      }

      // Straight onto its page, which is where the two profiles and the base
      // commit are fixed and where adopting is pressed.
      void queries.invalidateQueries({ queryKey: ["conversations"] });
      props.open(outcome.Started.id);
    },
  }));

  return (
    <Show when={abandoned.data?.length}>
      <div class="abandoned">
        <For each={abandoned.data}>
          {(repo) => (
            <section class="abandoned-notice">
              <p>
                <code>{repo.repo}</code> holds roadmaps nothing is driving.
              </p>
              <ul>
                <For each={repo.roadmaps}>
                  {(roadmap) => (
                    <li>
                      <button
                        type="button"
                        class="adopt-roadmap"
                        disabled={adopt.isPending}
                        onClick={() =>
                          adopt.mutate({
                            repoId: repo.repo_id,
                            roadmap: roadmap.name,
                          })
                        }
                      >
                        <code>{roadmap.name}</code>
                        <span class="stage">
                          next is stage {roadmap.stage}: {roadmap.stage_title}
                        </span>
                      </button>
                    </li>
                  )}
                </For>
              </ul>
              {/* A server that could not answer at all, which is the one thing
                  here that is an error rather than an outcome. */}
              <Show when={adopt.isError}>
                <p class="error">
                  The conversation could not be started: {adopt.error?.message}
                </p>
              </Show>
            </section>
          )}
        </For>
      </div>
    </Show>
  );
}

/// Which mark a card carries at its right edge, or nothing where it carries
/// none.
///
/// Waiting wins, and never both: a Conversation whose session is idling on a
/// Blocking Ask is working *and* waiting, and of the two the one the human can
/// do something about is the ask. So the dot is what a card shows the moment
/// there is anything to answer, and the spinner is what is left — a session
/// getting on with it, with nothing wanted from anybody.
function mark(entry: ConversationEntry): "waiting" | "working" | null {
  if (entry.waiting) return "waiting";
  if (entry.working) return "working";
  return null;
}

/// What a row says when it is read aloud.
///
/// The card says where a Conversation has got to in marks rather than in words —
/// see the row's classes and [`mark`] — and a mark is nothing to a screen
/// reader. So the whole of it goes on the button's label instead: the branch it
/// is named by, the Repo it is in, the state that used to be written under the
/// name, and what the mark would have said.
function spoken(entry: ConversationEntry): string {
  const which = mark(entry);
  const said =
    which === "waiting"
      ? `${entry.state}, waiting on you`
      : which === "working"
        ? `${entry.state}, a session is running`
        : entry.state;

  return `${entry.branch}, ${entry.repo}, ${said}`;
}

/// One Conversation: the branch it will be done on, the Repo it is in, and where
/// it has got to.
///
/// A button rather than a link, because the whole workbench is one page: opening
/// a Conversation moves the panes rather than going somewhere, and the URL that
/// follows is a record of what is open rather than a document to fetch.
///
/// Where it has got to is drawn rather than written: a dotted card is a draft, a
/// dimmed one is work that has stopped, and the mark at the right edge is a
/// session running or an answer wanted. Every other state is the ordinary card —
/// grilling, implementing and wrapping are not told apart here, because what the
/// sidebar is for is finding the Conversation to look at and all three are *this
/// one is under way*.
function ConversationRow(props: {
  entry: ConversationEntry;
  selected: boolean;
  open: (id: number) => void;
}): JSX.Element {
  const ended = (): boolean =>
    props.entry.state === "Done" || props.entry.state === "Aborted";

  return (
    <li
      class="conversation-row"
      classList={{
        selected: props.selected,
        draft: props.entry.state === "Draft",
        ended: ended(),
      }}
    >
      <button
        type="button"
        aria-current={props.selected ? "true" : undefined}
        aria-label={spoken(props.entry)}
        onClick={() => props.open(props.entry.id)}
      >
        <span class="what">
          <span class="title">{props.entry.branch}</span>
          <span class="meta">
            <span class="repo">{props.entry.repo}</span>
          </span>
        </span>
        {/* Drawn only where there is one, so a row with nothing to mark gives
            the whole width to its name. The label above has already said what
            it means, so there is nothing here for a screen reader to find. */}
        <Show when={mark(props.entry)}>
          {(which) => <span class={`mark ${which()}`} />}
        </Show>
      </button>
    </li>
  );
}
