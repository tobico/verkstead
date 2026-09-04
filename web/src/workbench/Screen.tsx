//! One session's Screen: the grid its terminal is showing, drawn as a terminal.
//!
//! **A session that is still running is watched over a socket**, and that half
//! is [`./Attached`] — the xterm, the repaint, the typing and the size going
//! back up, which a Conversation's own Terminal uses too. What is here is the
//! half that is a session's: which of the two records this is, and the wording
//! under the grid.
//!
//! **A session that has ended is fetched**, because its Screen is the one it
//! last stood on and nothing will move it again. It is drawn at its own size and
//! scrolls in the pane, there being nothing at the far end to redraw it — and it
//! says it is read-only, a terminal that silently swallows typing reading as
//! broken rather than as read-only.
//!
//! Typing into a live one commits Verkstead to nothing: keystrokes reach the
//! session's own terminal and nothing follows them, so a run goes on ending
//! sessions and advancing steps by the ordinary rules. Somebody who wants a
//! session left alone while they work in it presses **Stop** first.
//!
//! The grid and nothing above it: no scrollback, matching the server that
//! decided the repaint. A reader who wants everything the session printed wants
//! the Transcript beside this, or the Capture underneath it.

import { Match, Switch, type JSX } from "solid-js";

import { loadScreen, screenSocket } from "../api/client";
import type { AgentOutputEvent, ConversationView } from "../api/types";
import { useReading } from "../freshness";
import { Empty, ErrorLine } from "../notices";
import { Attached, Standing } from "./Attached";

export function Screen(props: {
  conversation: ConversationView;
  output: AgentOutputEvent;
}): JSX.Element {
  /// Whether this session is still printing, which is what decides where the
  /// Screen comes from — the socket or the fetch.
  const live = () => props.output.running;

  const screen = useReading(() => ({
    // The Event is in the key for the reason it is in the Transcript's: opening
    // another session's Screen is another query rather than this one showing
    // the wrong session's grid for a moment.
    queryKey: ["screen", props.conversation.id, props.output.id],
    queryFn: () => loadScreen(props.conversation.id, props.output.id),
    // A running session is watched instead. One request for the grid as the
    // store last had it would be a request for something a repaint is about to
    // replace.
    enabled: !live(),

    // Which is why this one is frozen rather than merged: it is only ever
    // fetched for a session that has stopped, and a stopped session's Screen is
    // the grid it last stood on and nothing after it. A session running when
    // the pane opened is watched over the socket until it ends, and the one
    // read that follows is the read of a record that cannot move again.
    freshness: "static",
  }));

  return (
    <Switch>
      <Match when={screen.isError}>
        <ErrorLine>
          Could not read this screen: {screen.error?.message}
        </ErrorLine>
      </Match>
      <Match when={live()}>
        <Attached
          at={screenSocket(props.conversation.id, props.output.id)}
          say={{
            waiting: "Waiting for this session's screen…",
            watching:
              "Watching. Type to work in this session — press Stop first if the run must not advance under you.",
            lost: "The connection to this session's screen was lost.",
          }}
        />
      </Match>
      <Match when={screen.data}>
        {(painted) => (
          <Standing
            painted={painted()}
            say="Read-only: this is what the session's terminal is showing."
          />
        )}
      </Match>
      <Match when={true}>
        <Empty>Loading…</Empty>
      </Match>
    </Switch>
  );
}
