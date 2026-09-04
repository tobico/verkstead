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
//! per terminal, in the order they were opened, and a plus at the end opens
//! another. It is the Output pane's Transcript/Screen switch built again —
//! pressed-or-not buttons in a group rather than a tablist, which is the house's
//! answer to this shape — and each tab is called *Terminal N* by the number the
//! server gave it, which is why those numbers are never reused.
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
//! **And a tab is closed on purpose or not at all.** Close is a row on the
//! tab's own context menu — a right-click under a pointer, a long press under a
//! finger — rather than a × on the tab, because a × beside a label this small is
//! a thing to hit by accident and what it would end is a shell somebody is
//! working in. The row asks the server to end that shell, and the tab then goes
//! the way every ended shell's tab goes: its socket closes, and where it was the
//! last one another opens.
//!
//! **And the pane never stands empty.** A shell that exits closes its socket,
//! which is how this side hears about it: the tab goes, and where it was the
//! last one another opens. The whole of the guard on that is time — a shell that
//! ended within [`AT_ONCE`] of being asked for, or that the server refused to
//! open at all, is one that could not start rather than one that ran, so its tab
//! *stays* saying why and nothing opens until plus is pressed, which replaces
//! it. Without it a Sandbox that will not start would be an endless spawn loop.
//! The clock is this pane's own, started when it asked: the server opens nothing
//! of its own accord and knows nothing about tabs.
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
import { ContextMenu } from "../Menu";
import { PaneSticky } from "../Panes";
import {
  closeTerminal,
  listTerminals,
  openTerminal,
  terminalSocket,
} from "../api/client";
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

/// How soon after being asked for a shell has to end for its tab to stay.
///
/// The line between a shell that ran and a shell that could not start, and there
/// is no better one to draw: what comes back from the server is a terminal that
/// opened, and a shell that dies on its first line dies after the answer. Five
/// seconds is long enough that nothing a human did in the terminal is inside it
/// — a `exit` typed by hand takes longer than that to type — and short enough
/// that a pane sitting behind a broken Sandbox says so at once.
///
/// Measured from when *this pane* asked, because this pane is the only thing
/// that knows: the server opens nothing of its own accord and holds no clock.
export const AT_ONCE = 5_000;

/// What a tab says when its shell ended inside that.
///
/// The grid it left stands under it, read-only, because whatever the shell
/// managed to print on its way out is the only thing here that says why — and
/// the press that would try again is named, since nothing is going to try on its
/// own.
export const ENDED_AT_ONCE =
  "The shell ended as soon as it started. Press plus to open another.";

