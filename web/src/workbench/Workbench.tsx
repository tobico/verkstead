//! The workbench: the three panes everything about a piece of work is done in.
//!
//! Conversations down the left, the selected Conversation's Timeline in the
//! middle, and the details of what the Timeline cannot show on the right. The
//! frame they stand in is `Panes.tsx` — the grid, the dividers and the widths a
//! device remembers, shared with the settings page — and what this page has to
//! say about it is which level a narrow window is showing and what goes in each
//! of the three.
//!
//! Two of them, on a Conversation whose record is the one Event. There is
//! nothing to read on such a Timeline — one card, under a header saying what
//! the sidebar row beside it already says — so the middle pane is not handed to
//! the frame at all and the details take its column as well as their own. The
//! whole pane goes rather than most of it: no strip of what its header carried,
//! no pins, no status. What is left is the conversations and the composer, and
//! a narrow window walks between exactly those two — Back from the composer is
//! Back out of the Conversation, there being no level in between. A second
//! Event of any kind puts the Timeline back at the width this device left it.
//!
//! Which level it is follows the URL: naming a Conversation walks the page into
//! it, and walking back out to the list takes the name off again. One account of
//! where the page stands rather than two — left selected behind the list, the
//! card the human had just walked out of navigated to where the page already
//! was, so nothing changed and a phone could not get back into the Conversation
//! it had only just left.
//!
//! What is open is the URL's rather than this page's, because it is what the
//! third pane is *about*: the pane is that one thing's full self and nothing
//! else, so with nothing open it is bare paper. Nearly always that is an Event;
//! the backlog, the roadmap, the Share pane and the Terminal are the exceptions
//! — the two lists are read off the worktree rather than recorded, and sharing
//! and a shell in the Sandbox belong to the Conversation rather than to any
//! moment on it — and each names itself by a word instead of an id. Every one of them has a path of its own under the
//! Conversation — see `openings.ts` — so a details pane survives being navigated
//! away from and back, and can be linked to.
//!
//! Opening a Conversation lands on the end of its record: the last Event with a
//! pane behind it is selected and the URL is rewritten to its path, so the human
//! arrives at where the work got to rather than at the beginning of it. It
//! happens here rather than in the sidebar because the sidebar has no Timeline
//! to pick from, and only where the path names no pane already — a cold load of
//! a details pane keeps the selection it was opened at.
//!
//! And it stays there. A Timeline landed on the end of its record is advancing:
//! each Event that arrives with a pane behind it is opened as it lands, so a
//! human watching a session work is shown what it just did rather than what it
//! had done when they opened it. There is nothing on screen to say so — the
//! newest card is open and an open card looks like an open card — and nothing
//! to turn on either: arriving is the whole of it, whether that was a press on
//! the sidebar or the walk a Conversation is started with. Picking anything by
//! hand ends it, and the next arrival begins it again.
//!
//! What a Conversation *is* is not drawn there — the setup it needs is on the
//! Brief card, where it is used — and the way on to an empty pane is not
//! offered, so a narrow window can only walk into the pane by opening something.
//!
//! Which navigations push and which replace is the difference between a page and
//! a place in one. Entering a Conversation and leaving it push, because they are
//! the page changing; opening a details pane and switching between them replace,
//! because walking between the details of one Conversation is not a walk the
//! history stack should be growing with. So Back from a details pane leaves the
//! Conversation, which is where the human came in from.
//!
//! The Conversation itself is read once here and drawn in the two panes it is
//! read in, each of them keyed on its id so that switching between Conversations
//! builds those panes again rather than reading the second Conversation into the
//! first one's page.

import { useLocation, useNavigate, useParams } from "@solidjs/router";
import {
  Match,
  Show,
  Switch,
  createEffect,
  createMemo,
  createSignal,
  on,
  type JSX,
} from "solid-js";

