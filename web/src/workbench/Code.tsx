//! **Code**: the Conversation's editor, which so far is its terminals.
//!
//! Shells of the human's inside its Sandbox, with its Worktree as the working
//! directory — for the moment the agent's work is done and somebody wants to
//! try it, make a small change or work with git, without leaving the workbench
//! and without the run noticing
//! ([ADR 0013](../../../docs/adr/0013-conversation-terminals.md)).
//!
//! **The pane the Terminal pane became**
//! ([ADR 0019](../../../docs/adr/0019-the-code-pane.md)). What it grows into is
//! a file tree over every Worktree the Conversation has, with a group of tabs
//! beside it holding files and terminals alike; what is here is that group's
//! terminals, under the new name and the new path and otherwise untouched. Two
//! panes, one holding shells and one holding shells and files, would be the
//! same thing drawn twice — so there is no Terminal pane any more, and the path
//! it stood at redirects here (see `App.tsx`).
//!
//! Opened by the code icon on the Timeline's header — see `Timeline.tsx` —
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
//! answer to this shape — restyled after VS Code's bar, which is what the pane
//! is drawn after from here on: a kind icon at one end of every tab and a × at
//! the other, and the tabs abutting rather than spaced. The kind is the whole
//! of what an icon there says, and the one kind there is so far is a terminal.
//!
//! The bar is drawn where there are tabs to draw. A strip holding nothing but
//! its own plus is furniture about tabs that are not there, and a pane with
//! nothing open has the hint under it to say the same thing in words.
//!
//! **And a tab is called what its shell calls itself.** A prompt sets the
//! terminal's title at every prompt — the directory it is in, the command it is
//! running — and that is a better name for a tab than anything this side could
//! invent, so xterm's reading of the title escape is the label. Where the shell
//! has set none, or has cleared the one it set, the tab falls back to
//! *Terminal N* by the number the server gave it, which is why those numbers are
//! never reused.
//!
//! A fresh attach starts from the number again: a repaint carries the grid and
//! not the title, so a pane that has just loaded knows the number and nothing
//! else until the shell next says a name — which, for most prompts, is the next
//! time it draws one.
//!
//! Every tab keeps its socket open and its grid mounted whether or not it is the
//! one showing, so a shell that printed while somebody was reading another tab
//! has printed it by the time they turn back. Only the tab showing measures the
//! pane and says so up its socket — see [`./Attached`], where the hiding, the
//! measuring and the focus all are.
//!
//! **The server holds the shells, so this pane is the way back to them rather
//! than where they live.** On load it asks which of the Conversation's terminals
//! are live and draws a tab for each — so a reload, a second device or a tab
//! closed by accident comes back to what was already there, still running and
//! showing what it last showed.
//!
//! **And a tab is closed by the × at its end.** ADR 0013 kept Close on a context
//! menu, a × beside a label this small being a thing to hit by accident and what
//! it would end a shell somebody is working in; ADR 0019 puts it on the tab,
//! because the × is what VS Code's bar has, what carries that worry now is the
//! confirm a *busy* shell asks for, and a long press with no menu behind it is
//! what frees the gesture for dragging a tab. The press asks the server to end
//! that shell, and the tab then goes the way every ended shell's tab goes: its
//! socket closes, and the tab closes with it.
//!
//! **And the pane opens empty.** It opens no shell of its own accord: the live
//! ones come back as tabs, and where there are none it draws a hint and a **New
//! terminal** button where a tab's content goes. The Terminal pane never stood
//! empty because a shell was the whole of what it held; a pane that will hold
//! files has something to show without one, and a shell nobody asked for is a
//! Sandbox started by the opening of a pane. A shell that exits closes its
//! socket, which is how this side hears about it: the tab goes, and where it was
//! the last the pane stands empty again.
//!
//! The guard on a shell that could not start stays, and is time — a shell that
//! ended within [`AT_ONCE`] of being asked for, or that the server refused to
//! open at all, is one that could not start rather than one that ran, so its tab
//! *stays* saying why, and **New terminal** replaces it. Without it a Sandbox
//! that will not start would leave an empty pane and nothing to read about why.
//! The clock is this pane's own, started when it asked: the server opens nothing
//! of its own accord and knows nothing about tabs.
//!
//! Nothing here is a record: no Capture, no Event on the Timeline, nothing in a
//! Share. And nothing here holds the run off — typing into a terminal is the
//! human doing something, exactly as typing into a Screen is, and somebody who
//! means to take the work on presses **Stop** first.

