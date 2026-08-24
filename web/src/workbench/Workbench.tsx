//! The workbench: the three panes everything about a piece of work is done in.
//!
//! Conversations down the left, the selected Conversation's Timeline in the
//! middle, and the details of what the Timeline cannot show on the right. On a
//! window wide enough they stand side by side; on a phone they are one pane at a
//! time, and the same page answers both — this is a hierarchy the human walks
//! into and back out of, and a phone simply shows one level of it.
//!
//! Which level is showing is `data-pane` on the frame, and the stylesheet is
//! what makes it mean anything: a wide window ignores it and draws all three.
//! The attribute rather than a rendered-or-not pane, because walking back out
//! should not throw away what the pane it came from had drawn.
//!
//! How wide the panes stand is held here too, for the same reason: the widths
//! are a property of the frame rather than of anything drawn in it. They are
//! percentages of the workbench kept per device (`panes.ts`), and the dividers
//! that set them exist only in the layouts that stand panes side by side —
//! below that breakpoint the page is walked through one pane at a time, so
//! there is no border to drag and nothing remembered is read.
//!
//! Which Event is open is held here rather than in the Timeline, because it is
//! what the third pane is *about*: the pane is that Event's full self and
//! nothing else, so with none open it is bare paper. What a Conversation *is* is
//! not drawn there — the setup it needs is on the Brief card, where it is used —
//! and the way on to an empty pane is not offered, so a narrow window can only
//! walk into the pane by opening something. The selection is not in the URL —
//! an Event opened is a place in a page rather than a page, and a Conversation
//! whose Timeline has moved on is not one to restore a scroll position into.
//!
//! What is *not* held here is the Conversation itself. Reading one, and drawing
//! the two panes it is read in, is `Reading` below — keyed on the id, so that
//! switching between Conversations builds those panes again rather than reading
//! the second Conversation into the first one's page.

import { useNavigate, useParams } from "@solidjs/router";
import {
  Match,
  Show,
  Switch,
  createEffect,
  createMemo,
  createSignal,
  on,
  onCleanup,
  type Accessor,
  type JSX,
} from "solid-js";

import { loadConversation } from "../api/client";
import type {
  AgentOutputEvent,
  BriefEvent,
  CommitEvent,
  ConversationView,
  HandoffEvent,
  ManualTaskEvent,
  PullRequestEvent,
  QuestionSetEvent,
} from "../api/types";
import { useReading } from "../freshness";
import { Asked } from "./Asked";
import { Commit } from "./Commit";
import { Conversations } from "./Conversations";
import { Document } from "./Document";
import { Output } from "./Output";
import { PullRequest } from "./PullRequest";
import { Timeline } from "./Timeline";
import {
  ALL_THREE,
  BESIDE,
  DEFAULTS,
  clamped,
  dragged,
  nudged,
  range,
  remember,
  restore,
  widths as remembered,
  type Divider,
  type Widths,
} from "./panes";

/// Which level of the hierarchy a narrow window is showing.
export type Pane = "conversations" | "timeline" | "details";

/// An Event with a full self, as the details pane holds it: which kind, and the
/// Event itself.
type Opened =
  | { output: AgentOutputEvent }
  | { asked: QuestionSetEvent }
  | { commit: CommitEvent }
  | { opened: PullRequestEvent }
  | { brief: BriefEvent }
  | { handoff: HandoffEvent }
  | { manual: ManualTaskEvent };

/// The Event inside, whichever kind it turned out to be — what they have in
/// common is the id the pane was opened by.
function which(
  open: Opened,
):
  | AgentOutputEvent
  | QuestionSetEvent
  | CommitEvent
  | PullRequestEvent
  | BriefEvent
  | HandoffEvent
  | ManualTaskEvent {
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
  return "manual" in open ? open.manual : open.opened;
}

/// And each kind on its own, for the pane that draws it: the Event where this is
/// one of that kind, and nothing where it is another.
function outputIn(open: Opened): AgentOutputEvent | undefined {
  return "output" in open ? open.output : undefined;
}