import { Panes, type Pane } from "../Panes";
import { loadConversation, seeConversation } from "../api/client";
import type {
  AgentOutputEvent,
  BriefEvent,
  CommitEvent,
  ConversationView,
  HandoffEvent,
  NoticeEvent,
  SteerEvent,
  PullRequestEvent,
  QuestionSetEvent,
  UnreadableSetEvent,
} from "../api/types";
import { useReading } from "../freshness";
import { Empty, ErrorLine } from "../notices";
import { Asked } from "./Asked";
import { Backlog } from "./Backlog";
import { Brief } from "./Brief";
import { Commit } from "./Commit";
import { Composer, composing } from "./Composer";
import { Conversations } from "./Conversations";
import { Document } from "./Document";
import { Hatch } from "./Hatch";
import { Output } from "./Output";
import { PullRequest } from "./PullRequest";
import { Roadmap } from "./Roadmap";
import { Share } from "./Share";
import { Terminal } from "./Terminal";
import { Timeline } from "./Timeline";
import { pressed } from "./eager";
import {
  lastOpening,
  openingAt,
  pathOf,
  pathTo,
  roadmapOpened,
  type Opening,
} from "./openings";

/// The read the two panes share, as each of them is handed it: one query behind
/// both, because they are two views of the one Conversation.
///
/// The query's own four fields rather than the query, because what the panes
/// are handed is not quite the query's answer: whatever a press has already
/// said about this Conversation is laid over it on the way through — see
/// `eager.ts`, and [`Workbench`], where the two are put together.
type Read = {
  data: ConversationView | undefined;
  isPending: boolean;
  isError: boolean;
  error: Error | null;
};

/// An Event with a full self, as the details pane holds it: which kind, and the
/// Event itself.
type Opened =
  | { output: AgentOutputEvent }
  | { asked: QuestionSetEvent | UnreadableSetEvent }
  | { commit: CommitEvent }
  | { opened: PullRequestEvent }
  | { brief: BriefEvent }
  | { handoff: HandoffEvent }
  | { steer: SteerEvent }
  | { notice: NoticeEvent };

/// The Event inside, whichever kind it turned out to be — what they have in
/// common is the id the pane was opened by.
function which(
  open: Opened,
):
  | AgentOutputEvent
  | QuestionSetEvent
  | UnreadableSetEvent
  | CommitEvent
  | PullRequestEvent
  | BriefEvent
  | HandoffEvent
  | SteerEvent
  | NoticeEvent {
  if ("output" in open) {
    return open.output;
  }
  if ("asked" in open) {
    return open.asked;
  }
  if ("commit" in open) {
    return open.commit;
  }
  if ("brief" in open) {
    return open.brief;
  }
  if ("handoff" in open) {
    return open.handoff;
  }
  if ("steer" in open) {
    return open.steer;
  }
  return "notice" in open ? open.notice : open.opened;
}

/// And each kind on its own, for the pane that draws it: the Event where this is
/// one of that kind, and nothing where it is another.
function outputIn(open: Opened): AgentOutputEvent | undefined {
  return "output" in open ? open.output : undefined;
}

function setIn(open: Opened): QuestionSetEvent | UnreadableSetEvent | undefined {
  return "asked" in open ? open.asked : undefined;
}

function commitIn(open: Opened): CommitEvent | undefined {
  return "commit" in open ? open.commit : undefined;
}

function pullRequestIn(open: Opened): PullRequestEvent | undefined {
  return "opened" in open ? open.opened : undefined;
}

function briefIn(open: Opened): BriefEvent | undefined {
  return "brief" in open ? open.brief : undefined;
}

function handoffIn(open: Opened): HandoffEvent | undefined {
  return "handoff" in open ? open.handoff : undefined;
}

function steerIn(open: Opened): SteerEvent | undefined {
  return "steer" in open ? open.steer : undefined;
}

function noticeIn(open: Opened): NoticeEvent | undefined {
  return "notice" in open ? open.notice : undefined;
}