export function Terminal(props: {
  conversation: ConversationView;
  back: () => void;
}): JSX.Element {
  /// Which of this Conversation's terminals are live.
  ///
  /// Read when the pane opens rather than followed: the register is the
  /// server's and moves only when a shell exits or this pane opens one, and both
  /// of those are answered where they happen — an exit down the tab's own
  /// socket. Frozen for that reason — a terminal is no part of the record, so no
  /// Nudge is ever about one.
  const terminals = useReading(() => ({
    queryKey: ["terminals", props.conversation.id],
    queryFn: () => listTerminals(props.conversation.id),
    freshness: "static",
  }));

  /// Every tab there is, in the order they were opened: what was live when the
  /// pane loaded, and what it has opened since.
  ///
  /// The server issues its numbers in order and never reuses one, so the list it
  /// answers with is already in that order and everything opened here goes on
  /// the end. A tab standing on a shell that never started is one of these too,
  /// under a key of its own — see [`stand`].
  const [tabs, setTabs] = createSignal<number[]>([]);

  /// And what each tab that is standing rather than running says: the shell
  /// ended at once, or the refusal the server answered the open with.
  ///
  /// A tab is in here or it is not, and that is the whole of the difference
  /// between the two kinds: one with an entry keeps its grid and takes no
  /// typing, and one without is a shell somebody is working in.
  const [over, setOver] = createSignal<Record<number, string>>({});

  /// Which tab the human turned to, where they have turned to one.
  const [chosen, setChosen] = createSignal<number | undefined>();

  /// The tab whose menu is open and where the hand asked for it, or `null` while
  /// no menu is down.
  ///
  /// Which tab it is about is kept here rather than in a menu per tab, for the
  /// reason the sidebar's is: one menu is open at a time, and one of these per
  /// tab would be a component held open by a pane that only ever wants one.
  const [pointed, setPointed] = createSignal<{
    tab: number;
    x: number;
    y: number;
  } | null>(null);

  /// Whether the list has been read, which is what says the pane knows how many
  /// terminals there are. Before it, an empty tab bar is a pane that has not
  /// looked yet rather than a Conversation with no shells.
  const [read, setRead] = createSignal(false);

  /// When this pane asked for each terminal it opened, which is what
  /// [`AT_ONCE`] is measured from. Nothing for the ones that were already live:
  /// this pane never asked for those, so a shell of theirs that ends is one that
  /// ran.
  const askedAt = new Map<number, number>();

  /// Whether an open is in flight, so that nothing asks for a second while the
  /// pane is empty and waiting on the first.
  let opening = false;

  /// The key the next tab standing on a refusal gets. Below every number the
  /// server issues, counting the other way, because a refused open was never
  /// given one — there is no shell for it to name.
  let refusals = 0;

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

  /// What a tab is called. The number the server issued it, and the bare word
  /// for one standing on an open that was refused — the server never got as far
  /// as a number for that one, and a made-up one would be a name for a shell
  /// that is not there.
  const called = (tab: number): string =>
    tab > 0 ? `Terminal ${tab}` : "Terminal";

  /// Take away whatever is only standing there to say why, which is what plus
  /// replacing one means: a tab that is a sentence about a shell that never
  /// started is not something to keep beside a shell that has.
  const replace = (): void => {
    const standing = Object.keys(over()).map(Number);

    if (standing.length === 0) {
      return;
    }

    setOver({});
    setTabs((was) => was.filter((one) => !standing.includes(one)));
  };

  /// A tab that says why there is no shell in it, and stops the pane opening
  /// another until somebody presses plus.
  const stand = (why: string): void => {
    replace();

    refusals -= 1;
    const tab = refusals;

    setTabs((was) => [...was, tab]);
    setOver((was) => ({ ...was, [tab]: why }));
    setChosen(tab);
  };

  /// Open another, and show it. What plus does, and what the pane does for
  /// itself where there is nothing live to come back to.
  const open = (): Promise<void> => {
    if (opening) {
      return Promise.resolve();
    }

    opening = true;

    return openTerminal(props.conversation.id)
      .then((outcome) => {
        if (typeof outcome === "string") {
          stand(TERMINAL_REFUSAL[outcome]);
          return;
        }

        const { number } = outcome.Opened;

        replace();
        askedAt.set(number, Date.now());
        setTabs((was) => [...was, number]);
        setChosen(number);
      })
      // A request that never landed is a shell that did not start, and it is
      // read as one: the pane says so in a tab and waits to be asked again,
      // rather than asking again itself against a server that is not there.
      .catch((error: Error) => stand(error.message))
      .finally(() => {
        opening = false;
      });
  };

  /// One tab's socket closed, which is its shell gone: the server takes a
  /// terminal off its register the moment its shell exits, and every watcher's
  /// socket closes with it.
  ///
  /// What follows is the whole of the ending rule. A shell that ran goes, and
  /// the pane opens another where it was the last; one that ended inside
  /// [`AT_ONCE`] of being asked for could not start, so its tab stays saying so
  /// and nothing opens on its own after it.
  const ended = (tab: number): void => {
    const asked = askedAt.get(tab);

    if (asked === undefined || Date.now() - asked >= AT_ONCE) {
      askedAt.delete(tab);
      setTabs((was) => was.filter((one) => one !== tab));
      return;
    }

    setOver((was) => ({ ...was, [tab]: ENDED_AT_ONCE }));
  };

  /// What there is to do about a tab, asked for with a right-click or a long
  /// press: the browser's own menu is not what either is asking for, so that
  /// goes.
  ///
  /// Both hands, unlike the sidebar's cards — a long press on one of those is
  /// already how a card is picked up to be dragged, and a tab has no second
  /// gesture to protect.
  const ask = (event: MouseEvent, tab: number): void => {
    event.preventDefault();
    setPointed({ tab, x: event.clientX, y: event.clientY });
  };

  /// Close one, which is the whole of what that menu holds.
  ///
  /// The shell is ended at the server and the tab goes when its socket closes,
  /// which is how a tab hears about every shell that ends — one rule, whichever
  /// end asked for it. What this takes off first is the pane's clock: a shell
  /// somebody closed is a shell that ran, however long it ran for, and left in
  /// place the five seconds would read a tab closed straight after it opened as
  /// a shell that could not start.
  ///
  /// A tab that is only standing there to say why has no shell to end and no
  /// socket to hear it on, so it simply goes. Nothing is drawn about a request
  /// that failed: the shell is the server's, and a tab still there is what says
  /// it is still running.
  const close = (tab: number): void => {
    setPointed(null);

    if (tab < 0 || over()[tab] !== undefined) {
      setOver((was) => {
        const rest = { ...was };
        delete rest[tab];
        return rest;
      });
      setTabs((was) => was.filter((one) => one !== tab));
      return;
    }

    askedAt.delete(tab);

    void closeTerminal(props.conversation.id, tab);
  };

  /// The tabs the pane loads with: one for each terminal the server is already
  /// holding.
  ///
  /// Once, for this Conversation. The list is a reading of the register at the
  /// moment the pane opened, and everything that happens to it after that
  /// happens here — a second seeding would put back a tab whose shell has since
  /// ended.
  let seeded: number | undefined;

  createEffect(() => {
    const live = terminals.data?.live;

    if (live === undefined || seeded === props.conversation.id) {
      return;
    }

    seeded = props.conversation.id;
    setTabs(live);
    setRead(true);
  });

  /// And the pane never standing empty: nothing live is a terminal opened,
  /// whether that is a Conversation that had none or the last of its shells
  /// having just exited.
  ///
  /// A tab standing on a shell that could not start counts as a tab, which is
  /// what stops this asking again over a Sandbox that will not have it — see
  /// [`AT_ONCE`]. Plus is a press and is under no such rule: somebody who asks
  /// again meant to.
  createEffect(() => {
    if (!read() || tabs().length > 0 || opening) {
      return;
    }

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
              {(tab) => (
                <button
                  type="button"
                  class={styles.tab}
                  aria-pressed={showing() === tab}
                  onClick={() => setChosen(tab)}
                  onContextMenu={(event) => ask(event, tab)}
                >
                  {called(tab)}
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

      {/* What there is to do about a tab, where the hand asked for it. One row,
          because closing is the one thing a tab has that pressing it does not
          already do — and on a menu rather than on the tab because a shell
          somebody is working in is not a thing to end by a misplaced press. */}
      <ContextMenu
        class={styles.tabActions!}
        name="Terminal actions"
        at={pointed()}
        close={() => setPointed(null)}
      >
        {() => (
          <button
            type="button"
            role="menuitem"
            onClick={() => close(pointed()!.tab)}
          >
            Close
          </button>
        )}
      </ContextMenu>

      <Switch fallback={<Empty>Opening a terminal…</Empty>}>
        <Match when={terminals.isError}>
          <ErrorLine>
            Could not read this conversation's terminals:{" "}
            {terminals.error?.message}
          </ErrorLine>
        </Match>
        <Match when={tabs().length > 0}>
          <For each={tabs()}>
            {(tab) =>
              tab > 0 ? (
                <Attached
                  at={terminalSocket(props.conversation.id, tab)}
                  class={shell.paneWide}
                  showing={showing() === tab}
                  over={over()[tab]}
                  ended={() => ended(tab)}
                  say={{
                    waiting: "Starting a shell in this conversation's worktree…",
                    lost: "The connection to this terminal was lost.",
                  }}
                />
              ) : (
                // A tab the server never opened a shell for has no grid to
                // stand under the sentence, and nothing to attach to: the
                // refusal is the whole of it.
                <Show when={showing() === tab}>
                  <ErrorLine>{over()[tab]}</ErrorLine>
                </Show>
              )
            }
          </For>
        </Match>
      </Switch>
    </>
  );
}
