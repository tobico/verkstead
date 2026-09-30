//! The conversations sidebar: what there is to work on, and the ways to add
//! to it.
//!
//! **One sidebar for the cluster.** The list is every device's Conversations
//! merged by **Rank** — this device's own and each member's, the merge made by
//! the device the browser opened out of the lists it holds of them (ADR-0020,
//! *The opened device relays*). So this pane is not this machine's work any
//! more, and nothing in it is addressed by a bare Conversation id: every
//! Verkstead issues a Conversation 1, so which row is selected, which one a
//! press was on, what the drag is holding and what the DOM carries are each a
//! Conversation *and* a device — see `rowKey`, and [`keyFor`], which is how this
//! pane says one. Opening a row goes to that device's own page, and the card's
//! own menu acts on the Conversation on the machine that holds it.
//!
//! There is one of them: the compose page — a link at the head of the pane, and
//! the composer with nothing behind it yet. The brief is written, the setup is
//! settled, and the Conversation is created by the press at the end of it. See
//! `Compose.tsx`.
//!
//! The roadmaps nothing is driving are reached the same way, from a dropdown
//! under that page's box rather than from anything here: adopting one is another
//! way work gets into the pipeline rather than something waiting on the human,
//! and it asks for the same setup in the same box. The menu that used to hold
//! both — a press, a repo, and a Conversation created before the human had
//! written a word — is gone, the page having taken over the last of what it
//! offered.
//!
//! The row's name is the branch. A Conversation has no title of its own, and of
//! what it does have the branch is the short line the human chose — and the one
//! they can change while it is still drafting. Until they do the name is one
//! Verkstead invented, and a draft still carrying one reads *Draft* rather than
//! a name that says nothing — see `naming.ts`.
//!
//! And under the name, which machine the work is on and then the Repo it is in:
//! the mark for that device's OS, its name, and the Repo after them. On every
//! row there is a device on, this device's own included, because that is what
//! makes this one list rather than this device's list with visitors on it —
//! and on no row at all where there is no cluster, a lone Verkstead drawing
//! the Repo alone the way it always has. A row whose device has stopped
//! answering is dimmed, which is the fade a finished Conversation wears, and
//! says *unreachable* where it is read aloud, which is where the two are told
//! apart.
//!
//! The order the rows are in is the human's own. This is one person's working
//! set, so which piece of work sits at the top is theirs to say rather than a
//! sort's — they say it by dragging a card, and what they said is the server's
//! to keep. So letting go of a card says where that one row landed — the row it
//! now sits under, or nothing at all for the top — and the list comes back from
//! the server on every read, which is what makes the order survive a reload, a
//! restart and a second device without any of the three being a case.
//!
//! One row rather than the whole list, because one row is what moved: the order
//! is a **Rank** per Conversation, and the key that says where this one sits is
//! minted on the server, between the ranks of the two rows it landed between.
//! Nothing here knows what a rank looks like. Both rows are named by device and
//! id all the same — a bare id names a row on no particular machine — and the
//! device the browser opened is the one that mints, holding every rank in the
//! cluster; what it does with the answer is write it to the device that owns the
//! row and to nobody else. See `ranking.rs`, and the error line under the list,
//! which is where a member that could not be told is named.
//!
//! A card also answers a right-click with what there is to do about the
//! Conversation it stands for — the same rows the status button at the head of
//! the Conversation pane offers, drawn by the same component and acting on
//! the card that was pressed rather than on whatever is open. Both menus are
//! `Actions.tsx`, which is where the rows and everything behind them live.
//!
//! The sidebar is also where the rest of Verkstead is reached from, because the
//! workbench has the root: the gear at the head of the pane opens the settings,
//! and the Repos and the Agent Profiles are in there rather than being a page
//! each to find.
//!
//! And it is drawn on that page as well as on this one, this being the app's
//! navigation rather than the workbench's furniture: the settings stand on the
//! same three panes with this pane down the left of them, so the list rides
//! along while a machine is being set up instead of being left behind by the
//! trip out to configure it. One component in both places, which is why the
//! gear asks the URL whether the settings are open rather than being told.
//!
//! And at the foot of the pane, under the list rather than over it, the one
//! setting that is about these conversations rather than about anything else:
//! whether the ones put away are drawn among them.
//!
//! Neither the head nor that foot is written here. Both are drawn on the compose
//! page as well while there is nothing to list — the pane is not drawn at all
//! then, and what stands in its place is entered the same way and offers the
//! same switch — so they are `Wordmark.tsx` and `Archived.tsx`, and this pane
//! puts them where they go. See `zero.ts` for what decides that this pane is
//! drawn at all.

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

