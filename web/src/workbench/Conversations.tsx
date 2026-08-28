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
//! sort's — they say it by dragging a card, and what they said is the server's
//! to keep. So a drag sends the whole list and the list comes back from the
//! server on every read, which is what makes the order survive a reload, a
//! restart and a second device without any of the three being a case.
//!
//! A card also answers a right-click with what there is to do about the
//! Conversation it stands for — the same rows the ⋯ at the head of the
//! Conversation pane would offer it, drawn by the same component and acting on
//! the card that was pressed rather than on whatever is open. Both menus are
//! `Actions.tsx`, which is where the rows and everything behind them live.
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
  onCleanup,
  type JSX,
} from "solid-js";

import { Menu } from "../Menu";
import { Switch as Toggle } from "../Switch";
import {
  listAbandonedRoadmaps,
  listConversations,
  listRepos,
  placeConversations,
  showArchived,
  showingArchived,
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
import { CardActions } from "./Actions";
import styles from "./Conversations.module.css";
import { SPOKEN } from "./Mark";
// The rings and the badge a card carries at its right edge. Drawn here rather
// than by `Mark` because the sidebar has a state no running session has —
// something is waiting on you — but read out of the one module all the same,
// so a ring means the same thing in the list that it means on the row it
// opens.
import marks from "./Mark.module.css";
import { PaneHead } from "./PaneHead";
import { WAITING_ON_CHECKS } from "./conditions";
import { STATE } from "./states";

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

  // Which card was right-clicked and where the pointer was, or null while no
  // context menu is open. A signal because the menu is drawn from it, unlike
  // the press below.
  const [pointed, setPointed] = createSignal<{
    id: number;
    x: number;
    y: number;
  } | null>(null);

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

  // The press in flight: which card it is on, where on the screen it started,
  // whether it is a finger, and whether the card has lifted under it yet. Null
  // every moment nothing is pressed, which is nearly all of them.
  //
  // Not a signal, because nothing is drawn from it: what a lifted card is drawn
  // from is `held` above, and the rest of this is bookkeeping between one
  // pointer event and the next.
  let press: {
    id: number;
    x: number;
    y: number;
    touch: boolean;
    lifted: boolean;
    waiting?: ReturnType<typeof setTimeout>;
  } | null = null;

  // Whether the press that has just ended moved a card. The click arrives after
  // the pointer is up, and a card dragged into place should not open as well.
  //
  // Spent by the one click it is for, rather than standing until the next
  // press. A keyboard press is a click with no pointer behind it, so a flag
  // left set would leave the card the drag ended on deaf to Enter and Space
  // until the hand went back to it — which is the one way in for somebody who
  // is not dragging anything.
  let reordered = false;

  // And whether the press this gesture began with was a finger rather than a
  // mouse. Kept past the press itself, because what it answers arrives late: a
  // phone fires `contextmenu` from a long press, and telling that from a
  // right-click is the one thing the event cannot say for itself.
  let fromTouch = false;

  /// The card lifts: the order stops being the server's for as long as the hand
  /// is on it.
  const lift = (at: NonNullable<typeof press>) => {
    at.lifted = true;

    // Held first and ordered second, in that order: the two are read together
    // by the effect above, and an order taken hold of by nobody is one it is
    // entitled to throw away.
    setHeld(at.id);
    setDragged(shown().map((row) => row.id));

    // The list must not scroll out from under a card being moved. A
    // `touch-action` on the card would have said so before the finger landed
    // and taken the swipe that scrolls the list with it, so the scroll is
    // refused here instead: from the lift until the hand lets go, and never
    // while a finger is merely passing through.
    if (at.touch) {
      document.addEventListener("touchmove", refuse, { passive: false });
    }
  };

  /// A press begins somewhere on a card. Which of the three things it is — a
  /// click, a scroll or a drag — is settled by what the hand does next.
  const grab = (event: PointerEvent, id: number) => {
    // Which hand this is, before anything is decided about the press: a
    // right-click leaves at the next line, and the `contextmenu` behind it is
    // the one thing that still needs to know.
    fromTouch = event.pointerType !== "mouse";

    // The primary button, a finger or a pen. A right-click is not a drag.
    if (event.button !== 0) return;

    // Every move from here reaches this card, whatever the pointer ends up
    // over — including the gap between two rows and the world outside the
    // sidebar.
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);

    const began: NonNullable<typeof press> = {
      id,
      x: event.clientX,
      y: event.clientY,
      touch: event.pointerType !== "mouse",
      lifted: false,
    };
    press = began;
    reordered = false;

    // A finger lifts a card by holding still. No distance tells a drag from a
    // scroll on a phone — both of them are the finger moving — so what tells
    // the two apart is the time before it does.
    if (began.touch) {
      began.waiting = setTimeout(() => {
        if (press === began) lift(began);
      }, LIFT);
    }
  };

  /// The hand moved: past the grace it is a drag, and a drag puts the lifted
  /// card where the pointer is.
  const drag = (event: PointerEvent) => {
    const at = press;
    if (!at) return;

    if (!at.lifted) {
      // Inside the grace the hand has not said anything yet: this is the wobble
      // between pressing a card and letting go of it, and a card that started
      // moving here is a card that could not be clicked at all.
      if (Math.hypot(event.clientX - at.x, event.clientY - at.y) <= GRACE) {
        return;
      }

      // A finger that travels before its card has lifted is scrolling the list,
      // so the press is over and the browser has it.
      if (at.touch) {
        clearTimeout(at.waiting);
        press = null;
        return;
      }

      lift(at);
    }

    const order = dragged();
    if (!order || !list) return;

    const to = order.indexOf(under(list, event.clientY));
    if (to < 0 || to === order.indexOf(at.id)) return;

    setDragged(moved(order, at.id, to));
  };

  /// The hand let go: what is on the screen is what the human meant, so that is
  /// what is sent.
  const drop = () => {
    const at = press;
    if (!at) return;

    clearTimeout(at.waiting);
    press = null;
    if (!at.lifted) return;

    document.removeEventListener("touchmove", refuse);
    reordered = true;
    setHeld(null);

    const order = dragged();
    if (order) place.mutate(order);
  };

  // A sidebar that goes away mid-drag takes its refusal of the scroll with it.
  // Nothing else would ever take it off the document again: the drop that would
  // have is on an element that is no longer there.
  onCleanup(() => document.removeEventListener("touchmove", refuse));

  /// A press that let go about where it landed is a click, and a click opens the
  /// Conversation. One that moved a card is not: the card is where they put it,
  /// and opening it as well would be answering one gesture twice.
  ///
  /// The flag is read once and spent, so the drag it belongs to swallows the
  /// click that follows it and nothing after that.
  const opened = (id: number) => {
    const dragged = reordered;
    reordered = false;

    if (dragged) return;
    props.open(id);
  };

  /// A right-click asks what there is to do about the Conversation the card
  /// stands for, which is the fourth thing a press on a card can be and the one
  /// the hand can only make with a mouse. The browser's own menu is not what it
  /// is asking for, so that goes.
  ///
  /// The card is not opened and the order is not touched: `grab` above takes
  /// the primary button and nothing else, so a right-click never begins a drag,
  /// and a right-click fires no click for `opened` to answer.
  ///
  /// A phone fires this from a long press, which is already how a card is picked
  /// up to be dragged — so a press that began under a finger is left entirely
  /// alone, the browser's own answer to it included. What tells the two apart is
  /// the pointer that started the gesture rather than this event, which carries
  /// nothing about the hand that made it.
  const ask = (event: MouseEvent, id: number) => {
    if (fromTouch) return;

    event.preventDefault();
    setPointed({ id, x: event.clientX, y: event.clientY });
  };

  /// And the same move made from the keyboard, which is the whole of what a card
  /// has to offer somebody who is not dragging anything: one row up, one row
  /// down, and the list saved each time as a drag saves it.
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
      {/* The mark rather than a title: this pane is where Verkstead is entered
          and the list under it says what it is a list of. The icon is served
          from `assets/`, which vite copies to the site root untouched, and it is
          cut from the same artwork the favicon is, at the size this draws it.

          No alt text on it, because the word it stands beside is the alt text: a
          screen reader that read both would say the name twice.

          The wordmark is the class the pane head is handed for its `<h1>`, and
          it is styled with the rest of what this pane draws — no way back
          either, this being the level every other pane is entered from. */}
      <PaneHead
        heading={styles.wordmark}
        title={
          <>
            <img src="/icons/icon-192.png" alt="" />
            Verkstead
          </>
        }
      >
        <WorkbenchActions />
      </PaneHead>

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
          <ul class={styles.conversationList} ref={list}>
            <For each={shown()}>
              {(entry) => (
                <ConversationRow
                  entry={entry}
                  selected={String(entry.id) === props.selected}
                  held={held() === entry.id}
                  open={opened}
                  grab={grab}
                  drag={drag}
                  drop={drop}
                  step={step}
                  ask={ask}
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

      {/* What a right-click on a card asks for. One for the whole list rather
          than one per row: it is drawn where the pointer was rather than where
          the card is, so there is nothing about it that belongs to a row — and
          the steer it can open outlives the menu, which a row being dragged
          about underneath it would not. */}
      <CardActions pointed={pointed()} close={() => setPointed(null)} />
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
///
/// Beside it, the one thing that is about this list rather than about the rest
/// of Verkstead: whether the conversations the human has archived are drawn in
/// it. A switch rather than a row that presses, because it is a state the list
/// is in rather than something to do to it — and it stays where it is put, so
/// the menu does not shut under a hand that may want it back.
function WorkbenchActions(): JSX.Element {
  const queries = useQueryClient();

  /// The server's answer rather than this device's: the choice is the human's,
  /// so a phone opened afterwards is looking at the same list.
  const showing = useReading(() => ({
    queryKey: ["conversations", "archived"],
    queryFn: showingArchived,

    // One boolean, so there is nothing in it to hold on to and nothing to
    // match up: what a re-read lands on is the whole payload either way.
    freshness: { reconcile: "id" } as const,
  }));

  const flip = useMutation(() => ({
    mutationFn: (on: boolean) => showArchived(on),
    onSuccess: () => {
      // The list itself and the switch over it: what is drawn changes with the
      // setting, which is the entire point of it. The other devices hear the
      // same news as a Nudge.
      void queries.invalidateQueries({ queryKey: ["conversations"] });
    },
  }));

  /// Where the switch stands: the position asked for while that is in flight,
  /// and the server's the rest of the time. A switch that snapped back to the
  /// old position for the length of a round trip would read as a press that
  /// failed.
  const on = (): boolean =>
    flip.isPending ? (flip.variables ?? false) : (showing.data ?? false);

  return (
    <Menu
      class={styles.workbenchActions!}
      label="Workbench actions"
      name="Workbench actions"
      mark
    >
      {() => (
        <>
          <div class={styles.showArchived}>
            <Toggle
              label="Show archived conversations"
              on={on()}
              disabled={showing.isPending || flip.isPending}
              flip={(wanted) => flip.mutate(wanted)}
            />
            <Show when={flip.isError}>
              <ErrorLine>
                The setting could not be saved: {flip.error?.message}
              </ErrorLine>
            </Show>
          </div>

          <A role="menuitem" href="/settings">
            Settings
          </A>
        </>
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
      class={styles.newConversation!}
      name="New conversation"
      closer={(close) => (shut = close)}
      opening={() => (taken = false)}
      trigger={
        <>
          New conversation
          {/* Which way the menu will go, and no part of what the button
              says. */}
          <span aria-hidden="true">
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
              <Empty class={styles.nothing}>
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
              class={styles.menuGroup}
              role="group"
              aria-labelledby="adopt-a-roadmap"
            >
              <p class={styles.menuHeading} id="adopt-a-roadmap">
                Adopt a roadmap
              </p>
              <For each={roadmaps()}>
                {(held) => (
                  <button
                    type="button"
                    role="menuitem"
                    class={styles.adoptRoadmap}
                    disabled={adopt.isPending}
                    onClick={() =>
                      adopt.mutate({
                        repoId: held.repoId,
                        roadmap: held.roadmap.name,
                      })
                    }
                  >
                    <span class={styles.what}>
                      <code>{held.roadmap.name}</code>
                      <span class={styles.in}>in {held.repo}</span>
                    </span>
                    <span class={styles.stage}>
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
            <ErrorLine class={styles.failure}>
              The conversation could not be started: {start.error?.message}
            </ErrorLine>
          </Show>
          <Show when={adopt.isError}>
            <ErrorLine class={styles.failure}>
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
/// The disc wins, and never both: a Conversation whose session is sitting on a
/// Blocking Ask is working *and* waiting, and of the two the one the human can
/// do something about is the ask. So the dot is what a card shows the moment
/// there is anything to answer, and a ring is what is left.
///
/// Two things draw that one disc, because they say the same thing to the person
/// glancing down the list: *look here*. One is something waiting on them; the
/// other is news they have not looked at yet — a wrap-up that carried the work
/// to Done while nobody was watching, which is stamped on the Conversation at
/// the moment the push goes out and comes off when they open it. Two marks for
/// one instruction would be a list to decode rather than one to glance at, so
/// which of the two it is is said in the label instead — see [`spoken`].
///
/// Which of the two rings it is says whether that session is doing anything: the
/// turning one while it prints, and the empty one once it has gone quiet — the
/// same pair the Timeline row and the details pane draw, so a card and the
/// session it stands for say the same thing. A grilling that has been sitting on
/// an ask for an hour turning a spinner is the case this is for, and the reason
/// the empty ring is the quieter mark of the two.
function mark(entry: ConversationEntry): "waiting" | "working" | "idle" | null {
  if (entry.waiting || entry.unseen) return "waiting";
  if (entry.working) return entry.idle ? "idle" : "working";
  return null;
}

/// What the disc says, for the two things that draw it.
///
/// Waiting first where both are true: a Conversation with something to answer
/// on it is asking for a reply, and one with news on it is only asking to be
/// read. The one the human can do something about is the one worth saying.
const DISC = {
  waiting: "waiting on you",
  unseen: "not looked at yet",
} as const;

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
///
/// And the disc is where the two reasons for it are told apart, because the
/// mark itself does not tell them apart: the words are [`DISC`], and waiting
/// wins where both are true for the reason [`mark`] gives.
///
/// And a wrap-up down to its checks is said in place of the state word rather
/// than beside it — *Waiting on checks* is what Wrapping has narrowed to, so
/// saying both would be saying it twice. The words are [`WAITING_ON_CHECKS`],
/// which the Timeline's own header draws from the same constant.
function spoken(entry: ConversationEntry): string {
  const which = mark(entry);
  const where = entry.waiting_on_checks ? WAITING_ON_CHECKS : STATE[entry.state];
  const said =
    which === "waiting"
      ? `${where}, ${entry.waiting ? DISC.waiting : DISC.unseen}`
      : which
        ? `${where}, ${SPOKEN[which]}`
        : where;

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
/// session running or an answer wanted. The mark is the whole of what a waiting
/// card says — the accent border and inset ring it used to carry as well are
/// gone, because a card that was both waiting and open had two edge treatments
/// arguing over one edge. Every other state is the ordinary card — grilling,
/// implementing and wrapping are not told apart here, because what the sidebar
/// is for is finding the Conversation to look at and all three are *this one is
/// under way*.
///
/// The card is also what is dragged to move the Conversation up the list. There
/// was a grip beside it until there was not: a second target to aim at, and one
/// that had to be aimed at, for something the card can carry itself. What tells
/// the gestures apart is now the hand rather than the place — see `grab`, `drag`
/// and `drop` above — and the arrow keys do from the keyboard what the grip's
/// did.
///
/// And it answers a right-click with what there is to do about the Conversation
/// — see `ask` above. A mouse's gesture and only a mouse's: a finger has no
/// right-click, and the long press it might have been is already how a card is
/// picked up.
function ConversationRow(props: {
  entry: ConversationEntry;
  selected: boolean;
  held: boolean;
  open: (id: number) => void;
  grab: (event: PointerEvent, id: number) => void;
  drag: (event: PointerEvent) => void;
  drop: () => void;
  step: (id: number, by: number) => void;
  ask: (event: MouseEvent, id: number) => void;
}): JSX.Element {
  const ended = (): boolean =>
    props.entry.state === "Done" || props.entry.state === "Closed";

  return (
    <li
      class={styles.conversationRow}
      // Read by the drag to say which row the pointer is over, which is a
      // question about the rendered list rather than about the data behind it.
      data-id={props.entry.id}
      classList={{
        [styles.selected!]: props.selected,
        [styles.draft!]: props.entry.state === "Draft",
        [styles.ended!]: ended(),
        [styles.held!]: props.held,
      }}
    >
      <button
        type="button"
        class={styles.open}
        aria-current={props.selected ? "true" : undefined}
        aria-label={spoken(props.entry)}
        // What the grip's own label used to say, now that there is no second
        // control to say it in: this card can be moved, and these are the keys
        // that move it.
        aria-keyshortcuts="ArrowUp ArrowDown"
        onClick={() => props.open(props.entry.id)}
        onPointerDown={(event) => props.grab(event, props.entry.id)}
        onPointerMove={props.drag}
        onPointerUp={props.drop}
        onPointerCancel={props.drop}
        onContextMenu={(event) => props.ask(event, props.entry.id)}
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
        <span class={styles.what}>
          <span class={styles.title}>{props.entry.branch}</span>
          <span class={styles.meta}>
            <span>{props.entry.repo}</span>
          </span>
        </span>
        {/* Drawn only where there is one, so a row with nothing to mark gives
            the whole width to its name. The label above has already said what
            it means, so there is nothing here for a screen reader to find. */}
        <Show when={mark(props.entry)}>
          {(which) => (
            <span class={`${marks.mark} ${marks[which()]}`} aria-hidden="true" />
          )}
        </Show>
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
  const rows = [
    ...list.querySelectorAll<HTMLElement>(`.${styles.conversationRow}`),
  ];
  const over =
    rows.find((row) => y < row.getBoundingClientRect().bottom) ?? rows.at(-1);

  return Number(over?.dataset.id ?? NaN);
}

/// How far a pointer may travel and still have been a click, in pixels. A press
/// is not a steady thing — a mouse moves a pixel or two between going down and
/// coming up — so a card that began moving at the first move would be a card
/// that could not be clicked at all.
const GRACE = 5;

/// How long a finger holds a card still before it lifts, in milliseconds. Long
/// enough that a swipe down the list is never taken for it, short enough that
/// holding a card is not waiting for it.
const LIFT = 400;

/// What a card being dragged does to the scroll under it: refuses it. Hung on
/// the document at the lift and taken off again at the drop, so a finger scrolls
/// the sidebar every other moment of the day.
///
/// A function of its own rather than one made per drag, because removing a
/// listener means handing back the very same function.
const refuse = (event: Event): void => event.preventDefault();
