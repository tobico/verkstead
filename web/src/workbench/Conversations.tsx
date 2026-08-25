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
//! The order the rows are in is the human's own. This is one person's working
//! set, so which piece of work sits at the top is theirs to say rather than a
//! sort's — they say it by dragging a row's grip, and what they said is the
//! server's to keep. So a drag sends the whole list and the list comes back from
//! the server on every read, which is what makes the order survive a reload, a
//! restart and a second device without any of the three being a case.
//!
//! The sidebar is also where the rest of Verkstead is reached from, because the
//! workbench has the root: the Repos and the Agent Profiles are behind the ⋯ at
//! the head of the pane rather than a page of their own to find.

import { A } from "@solidjs/router";
import { useMutation, useQueryClient } from "@tanstack/solid-query";
import {
  For,
  Match,
  Show,
  Switch,
  createEffect,
  createSignal,
  type JSX,
} from "solid-js";

import { Menu } from "../Menu";
import {
  listAbandonedRoadmaps,
  listConversations,
  listRepos,
  placeConversations,
  startAdoption,
  startConversation,
} from "../api/client";
import type {
  AbandonedRoadmap,
  ConversationEntry,
  Started,
} from "../api/types";
import { useReading } from "../freshness";
import { Empty, ErrorLine } from "../notices";
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

  const queries = useQueryClient();

  // The order the human is making with their hand, until the server's own list
  // says the same thing. Null the rest of the time, which is every moment
  // nobody is dragging: the order is the server's fact and this is only ever
  // the half-second before it has heard about it.
  const [dragged, setDragged] = createSignal<number[] | null>(null);

  // Which row is under the hand, or null when none is.
  const [held, setHeld] = createSignal<number | null>(null);

  // The list to draw: the server's, in the order being dragged where there is
  // one. A Conversation that has appeared since the drag began is not in that
  // order and goes to the top, which is where an unplaced one goes on the
  // server too.
  const shown = (): ConversationEntry[] => {
    const rows = conversations.data ?? [];
    const order = dragged();
    if (!order) return rows;

    const placed = order
      .map((id) => rows.find((row) => row.id === id))
      .filter((row): row is ConversationEntry => row !== undefined);

    return [...rows.filter((row) => !order.includes(row.id)), ...placed];
  };

  const place = useMutation(() => ({
    mutationFn: (order: number[]) => placeConversations(order),
    onSuccess: () => {
      // Read the list back, which is what lets go of the local order below.
      // The other devices hear the same news as a Nudge.
      void queries.invalidateQueries({ queryKey: ["conversations"] });
    },
    onError: () => {
      // The order was not saved, so drawing it would be drawing something that
      // is not true. The server's list comes back instead, with the error under
      // the list saying why it is what it is.
      setDragged(null);
    },
  }));

  // Let go of the local order the moment the server's list agrees with it,
  // rather than when the request came back: between those two is the re-read,
  // and a list swapped for the old order in the middle of it is a list that
  // jumps back and then forward again.
  createEffect(() => {
    const order = dragged();
    const arrived = conversations.data;
    if (!order || !arrived || held() !== null) return;

    if (
      arrived.length === order.length &&
      arrived.every((row, n) => row.id === order[n])
    ) {
      setDragged(null);
    }
  });

  // The list element, so a drag can ask where the rows actually are. A drag is
  // about pixels, and pixels are something only the DOM knows.
  let list: HTMLUListElement | undefined;

  /// Take hold of a row: the order stops being the server's for as long as the
  /// hand is on it.
  const grab = (event: PointerEvent, id: number) => {
    // The primary button, a finger or a pen. A right-click is not a drag.
    if (event.button !== 0) return;

    // So a touch drags the row instead of selecting the text under it.
    event.preventDefault();

    // Every move from here reaches this grip, whatever the pointer ends up
    // over — including the gap between two rows and the world outside the
    // sidebar.
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);

    // Held first and ordered second, in that order: the two are read together
    // by the effect above, and an order taken hold of by nobody is one it is
    // entitled to throw away.
    setHeld(id);
    setDragged(shown().map((row) => row.id));
  };

  /// The hand moved: put the held row where the pointer is.
  const drag = (event: PointerEvent) => {
    const id = held();
    const order = dragged();
    if (id === null || !order || !list) return;

    const to = order.indexOf(under(list, event.clientY));
    if (to < 0 || to === order.indexOf(id)) return;

    setDragged(moved(order, id, to));
  };

  /// The hand let go: what is on the screen is what the human meant, so that is
  /// what is sent.
  const drop = () => {
    const order = dragged();
    if (held() === null) return;

    setHeld(null);
    if (order) place.mutate(order);
  };

  /// And the same move made from the keyboard, which is the whole of what a
  /// grip has to offer somebody who is not dragging anything: one row up, one
  /// row down, and the list saved each time as a drag saves it.
  const step = (id: number, by: number) => {
    const order = shown().map((row) => row.id);
    const from = order.indexOf(id);
    const to = from + by;
    if (from < 0 || to < 0 || to >= order.length) return;

    const put = moved(order, id, to);
    setDragged(put);
    place.mutate(put);
  };

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
        <WorkbenchActions />
      </div>

      <NewConversation open={props.open} />

      <Switch>
        <Match when={conversations.isPending}>
          <Empty>Loading…</Empty>
        </Match>
        <Match when={conversations.isError}>
          <ErrorLine>
            Could not read the conversations: {conversations.error?.message}
          </ErrorLine>
        </Match>
        <Match when={conversations.data?.length === 0}>
          <Empty>Nothing is being worked on yet.</Empty>
        </Match>
        <Match when={conversations.data}>
          <ul class="conversation-list" ref={list}>
            <For each={shown()}>
              {(entry) => (
                <ConversationRow
                  entry={entry}
                  selected={String(entry.id) === props.selected}
                  held={held() === entry.id}
                  open={props.open}
                  grab={grab}
                  drag={drag}
                  drop={drop}
                  step={step}
                />
              )}
            </For>
          </ul>
        </Match>
      </Switch>

      {/* The order was not saved, which is worth saying because what is on the
          screen is the server's order rather than the one they just made. */}
      <Show when={place.isError}>
        <ErrorLine>
          The order could not be saved: {place.error?.message}
        </ErrorLine>
      </Show>
    </>
  );
}

