//! The conversations sidebar: what there is to work on, and the one way to add
//! to it.
//!
//! A Conversation is started against a registered Repo and nothing else — the
//! branch name is the server's to prefill and the brief is written afterwards,
//! in the Timeline, which is where it lives. So the whole of starting one is
//! saying which repository the work is in, and one press of a menu says it:
//! the button drops the registered Repos, and the Repo pressed *is* the
//! Conversation started. See [`NewConversation`].
//!
//! The roadmaps nothing is driving are in that menu too, under a heading of
//! their own — see [`NewConversation`] again, because they are the same thing:
//! another way work gets into the pipeline, rather than something waiting on
//! the human.
//!
//! The row's name is the branch. A Conversation has no title of its own, and of
//! what it does have the branch is the short line the human chose — and the one
//! they can change while it is still drafting.
//!
//! The sidebar is also where the rest of Verkstead is reached from, because the
//! workbench has the root: the Repos and the Agent Profiles are a line at the
//! bottom of it rather than a page of their own to find.

import { A } from "@solidjs/router";
import { useMutation, useQueryClient } from "@tanstack/solid-query";
import {
  For,
  Match,
  Show,
  Switch,
  createSignal,
  onCleanup,
  type JSX,
} from "solid-js";

import {
  listAbandonedRoadmaps,
  listConversations,
  listRepos,
  startAdoption,
  startConversation,
} from "../api/client";
import type {
  AbandonedRoadmap,
  ConversationEntry,
  Started,
} from "../api/types";
import { useReading } from "../freshness";
import { SPOKEN } from "./Mark";

export function Conversations(props: {
  selected: string;
  open: (id: number) => void;
}): JSX.Element {
  const conversations = useReading(() => ({
    queryKey: ["conversations"],
    queryFn: listConversations,

    // Merged by the id each row carries flat, because this list is re-read
    // constantly — a session talking moves the *working* badge on one row of
    // it — and a rebuilt row is a row whose spinner starts its animation again.
    freshness: { reconcile: "id" },
  }));

  return (
    <>
      <div class="pane-head">
        {/* The mark rather than a title: this pane is where Verkstead is entered
            and the list under it says what it is a list of. The icon is served
            from `assets/`, which vite copies to the site root untouched, and it
            is the same file the favicon is.

            No alt text on it, because the word it stands beside is the alt text:
            a screen reader that read both would say the name twice. */}
        <h1 class="wordmark">
          <img src="/icons/verkstead.svg" alt="" />
          Verkstead
        </h1>
      </div>

      <NewConversation open={props.open} />

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

      {/* The rest of Verkstead, which is one page: the Repos and the Agent
          Profiles a Conversation is settled against, and what Verkstead itself
          has been told. What is waiting on the human is not there — a Question
          Set is reached through the Conversation it was asked from, which is
          the list above. */}
      <nav class="elsewhere">
        <A class="to-settings" href="/settings">
          Settings →
        </A>
      </nav>
    </>
  );
}