import { CardButton } from "../CardButton";
import { Icon } from "../Icon";
import { PaneSticky } from "../Panes";
import { Truncated } from "../Truncated";
import {
  listConversations,
  rankConversation,
  showingArchived,
} from "../api/client";
import type { ConversationEntry, MergedRow } from "../api/types";
import { osIcon } from "../devices";
import { useReading } from "../freshness";
import { Empty, ErrorLine } from "../notices";
import { Reaching, rowKey, whose, type Device } from "../reaching";
import { CardActions } from "./Actions";
import { ShowArchived } from "./Archived";
import { caughtUp, pressedRows } from "./eager";
import styles from "./Conversations.module.css";
import { SPOKEN } from "./Mark";
// The rings and the badge a card carries at its right edge. Drawn here rather
// than by `Mark` because the sidebar has a state no running session has —
// something is waiting on you — but read out of the one module all the same,
// so a ring means the same thing in the list that it means on the row it
// opens.
import marks from "./Mark.module.css";
import { WAITING_ON_CHECKS, WAITING_TO_JOIN, parked } from "./conditions";
import { titled } from "./naming";
import { STATE } from "./states";
import { Wordmark } from "./Wordmark";

export function Conversations(props: {
  /// Which Conversation the page is open on, as the URL spells its id — and the
  /// empty string where it names none.
  selected: string;

  /// And which device that Conversation lives on, `null` for this one. Beside
  /// the id rather than folded into it because that is how the URL carries the
  /// pair: a row is selected when both agree, ids colliding by construction.
  device: Device;

  /// Open one, which is a Conversation *and* a device: the page it opens is
  /// that device's own — see `openings.ts`.
  open: (id: number, device: Device) => void;
}): JSX.Element {
  const conversations = useReading(() => ({
    queryKey: ["conversations"],
    queryFn: listConversations,

    // Merged by the Rank each row carries flat, because this list is re-read
    // constantly — a session talking moves the *working* badge on one row of
    // it — and a rebuilt row is a row whose spinner starts its animation again.
    //
    // The rank rather than the id, which is what it was while this list was
    // one device's: the list is merged from the whole cluster now and every
    // Verkstead issues a Conversation 1, so an id names a row on no particular
    // machine. A rank names one — every one of them carries the device that
    // issued it, so they are distinct cluster-wide by construction (ADR-0020,
    // *Ranks*). A row that has just been dragged is rebuilt for it, which is
    // the row the hand is on and the one the local order below is holding
    // steady anyway.
    freshness: { reconcile: "rank" },
  }));

  /// Whether the ones put away are drawn among them, read under the key the
  /// switch at the foot of the pane reads it under — so this is the answer
  /// already in hand rather than a second fetch of it.
  ///
  /// Here as well as there because it is what decides whether an archive
  /// pressed a moment ago takes a row off this list: the same rule the server's
  /// own list is filtered by, read the same way. See `eager.ts`.
  const archived = useReading(() => ({
    queryKey: ["conversations", "archived"],
    queryFn: showingArchived,
    freshness: { reconcile: "id" } as const,
  }));

  const queries = useQueryClient();

  // The order the human is making with their hand, until the server's own list
  // says the same thing. Null the rest of the time, which is every moment
  // nobody is dragging: the order is the server's fact and this is only ever
  // the half-second before it has heard about it.
  //
  // Rows rather than ids, because this list is the cluster's: every Verkstead
  // issues a Conversation 1, so what an order of ids held would be an order of
  // rows on no particular machine — see `rowKey`, which is what a row is here.
  const [dragged, setDragged] = createSignal<string[] | null>(null);

  // Which row is under the hand, or null when none is.
  const [held, setHeld] = createSignal<string | null>(null);

  // Which card was right-clicked and where the pointer was, or null while no
  // context menu is open. A signal because the menu is drawn from it, unlike
  // the press below.
  //
  // The device beside the id, because the menu acts on the Conversation the card
  // stands for and has to act on it *on its own device*: a menu that closed this
  // device's Conversation 4 because a member's row said 4 is the whole reason
  // every press in this pane is addressed this way.
  const [pointed, setPointed] = createSignal<{
    id: number;
    device: Device;
    x: number;
    y: number;
  } | null>(null);

  // And that device again, kept past the menu going.
  //
  // Every row of that menu shuts the menu and *then* posts — and one of them
  // waits for a card over the page to be answered first — so a value that went
  // with `pointed` above would read as this device's at the very moment the
  // press was made, on a Conversation of a member's. Overwritten by the next
  // right-click and never cleared, which is what `fromTouch` below is for the
  // same reason.
  const [pointedDevice, setPointedDevice] = createSignal<Device>(null);

  // The list to draw: the server's, with what a press has already said about a
  // Conversation laid over it — a row closed a moment ago, one taken off the
  // list, one put back on it — and in the order being dragged where there is
  // one. A Conversation that has appeared since the drag began is not in that
  // order and goes to the top, which is where a start ranks one on the server
  // too.
  const shown = (): ConversationEntry[] => {
    const rows = pressedRows(
      conversations.data ?? [],
      archived.data?.showing ?? false,
    );
    const order = dragged();
    if (!order) return rows;

    const placed = order
      .map((key) => rows.find((row) => keyFor(row) === key))
      .filter((row): row is ConversationEntry => row !== undefined);

    return [
      ...rows.filter((row) => !order.includes(keyFor(row))),
      ...placed,
    ];
  };

  const place = useMutation(() => ({
    mutationFn: (put: { row: MergedRow; below: MergedRow | null }) =>
      rankConversation(put.row, put.below),
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
      arrived.every((row, n) => keyFor(row) === order[n])
    ) {
      setDragged(null);
    }
  });

  // And let go of what a press said the same way, for the same reason: the read
  // behind a press comes back whether or not anything was read — a refetch that
  // failed is swallowed, and a query nothing is reading is not fetched at all —
  // so a press released on its request coming back would have the page take a
  // close back the first time either happened. Released on this list having
  // answered since instead, which is what `dataUpdatedAt` is. See `eager.ts`.
  createEffect(() => caughtUp(conversations.dataUpdatedAt));

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
    /// Which row, as this pane spells one — see `rowKey`.
    row: string;
    /// And the Conversation it stands for, which is what the save names.
    id: number;
    device: Device;
    pointer: number;
    x: number;
    y: number;
    touch: boolean;
    lifted: boolean;
    waiting?: ReturnType<typeof setTimeout>;
  } | null = null;

  // What takes the drag's listeners back off the window, or null while nothing
  // is pressed. They are made per press — each of them closes over the press it
  // belongs to — so what removes them is made alongside them.
  let stop: (() => void) | null = null;

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
    setHeld(at.row);
    setDragged(shown().map(keyFor));

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
  const grab = (event: PointerEvent, entry: ConversationEntry) => {
    // Which hand this is, before anything is decided about the press: a
    // right-click leaves at the next line, and the `contextmenu` behind it is
    // the one thing that still needs to know.
    fromTouch = event.pointerType !== "mouse";

    // The primary button, a finger or a pen. A right-click is not a drag.
    if (event.button !== 0) return;

    // A press whose ending never reached us is over the moment another begins.
    // Nothing should get this far with one still in flight — every way a drag
    // can end is listened for below — and one left standing would be a list
    // held by a hand that is no longer on it.
    drop();

    // The card takes the pointer for as long as the browser will leave it
    // there, so nothing it is carried over lights up under a hand that is
    // already holding something. For as long as it will leave it and no longer:
    // the list moving this very card is what the drag is for, and a card that
    // moves in the DOM has the pointer taken back off it. So the capture is a
    // courtesy that can go at any moment rather than the thing the drag runs
    // on — what the drag runs on is the window, which hears the pointer
    // whoever is holding it.
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);

    const began: NonNullable<typeof press> = {
      row: keyFor(entry),
      id: entry.id,
      device: whose(entry),
      pointer: event.pointerId,
      x: event.clientX,
      y: event.clientY,
      touch: event.pointerType !== "mouse",
      lifted: false,
    };
    press = began;
    reordered = false;

    // The rest of the gesture is watched at the window rather than at the card,
    // which is what the pane dividers next door do: a pointer that has outrun
    // the card is still dragging it, and a release out beyond the sidebar — or
    // beyond the window — is still the release. A cancel is an ending too,
    // being what the browser says when it has taken the gesture over. Both of
    // them put the card down, so there is no way for a drag to end that leaves
    // the list held.
    //
    // The capture going is not an ending, whatever it looks like: the first row
    // the drag moves takes the capture with it, so a drag that ended there
    // would be a drag of exactly one place — press, one row, and then nothing
    // until the hand let go and took the card again.
    const moved = (at: PointerEvent) => {
      if (at.pointerId === began.pointer) drag(at);
    };
    const ended = (at: PointerEvent) => {
      if (at.pointerId === began.pointer) drop();
    };

    stop = () => {
      window.removeEventListener("pointermove", moved);
      window.removeEventListener("pointerup", ended);
      window.removeEventListener("pointercancel", ended);
    };

    window.addEventListener("pointermove", moved);
    window.addEventListener("pointerup", ended);
    window.addEventListener("pointercancel", ended);

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
      // so the press is over and the browser has it. Nothing has lifted, so
      // ending it here moves nothing and sends nothing.
      if (at.touch) {
        drop();
        return;
      }

      lift(at);
    }

    const order = dragged();
    if (!order || !list) return;

    const to = order.indexOf(under(list, event.clientY));
    if (to < 0 || to === order.indexOf(at.row)) return;

    setDragged(moved(order, at.row, to));
  };

  /// The drag is over: what is on the screen is what the human meant, so that
  /// is what is sent.
  ///
  /// Every ending comes through here — the release, a cancel, a press that
  /// turned out to be a scroll, and the next press finding this one still
  /// standing — so there is one place the listeners come off and one place the
  /// held card is put down.
  const drop = () => {
    const at = press;

    stop?.();
    stop = null;
    press = null;
    if (!at) return;

    clearTimeout(at.waiting);
    if (!at.lifted) return;

    document.removeEventListener("touchmove", refuse);
    reordered = true;
    setHeld(null);

    const order = dragged();
    if (order) {
      place.mutate({
        row: { device: at.device, id: at.id },
        below: sitsUnder(order, shown(), at.row),
      });
    }
  };

  // A sidebar that goes away mid-drag takes the whole drag with it: the
  // listeners it hung on the window, and its refusal of the scroll. Nothing
  // else would ever take those off again — what would have is a drop that is
  // never coming.
  onCleanup(() => {
    stop?.();
    document.removeEventListener("touchmove", refuse);
  });

  /// A press that let go about where it landed is a click, and a click opens the
  /// Conversation. One that moved a card is not: the card is where they put it,
  /// and opening it as well would be answering one gesture twice.
  ///
  /// The flag is read once and spent, so the drag it belongs to swallows the
  /// click that follows it and nothing after that.
  const opened = (entry: ConversationEntry) => {
    const dragged = reordered;
    reordered = false;

    if (dragged) return;
    props.open(entry.id, whose(entry));
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
  const ask = (event: MouseEvent, entry: ConversationEntry) => {
    if (fromTouch) return;

    event.preventDefault();
    setPointedDevice(whose(entry));
    setPointed({
      id: entry.id,
      device: whose(entry),
      x: event.clientX,
      y: event.clientY,
    });
  };

  /// And the same move made from the keyboard, which is the whole of what a card
  /// has to offer somebody who is not dragging anything: one row up, one row
  /// down, and the row saved each time as a drag saves it.
  const step = (entry: ConversationEntry, by: number) => {
    const key = keyFor(entry);
    const order = shown().map(keyFor);
    const from = order.indexOf(key);
    const to = from + by;
    if (from < 0 || to < 0 || to >= order.length) return;

    const put = moved(order, key, to);
    setDragged(put);
    place.mutate({
      row: { device: whose(entry), id: entry.id },
      below: sitsUnder(put, shown(), key),
    });
  };

  return (
    <>
      {/* The mark rather than a title: this pane is where Verkstead is entered
          and the list under it says what it is a list of. Drawn by `Wordmark`
          rather than here, because the compose page standing without this pane
          is entered at the same head — see `zero.ts`. */}
      <PaneSticky>
        <Wordmark />
      </PaneSticky>

      {/* The one way work gets into the pipeline from here, and the whole of
          what this pane offers beyond the list: the compose page, where a
          Conversation is written before it exists — the brief, the setup and the
          roadmaps there are to adopt all being questions asked in the one box.
          A link rather than a button because it is a page: it opens in a new tab
          if somebody asks it to, and Back leaves it. */}
      <A class={styles.compose} href="/compose">
        New conversation
      </A>

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
            {/* What keeps a row's own element across a re-read is the merge the
                reading is made with, which matches by the Rank each row carries
                — the one field that is nobody else's on a list merged from the
                whole cluster. See the reading above. */}
            <For each={shown()}>
              {(entry) => (
                <ConversationRow
                  entry={entry}
                  selected={
                    String(entry.id) === props.selected &&
                    whose(entry) === props.device
                  }
                  held={held() === keyFor(entry)}
                  open={opened}
                  grab={grab}
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
          the cards it can open outlive the menu, which a row being dragged
          about underneath it would not.

          Under a provider naming the card's own device, because every row of
          that menu acts on the Conversation the card stands for and has to act
          on it on the machine that holds it: the rows read the device the way
          every other pane does, off the context around them (see
          `reaching.ts`), and the card that was pressed is what says which one
          this is — held apart from `pointed` because a press shuts the menu
          before it posts. */}
      <Reaching.Provider value={pointedDevice}>
        <CardActions pointed={pointed()} close={() => setPointed(null)} />
      </Reaching.Provider>

      {/* And the foot of the pane, under everything the pane is a list of. Last
          in the column so that the room left over is left over its head: that
          is what stands it against the bottom of the screen while the list is
          short, and what leaves it after the last card once the list is long
          enough to scroll. */}
      <ShowArchived />
    </>
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

/// And what a row whose device has stopped answering says after the name of it.
///
/// The Devices section's own word for the same finding, said in the same
/// voice: one dial worked down that machine's addresses and reached none of
/// them, which is a lid shut rather than an error.
const UNREACHABLE = "unreachable";

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
///
/// And a stage waiting to join is said the same way one state earlier, for the
/// same reason: what Implementing has come down to is the chain below it, and the
/// words are [`WAITING_TO_JOIN`].
///
/// And a session the Rescue is watching sit there is said *beside* that word
/// rather than in place of it: *idle 4 min, spoken to once* is something true
/// of a run that is still Implementing. The words are [`parked`], which the
/// card this row opens says the same condition in — and this is the only place
/// the row says it at all, the ring that marks the quiet being nothing to
/// anybody reading by ear.
///
/// And the device the work is on, where the row draws one: the card says which
/// machine in a mark and a name beside the Repo, and of those the mark is
/// nothing to anybody reading by ear — so the device belongs in this sentence
/// for the reason everything else in it does, in the place the row draws it.
///
/// And *unreachable* beside it where that machine has stopped answering, which
/// is the other half of what the row says in a fade: the dimming is the same
/// one a finished Conversation wears, and this is where the two are told
/// apart.
///
/// And a Conversation nobody has named is called a Draft, which is the word its
/// state is said in as well — so where the name and the state are the one word
/// it is said once rather than twice over. Whatever is drawn on the card is
/// what opens this label, which is the whole of what agreeing means here.
function spoken(entry: ConversationEntry): string {
  const which = mark(entry);
  const where = entry.waiting_on_checks
    ? WAITING_ON_CHECKS
    : entry.waiting_to_join
      ? WAITING_TO_JOIN
      : STATE[entry.state];
  const name = titled(entry);
  const marked =
    which === "waiting"
      ? entry.waiting
        ? DISC.waiting
        : DISC.unseen
      : which
        ? SPOKEN[which]
        : null;

  return [
    name,
    entry.device?.name ?? null,
    entry.device && !entry.device.reachable ? UNREACHABLE : null,
    entry.repo,
    name === where ? null : where,
    entry.parked ? parked(entry.parked) : null,
    marked,
  ]
    .filter((part) => part !== null)
    .join(", ");
}

/// One Conversation: the branch it will be done on, the Repo it is in, and where
/// it has got to.
///
/// A `CardButton`, which is the card every pressable thing in the app is: the
/// surface, the pointer, and the fill that says this is the one whose pane is
/// open, are that component's, and what is here is what stands on it. A button
/// rather than a link, because the whole workbench is one page: opening a
/// Conversation moves the panes rather than going somewhere, and the URL that
/// follows is a record of what is open rather than a document to fetch.
///
/// Where it has got to is drawn rather than written: an italic name is a draft,
/// a dimmed card is work that has stopped, and the mark at the right edge is a
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
/// The press is the whole of what the card hears about. Where the hand goes
/// after it and where it lets go are the window's to say — a pointer that has
/// outrun the card is still dragging it — so `grab` is the only handler here.
///
/// And it answers a right-click with what there is to do about the Conversation
/// — see `ask` above. A mouse's gesture and only a mouse's: a finger has no
/// right-click, and the long press it might have been is already how a card is
/// picked up.
function ConversationRow(props: {
  entry: ConversationEntry;
  selected: boolean;
  held: boolean;
  open: (entry: ConversationEntry) => void;
  grab: (event: PointerEvent, entry: ConversationEntry) => void;
  step: (entry: ConversationEntry, by: number) => void;
  ask: (event: MouseEvent, entry: ConversationEntry) => void;
}): JSX.Element {
  const ended = (): boolean =>
    props.entry.state === "Done" || props.entry.state === "Closed";

  /// And whether the machine this row's work is on has stopped answering,
  /// which the row wears the same dimming for.
  ///
  /// The same treatment for a different reason: what the fade says on a
  /// finished Conversation is that there is nothing here to do, and what it
  /// says here is that there is nothing here that can be done — a press on
  /// this row is refused by the Relay, by name. Which of the two it is is said
  /// in the label read aloud, the fade itself being nothing to a screen
  /// reader. The row keeps everything it had: it is the last list this device
  /// held of that member, and none of it has stopped being true.
  const away = (): boolean => props.entry.device?.reachable === false;

  return (
    <li
      class={styles.conversationRow}
      // Read by the drag to say which row the pointer is over, which is a
      // question about the rendered list rather than about the data behind it.
      //
      // The row rather than the Conversation's id: the list is merged from the
      // whole cluster, so a number read back off the DOM would name a row on no
      // particular machine — see `rowKey`.
      data-row={keyFor(props.entry)}
      classList={{
        [styles.selected!]: props.selected,
        [styles.draft!]: props.entry.state === "Draft",
        [styles.ended!]: ended(),
        [styles.unreachable!]: away(),
        [styles.held!]: props.held,
      }}
    >
      <CardButton
        class={styles.open}
        open={props.selected}
        press={() => props.open(props.entry)}
        aria-label={spoken(props.entry)}
        // What the grip's own label used to say, now that there is no second
        // control to say it in: this card can be moved, and these are the keys
        // that move it.
        aria-keyshortcuts="ArrowUp ArrowDown"
        onPointerDown={(event) => props.grab(event, props.entry)}
        onContextMenu={(event) => props.ask(event, props.entry)}
        keys={(event) => {
          if (event.key === "ArrowUp") {
            event.preventDefault();
            props.step(props.entry, -1);
          } else if (event.key === "ArrowDown") {
            event.preventDefault();
            props.step(props.entry, 1);
          }
        }}
      >
        <span class={styles.what}>
          {/* Held to one line and cut at the front where it does not fit, with
              the whole of it under the pointer — the pane header this card
              opens draws the same name the same way, and the two are the one
              name said twice. Nothing here for a screen reader: the label on
              the card above already says the whole sentence. */}
          <Truncated class={styles.title} text={titled(props.entry)} />
          <span class={styles.meta}>
            {/* Which machine the work is on, ahead of the Repo it is in and
                drawn on every row there is one on — this device's own
                included, which is what makes this one list rather than this
                device's list with visitors on it. Nothing at all where there
                is no cluster: the row carries no device then, and a Verkstead
                linked to nothing draws the line it always drew.

                The mark is the one a device wears wherever it is drawn — see
                `devices.ts` — so a WSL wears the Linux mark here exactly as it
                does on the Devices section, and a word this build has no mark
                for still draws a row. Unlabelled, like the mark at the right
                edge: the label on the card above has already said the whole
                sentence. */}
            <Show when={props.entry.device}>
              {(device) => (
                <span class={styles.device}>
                  <Icon of={osIcon(device().os)} class={styles.os} />
                  {device().name}
                </span>
              )}
            </Show>
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
      </CardButton>
    </li>
  );
}

/// What a row is to everything in this pane that holds one: the Conversation
/// and the device it lives on, as the one string `rowKey` writes.
///
/// Named here as well as imported so that the whole pane says *the row* in one
/// word: the order a drag is holding, the card under the hand, the key the DOM
/// carries and the entry a press is looked up under are the one thing.
function keyFor(entry: ConversationEntry): string {
  return rowKey(whose(entry), entry.id);
}

/// The same list with one row moved to a place in it, which is the whole of
/// what a drag and an arrow key each do.
function moved(order: string[], row: string, to: number): string[] {
  const put = [...order];
  put.splice(put.indexOf(row), 1);
  put.splice(to, 0, row);
  return put;
}

/// The row a moved one now sits directly under, or `null` where it has landed
/// at the top of the list — which is the whole of what the server is told about
/// a move.
///
/// The row above rather than the row below, because the top of the list is the
/// one end with nothing to name: a card dropped at the foot still has a row
/// above it.
///
/// **Whichever device it belongs to**, named by that device and its id: the list
/// is one list, so the row a card landed under is the row a card landed under.
/// The device the browser opened holds every rank in the cluster — its own and
/// each member's — so it mints the key between the two merged neighbours itself
/// and hands the answer to whichever device owns the row that moved (see
/// `ranking.rs`). Nothing here has to find a neighbour the owner would recognise.
function sitsUnder(
  order: string[],
  rows: ConversationEntry[],
  row: string,
): MergedRow | null {
  const at = order.indexOf(row);

  for (let above = at - 1; above >= 0; above -= 1) {
    const entry = rows.find((one) => keyFor(one) === order[above]);

    if (entry !== undefined) return { device: whose(entry), id: entry.id };
  }

  return null;
}

/// Which row the pointer is over: the first whose bottom edge is below it, and
/// the last row when it is below all of them.
///
/// By the rendered rows rather than by arithmetic over a row height, because a
/// row height is not a constant this is allowed to assume — the repo line under
/// a name wraps, and a row height written down here would go stale the first
/// time anything in a card changed — and a drag that guessed would put the row
/// somewhere the human was not pointing.
function under(list: HTMLUListElement, y: number): string {
  const rows = [
    ...list.querySelectorAll<HTMLElement>(`.${styles.conversationRow}`),
  ];
  const over =
    rows.find((row) => y < row.getBoundingClientRect().bottom) ?? rows.at(-1);

  return over?.dataset.row ?? "";
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