import {
  faPlus,
  faTerminal,
  faXmark,
} from "@fortawesome/free-solid-svg-icons";
import { useQueryClient } from "@tanstack/solid-query";
import {
  For,
  Match,
  Show,
  Switch,
  createEffect,
  createMemo,
  createSignal,
  onCleanup,
  type JSX,
} from "solid-js";

import { Icon } from "../Icon";
import { IconButton } from "../IconButton";
import { PaneSticky } from "../Panes";
import { QuietButton } from "../QuietButton";
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
import styles from "./Code.module.css";
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
  Refused: "The shell would not start. The server's log says why.",
};

/// How many lines a tab keeps above the grid, so that what scrolled past can be
/// read back.
///
/// The server holds the grid alone — a terminal is memory only, and a repaint is
/// the whole of what a fresh attach is given (ADR 0013) — so this is the tab's
/// own memory of watching and goes when the tab does. A Screen keeps none, and
/// is right to: what an agent printed is in the Transcript beside it and the
/// Capture under it. Nothing writes down what a shell printed, and a human who
/// has just run a build in one wants to read it.
///
/// Enough for a build or a test run and not much more, because it is held per
/// tab and a pane may have several. What a repaint clears with it is this and
/// nothing of the server's — see [`./Attached`].
export const SCROLLBACK = 5_000;

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
  "The shell ended as soon as it started. Press New terminal to open another.";

/// And what the pane says when it is holding nothing at all.
///
/// The state the Terminal pane never had. What it says is what there is here to
/// open, which for now is a shell — the tree and the files it opens are the
/// stages after this one, and a hint naming a tree that is not drawn yet would
/// be a sentence about somewhere else.
export const NOTHING_OPEN =
  "Nothing is open. A terminal here is a shell of your own in this conversation's worktree.";