/// Every way work gets into the pipeline, behind one button: the registered
/// Repos a Conversation can be started in, and under them the roadmaps nothing
/// is driving.
///
/// A menu rather than the box this used to be. Starting a Conversation is one
/// decision — which repository — and a form that held a dropdown, a label and a
/// Start button spent three controls and a permanent corner of the sidebar on
/// it. Here the Repo pressed *is* the choice made: there is nothing held
/// between picking and sending, so what the human read off the row and what
/// went on the wire cannot come apart, which is the whole of what the
/// [`Picker`](../picking.tsx) was guarding in this one place.
///
/// The roadmaps are in the same menu because they are the same kind of thing —
/// another way to start work, on something somebody staged before Verkstead was
/// driving anything. They were notices stacked under the box, and being always
/// in view was what was wrong with them: nothing is waiting on the human there.
/// Flat rather than nested under their Repo, so the group is a list of roadmaps
/// to adopt rather than a tree to walk.
///
/// There is still no way to dismiss one. The repository is the source of truth
/// for its own roadmaps everywhere else, so a roadmap that is true and unwanted
/// is silenced in the repository — tick the box, or annotate the stage — and
/// what stands in for never dismissing it is that the group is here every time
/// the menu opens.
///
/// Pressing a roadmap starts a Conversation to adopt it with — a draft, on a
/// page shaped for adopting rather than for grilling. Nothing is adopted by
/// pressing it: both pairings have to be fixed first, and there is a press on
/// that page for the adopting itself.
function NewConversation(props: { open: (id: number) => void }): JSX.Element {
  const queries = useQueryClient();

  // The Repos are the sidebar's business only because starting a Conversation
  // needs one. Read here rather than passed down, so the menu is whole wherever
  // it is drawn.
  const repos = useReading(() => ({
    queryKey: ["repos"],
    queryFn: listRepos,

    // Merged, and for this menu rather than for any list: a rebuilt row is a
    // new element, and a Nudge landing while the menu is open would take the
    // human's focus off the row they had tabbed to.
    freshness: { reconcile: "id" },
  }));

  const abandoned = useReading(() => ({
    queryKey: ["abandoned-roadmaps"],
    queryFn: listAbandonedRoadmaps,

    // Keyed by `repo_id` and not `id`: this list is Repos rather than records
    // of its own, so what identifies a row is which Repo it is about. The
    // roadmaps nested under each one carry no key at all and are matched by
    // position, which is what the server sorts them into.
    freshness: { reconcile: "repo_id" },
  }));

  // `true` while the menu hangs open under the button.
  const [open, setOpen] = createSignal(false);

  // The button, so that the keyboard's way out puts the focus back where it
  // came from rather than at the top of the page.
  let trigger!: HTMLButtonElement;

  // `false` until the open menu has taken the focus, which it does once and on
  // the first repo there is.
  let taken = false;

  /// Take the focus to the first row of the menu, once per opening.
  ///
  /// Opened from the keyboard the first row is where the human is going, and a
  /// menu whose first Tab lands past it is one they have to walk backwards out
  /// of. Hung off the row itself rather than off the menu, because the menu
  /// opens whether or not the repos have arrived — a menu that looked for a row
  /// the moment it was drawn would as often as not find none. In a microtask
  /// because focusing an element the document does not hold yet does nothing at
  /// all.
  const take = (row: HTMLElement): void => {
    queueMicrotask(() => {
      if (taken || !row.isConnected) return;
      taken = true;
      row.focus();
    });
  };

  /// Every roadmap there is to adopt, flat, each still knowing which Repo it is
  /// in — which is what a press needs and what a line with two `mvp`s in it
  /// would otherwise be missing.
  const roadmaps = (): Array<{
    repoId: number;
    repo: string;
    roadmap: AbandonedRoadmap;
  }> =>
    (abandoned.data ?? []).flatMap((held) =>
      held.roadmaps.map((roadmap) => ({
        repoId: held.repo_id,
        repo: held.repo,
        roadmap,
      })),
    );

  const start = useMutation(() => ({
    mutationFn: (repoId: number) => startConversation(repoId),
    onSuccess: (outcome: Started) => {
      if (typeof outcome === "string") {
        // `NoSuchRepo`, from a list this menu read a moment ago: the Repo was
        // there and is not now. Reading the list again is both the correction
        // and the explanation, and the menu stays open to be read.
        void queries.invalidateQueries({ queryKey: ["repos"] });
        return;
      }

      // Straight into it: what the human does next is write the brief, and the
      // Conversation appearing in the sidebar is the confirmation on the way.
      setOpen(false);
      void queries.invalidateQueries({ queryKey: ["conversations"] });
      props.open(outcome.Started.id);
    },
  }));

  const adopt = useMutation(() => ({
    mutationFn: ({ repoId, roadmap }: { repoId: number; roadmap: string }) =>
      startAdoption(repoId, roadmap),
    onSuccess: (outcome: Started) => {
      if (typeof outcome === "string") {
        // `NoSuchRepo`, against a row read a moment ago: the Repo was there and
        // is not now. Reading both lists again is the correction and the
        // explanation together.
        void queries.invalidateQueries({ queryKey: ["repos"] });
        void queries.invalidateQueries({ queryKey: ["abandoned-roadmaps"] });
        return;
      }

      // Straight onto its page, which is where the two pairings and the base
      // commit are fixed and where adopting is pressed.
      setOpen(false);
      void queries.invalidateQueries({ queryKey: ["conversations"] });
      props.open(outcome.Started.id);
    },
  }));

  // The way out that needs no aim: a menu drawn over the page has to be
  // dismissible from the keyboard, and the focus goes back to the button that
  // opened it. The other way — a press on the page — is the backdrop's, so the
  // press taking the menu back cannot also press something underneath it.
  const escape = (ev: KeyboardEvent) => {
    if (ev.key === "Escape" && open()) {
      setOpen(false);
      trigger.focus();
    }
  };

  document.addEventListener("keydown", escape);
  onCleanup(() => document.removeEventListener("keydown", escape));

  return (
    <div class="new-conversation">
      <button
        type="button"
        class="new-conversation-trigger"
        ref={trigger}
        aria-haspopup="menu"
        aria-expanded={open() ? "true" : "false"}
        aria-controls="new-conversation-menu"
        onClick={() => {
          taken = false;
          setOpen(!open());
        }}
      >
        New conversation
        {/* Which way the menu will go, and no part of what the button says. */}
        <span class="new-conversation-mark" aria-hidden="true">
          ▾
        </span>
      </button>

      <Show when={open()}>
        <div
          class="new-conversation-backdrop"
          aria-hidden="true"
          onClick={() => setOpen(false)}
        />
        <div
          class="new-conversation-menu"
          id="new-conversation-menu"
          role="menu"
          aria-label="New conversation"
        >
          <Switch>
            <Match when={repos.data?.length === 0}>
              {/* Nothing to attach a Conversation to, so the only thing to
                  offer is the page that fixes that. */}
              <p class="empty">
                No repos are registered yet —{" "}
                <A href="/settings">register one</A> to start a conversation.
              </p>
            </Match>
            <Match when={repos.data}>
              {(registered) => (
                <For each={registered()}>
                  {(repo, at) => (
                    <button
                      type="button"
                      role="menuitem"
                      class="in-repo"
                      ref={(row) => at() === 0 && take(row)}
                      disabled={start.isPending}
                      onClick={() => start.mutate(repo.id)}
                    >
                      {repo.name}
                    </button>
                  )}
                </For>
              )}
            </Match>
          </Switch>

          <Show when={roadmaps().length}>
            <div
              class="menu-group"
              role="group"
              aria-labelledby="adopt-a-roadmap"
            >
              <p class="menu-heading" id="adopt-a-roadmap">
                Adopt a roadmap
              </p>
              <For each={roadmaps()}>
                {(held) => (
                  <button
                    type="button"
                    role="menuitem"
                    class="adopt-roadmap"
                    disabled={adopt.isPending}
                    onClick={() =>
                      adopt.mutate({
                        repoId: held.repoId,
                        roadmap: held.roadmap.name,
                      })
                    }
                  >
                    <span class="what">
                      <code>{held.roadmap.name}</code>
                      <span class="in">in {held.repo}</span>
                    </span>
                    <span class="stage">
                      next is stage {held.roadmap.stage}:{" "}
                      {held.roadmap.stage_title}
                    </span>
                  </button>
                )}
              </For>
            </div>
          </Show>

          {/* A server that could not answer at all, which is the one thing here
              that is an error rather than an outcome. Said inside the menu, and
              the menu is still open to say it in: a press that failed left
              nothing else on the screen to carry the news. */}
          <Show when={start.isError}>
            <p class="error">
              The conversation could not be started: {start.error?.message}
            </p>
          </Show>
          <Show when={adopt.isError}>
            <p class="error">
              The conversation could not be started: {adopt.error?.message}
            </p>
          </Show>
        </div>
      </Show>
    </div>
  );
}

