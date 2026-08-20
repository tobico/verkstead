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
//! what the third pane is *about*: with none open the pane says what the
//! Conversation is, and with one open it shows that Event's full self. The
//! selection is not in the URL — an Event opened is a place in a page rather
//! than a page, and a Conversation whose Timeline has moved on is not one to
//! restore a scroll position into.

import { useNavigate, useParams } from "@solidjs/router";
import { useQuery } from "@tanstack/solid-query";
import { Match, Show, Switch, createEffect, createSignal, on, type JSX } from "solid-js";

import { loadConversation } from "../api/client";
import type {
  AgentOutputEvent,
  ConversationView,
  QuestionSetEvent,
} from "../api/types";
import { Asked } from "./Asked";
import { Conversations } from "./Conversations";
import { Details } from "./Details";
import { Output } from "./Output";
import { Timeline } from "./Timeline";

/// How often the open page reads its Conversation again, in milliseconds.
///
/// The Timeline has to keep up with a session that is writing into it and with
/// a Question Set that arrives while nobody is touching the page — and while the
/// page is asking, it also brings a Set answered on another device into view.
const REFRESH = 10_000;

/// Which level of the hierarchy a narrow window is showing.
export type Pane = "conversations" | "timeline" | "details";

/// An Event with a full self, as the details pane holds it: which kind, and the
/// Event itself.
type Opened = { output: AgentOutputEvent } | { asked: QuestionSetEvent };

/// The Event inside, whichever kind it turned out to be — what the two have in
/// common is the id the pane was opened by.
function which(open: Opened): AgentOutputEvent | QuestionSetEvent {
  return "output" in open ? open.output : open.asked;
}

/// And each kind on its own, for the pane that draws it: the Event where this is
/// one of that kind, and nothing where it is the other.
function outputIn(open: Opened): AgentOutputEvent | undefined {
  return "output" in open ? open.output : undefined;
}

function setIn(open: Opened): QuestionSetEvent | undefined {
  return "asked" in open ? open.asked : undefined;
}

export function Workbench(): JSX.Element {
  const params = useParams();
  const navigate = useNavigate();

  const [pane, setPane] = createSignal<Pane>("conversations");

  /// Which Timeline Event the details pane is showing, where one is open.
  const [event, setEvent] = createSignal<number | null>(null);

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
  /// self to show. An id whose Event has gone shows the Conversation instead,
  /// which is what the pane says when nothing is open.
  ///
  /// Two kinds have one: a session's output, whose full self is its transcript,
  /// and a Question Set, whose full self is the document it was asked as. The
  /// kind travels with it, because it is what decides which pane is drawn.
  const opened = (conversation: ConversationView): Opened | undefined => {
    const id = event();

    return conversation.timeline
      .map((entry): Opened | undefined => {
        if ("AgentOutput" in entry) {
          return { output: entry.AgentOutput };
        }
        if ("QuestionSet" in entry) {
          return { asked: entry.QuestionSet };
        }
        return undefined;
      })
      .find((open) => open !== undefined && which(open).id === id);
  };

  const conversation = useQuery(() => ({
    queryKey: ["conversation", selected()],
    queryFn: () => loadConversation(selected()),
    enabled: selected() !== "",
    // The fallback underneath both Nudge channels (ADR-0005), which this page
    // inherited when the pending list retired: a Timeline is where a Question
    // Set arrives now, and where a session's output grows while it runs. The
    // stream is instant while the page is alive and the relayed push survives
    // an iOS PWA being suspended; neither has to work, because this is here.
    //
    // The query keeps the last Conversation while the next one is in flight, so
    // a refetch every ten seconds swaps Events rather than blinking the pane
    // through a loading state the human is trying to read past.
    refetchInterval: REFRESH,
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
        <Show
          when={conversation.data}
          fallback={<p class="empty">Nothing to show yet.</p>}
        >
          {(conversation) => (
            <Show
              when={opened(conversation())}
              fallback={
                <Details
                  conversation={conversation()}
                  back={() => setPane("timeline")}
                />
              }
            >
              {(open) => (
                <Switch>
                  <Match when={outputIn(open())}>
                    {(output) => (
                      <Output
                        conversation={conversation()}
                        output={output()}
                        back={() => setPane("timeline")}
                        close={() => setEvent(null)}
                      />
                    )}
                  </Match>
                  <Match when={setIn(open())}>
                    {(asked) => (
                      <Asked
                        asked={asked()}
                        back={() => setPane("timeline")}
                        close={() => setEvent(null)}
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
