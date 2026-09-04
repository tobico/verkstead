//! A Conversation's own terminal, opened: a shell of the human's inside its
//! Sandbox, with its Worktree as the working directory.
//!
//! For the moment the agent's work is done and somebody wants to try it, make a
//! small change or work with git — without leaving the workbench, and without
//! the run noticing
//! ([ADR 0013](../../../docs/adr/0013-conversation-terminals.md)).
//!
//! Opened by the terminal icon on the Timeline's header — see `Timeline.tsx` —
//! which is a details pane like every other, at a path of its own so it survives
//! a reload and can be linked to. The second pane nothing on the record opens: a
//! terminal belongs to the Conversation rather than to any moment on it, the way
//! sharing does.
//!
//! **The Screen's own viewer, filling the pane.** What is drawn is
//! [`./Attached`], the same xterm over the same socket to the same server-held
//! virtual terminal a session's Screen is watched through — what runs on this
//! one is a shell rather than an agent, and that is the whole of the difference.
//! The pane gives it every inch it has: the reading measure every other details
//! pane pads its content to comes off, the way the composer takes it off, and
//! the pane ends where the window does, so the terminal is sized to the pane
//! rather than scrolling it.
//!
//! **The server holds the shell, so this pane is the way back to it rather than
//! where it lives.** On load it asks which of the Conversation's terminals are
//! live and attaches to the first, and opens one only where none is — so a
//! reload, a second device or a tab closed by accident comes back to the shell
//! that was already there, still running and showing what it last showed. The
//! tab bar over several of them is a later task; here there is one terminal and
//! no bar.
//!
//! Nothing here is a record: no Capture, no Event on the Timeline, nothing in a
//! Share. And nothing here holds the run off — typing into a terminal is the
//! human doing something, exactly as typing into a Screen is, and somebody who
//! means to take the work on presses **Stop** first.

import {
  Match,
  Switch,
  createEffect,
  createMemo,
  createSignal,
  type JSX,
} from "solid-js";

import { PaneSticky } from "../Panes";
import { listTerminals, openTerminal, terminalSocket } from "../api/client";
import type { ConversationView, TerminalOpened } from "../api/types";
import { useReading } from "../freshness";
import { Empty, ErrorLine } from "../notices";
import { Attached } from "./Attached";
import { PaneHead } from "./PaneHead";
import { NO_SESSIONS } from "./sessions";
import shell from "../Panes.module.css";

/// Each way of being refused a terminal, in the words of what to go and do
/// about it.
///
/// One line each rather than a single "could not open one", because the server
/// names them separately for exactly this: a worktree that is not there, a
/// profile nobody chose and a shell that would not start are three different
/// jobs, and only the human can tell which they are looking at.
///
/// The first two are what the icon beside Share is already drawn against, said
/// again here because a pane is drawn against a Conversation that may have
/// moved since it was read.
export const TERMINAL_REFUSAL: Record<
  Extract<TerminalOpened, string>,
  string
> = {
  NoSuchConversation: "This conversation is gone.",
  NoWorktree: "This conversation has no worktree to open a terminal in.",
  NoProfile:
    "This conversation has no agent profile settled, so there is no account to run a shell under.",
  // The one refusal here that is about this Verkstead rather than about this
  // conversation, and every press that starts a session meets it too — see
  // `sessions.tsx`, which is where the sentence is.
  NotOnWindowsYet: NO_SESSIONS,
  Refused: "The shell would not start. The server's log says why.",
};

export function Terminal(props: {
  conversation: ConversationView;
  back: () => void;
}): JSX.Element {
  /// Which of this Conversation's terminals are live.
  ///
  /// Read when the pane opens rather than followed: the register is the
  /// server's and moves only when a shell exits or this pane opens one, and both
  /// of those are answered where they happen. Frozen for that reason — a
  /// terminal is no part of the record, so no Nudge is ever about one.
  const terminals = useReading(() => ({
    queryKey: ["terminals", props.conversation.id],
    queryFn: () => listTerminals(props.conversation.id),
    freshness: "static",
  }));

  /// The one this pane opened, where it opened one.
  const [opened, setOpened] = createSignal<number | undefined>();

  /// And what the open was refused with, where it was refused.
  const [turned, setTurned] = createSignal<string | undefined>();

  /// What to attach to: the first terminal already live, or the one opened for
  /// this pane.
  ///
  /// The live one first, which is the whole point of asking: a reload comes back
  /// to the shell it left rather than starting a second one beside it.
  const attached = createMemo(() => terminals.data?.live[0] ?? opened());

  /// Open one where none is live. The pane never stands empty, so this is the
  /// pane's own doing rather than a press.
  ///
  /// Asked once per Conversation, which is what `asked` holds: a refusal leaves
  /// the list exactly as it was, and an effect that asked again on the strength
  /// of it would be a refused Sandbox spawning for ever.
  let asked: number | undefined;

  createEffect(() => {
    const live = terminals.data?.live;

    if (live === undefined || live.length > 0 || asked === props.conversation.id) {
      return;
    }

    asked = props.conversation.id;

    void openTerminal(props.conversation.id)
      .then((outcome) => {
        if (typeof outcome === "string") {
          setTurned(TERMINAL_REFUSAL[outcome]);
          return;
        }

        setTurned(undefined);
        setOpened(outcome.Opened.number);
      })
      .catch((error: Error) => setTurned(error.message));
  });

  return (
    <>
      <PaneSticky>
        <PaneHead back={{ to: "Timeline", go: props.back }} title="Terminal" />
      </PaneSticky>

      <Switch>
        <Match when={terminals.isError}>
          <ErrorLine>
            Could not read this conversation's terminals:{" "}
            {terminals.error?.message}
          </ErrorLine>
        </Match>
        <Match when={turned()}>{(why) => <ErrorLine>{why()}</ErrorLine>}</Match>
        <Match when={attached()}>
          {(number) => (
            <Attached
              at={terminalSocket(props.conversation.id, number())}
              class={shell.paneWide}
              say={{
                waiting: "Starting a shell in this conversation's worktree…",
                lost: "The connection to this terminal was lost.",
              }}
            />
          )}
        </Match>
        <Match when={true}>
          <Empty>Opening a terminal…</Empty>
        </Match>
      </Switch>
    </>
  );
}
