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
//! The order the rows are in is the human's own. This is one person's working
//! set, so which piece of work sits at the top is theirs to say rather than a
//! sort's — they say it by dragging a row's grip, and what they said is the
//! server's to keep. So a drag sends the whole list and the list comes back from
//! the server on every read, which is what makes the order survive a reload, a
//! restart and a second device without any of the three being a case.
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
  createEffect,
  createSignal,
  type JSX,
} from "solid-js";

import {
  listAbandonedRoadmaps,
  listConversations,
  listRepos,
  placeConversations,
  startAdoption,
  startConversation,
} from "../api/client";
import type { ConversationEntry, Started } from "../api/types";
import { useReading } from "../freshness";
import { Picker } from "../picking";

export function Conversations(props: {
  selected: string;
  open: (id: number) => void;
}): JSX.Element {
  const queries = useQueryClient();

  const conversations = useReading(() => ({
    queryKey: ["conversations"],
    queryFn: listConversations,

    // Merged by the id each row carries flat, because this list is re-read
    // constantly — a session talking moves the *working* badge on one row of
    // it — and a rebuilt row is a row whose spinner starts its animation again.
    freshness: { reconcile: "id" },
  }));

  // The Repos are the sidebar's business only because starting a Conversation
  // needs one picked. Read here rather than passed down, so the picker is whole
  // wherever it is drawn.
  const repos = useReading(() => ({
    queryKey: ["repos"],
    queryFn: listRepos,

    // Merged, and for the picker below rather than for the list: a rebuilt
    // `<option>` is a new element in a `<select>` the human may have open, and
    // the choice they were part-way through goes with the old one.
    freshness: { reconcile: "id" },
  }));

  // Which Repo the next Conversation is against. Empty until the list has
  // arrived and the first row can stand as the choice already made.
  const [against, setAgainst] = createSignal("");

  const chosen = () => against() || String(repos.data?.[0]?.id ?? "");

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
      // the box saying why it is what it is.
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
    // by the effect below, and an order taken hold of by nobody is one it is
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
            No repos are registered yet —{" "}
            <A href="/settings">register one</A> to start a conversation.
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
                {/* A [`Picker`] rather than a `<select>`, so what this shows and
                    what the press below sends are the same repository even when
                    the list moved under it — see `src/picking.tsx`. */}
                <Picker
                  id="against"
                  options={registered()}
                  value={(repo) => String(repo.id)}
                  label={(repo) => repo.name}
                  chosen={chosen()}
                  pick={setAgainst}
                  // The Repo that was picked is gone from the list: the choice
                  // goes with it, and `chosen` falls back to the first row that
                  // is left — which is the state this box opened in.
                  gone={() => setAgainst("")}
                />
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
        <p class="error">The order could not be saved: {place.error?.message}</p>
      </Show>

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

  const abandoned = useReading(() => ({
    queryKey: ["abandoned-roadmaps"],
    queryFn: listAbandonedRoadmaps,

    // Keyed by `repo_id` and not `id`: this list is Repos rather than records
    // of its own, so what identifies a row is which Repo it is about. The
    // roadmaps nested under each one carry no key at all and are matched by
    // position, which is what the server sorts them into.
    freshness: { reconcile: "repo_id" },
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
