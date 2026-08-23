//! One commit, opened: its diff, in the details pane.
//!
//! Fetched here rather than carried by the Conversation, for the reason a
//! Capture is: a diff is worth reading when somebody opens the one event it
//! belongs to, and the timeline is read again every time the page hears the
//! world moved.
//!
//! It arrives as HTML the server already rendered — parsed per file,
//! highlighted, and folded — which is the same diff renderer the question sets'
//! attached diff goes through. So there is nothing here to render, no diff
//! parser in the browser, and one place where a diff is decided to look the way
//! it does.
//!
//! The file list down its margin is the Set page's own table of contents, drawn
//! from the paths that travel beside the rendered markup: the same entries, the
//! same scroll-spy and the same jump into a folded file. Which shape it takes is
//! the pane's width's answer — see `set/Contents`.
//!
//! What the commit was called comes off the event rather than out of the diff.
//! The diff arrives headerless on purpose: the renderer splits on `diff --git`,
//! so a commit header above the first file would be dropped rather than shown.

import { Match, Show, Switch, createMemo, createSignal, type JSX } from "solid-js";

import { Switch as Toggle } from "../Switch";
import { loadCommitDiff } from "../api/client";
import type { CommitEvent, ConversationView } from "../api/types";
import { setWrapping, wrapping } from "../device";
import { useReading } from "../freshness";
import { Contents, navigation } from "../set/Contents";
import type { Section } from "../set/outline";
import { files, spied } from "../set/outline";
import { ABBREVIATED } from "./Timeline";

/// What the diff section is reached by, from the nav's own heading line. Its
/// own name rather than the Set page's `diff`, because a commit's pane and a
/// Set's page can be open at once and an id names one element.
const DIFF = "commit-diff";

export function Commit(props: {
  conversation: ConversationView;
  commit: CommitEvent;
  back: () => void;
  close: () => void;
}): JSX.Element {
  const diff = useReading(() => ({
    // The event is in the key, so opening another commit is another query
    // rather than the same one showing the wrong diff for a moment.
    queryKey: ["commit", props.conversation.id, props.commit.id],
    queryFn: () => loadCommitDiff(props.conversation.id, props.commit.id),

    // A commit's diff cannot change, so it is read once and never again.
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

  // The one section this pane is made of, and every fold in it. A commit's diff
  // is the whole of what is here, so the outline is that section and its files
  // rather than anything worked out from a Set — but the entries are the Set
  // page's own, off the same paths and pointing at the same renderer-stamped
  // anchors.
  const sections = createMemo((): Section[] => {
    const view = diff.data?.diff;
    return view === null || view === undefined
      ? []
      : [{ anchor: DIFF, name: "Diff", entries: files(view) }];
  });

  const watched = createMemo(() => spied(sections()));

  const nav = navigation();

  return (
    <>
      <div class="pane-head">
        <button type="button" class="pane-back" onClick={props.back}>
          ← Timeline
        </button>
        <h1>Commit</h1>
        {/* The way back to what the conversation is, which is what this pane
            shows when no event is open. */}
        <button type="button" class="close-event" onClick={props.close}>
          Close
        </button>
      </div>

      <div class="commit-summary">
        <p class="subject">{props.commit.subject}</p>
        <p class="changed">
          <span class="sha">{props.commit.sha.slice(0, ABBREVIATED)}</span>
          <span class="files">
            {props.commit.files} {props.commit.files === 1 ? "file" : "files"}
          </span>
          <span class="added">+{props.commit.insertions}</span>
          <span class="removed">−{props.commit.deletions}</span>
        </p>
      </div>

      {/* After the summary and before the diff, which is the order it is read
          in: what the commit is, then the way around what it changed. The
          stylesheet takes it out of the flow and puts it in the pane's margin
          where there is one. A commit that changed no files has no folds to
          list, and gets none. */}
      <Show when={sections().length > 0}>
        <Contents sections={sections()} watched={watched()} nav={nav} paned />
      </Show>

      <Switch>
        <Match when={diff.isPending}>
          <p class="empty">Loading…</p>
        </Match>
        <Match when={diff.isError}>
          <p class="error">Could not read this commit: {diff.error?.message}</p>
        </Match>
        <Match when={diff.data}>
          {(read) => (
            <Show
              when={read().diff}
              fallback={
                <p class="empty">This commit changed no files.</p>
              }
            >
              {(diff) => (
                <section
                  class={wrapped() ? "diff wrapped" : "diff"}
                  id={DIFF}
                >
                  <div class="section-head">
                    <h2 class="section-heading">Diff</h2>
                    <Toggle label="Word wrap" on={wrapped()} flip={flip} />
                  </div>
                  {/* The per-file folds and their anchors are stamped by the
                      renderer, since this arrives already rendered. */}
                  <div class="diff-files" innerHTML={diff().html} />
                </section>
              )}
            </Show>
          )}
        </Match>
      </Switch>
    </>
  );
}
