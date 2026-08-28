//! One commit, opened: what it said about itself and its diff, in the details
//! pane.
//!
//! Fetched here rather than carried by the Conversation, for the reason a
//! Capture is: a commit is worth reading whole when somebody opens the one event
//! it belongs to, and the timeline is read again every time the page hears the
//! world moved.
//!
//! The diff arrives as HTML the server already rendered — parsed per file,
//! highlighted, and folded — which is the same diff renderer the question sets'
//! attached diff goes through. So there is nothing here to render, no diff
//! parser in the browser, and one place where a diff is decided to look the way
//! it does.
//!
//! The Message above it is the agent's own account of the commit: its message
//! body, with git's trailers off it, rendered and sanitized on the server like
//! every other document on a timeline. So there is no markdown parser here
//! either — and a commit that carried none draws the pane as it always did.
//! Drawn by the same component a Set's Preface is — `Card` — because it is the
//! same kind of thing read the same way: the agent's markdown, in one padded
//! card under a heading the table of contents offers a way to. So it spans the
//! pane's column with the Gutter hanging off its left, and a wide Diagram in it
//! bleeds back across that Gutter exactly as one in a Preface does. They were
//! the same box copied into two stylesheets until they were one component,
//! which is how they came to look unalike in the first place.
//!
//! The one thing that is drawn here is a Diagram in that Message, which is the
//! Set page's own arrangement: the server leaves the source block, the client
//! draws over it, and a pane whose Message holds none never asks for mermaid.
//!
//! The file list down its margin is the Set page's own table of contents, drawn
//! from the paths that travel beside the rendered markup: the same entries, the
//! same scroll-spy and the same jump into a folded file. Which shape it takes is
//! the pane's width's answer — see `set/Contents`. It sits directly under what
//! the pane says the commit is, above everything it lists, which is where a Set
//! puts its own: the sidebar is pinned from where it stands in the flow, so a
//! nav written below the Message would start level with the diff and leave the
//! margin beside the Message empty.
//!
//! What the commit was called comes off the event rather than out of the diff.
//! The diff arrives headerless on purpose: the renderer splits on `diff --git`,
//! so a commit header above the first file would be dropped rather than shown.

import {
  Match,
  Show,
  Switch,
  createEffect,
  createMemo,
  createSignal,
  on,
  onCleanup,
  type JSX,
} from "solid-js";

import app from "../App.module.css";
import { Card } from "../Card";
import { Switch as Toggle } from "../Switch";
import { loadCommitPane } from "../api/client";
import type { CommitEvent, ConversationView } from "../api/types";
import { setWrapping, wrapping } from "../device";
import { useReading } from "../freshness";
import { Empty, ErrorLine } from "../notices";
import { Contents, navigation } from "../set/Contents";
import { drawDiagrams } from "../set/diagrams";
import type { Section } from "../set/outline";
// The Diff section, drawn as the Set page draws it: the box, the column of
// files, and what the wrap switch does to the lines inside them. One definition
// wherever a Diff is read — the renderer's own markup inside it is global.
import diffStyles from "../set/Diff.module.css";
import { files, spied } from "../set/outline";
import styles from "./Commit.module.css";
import { PaneHead } from "./PaneHead";
import { ABBREVIATED } from "./Timeline";

/// What the diff section is reached by, from the nav's own heading line. Its
/// own name rather than the Set page's `diff`, because a commit's pane and a
/// Set's page can be open at once and an id names one element.
const DIFF = "commit-diff";

/// And what the Message above it is reached by, for the same reason.
const MESSAGE = "commit-message";

/// What the commit said about itself, put in the page and — where it holds one
/// — drawn.
///
/// The card is the shared one, so what the section looks like is settled in one
/// place for this and a Set's Preface both. What is this component's own is the
/// drawing, and it is worked out from the message rather than from the pane: the
/// message arrives with the fetch, and a pane that reached for the renderer on
/// its own mount would be reaching before there was anything to draw over.
///
/// The renderer is turned loose on this block alone rather than on the document,
/// because a Set's page can be open behind the workbench and its Diagrams are its
/// own to draw.
function Message(props: { html: string; diagrams: boolean }): JSX.Element {
  let block!: HTMLDivElement;

  // On the message that is in the block, rather than on this component's mount:
  // opening a second commit is not a second mount. Neither the `Show` this sits
  // under nor the `Match` holding the whole pane is keyed, so the next commit's
  // markup is assigned into the block of the component the first one built — and
  // a draw hung on `onMount` would have happened once, leaving the server's
  // source block standing where the second commit's Diagram belongs.
  //
  // Following the HTML rather than the commit, because the HTML is the thing
  // being drawn over: assigning it is a render effect, and those are all through
  // before the first of these runs.
  createEffect(
    on(
      () => props.html,
      () => {
        if (!props.diagrams) {
          return;
        }

        // Stopped when the next message arrives as much as when the pane goes: a
        // drawing nobody stopped is still watching the colour scheme, and would
        // go on redrawing nodes this block no longer holds.
        onCleanup(drawDiagrams({ root: block }));
      },
    ),
  );

  return (
    <Card
      anchor={MESSAGE}
      heading="Message"
      html={props.html}
      ref={(body) => (block = body)}
    />
  );
}