/// The rest of Verkstead, which is one page: the Repos and the Agent Profiles a
/// Conversation is settled against, and what Verkstead itself has been told.
/// What is waiting on the human is not there — a Question Set is reached
/// through the Conversation it was asked from, which is the list this sits over.
///
/// A ⋯ at the head of the pane rather than the link that used to sit at its
/// foot. That foot is under the conversations, and the conversations are the one
/// part of the pane with no end: a long enough list and the way out to the
/// settings was somewhere the human had to scroll to find. Up here it is where
/// the ⋯ at the top of a Conversation is, drawn through the same component and
/// painted by the same rule — and the two of them mean the same thing in their
/// two places, which is *what there is about this pane that is not in it*.
///
/// A menu for one entry, because the entry is what is behind it rather than what
/// it is: the next thing that is about the workbench as a whole goes in beside
/// Settings rather than beside the wordmark.
///
/// A link rather than a button, and the only row of any menu that is one: the
/// settings are a page of their own, so this is going somewhere in the way that
/// opening a Conversation is not. Nothing here has to shut the menu — the
/// navigation takes the whole sidebar with it.
function WorkbenchActions(): JSX.Element {
  return (
    <Menu
      class="workbench-actions"
      label="Workbench actions"
      name="Workbench actions"
      trigger="⋯"
    >
      {() => (
        <A role="menuitem" href="/settings">
          Settings
        </A>
      )}
    </Menu>
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

  // `false` until the open menu has taken the focus, which it does once and on
  // the first repo there is.
  let taken = false;

  // The menu's own way to shut, held here because what closes this one is a
  // request coming back rather than the press that sent it.
  let shut = (): void => {};

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
      shut();
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
      shut();
      void queries.invalidateQueries({ queryKey: ["conversations"] });
      props.open(outcome.Started.id);
    },
  }));

  return (
    <Menu
      class="new-conversation"
      name="New conversation"
      closer={(close) => (shut = close)}
      opening={() => (taken = false)}
      trigger={
        <>
          New conversation
          {/* Which way the menu will go, and no part of what the button
              says. */}
          <span class="new-conversation-mark" aria-hidden="true">
            ▾
          </span>
        </>
      }
    >
      {() => (
        <>
          <Switch>
            <Match when={repos.data?.length === 0}>
              {/* Nothing to attach a Conversation to, so the only thing to
                  offer is the page that fixes that. */}
              <Empty class="nothing">
                No repos are registered yet —{" "}
                <A href="/settings">register one</A> to start a conversation.
              </Empty>
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
            <ErrorLine class="failure">
              The conversation could not be started: {start.error?.message}
            </ErrorLine>
          </Show>
          <Show when={adopt.isError}>
            <ErrorLine class="failure">
              The conversation could not be started: {adopt.error?.message}
            </ErrorLine>
          </Show>
        </>
      )}
    </Menu>
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
  held: boolean;
  open: (id: number) => void;
  grab: (event: PointerEvent, id: number) => void;
  drag: (event: PointerEvent) => void;
  drop: () => void;
  step: (id: number, by: number) => void;
}): JSX.Element {
  const ended = (): boolean =>
    props.entry.state === "Done" || props.entry.state === "Aborted";

  return (
    <li
      class="conversation-row"
      // Read by the drag to say which row the pointer is over, which is a
      // question about the rendered list rather than about the data behind it.
      data-id={props.entry.id}
      classList={{
        selected: props.selected,
        draft: props.entry.state === "Draft",
        ended: ended(),
        waiting: mark(props.entry) === "waiting",
        held: props.held,
      }}
    >
      <button
        type="button"
        class="open"
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
          {(which) => (
            <span class={`mark ${which()}`} aria-hidden="true">
              {which() === "waiting" ? WANTS_YOU : ""}
            </span>
          )}
        </Show>
      </button>

      {/* The grip: what is dragged, and the one control on this row that is
          about the list rather than about the Conversation. Its own label,
          because what it does is not what the card beside it does — and its own
          keys, because a control that could only be dragged would be a control
          half the people using it could not reach. */}
      <button
        type="button"
        class="grip"
        aria-label={`Move ${props.entry.branch}`}
        onPointerDown={(event) => props.grab(event, props.entry.id)}
        onPointerMove={props.drag}
        onPointerUp={props.drop}
        onPointerCancel={props.drop}
        onKeyDown={(event) => {
          if (event.key === "ArrowUp") {
            event.preventDefault();
            props.step(props.entry.id, -1);
          } else if (event.key === "ArrowDown") {
            event.preventDefault();
            props.step(props.entry.id, 1);
          }
        }}
      >
        {GRIP}
      </button>
    </li>
  );
}