export function Workbench(): JSX.Element {
  const params = useParams();
  const navigate = useNavigate();
  const where = useLocation();

  const [pane, setPane] = createSignal<Pane>("conversations");

  /// Which Conversation the URL names, or the empty string on the bare
  /// workbench. Unparsed, like a Set's id: the server decides what names
  /// nothing.
  ///
  /// A memo rather than a read of the params, because what hangs off it is a
  /// key: it has to say nothing at all when the router has moved and the id has
  /// not.
  const selected = createMemo(() => params.id ?? "");

  /// And what the details pane is showing, where anything is open: a Timeline
  /// Event, or the backlog or a roadmap, neither of which has an Event to be
  /// named by — see [`Opening`].
  ///
  /// Derived from the path rather than held beside it, so there is one account
  /// of what is open. Nothing has to be closed when the Conversation changes
  /// either: a path names the Conversation and the detail together, so a new
  /// Conversation's path names no detail of the old one's.
  const event = createMemo(() => openingAt(where.pathname));

  /// The Conversation whose Timeline is advancing itself, or `null` where none
  /// is — see [`advancing`].
  const [followed, setFollowed] = createSignal<string | null>(null);

  /// Whether what is open moves on to each Event as it lands.
  ///
  /// A Timeline opened at the end of its record goes on standing there: the
  /// human asked to be shown where the work got to, and where the work got to
  /// keeps moving. It has nothing on screen to say so — the newest card is open
  /// and looks exactly as an open card looks — because it is the same answer as
  /// before, given again as often as the answer changes.
  ///
  /// Held as the Conversation it belongs to rather than as a flag, so that
  /// another Conversation is not advancing by the mere fact of being another
  /// one. A flag would have had to be put out when the selection changed, and
  /// putting it out is an effect racing the landing that sets it.
  const advancing = () => followed() !== null && followed() === selected();

  /// Opening a details pane, which is a navigation to where that pane stands.
  ///
  /// It replaces rather than pushes: the details of one Conversation are places
  /// in a page rather than pages, so walking between them should not have to be
  /// walked back out of one at a time. What Back leaves is the Conversation.
  ///
  /// And it is the human picking, which is the whole of what takes a Timeline
  /// out of advancing: every card on the record, every pinned card and the
  /// Share button open what they open through here, and each of them is a
  /// press. Somebody who has said what they want to be looking at does not have
  /// it taken off them by the next thing a session does. Nothing else selects —
  /// the landing below navigates for itself, so advancing cannot end its own
  /// mode by advancing.
  const select = (opening: Opening) => {
    setFollowed(null);
    navigate(pathTo(selected(), opening), { replace: true });
  };

  // Opening a Conversation is what walks a phone into the Timeline, and leaving
  // the workbench route walks it back out. Written as an effect on the URL
  // rather than done in the click handler, because Back is a way of changing the
  // selection too and it never goes through one.
  //
  // Straight to the details where the path names one, which is what a cold load
  // of a details pane is — a reload, or a link somebody kept. The URL says which
  // pane it is about, so that is the pane it opens on; the walk back out of it
  // is the pane's own "← Timeline".
  createEffect(
    on(selected, (id) => {
      setPane(
        id === "" ? "conversations" : event() === null ? "middle" : "details",
      );

      // And opening one is the human having looked at it, which takes the news
      // mark off its sidebar row — on every device, the mark being the
      // server's. Said here rather than in the card's own click handler
      // because Back, a typed URL and a reload all open a Conversation without
      // going anywhere near one, and a mark that only a click cleared would
      // outlive the reading it was about.
      //
      // Nothing waits on it and nothing is done about a failure: the mark is a
      // nudge to look rather than a record to keep, and the worst a lost call
      // costs is a dot that comes off the next time the Conversation is
      // opened.
      if (id !== "") {
        void seeConversation(id).catch(() => {});
      }
    }),
  );

  /// The selection as something to key on: a new object each time the id really
  /// changes, and the same one for as long as it does not.
  ///
  /// Both of the panes the Conversation is read in stand inside a `keyed` Show
  /// over it, so a switch tears them down and builds them again from nothing.
  /// The frame around them is not keyed and must not be: the conversations pane
  /// is the same list whichever row is picked. Without the key the switch was
  /// dropped, and dropped worst where it should have been cheapest: on a
  /// Conversation already read once, answered out of the cache. The query
  /// below has its payload merged into the store rather than put in its place,
  /// and reconcile exempts the root of a store from the key it is told to match
  /// by — so the second Conversation went into the first one's object, the
  /// object stayed the object it had always been, and with nothing to fetch
  /// there was not even a moment of loading to rebuild the page at. Everything
  /// the middle pane was holding went on standing over a Conversation that was
  /// no longer on screen: a Brief half typed into above all, which is the only
  /// copy of itself there is.
  ///
  /// The merge itself is right and stays. What it is for is a re-read of the
  /// Conversation already open, where keeping the rows is the whole point; it
  /// is only across a change of Conversation that it has nothing to say, and
  /// this is what says so.
  const open = createMemo(() => ({ id: selected() }));

  /// The Conversation the URL names, read once for the two panes that draw it:
  /// they are two views of the one thing, and a query apiece would be two reads
  /// of it. Out here rather than inside either, so that neither pane being
  /// built again is a re-read.
  const read = useReading(() => ({
    queryKey: ["conversation", selected()],
    queryFn: () => loadConversation(selected()),
    enabled: selected() !== "",

    // Nothing polls this. What a Timeline keeps up with is the Nudges about its
    // own Conversation — a Question Set arriving, a session's output growing,
    // a commit landing — and what stands behind a Nudge that never arrived is
    // the catch-up in `nudge.ts`: coming back to the page reads it whole
    // (ADR-0009).

    // Merge each read into the Conversation already drawn rather than replacing
    // it, so that an Event which did not change stays the same Event and the row
    // drawn for it is left alone.
    //
    // Solid Query turns the core's structural sharing off and offers this in its
    // place, and off is not a setting this page can live with: a talking session
    // has this re-read a second at a time over a Timeline that has mostly not
    // moved, and without this each read is a new object for every Event on it,
    // so `For` throws away every row and builds it again. What goes with the
    // rows is everything they were holding — the Brief being typed into above
    // all, which is a half-written document and the only copy of itself there
    // is.
    //
    // What actually matches the rows up is position, not the key named here. A
    // Timeline Event is `{"Brief": {…, "id": 4}}` on the wire, so its `id` sits
    // a level down where reconcile — which reads the key off the array element
    // itself — cannot see it, and elements without the key are matched by
    // index. That is sound for this array: Events are only ever appended, so
    // the prefix is stable and every row keeps its identity. The Transcript's
    // turns carry their `id` flat for exactly this reason.
    freshness: { reconcile: "id" },
  }));

  /// The same Conversation as the panes are handed it: the server's answer with
  /// whatever a press has already said about it laid over — a close drawn at
  /// the press rather than a round trip later. See `eager.ts`.
  ///
  /// A memo rather than a call per reader, because the panes are merged into
  /// rather than rebuilt: with nothing pressed this is the very object the query
  /// answered with, and while something is it is one object rather than a fresh
  /// one for every look.
  const overlaid = createMemo(() => {
    const answer = read.data;
    return answer === undefined ? undefined : pressed(answer);
  });

  /// Which the two panes read through, as they read the query itself before:
  /// getters, so that what they are reading is still the reading rather than a
  /// copy of one moment of it.
  const conversation: Read = {
    get data() {
      return overlaid();
    },
    get isPending() {
      return read.isPending;
    },
    get isError() {
      return read.isError;
    },
    get error() {
      return read.error;
    },
  };

  /// Whether this Conversation's record is the one Event, which is what takes
  /// the Timeline away.
  ///
  /// A record with nothing on it but the Brief has nothing to read: the pane
  /// beside the composer would be one card and a header saying what the sidebar
  /// row already said. So the middle pane is not handed to the frame at all —
  /// the whole of it, header and pins included, rather than a strip of what it
  /// held — and the details take the room. A second Event of any kind brings it
  /// back, at the width this device left it: the frame keeps that width
  /// untouched while the column is away (see `widths.ts`).
  ///
  /// Nothing while the read is still in flight, which is the three-pane frame:
  /// what a Conversation's record holds is not known until it has arrived, and
  /// the layout it does not yet call for is the ordinary one.
  const alone = createMemo(() => conversation.data?.timeline.length === 1);

  /// Which level a narrow window is showing, with the one it cannot be standing
  /// at taken off: a Timeline that is not drawn is not a level to walk through,
  /// in either direction. So opening such a Conversation lands on the composer,
  /// and the way out of it is the way out of the Conversation — see [`leaving`].
  const showing = (): Pane =>
    alone() && pane() === "middle" ? "details" : pane();

  /// The way off a details pane: where it goes, and what it is called.
  ///
  /// The Timeline ordinarily, that being the level the details were opened
  /// from — a change of level rather than a navigation, the Conversation being
  /// where the page still stands. Where the record is the one Event there is no
  /// Timeline to go back to, so the way out is out of the Conversation
  /// altogether, which is a navigation exactly as the Timeline's own "←
  /// Conversations" is.
  const leaving = () =>
    alone()
      ? { to: "Conversations", go: () => navigate("/") }
      : { to: "Timeline", go: () => setPane("middle") };

  // Arriving at a Conversation with nothing open lands on the end of its record:
  // the last Event that has a pane behind it, opened by rewriting the URL to its
  // path with replace. What somebody pressing a Conversation asked for is where
  // the work got to, and the end of the record is that answer — so it is shown
  // rather than left one press away.
  //
  // Done here rather than by the card, because the sidebar has no Timeline to
  // pick from: its list says a Conversation has moved and nothing about what
  // moved. So the card navigates to the Conversation as it always did, and this
  // finishes the walk once the record has arrived.
  //
  // Only where the path names no pane already. A URL that names one is a cold
  // load of that pane — a reload, or a link somebody kept — and it keeps its own
  // selection. Which is also what settles this: the navigation it makes names a
  // pane, so the very next run has nothing left to do.
  //
  // A record with nothing openable on it selects nothing and the pane stays bare
  // paper, which is a Draft with only the Brief being written.
  //
  // Which level a narrow window is showing is not touched, and that is the point:
  // it follows the Conversation changing, and this changes no Conversation. So a
  // phone lands on the Timeline with the newest thing marked open and the details
  // one tap away, rather than being carried past the record it was opened to
  // read.
  //
  // And landing there is what puts the Timeline into advancing: arriving at the
  // end of a record is arriving where the work got to, and a record that is
  // still being written moves on from there. See [`advancing`] and the effect
  // under this one.
  createEffect(() => {
    const id = selected();
    const read = conversation.data;

    if (id === "" || read === undefined || event() !== null) {
      return;
    }

    const last = lastOpening(read.timeline);
    if (last !== null) {
      navigate(pathTo(id, last), { replace: true });
      setFollowed(id);
    }
  });

  // A Timeline that is advancing stays at the end of its record: every Event
  // that lands with a pane behind it is opened as it arrives, the way the
  // landing opened the last one before it.
  //
  // The same walk as the landing's and made the same way — the URL rewritten to
  // where the newest pane stands, with replace — because what is open is the
  // URL's. It goes nowhere near [`select`]: that is the human picking, and this
  // is the record moving under a human who picked nothing.
  //
  // Which level a narrow window is showing is untouched here too. A phone left
  // on the Timeline stays on it while the cards under its finger open one after
  // another, and one left in the details pane is reading the newest thing there
  // — which is what advancing is for.
  createEffect(() => {
    const id = selected();
    const read = conversation.data;

    if (!advancing() || read === undefined) {
      return;
    }

    const last = lastOpening(read.timeline);
    if (last !== null && last !== event()) {
      navigate(pathTo(id, last), { replace: true });
    }
  });

  return (
    <Panes
      pane={showing()}
      middleLabel="Timeline"
      conversations={
        <Conversations
          selected={selected()}
          open={(id) => navigate(pathOf(id))}
        />
      }
      middle={
        // Nothing at all where the record is the one Event: the frame draws no
        // middle pane when it is handed none, and the details pane takes the
        // column it would have stood in. See [`alone`].
        alone() ? undefined : (
          <Show when={open()} keyed>
            <TimelinePane
              id={selected()}
              conversation={conversation}
              event={event()}
              select={select}
              pane={setPane}
              list={() => navigate("/")}
            />
          </Show>
        )
      }
      details={
        <Show when={open()} keyed>
          <DetailsPane
            conversation={conversation}
            event={event()}
            back={leaving()}
          />
        </Show>
      }
    />
  );
}

