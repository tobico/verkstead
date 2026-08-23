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
//! Which Event is open is held here rather than in the Timeline, because it is
//! what the third pane is *about*: the pane is that Event's full self and
//! nothing else, so with none open it is bare paper. What a Conversation *is* is
//! not drawn there — the setup it needs is on the Brief card, where it is used —
//! and the way on to an empty pane is not offered, so a narrow window can only
//! walk into the pane by opening something. The selection is not in the URL —
//! an Event opened is a place in a page rather than a page, and a Conversation
//! whose Timeline has moved on is not one to restore a scroll position into.

import { useNavigate, useParams } from "@solidjs/router";
import { Match, Show, Switch, createEffect, createSignal, on, type JSX } from "solid-js";

import { loadConversation } from "../api/client";
import type {
  AgentOutputEvent,
  BriefEvent,
  CommitEvent,
  ConversationView,
  HandoffEvent,
  InterruptionEvent,
  ManualTaskEvent,
  PullRequestEvent,
  QuestionSetEvent,
} from "../api/types";
import { useReading } from "../freshness";
import { Asked } from "./Asked";
import { Commit } from "./Commit";
import { Conversations } from "./Conversations";
import { Document } from "./Document";
import { Interrupted } from "./Interruption";
import { Output } from "./Output";
import { PullRequest } from "./PullRequest";
import { Timeline } from "./Timeline";

/// Which level of the hierarchy a narrow window is showing.
export type Pane = "conversations" | "timeline" | "details";

/// An Event with a full self, as the details pane holds it: which kind, and the
/// Event itself.
type Opened =
  | { output: AgentOutputEvent }
  | { asked: QuestionSetEvent }
  | { commit: CommitEvent }
  | { stopped: InterruptionEvent }
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
  | InterruptionEvent
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
  if ("stopped" in open) {
    return open.stopped;
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

function stoppedIn(open: Opened): InterruptionEvent | undefined {
  return "stopped" in open ? open.stopped : undefined;
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

export function Workbench(): JSX.Element {
  const params = useParams();
  const navigate = useNavigate();

  const [pane, setPane] = createSignal<Pane>("conversations");

  /// Which Timeline Event the details pane is showing, where one is open.
  const [event, setEvent] = createSignal<number | null>(null);

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
  const selected = () => params.id ?? "";

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

  /// The Event the details pane is showing, where it is one that has a full
  /// self to show. An id whose Event has gone leaves the pane empty, which is
  /// what it is when nothing is open at all.
  ///
  /// Eight kinds have one: a session's output, whose full self is its
  /// Capture; a Question Set, whose full self is the document it was asked
  /// as; a commit, whose full self is its diff; an interruption, whose full
  /// self is the evidence it was raised with; the pull request, whose full
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
    const id = event();

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
        if ("Interruption" in entry) {
          return { stopped: entry.Interruption };
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

  return (
    <div class="workbench" data-pane={pane()}>
      <section class="pane conversations-pane" aria-label="Conversations">
        <Conversations
          selected={selected()}
          open={(id) => navigate(`/conversations/${id}`)}
        />
      </section>

      <section class="pane timeline-pane" aria-label="Timeline">
        <Switch>
          <Match when={selected() === ""}>
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
                back={() => setPane("conversations")}
                details={() => setPane("details")}
                selected={event()}
                select={setEvent}
              />
            )}
          </Match>
        </Switch>
      </section>

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
                        back={() => setPane("timeline")}
                      />
                    )}
                  </Match>
                  <Match when={setIn(open())}>
                    {(asked) => (
                      <Asked
                        asked={asked()}
                        back={() => setPane("timeline")}
                        close={close}
                      />
                    )}
                  </Match>
                  <Match when={commitIn(open())}>
                    {(commit) => (
                      <Commit
                        conversation={conversation()}
                        commit={commit()}
                        back={() => setPane("timeline")}
                        close={close}
                      />
                    )}
                  </Match>
                  <Match when={stoppedIn(open())}>
                    {(stopped) => (
                      <Interrupted
                        conversation={conversation()}
                        stopped={stopped()}
                        back={() => setPane("timeline")}
                        close={close}
                      />
                    )}
                  </Match>
                  <Match when={pullRequestIn(open())}>
                    {(opened) => (
                      <PullRequest
                        conversation={conversation()}
                        opened={opened()}
                        back={() => setPane("timeline")}
                        close={close}
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
                        back={() => setPane("timeline")}
                        close={close}
                      />
                    )}
                  </Match>
                  <Match when={handoffIn(open())}>
                    {(handoff) => (
                      <Document
                        heading="Handoff"
                        html={handoff().html}
                        empty="The grilling wrote nothing down."
                        back={() => setPane("timeline")}
                        close={close}
                      />
                    )}
                  </Match>
                  <Match when={manualIn(open())}>
                    {(manual) => (
                      <Document
                        heading="Manual task"
                        html={manual().html}
                        empty="Nothing was asked for."
                        back={() => setPane("timeline")}
                        close={close}
                      />
                    )}
                  </Match>
                </Switch>
              )}
            </Show>
          )}
        </Show>
      </section>
    </div>
  );
}
