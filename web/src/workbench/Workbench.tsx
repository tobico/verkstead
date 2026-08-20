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
//! Nothing here starts anything. A Conversation is a record a grilling session
//! will be run against, and the button that runs one arrives with the stage that
//! has something to run.

import { useNavigate, useParams } from "@solidjs/router";
import { useQuery } from "@tanstack/solid-query";
import { Match, Show, Switch, createEffect, createSignal, on, type JSX } from "solid-js";

import { loadConversation } from "../api/client";
import { Conversations } from "./Conversations";
import { Details } from "./Details";
import { Timeline } from "./Timeline";

/// Which level of the hierarchy a narrow window is showing.
export type Pane = "conversations" | "timeline" | "details";

export function Workbench(): JSX.Element {
  const params = useParams();
  const navigate = useNavigate();

  const [pane, setPane] = createSignal<Pane>("conversations");

  /// Which Conversation the URL names, or the empty string on the bare
  /// workbench. Unparsed, like a Set's id: the server decides what names
  /// nothing.
  const selected = () => params.id ?? "";

  // Opening a Conversation is what walks a phone into the Timeline, and leaving
  // the workbench route walks it back out. Written as an effect on the URL
  // rather than done in the click handler, because Back is a way of changing the
  // selection too and it never goes through one.
  createEffect(
    on(selected, (id) => setPane(id === "" ? "conversations" : "timeline")),
  );

  const conversation = useQuery(() => ({
    queryKey: ["conversation", selected()],
    queryFn: () => loadConversation(selected()),
    enabled: selected() !== "",
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
            <Details
              conversation={conversation()}
              back={() => setPane("timeline")}
            />
          )}
        </Show>
      </section>
    </div>
  );
}