/// The middle pane: one Conversation's Timeline, or what is standing in the way
/// of reading it.
function TimelinePane(props: {
  /// The Conversation to read, or the empty string on the bare workbench.
  id: string;

  /// The read of it, which is the page's rather than this pane's: one query
  /// stands behind this pane and the details together.
  conversation: Read;

  /// What the details pane is showing, and how to change it.
  event: Opening | null;
  select: (opening: Opening) => void;

  /// Which level a narrow window is showing, which is the way on into the
  /// details pane.
  pane: (pane: Pane) => void;

  /// And the way back out to the list, which is a navigation rather than a
  /// change of level: what is being let go of is the selection, and the URL is
  /// where the selection is kept.
  list: () => void;
}): JSX.Element {
  return (
    <Switch>
      <Match when={props.id === ""}>
        {/* The resting state of the workbench, and what it says is the one
            thing there is to do from here. */}
        <Empty>Pick a conversation, or start one.</Empty>
      </Match>
      <Match when={props.conversation.isPending}>
        <Empty>Loading…</Empty>
      </Match>
      <Match when={props.conversation.isError}>
        {/* The reading failed, so there is no header to draw and no menu to
            hang off one — and this is the Conversation the human is most
            likely to want the end of. So the header is drawn anyway, in the
            little that can be known without the reading, and what it carries
            is the way out: see `Hatch.tsx`. */}
        <Hatch id={props.id} back={props.list} />
        <ErrorLine>
          Could not read this conversation: {props.conversation.error?.message}
        </ErrorLine>
      </Match>
      <Match when={props.conversation.data}>
        {(conversation) => (
          <Timeline
            conversation={conversation()}
            back={props.list}
            details={() => props.pane("details")}
            selected={props.event}
            select={props.select}
          />
        )}
      </Match>
    </Switch>
  );
}

