//! One commit, opened: its diff, in the details pane.
//!
//! Fetched here rather than carried by the Conversation, for the reason a
//! transcript is: a diff is worth reading when somebody opens the one event it
//! belongs to, and the timeline is read again every time the page hears the
//! world moved.
//!
//! It arrives as HTML the server already rendered — parsed per file,
//! highlighted, and folded — which is the same diff renderer the question sets'
//! attached diff goes through. So there is nothing here to render, no diff
//! parser in the browser, and one place where a diff is decided to look the way
//! it does.
//!
//! What the commit was called comes off the event rather than out of the diff.
//! The diff arrives headerless on purpose: the renderer splits on `diff --git`,
//! so a commit header above the first file would be dropped rather than shown.

import { useQuery } from "@tanstack/solid-query";
import { Match, Show, Switch, createSignal, type JSX } from "solid-js";

import { Switch as Toggle } from "../Switch";
import { loadCommitDiff } from "../api/client";
import type { CommitEvent, ConversationView } from "../api/types";
import { setWrapping, wrapping } from "../device";
import { ABBREVIATED } from "./Timeline";

export function Commit(props: {
  conversation: ConversationView;
  commit: CommitEvent;
  back: () => void;
  close: () => void;
}): JSX.Element {
  const diff = useQuery(() => ({
    // The event is in the key, so opening another commit is another query
    // rather than the same one showing the wrong diff for a moment.
    queryKey: ["commit", props.conversation.id, props.commit.id],
    queryFn: () => loadCommitDiff(props.conversation.id, props.commit.id),
  }));

  // How this device wants diffs drawn — the same setting a question set's
  // attached diff is read with, because it is a setting about reading diffs
  // rather than about any one page.
  const [wrapped, setWrapped] = createSignal(wrapping());

  const flip = (on: boolean) => {
    setWrapped(on);
    setWrapping(on);
  };

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
                  id="commit-diff"
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