export function Commit(props: {
  conversation: ConversationView;
  commit: CommitEvent;
  back: () => void;
}): JSX.Element {
  const opened = useReading(() => ({
    // The event is in the key, so opening another commit is another query
    // rather than the same one showing the wrong commit for a moment.
    queryKey: ["commit", props.conversation.id, props.commit.id],
    queryFn: () => loadCommitPane(props.conversation.id, props.commit.id),

    // A commit cannot change, so it is read once and never again.
    // "static" and not a finite time: a Nudge invalidates every active query,
    // and invalidation beats any staleTime that is not this one. A re-read
    // would reassign the `innerHTML` below whether or not a byte changed —
    // that assignment compiles to an unguarded effect over the query's data —
    // and close every per-file fold the reader had opened with it.
    freshness: "static",
  }));

  // How this device wants diffs drawn — the same setting a question set's
  // attached diff is read with, because it is a setting about reading diffs
  // rather than about any one page.
  const [wrapped, setWrapped] = createSignal(wrapping());

  const flip = (on: boolean) => {
    setWrapped(on);
    setWrapping(on);
  };

  // The sections this pane is made of, in the order it is read: what the commit
  // said about itself, and then the diff with every fold in it. Worked out from
  // the pane rather than from a Set — but the entries are the Set page's own,
  // off the same paths and pointing at the same renderer-stamped anchors.
  //
  // A commit that said nothing has no Message line, exactly as it has no message
  // to jump to; one that changed nothing has no Diff line either, and a pane with
  // neither gets no nav at all.
  const sections = createMemo((): Section[] => {
    const pane = opened.data;
    const listed: Section[] = [];

    if (pane?.summary) {
      listed.push({ anchor: MESSAGE, name: "Message", entries: [] });
    }

    const view = pane?.diff;
    if (view !== null && view !== undefined) {
      listed.push({ anchor: DIFF, name: "Diff", entries: files(view) });
    }

    return listed;
  });

  const watched = createMemo(() => spied(sections()));

  const nav = navigation();

  return (
    <>
      <PaneHead back={{ to: "Timeline", go: props.back }} title="Commit" />

      <div class={styles.header}>
        <p class={styles.subject}>{props.commit.subject}</p>
        <p class={styles.changed}>
          <span class={styles.sha}>{props.commit.sha.slice(0, ABBREVIATED)}</span>
          <span>
            {props.commit.files} {props.commit.files === 1 ? "file" : "files"}
          </span>
          <span class={styles.added}>+{props.commit.insertions}</span>
          <span class={styles.removed}>−{props.commit.deletions}</span>
        </p>
      </div>

      {/* Above everything it lists, which is where a Set's page puts its own:
          the stylesheet takes it out of the flow and hangs it in the pane's
          margin from where it stands here, so this is the top of the sidebar as
          well as a place in the reading order. Written below the Message it
          would have started level with the diff, leaving the margin beside the
          Message empty. A commit with neither a message nor a file changed has
          nothing to list, and gets no nav. */}
      <Show when={sections().length > 0}>
        <Contents sections={sections()} watched={watched()} nav={nav} paned />
      </Show>

      {/* Between the header and the diff, which is the order it is read in:
          what the commit says about itself, then what it changed. A commit that
          said nothing — a bookkeeping one, or any commit recorded before
          summaries were kept — has nothing here at all. */}
      <Show when={opened.data?.summary}>
        {(summary) => (
          <Message html={summary()} diagrams={opened.data?.diagrams ?? false} />
        )}
      </Show>

      <Switch>
        <Match when={opened.isPending}>
          <Empty>Loading…</Empty>
        </Match>
        <Match when={opened.isError}>
          <ErrorLine>
            Could not read this commit: {opened.error?.message}
          </ErrorLine>
        </Match>
        <Match when={opened.data}>
          {(read) => (
            <Show
              when={read().diff}
              fallback={<Empty>This commit changed no files.</Empty>}
            >
              {(diff) => (
                <section
                  class={
                    wrapped()
                      ? `${diffStyles.diff} ${diffStyles.wrapped}`
                      : diffStyles.diff
                  }
                  id={DIFF}
                >
                  <div class={app.sectionHead}>
                    <h2 class={app.sectionHeading}>Diff</h2>
                    <Toggle label="Word wrap" on={wrapped()} flip={flip} />
                  </div>
                  {/* The per-file folds and their anchors are stamped by the
                      renderer, since this arrives already rendered. */}
                  <div class={diffStyles.diffFiles} innerHTML={diff().html} />
                </section>
              )}
            </Show>
          )}
        </Match>
      </Switch>
    </>
  );
}