/// The details pane: the full self of whatever the Timeline has open, and bare
/// paper where nothing is.
function DetailsPane(props: {
  /// The read of the Conversation, which is the one the pane beside this is
  /// drawn from.
  conversation: Read;

  /// What is open in it, if anything.
  event: Opening | null;

  /// The way off whatever pane is drawn, which a narrow window walks out
  /// through: where it goes and what it is called. Ordinarily the Timeline,
  /// which is a change of level; on a Conversation whose record is the one
  /// Event there is no Timeline drawn, so it is the conversations themselves
  /// and a navigation. Worked out once for the whole pane in `Workbench` above,
  /// because what decides it is what decided the frame.
  ///
  /// Where it goes reaches every pane; what it is called reaches the two a
  /// record of one Event can open — the composer and the frozen Brief — and the
  /// rest go on spelling *Timeline* for themselves, that being where they are
  /// always opened from. Nothing on a record of one Event opens them: the cards
  /// and the icon that do are the Timeline's, and the Timeline is not drawn.
  back: { to: string; go: () => void };
}): JSX.Element {
  /// The Event the details pane is showing, where it is one that has a full
  /// self to show. An id whose Event has gone leaves the pane empty, which is
  /// what it is when nothing is open at all.
  ///
  /// Seven kinds have one: a session's output, whose full self is its
  /// Capture; a Question Set, whose full self is the document it was asked
  /// as; a commit, whose full self is its diff; the pull request, whose full
  /// self is what is on it at GitHub right now; and the three documents — the
  /// Brief, the handoff and the instruction a steer carried — whose full self
  /// is the markdown their card shows three lines of. The kind travels with it,
  /// because it is what decides which pane is drawn.
  ///
  /// A Brief still being drafted is here too, and nothing ever selects it: the
  /// card is a field with the setup under it rather than a card to press, which
  /// is the Timeline's own rule about its own card. Saying it a second time here
  /// would be two rules to keep in step.
  ///
  /// The pull request is looked for among the pinned events rather than in the
  /// timeline, because that is where it is drawn: it is the one event that
  /// stays in view rather than scrolling past, and it opens all the same.
  ///
  /// The backlog, the roadmap, the Share pane and the Terminal are none of these
  /// and are not looked for here at all: none of the four has an Event — the two
  /// lists are read off the worktree every time the Conversation is, and sharing
  /// and a shell in the Sandbox belong to the Conversation rather than to any
  /// moment on it — so the pane draws them from the selection itself, see the
  /// `Switch` below.
  const opened = (conversation: ConversationView): Opened | undefined => {
    const id = props.event;

    return [
      ...conversation.timeline.map((entry): Opened | undefined => {
        if ("AgentOutput" in entry) {
          return { output: entry.AgentOutput };
        }
        if ("QuestionSet" in entry) {
          return { asked: entry.QuestionSet };
        }
        // The same pane, because it is the same Set reached the same way: what
        // comes back from the fetch is what says whether this build could read
        // the stored body.
        if ("UnreadableSet" in entry) {
          return { asked: entry.UnreadableSet };
        }
        if ("Commit" in entry) {
          return { commit: entry.Commit };
        }
        if ("Brief" in entry) {
          return { brief: entry.Brief };
        }
        if ("Handoff" in entry) {
          return { handoff: entry.Handoff };
        }
        // Only where it carries one. A steer into wrapping up or done says
        // nothing but the state, so there is no document under it to open —
        // which is why the Timeline draws one of those as a line rather than a
        // card.
        if ("Steer" in entry && entry.Steer.html !== null) {
          return { steer: entry.Steer };
        }
        if ("Notice" in entry) {
          return { notice: entry.Notice };
        }
        return undefined;
      }),
      ...conversation.pinned.map((pinned): Opened | undefined =>
        "PullRequest" in pinned ? { opened: pinned.PullRequest } : undefined,
      ),
    ].find((open) => open !== undefined && which(open).id === id);
  };

  // Nothing at all where nothing is open, which on a wide window is a blank
  // column beside the record and on a narrow one is a level there is no way in
  // to.
  return (
    <Show when={props.conversation.data}>
      {(conversation) => (
        <Switch>
          {/* The backlog, the roadmap, the Share pane and the Terminal, which
              are the four things this pane draws that are not Events: the two
              lists are read off the worktree every time the Conversation is,
              and sharing and a shell in the Sandbox belong to the Conversation
              rather than to anything on its record. So there is nothing on the
              record to name any of them by, and each is named by a word
              instead. Ahead of the Events because they are not among them —
              [`opened`] looks for an id, and none of the four selections is
              one. */}
          <Match when={props.event === "backlog"}>
            <Backlog
              conversation={conversation()}
              back={props.back.go}
            />
          </Match>
          {/* And sharing it, opened by the icon on the Timeline's header
              rather than by anything on the record. */}
          <Match when={props.event === "share"}>
            <Share
              conversation={conversation()}
              back={props.back.go}
            />
          </Match>
          {/* And the terminals it holds of its own, opened by the icon beside
              that one — a shell in the Conversation's Sandbox, which is no
              part of the record either (ADR 0013). */}
          <Match when={props.event === "terminal"}>
            <Terminal
              conversation={conversation()}
              back={props.back.go}
            />
          </Match>
          {/* And which roadmap, a worktree being allowed any number of
              them where it has one `.tasks/`. */}
          <Match when={roadmapOpened(props.event)}>
            {(name) => (
              <Roadmap
                conversation={conversation()}
                name={name()}
                back={props.back.go}
              />
            )}
          </Match>
          <Match when={opened(conversation())}>
            {(open) => (
              <Switch>
                <Match when={outputIn(open())}>
                  {(output) => (
                    <Output
                      conversation={conversation()}
                      output={output()}
                      back={props.back.go}
                    />
                  )}
                </Match>
                <Match when={setIn(open())}>
                  {(asked) => (
                    <Asked
                      asked={asked()}
                      back={props.back.go}
                    />
                  )}
                </Match>
                <Match when={commitIn(open())}>
                  {(commit) => (
                    <Commit
                      conversation={conversation()}
                      commit={commit()}
                      back={props.back.go}
                    />
                  )}
                </Match>
                <Match when={pullRequestIn(open())}>
                  {(opened) => (
                    <PullRequest
                      conversation={conversation()}
                      opened={opened()}
                      back={props.back.go}
                    />
                  )}
                </Match>
                {/* And the three documents, each the whole of what its
                    card showed three lines of. The handoff and the
                    instruction are one pane — rendered markdown under the
                    heading the card carries — and the Brief has two of its
                    own, because a Brief is a document only once its round is
                    past writing it.

                    While that round drafts, the pane is the composer: the
                    Brief as a field, the setup under it, and the press that
                    starts the work. Once the work has started from it, it is
                    the record of what that round was built from, with the
                    configuration frozen alongside it. */}
                <Match when={briefIn(open())}>
                  {(brief) => (
                    <Show
                      when={composing(conversation(), brief())}
                      fallback={
                        <Brief
                          conversation={conversation()}
                          brief={brief()}
                          back={props.back}
                        />
                      }
                    >
                      {/* The one pane that carries the whole of the way out
                          rather than the press behind it: it is the pane a
                          Conversation with nothing else on its record stands
                          on, so it is the one whose way out is sometimes the
                          conversations rather than a Timeline, and a button
                          that said Timeline while it left the Conversation
                          would be naming somewhere it does not go. */}
                      <Composer
                        conversation={conversation()}
                        brief={brief()}
                        back={props.back}
                      />
                    </Show>
                  )}
                </Match>
                <Match when={handoffIn(open())}>
                  {(handoff) => (
                    <Document
                      heading="Handoff"
                      html={handoff().html}
                      empty="The grilling wrote nothing down."
                      back={props.back.go}
                    />
                  )}
                </Match>
                {/* What a steer sent a session off with, read the way every
                    other document the human writes is read. Nothing opens a
                    steer that carried none — and what it is called follows
                    the target, an instruction being one session's whole job
                    and a follow-up's brief being what a conversation was
                    opened on. */}
                <Match when={steerIn(open())}>
                  {(steer) => (
                    <Document
                      heading={
                        steer().target === "FollowUp"
                          ? "Follow-up"
                          : "Instruction"
                      }
                      html={steer().html ?? ""}
                      empty="Nothing was asked for."
                      back={props.back.go}
                    />
                  )}
                </Match>
                {/* And what Verkstead said on its own account, which is a
                    document like the rest of them: the card shows the one
                    line that tells one notice from another, and the whole
                    of what a stop had to say — the reason and the terminal
                    output under it — is here. */}
                <Match when={noticeIn(open())}>
                  {(notice) => (
                    <Document
                      heading="Notice"
                      html={notice().html}
                      empty="Verkstead wrote nothing down."
                      back={props.back.go}
                    />
                  )}
                </Match>
              </Switch>
            )}
          </Match>
        </Switch>
      )}
    </Show>
  );
}