export function Code(props: {
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
  ///
  /// **And kept no longer than the pane it was read for.** Frozen means frozen:
  /// an answer still in the cache is never asked for again, on a mount or on a
  /// focus. So a pane coming back to a cached one would be reading the register
  /// as it stood when this pane last closed — and where the shell was opened in
  /// this pane, that is the empty list it started from, so it would come back to
  /// no tabs at all and leave the shell running with nothing over it and no way
  /// to close it. Dropped with the pane instead, which is what makes "read when
  /// the pane opens" true of every opening rather than of the first.
  ///
  /// Said twice, because `gcTime` is a timer: it is zero, so nothing is kept
  /// past the pane, and the answer is dropped outright as the pane goes so that
  /// a pane reopened before that timer has run — which is a press that swaps one
  /// details pane for another, both in the one tick — finds nothing to come
  /// back to either.
  const held = useQueryClient();
  const holding = () => ["terminals", props.conversation.id];

  const terminals = useReading(() => ({
    queryKey: holding(),
    queryFn: () => listTerminals(props.conversation.id),
    freshness: "static",
    gcTime: 0,
  }));

  onCleanup(() => held.removeQueries({ queryKey: holding(), exact: true }));

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

  /// What each tab's shell has called itself, where it has called itself
  /// anything: the title it last set, which is what the tab is labelled by.
  ///
  /// Kept here rather than in the window that heard it, because a name is a
  /// thing about the tab bar and the window drawing the grid may be hidden. A
  /// tab that has gone may leave an entry behind it, which names nothing: the
  /// server never reuses a number, so nothing can ever come back under it.
  const [titles, setTitles] = createSignal<Record<number, string>>({});

  /// Which tab the human turned to, where they have turned to one.
  const [chosen, setChosen] = createSignal<number | undefined>();

  /// Whether the list has been read, which is what says the pane knows how many
  /// terminals there are. Before it, a pane with no tabs is one that has not
  /// looked yet rather than a Conversation with no shells — so the hint waits on
  /// this, a sentence about an empty pane being a thing to say once somebody has
  /// looked.
  const [read, setRead] = createSignal(false);

  /// When this pane asked for each terminal it opened, which is what
  /// [`AT_ONCE`] is measured from. Nothing for the ones that were already live:
  /// this pane never asked for those, so a shell of theirs that ends is one that
  /// ran.
  const askedAt = new Map<number, number>();

  /// Whether an open is in flight, so that a second press while the first is
  /// still being answered does not open two shells.
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

  /// What a tab is called. What its shell last called itself, where it has
  /// called itself anything at all: a title of nothing but spaces is a shell
  /// clearing its name rather than setting a blank one, and reads as none.
  ///
  /// Failing that, the number the server issued it — and the bare word for one
  /// standing on an open that was refused, the server never having got as far as
  /// a number for that one, where a made-up one would be a name for a shell that
  /// is not there.
  const called = (tab: number): string => {
    const said = titles()[tab]?.trim();

    if (said) {
      return said;
    }

    return tab > 0 ? `Terminal ${tab}` : "Terminal";
  };

  /// Take away whatever is only standing there to say why, which is what **New
  /// terminal** replacing one means: a tab that is a sentence about a shell that
  /// never started is not something to keep beside a shell that has.
  const replace = (): void => {
    const standing = Object.keys(over()).map(Number);

    if (standing.length === 0) {
      return;
    }

    setOver({});
    setTabs((was) => was.filter((one) => !standing.includes(one)));
  };

  /// A tab that says why there is no shell in it, which stands until somebody
  /// asks for another.
  ///
  /// A sentence rather than nothing at all: a pane that simply went back to
  /// empty would say the same thing about a Sandbox that will not start as it
  /// says about a Conversation nobody has opened a shell in.
  const stand = (why: string): void => {
    replace();

    refusals -= 1;
    const tab = refusals;

    setTabs((was) => [...was, tab]);
    setOver((was) => ({ ...was, [tab]: why }));
    setChosen(tab);
  };

  /// Open another, and show it. What **New terminal** does, from the plus at the
  /// end of the strip or from the press in the empty pane — the only two things
  /// that ask for a shell, the pane asking for none of its own accord.
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
  /// where it was the last the pane stands empty behind it; one that ended
  /// inside [`AT_ONCE`] of being asked for could not start, so its tab stays
  /// saying so until somebody asks for another.
  const ended = (tab: number): void => {
    const asked = askedAt.get(tab);

    if (asked === undefined || Date.now() - asked >= AT_ONCE) {
      askedAt.delete(tab);
      setTabs((was) => was.filter((one) => one !== tab));
      return;
    }

    setOver((was) => ({ ...was, [tab]: ENDED_AT_ONCE }));
  };

  /// Close one, which is what the × at the end of a tab does.
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

  return (
    <>
      <PaneSticky>
        <PaneHead back={{ to: "Timeline", go: props.back }} title="Code">
          {/* The tabs beside the title, where a pane's own controls go, and the
              way to another at the end of them. Buttons that say which they are
              rather than tabs: they are all always there, `aria-pressed` is the
              one word that says which is showing, and what each one does is
              show a grid that is already drawn.

              Drawn where there are tabs. A strip holding nothing but its own
              plus says nothing the empty state under it does not say in
              words. */}
          <Show when={tabs().length > 0}>
            <div
              class={styles.tabs}
              role="group"
              aria-label="This conversation's terminals"
            >
              <For each={tabs()}>
                {(tab) => (
                  // Two buttons rather than one: the tab is pressed to show
                  // what it holds and the × is pressed to close it, and a
                  // button inside a button is not a thing a browser draws. The
                  // frame around them is the tab as the eye reads it, and is
                  // what takes the fill of the one showing.
                  <div class={styles.tabFrame}>
                    <button
                      type="button"
                      class={styles.tab}
                      aria-pressed={showing() === tab}
                      onClick={() => setChosen(tab)}
                    >
                      {/* What kind of thing the tab holds, which is the one
                          thing an icon at that end says. Files bring their own
                          when there are files. */}
                      <Icon of={faTerminal} class={styles.kind} />
                      <span class={styles.name}>{called(tab)}</span>
                    </button>

                    {/* And the way to end it. Called by the tab it would close:
                        an icon says nothing when it is read aloud, and a row of
                        these all saying "Close" would say nothing about
                        which. */}
                    <button
                      type="button"
                      class={styles.close}
                      aria-label={`Close ${called(tab)}`}
                      onClick={() => close(tab)}
                    >
                      <Icon of={faXmark} />
                    </button>
                  </div>
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
          </Show>
        </PaneHead>
      </PaneSticky>

      <Switch fallback={<Empty>Reading this conversation's terminals…</Empty>}>
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
                  scrollback={SCROLLBACK}
                  over={over()[tab]}
                  titled={(title) =>
                    setTitles((was) => ({ ...was, [tab]: title }))
                  }
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

        {/* And the pane with nothing in it, which is where a Conversation with
            no shells running lands and where the last of them leaves it. Drawn
            where a tab's content goes rather than over the whole pane: the tree
            stands beside this, and a pane-wide notice would be a sentence over
            that too. */}
        <Match when={read()}>
          <div class={styles.nothing}>
            <Empty>{NOTHING_OPEN}</Empty>
            <QuietButton onClick={() => void open()}>New terminal</QuietButton>
          </div>
        </Match>
      </Switch>
    </>
  );
}