/// The same list with one row moved to a place in it, which is the whole of
/// what a drag and an arrow key each do.
function moved(order: number[], id: number, to: number): number[] {
  const put = [...order];
  put.splice(put.indexOf(id), 1);
  put.splice(to, 0, id);
  return put;
}

/// Which row the pointer is over: the first whose bottom edge is below it, and
/// the last row when it is below all of them.
///
/// By the rendered rows rather than by arithmetic over a row height, because
/// rows are not all one height — a long branch name wraps — and a drag that
/// guessed would put the row somewhere the human was not pointing.
function under(list: HTMLUListElement, y: number): number {
  const rows = [...list.querySelectorAll<HTMLElement>(".conversation-row")];
  const over =
    rows.find((row) => y < row.getBoundingClientRect().bottom) ?? rows.at(-1);

  return Number(over?.dataset.id ?? NaN);
}

/// What the mark on a Conversation waiting on the human says, inside the accent
/// disc the stylesheet draws around it.
///
/// An icon rather than the dot this used to be, because what it has to survive
/// is a glance down a list on a phone: a shape is read where a dot has to be
/// looked for. Hidden from a screen reader, which is told the same thing in
/// words by the card's own label — see [`spoken`].
const WANTS_YOU = "!";

/// And what the grip says: the dots everything draggable is gripped by, so that
/// what it is for needs no explaining. Two columns of them, which is the shape
/// the convention is, in characters rather than in a drawing — every other mark
/// in this viewer is a character.
const GRIP = "⋮⋮";
