//! The pull request, opened: what is on it right now, in the details pane.
//!
//! Fetched here rather than carried by the conversation, and for a stronger
//! version of the reason a commit's diff is: the server answers this by asking
//! GitHub through the host's `gh`, so a timeline that carried it would make an
//! API call every time the page heard the world moved.
//!
//! What it shows is a reading of GitHub as it stands — the commits the PR
//! carries and everything said on it — rather than anything Verkstead wrote
//! down. The three facts that *are* written down are on the pinned event above:
//! the number, the title and the way out to GitHub itself.
//!
//! The comments arrive as HTML the server rendered and sanitized. They are
//! markdown from whoever can reach the repository, which is the strongest reason
//! on this page for the rendering to happen on the other side of the wire.

import { useQuery } from "@tanstack/solid-query";
import { For, Match, Show, Switch, type JSX } from "solid-js";

import { loadPullRequest } from "../api/client";
import type { ConversationView, PullRequestEvent } from "../api/types";
import { utcStamp } from "../set/when";
import { ABBREVIATED } from "./Timeline";

export function PullRequest(props: {
  conversation: ConversationView;
  opened: PullRequestEvent;
  back: () => void;
  close: () => void;
}): JSX.Element {
  const carried = useQuery(() => ({
    // The event is in the key, as a commit's diff is: opening another
    // conversation's pull request is another query rather than this one showing
    // the wrong commits for a moment.
    queryKey: ["pull-request", props.conversation.id, props.opened.id],
    queryFn: () => loadPullRequest(props.conversation.id, props.opened.id),
  }));

  return (
    <>
      <div class="pane-head">
        <button type="button" class="pane-back" onClick={props.back}>
          ← Timeline
        </button>
        <h1>Pull request</h1>
        <button type="button" class="close-event" onClick={props.close}>
          Close
        </button>
      </div>

      <div class="pull-request-summary">
        <p class="title">{props.opened.title}</p>
        <p class="where">
          <span class="number">#{props.opened.number}</span>
          <a href={props.opened.url} target="_blank" rel="noreferrer">
            {props.opened.url}
          </a>
        </p>
      </div>

      <Switch>
        <Match when={carried.isPending}>
          <p class="empty">Loading…</p>
        </Match>
        {/* The server's own wording, which is the thing to act on: no `gh` on
            the machine, an account not logged in, and a GitHub that would not
            answer are three different afternoons. */}
        <Match when={carried.isError}>
          <p class="error">
            Could not read this pull request: {carried.error?.message}
          </p>
        </Match>
        <Match when={carried.data}>
          {(read) => (
            <>
              <section class="pr-commits" aria-label="Commits">
                <h2>Commits</h2>
                <Show
                  when={read().commits.length > 0}
                  fallback={<p class="empty">Nothing is on it yet.</p>}
                >
                  <ol class="commits">
                    <For each={read().commits}>
                      {(commit) => (
                        <li>
                          <span class="sha">
                            {commit.sha.slice(0, ABBREVIATED)}
                          </span>
                          <span class="subject">{commit.subject}</span>
                        </li>
                      )}
                    </For>
                  </ol>
                </Show>
              </section>

              <section class="pr-comments" aria-label="Comments">
                <h2>Comments</h2>
                <Show
                  when={read().comments.length > 0}
                  fallback={<p class="empty">Nobody has said anything.</p>}
                >
                  <ol class="comments">
                    <For each={read().comments}>
                      {(comment) => (
                        <li>
                          <p class="said-by">
                            <span class="author">
                              {comment.author === ""
                                ? "somebody since gone"
                                : comment.author}
                            </span>
                            {/* The stamp GitHub gave it, said as the stamp on a
                                settled set is: one clock, in UTC, wherever the
                                comment was written. */}
                            <span class="when">{utcStamp(comment.at)}</span>
                          </p>
                          <div class="markdown" innerHTML={comment.html} />
                        </li>
                      )}
                    </For>
                  </ol>
                </Show>
              </section>
            </>
          )}
        </Match>
      </Switch>
    </>
  );
}