function setIn(open: Opened): QuestionSetEvent | undefined {
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

function manualIn(open: Opened): ManualTaskEvent | undefined {
  return "manual" in open ? open.manual : undefined;
}

/// Whether a media query holds, as something the page can be built out of.
///
/// The stylesheet answers the same question for itself; this is for the parts
/// of the layout that are the page's rather than the rules' — which dividers
/// exist, and whether this device's remembered widths are read at all. A
/// browser with no `matchMedia` to ask answers no to everything, which is the
/// narrow layout: one pane, no dividers, nothing read.
function matching(query: string): Accessor<boolean> {
  if (typeof window.matchMedia !== "function") {
    return () => false;
  }

  const asked = window.matchMedia(query);
  const [holds, setHolds] = createSignal(asked.matches);
  const changed = () => setHolds(asked.matches);

  asked.addEventListener?.("change", changed);
  onCleanup(() => asked.removeEventListener?.("change", changed));

  return holds;
}

/// The line between two panes, and the handle that moves it.
///
/// A separator rather than a button, because that is what it is: the thing it
/// does to the page is not an action but a value, and the value is the share of
/// the workbench the pane on its left is worth. Which is what the arrow keys
/// move it by, for the pointer nobody dragging with a keyboard has.
function Divider(props: {
  divider: Divider;
  label: string;
  share: number;
  travel: { least: number; most: number };
  drag: (divider: Divider, event: PointerEvent) => void;
  nudge: (divider: Divider, by: number) => void;
  restore: () => void;
}): JSX.Element {
  return (
    <div
      class="pane-divider"
      role="separator"
      aria-orientation="vertical"
      aria-label={props.label}
      aria-valuenow={Math.round(props.share)}
      aria-valuemin={Math.round(props.travel.least)}
      aria-valuemax={Math.round(props.travel.most)}
      tabindex="0"
      onPointerDown={(event) => props.drag(props.divider, event)}
      onDblClick={() => props.restore()}
      onKeyDown={(event) => {
        if (event.key === "ArrowLeft") {
          event.preventDefault();
          props.nudge(props.divider, -1);
        }
        if (event.key === "ArrowRight") {
          event.preventDefault();
          props.nudge(props.divider, 1);
        }
      }}
    />
  );
}

export function Workbench(): JSX.Element {
  const params = useParams();
  const navigate = useNavigate();

  const [pane, setPane] = createSignal<Pane>("conversations");

  /// Which Timeline Event the details pane is showing, where one is open.
  const [event, setEvent] = createSignal<number | null>(null);

  /// Which layout is standing, which decides how many dividers there are and
  /// how much room each pane is allowed to leave the others.
  const beside = matching(BESIDE);
  const allThree = matching(ALL_THREE);

  /// How wide the panes stand. Read off this device once, and written back when
  /// a drag is let go of rather than on the way — a width settled on is worth
  /// remembering, and the hundred it passed through on the way there are not.
  const [settled, setSettled] = createSignal<Widths>(remembered());

  /// And as they may actually be drawn: a sidebar dragged wide in the two-pane
  /// layout is not allowed to squeeze the timeline out of the three-pane one,
  /// so the minimums are met against the layout in front of the human rather
  /// than against the one the width was settled in.
  const shown = () => clamped(settled(), allThree());

  /// The frame the shares are shares *of* — a divider dropped at a point on the
  /// screen means nothing until it is measured against this.
  let frame!: HTMLDivElement;

  /// Dragging one. The listeners go on the window rather than on the handle,
  /// because a pointer that has outrun the handle — which every drag's does —
  /// is still dragging it.
  const drag = (divider: Divider, event: PointerEvent) => {
    // Which stops the drag selecting the text of both panes on the way past.
    event.preventDefault();

    const frameRect = frame.getBoundingClientRect();
    if (frameRect.width === 0) {
      return;
    }

    const moved = (at: PointerEvent) => {
      const share = ((at.clientX - frameRect.left) / frameRect.width) * 100;
      setSettled((was) => dragged(was, divider, share, allThree()));
    };

    const dropped = () => {
      window.removeEventListener("pointermove", moved);
      window.removeEventListener("pointerup", dropped);
      window.removeEventListener("pointercancel", dropped);
      remember(settled());
    };

    window.addEventListener("pointermove", moved);
    window.addEventListener("pointerup", dropped);
    window.addEventListener("pointercancel", dropped);
  };

  /// Moving one with the keyboard, which settles at once: there is no letting
  /// go of an arrow key.
  const nudge = (divider: Divider, by: number) => {
    setSettled((was) => nudged(was, divider, by, allThree()));
    remember(settled());
  };

  /// And putting them back, which is what a double-click on either divider
  /// does: both widths, because what it restores is the defaults rather than
  /// one of them.
  const defaults = () => {
    restore();
    setSettled(DEFAULTS);
  };

  /// What the frame carries the widths as. Only where a layout stands panes
  /// side by side: below that the page is walked through one pane at a time,
  /// and what this device remembers about a desktop's columns is nothing a
  /// phone should be reading. The stylesheet has a default of its own behind
  /// each name, so an absent pair is the untouched workbench rather than a
  /// broken one.
  const columns = () =>
    beside()
      ? {
          "--pane-sidebar": `${shown().sidebar}%`,
          "--pane-timeline": `${shown().timeline}%`,
        }
      : undefined;

  /// Closing whatever is open, which empties the pane — so on a narrow window it
  /// is also the way back out of it. Harmless on a wide one, where all three
  /// panes are drawn and `data-pane` means nothing.
  const close = () => {
    setEvent(null);
    setPane("timeline");
  };

  /// Which Conversation the URL names, or the empty string on the bare
  /// workbench. Unparsed, like a Set's id: the server decides what names
  /// nothing.
  ///
  /// A memo rather than a read of the params, because what hangs off it is a
  /// key: it has to say nothing at all when the router has moved and the id has
  /// not.
  const selected = createMemo(() => params.id ?? "");

  // Opening a Conversation is what walks a phone into the Timeline, and leaving
  // the workbench route walks it back out. Written as an effect on the URL
  // rather than done in the click handler, because Back is a way of changing the
  // selection too and it never goes through one.
  //
  // Whatever Event was open closes with it: an Event belongs to one
  // Conversation, and an id kept across the change would name nothing.
  createEffect(
    on(selected, (id) => {
      setPane(id === "" ? "conversations" : "timeline");
      setEvent(null);
    }),
  );

  /// The selection as something to key on: a new object each time the id
  /// really changes, and the same one for as long as it does not. What hangs
  /// off it is the whole of the Conversation-reading half of the page, so a
  /// switch tears that down and builds it again — which is what `Reading` is
  /// for, and why it is a component at all.
  const open = createMemo(() => ({ id: selected() }));

  return (
    <div
      class="workbench"
      data-pane={pane()}
      ref={frame}
      style={columns()}
    >
      <section class="pane conversations-pane" aria-label="Conversations">
        <Conversations
          selected={selected()}
          open={(id) => navigate(`/conversations/${id}`)}
        />
      </section>

      {/* One divider per border there is: the sidebar's wherever the sidebar
          stands beside something, and the timeline's only where all three panes
          are up. Each sits between the two panes it parts, so the grid places
          it without being told where. */}
      <Show when={beside()}>
        <Divider
          divider="sidebar"
          label="Resize the conversations pane"
          share={shown().sidebar}
          travel={range("sidebar", shown(), allThree())}
          drag={drag}
          nudge={nudge}
          restore={defaults}
        />
      </Show>

      {/* The Conversation the URL names, in the two panes it is read in.

          Keyed on its id, so that switching to another one throws this away
          and builds it again from nothing. Without that the switch was
          dropped, and dropped worst where it should have been cheapest: on a
          Conversation already read once, answered out of the cache. The query
          has its payload merged into the store rather than put in its place
          (see the query in `Reading`), and reconcile exempts the root of a
          store from the key it is told to match by — so the second
          Conversation went into the first one's object, the object stayed the
          object it had always been, and with nothing to fetch there was not
          even a moment of loading to rebuild the page at. Everything the
          middle pane was holding went on standing over a Conversation that
          was no longer on screen: a Brief half typed into above all, which is
          the only copy of itself there is.

          The merge itself is right and stays. What it is for is a re-read of
          the Conversation already open, where keeping the rows is the whole
          point; it is only across a change of Conversation that it has
          nothing to say, and this is what says so. */}
      <Show when={open()} keyed>
        {(open) => (
          <Reading
            id={open.id}
            event={event()}
            select={setEvent}
            pane={setPane}
            close={close}
            divider={
              <Show when={allThree()}>
                <Divider
                  divider="timeline"
                  label="Resize the timeline pane"
                  share={shown().timeline}
                  travel={range("timeline", shown(), allThree())}
                  drag={drag}
                  nudge={nudge}
                  restore={defaults}
                />
              </Show>
            }
          />
        )}
      </Show>
    </div>
  );
}

/// One Conversation, in the two panes it is read in: its Timeline in the
/// middle, and the full self of whatever is open in it on the right.
///
/// Both panes and the query behind them are one component so that one
/// `<Show keyed>` around it rebuilds all three together — the reason it
/// exists is in the comment at the call site. It draws no frame of its own:
/// the panes are grid children of the workbench, so what comes back is the
/// two of them and the divider between them, loose.
function Reading(props: {
  /// The Conversation to read, or the empty string on the bare workbench.
  id: string;

  /// Which Event the details pane is showing, and how to change it.
  event: number | null;
  select: (event: number) => void;

  /// Which level a narrow window is showing, and the way back out of the
  /// details pane.
  pane: (pane: Pane) => void;
  close: () => void;

  /// The line between the two panes, which is the frame's rather than this
  /// component's: how wide the panes stand is a property of the workbench,
  /// and the handle only stands here because that is where the grid puts it.
  divider: JSX.Element;
}): JSX.Element {
  /// The Event the details pane is showing, where it is one that has a full
  /// self to show. An id whose Event has gone leaves the pane empty, which is
  /// what it is when nothing is open at all.
  ///
  /// Seven kinds have one: a session's output, whose full self is its
  /// Capture; a Question Set, whose full self is the document it was asked
  /// as; a commit, whose full self is its diff; the pull request, whose full
  /// self is what is on it at GitHub right now; and the three documents — the
  /// Brief, the handoff and a Manual Task's instruction — whose full self is
  /// the markdown their card shows five lines of. The kind travels with it,
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
        if ("Commit" in entry) {
          return { commit: entry.Commit };
        }
        if ("Brief" in entry) {
          return { brief: entry.Brief };
        }
        if ("Handoff" in entry) {
          return { handoff: entry.Handoff };
        }
        if ("ManualTask" in entry) {
          return { manual: entry.ManualTask };
        }
        return undefined;
      }),
      ...conversation.pinned.map((pinned): Opened | undefined =>
        "PullRequest" in pinned ? { opened: pinned.PullRequest } : undefined,
      ),
    ].find((open) => open !== undefined && which(open).id === id);
  };

  const conversation = useReading(() => ({
    queryKey: ["conversation", props.id],
    queryFn: () => loadConversation(props.id),
    enabled: props.id !== "",

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

  return (
    <>
      <section class="pane timeline-pane" aria-label="Timeline">
        <Switch>
          <Match when={props.id === ""}>
            {/* The resting state of the workbench, and what it says is the one
                thing there is to do from here. */}
            <p class="empty">Pick a conversation, or start one.</p>
          </Match>
          <Match when={conversation.isPending}>
            <p class="empty">Loading…</p>
          </Match>
          <Match when={conversation.isError}>
            <p class="error">
              Could not read this conversation: {conversation.error?.message}
            </p>
          </Match>
          <Match when={conversation.data}>
            {(conversation) => (
              <Timeline
                conversation={conversation()}
                back={() => props.pane("conversations")}
                details={() => props.pane("details")}
                selected={props.event}
                select={props.select}
              />
            )}
          </Match>
        </Switch>
      </section>

      {props.divider}

      <section class="pane details-pane" aria-label="Details">
        {/* Nothing at all where nothing is open, which on a wide window is a
            blank column beside the record and on a narrow one is a level there
            is no way in to. */}
        <Show when={conversation.data}>
          {(conversation) => (
            <Show when={opened(conversation())}>
              {(open) => (
                <Switch>
                  <Match when={outputIn(open())}>
                    {(output) => (
                      <Output
                        conversation={conversation()}
                        output={output()}
                        back={() => props.pane("timeline")}
                      />
                    )}
                  </Match>
                  <Match when={setIn(open())}>
                    {(asked) => (
                      <Asked
                        asked={asked()}
                        back={() => props.pane("timeline")}
                        close={props.close}
                      />
                    )}
                  </Match>
                  <Match when={commitIn(open())}>
                    {(commit) => (
                      <Commit
                        conversation={conversation()}
                        commit={commit()}
                        back={() => props.pane("timeline")}
                        close={props.close}
                      />
                    )}
                  </Match>
                  <Match when={pullRequestIn(open())}>
                    {(opened) => (
                      <PullRequest
                        conversation={conversation()}
                        opened={opened()}
                        back={() => props.pane("timeline")}
                        close={props.close}
                      />
                    )}
                  </Match>
                  {/* And the three documents, which are one pane: each is
                      rendered markdown under the heading its card carries, and
                      the pane is the whole of what the card showed five lines
                      of. */}
                  <Match when={briefIn(open())}>
                    {(brief) => (
                      <Document
                        heading="Brief"
                        html={brief().html}
                        empty="Nothing was written."
                        back={() => props.pane("timeline")}
                        close={props.close}
                      />
                    )}
                  </Match>
                  <Match when={handoffIn(open())}>
                    {(handoff) => (
                      <Document
                        heading="Handoff"
                        html={handoff().html}
                        empty="The grilling wrote nothing down."
                        back={() => props.pane("timeline")}
                        close={props.close}
                      />
                    )}
                  </Match>
                  <Match when={manualIn(open())}>
                    {(manual) => (
                      <Document
                        heading="Manual task"
                        html={manual().html}
                        empty="Nothing was asked for."
                        back={() => props.pane("timeline")}
                        close={props.close}
                      />
                    )}
                  </Match>
                </Switch>
              )}
            </Show>
          )}
        </Show>
      </section>
    </>
  );
}
