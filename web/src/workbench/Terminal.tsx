//! A Conversation's own terminals, opened: shells of the human's inside its
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
//! **Several of them, one per tab.** The bar in the pane's header holds a tab
//! per terminal, in the order the server issued their numbers, and a plus at the
//! end opens another. It is the Output pane's Transcript/Screen switch built
//! again — pressed-or-not buttons in a group rather than a tablist, which is the
//! house's answer to this shape — and each tab is called *Terminal N* by the
//! number the server gave it, which is why those numbers are never reused.
//!
//! Every tab keeps its socket open and its grid mounted whether or not it is the
//! one showing, so a shell that printed while somebody was reading another tab
//! has printed it by the time they turn back. Only the tab showing measures the
//! pane and says so up its socket — see [`./Attached`], where the hiding, the
//! measuring and the focus all are.
//!
//! **The server holds the shells, so this pane is the way back to them rather
//! than where they live.** On load it asks which of the Conversation's terminals
//! are live and draws a tab for each, and opens one only where there is none —
//! so a reload, a second device or a tab closed by accident comes back to what
//! was already there, still running and showing what it last showed.
//!
//! Nothing here is a record: no Capture, no Event on the Timeline, nothing in a
//! Share. And nothing here holds the run off — typing into a terminal is the
//! human doing something, exactly as typing into a Screen is, and somebody who
//! means to take the work on presses **Stop** first.

import { faPlus } from "@fortawesome/free-solid-svg-icons";
import {
  For,
  Match,
  Show,
  Switch,
  createEffect,
  createMemo,
  createSignal,
  type JSX,
} from "solid-js";

import { IconButton } from "../IconButton";
import { PaneSticky } from "../Panes";
import { listTerminals, openTerminal, terminalSocket } from "../api/client";
import type { ConversationView, TerminalOpened } from "../api/types";
import { useReading } from "../freshness";
import { Empty, ErrorLine } from "../notices";
import { Attached } from "./Attached";
import { PaneHead } from "./PaneHead";
import { NO_SESSIONS } from "./sessions";
import styles from "./Terminal.module.css";
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

  /// The ones this pane has opened, in the order it opened them.
  const [added, setAdded] = createSignal<number[]>([]);

  /// And what an open was refused with, where one was refused.
  const [turned, setTurned] = createSignal<string | undefined>();

  /// Which tab the human turned to, where they have turned to one.
  const [chosen, setChosen] = createSignal<number | undefined>();

  /// Every tab there is: what was live when the pane loaded and what it has
  /// opened since.
  ///
  /// By number, which is the order they were opened in — the server issues them
  /// in order and never reuses one, so sorting them is reading the register's
  /// own order back. And the two sources are a set rather than a sum: a list
  /// read again would answer with the ones this pane opened, and a tab drawn
  /// twice is two sockets onto one shell.
  const tabs = createMemo(() =>
    [...new Set([...(terminals.data?.live ?? []), ...added()])].sort(
      (one, another) => one - another,
    ),
  );

  /// The one showing: the tab turned to, or the first while nobody has turned
  /// to one — and the first again once the one turned to is gone, which is what
  /// keeps the pane showing a terminal rather than a gap where one was.
  const showing = createMemo(() => {
    const open = tabs();
    const turnedTo = chosen();

    return turnedTo !== undefined && open.includes(turnedTo)
      ? turnedTo
      : open[0];
  });

  /// Open another, and show it. What plus does, and what the pane does for
  /// itself where there is nothing live to come back to.
  const open = (): Promise<void> =>
    openTerminal(props.conversation.id)
      .then((outcome) => {
        if (typeof outcome === "string") {
          setTurned(TERMINAL_REFUSAL[outcome]);
          return;
        }

        setTurned(undefined);
        setAdded((was) => [...was, outcome.Opened.number]);
        setChosen(outcome.Opened.number);
      })
      .catch((error: Error) => {
        setTurned(error.message);
      });

  /// Open one where none is live. The pane never stands empty, so this is the
  /// pane's own doing rather than a press.
  ///
  /// Asked once per Conversation, which is what `asked` holds: a refusal leaves
  /// the list exactly as it was, and an effect that asked again on the strength
  /// of it would be a refused Sandbox spawning for ever. Plus is a press and is
  /// under no such rule — somebody who asks again meant to.
  let asked: number | undefined;

  createEffect(() => {
    if (
      terminals.data === undefined ||
      tabs().length > 0 ||
      asked === props.conversation.id
    ) {
      return;
    }

    asked = props.conversation.id;

    void open();
  });

  return (
    <>
      <PaneSticky>
        <PaneHead back={{ to: "Timeline", go: props.back }} title="Terminal">
          {/* The tabs beside the title, where a pane's own controls go, and the
              way to another at the end of them. Buttons that say which they are
              rather than tabs: they are all always there, `aria-pressed` is the
              one word that says which is showing, and what each one does is
              show a grid that is already drawn. */}
          <div
            class={styles.tabs}
            role="group"
            aria-label="This conversation's terminals"
          >
            <For each={tabs()}>
              {(number) => (
                <button
                  type="button"
                  class={styles.tab}
                  aria-pressed={showing() === number}
                  onClick={() => setChosen(number)}
                >
                  Terminal {number}
                </button>
              )}
            </For>

            <IconButton
              of={faPlus}
              label="New terminal"
              class={styles.plus}
              // Nothing of this one is open: it opens a shell rather than a
              // pane, and there is no state of the page it is the way back
              // into.
              open={false}
              press={() => void open()}
            />
          </div>
        </PaneHead>
      </PaneSticky>

      {/* A refusal above the tabs rather than in place of them: the pane may
          have shells running in it, and an open that was turned down says
          nothing about them. */}
      <Show when={turned()}>{(why) => <ErrorLine>{why()}</ErrorLine>}</Show>

      <Switch>
        <Match when={terminals.isError}>
          <ErrorLine>
            Could not read this conversation's terminals:{" "}
            {terminals.error?.message}
          </ErrorLine>
        </Match>
        <Match when={tabs().length > 0}>
          <For each={tabs()}>
            {(number) => (
              <Attached
                at={terminalSocket(props.conversation.id, number)}
                class={shell.paneWide}
                showing={showing() === number}
                say={{
                  waiting: "Starting a shell in this conversation's worktree…",
                  lost: "The connection to this terminal was lost.",
                }}
              />
            )}
          </For>
        </Match>
        <Match when={turned() === undefined}>
          <Empty>Opening a terminal…</Empty>
        </Match>
      </Switch>
    </>
  );
}