/// Which mark a card carries at its right edge, or nothing where it carries
/// none.
///
/// Waiting wins, and never both: a Conversation whose session is sitting on a
/// Blocking Ask is working *and* waiting, and of the two the one the human can
/// do something about is the ask. So the dot is what a card shows the moment
/// there is anything to answer, and a ring is what is left.
///
/// Which of the two rings it is says whether that session is doing anything: the
/// turning one while it prints, and the empty one once it has gone quiet — the
/// same pair the Timeline row and the details pane draw, so a card and the
/// session it stands for say the same thing. A grilling that has been sitting on
/// an ask for an hour turning a spinner is the case this is for, and the reason
/// the empty ring is the quieter mark of the two.
function mark(entry: ConversationEntry): "waiting" | "working" | "idle" | null {
  if (entry.waiting) return "waiting";
  if (entry.working) return entry.idle ? "idle" : "working";
  return null;
}

/// What a row says when it is read aloud.
///
/// The card says where a Conversation has got to in marks rather than in words —
/// see the row's classes and [`mark`] — and a mark is nothing to a screen
/// reader. So the whole of it goes on the button's label instead: the branch it
/// is named by, the Repo it is in, the state that used to be written under the
/// name, and what the mark would have said.
///
/// What the two rings say is [`SPOKEN`], the words the mark itself carries
/// wherever it labels itself: the same ring should not mean one thing on a card
/// and another on the row it opens.
function spoken(entry: ConversationEntry): string {
  const which = mark(entry);
  const said =
    which === "waiting"
      ? `${entry.state}, waiting on you`
      : which
        ? `${entry.state}, ${SPOKEN[which]}`
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
