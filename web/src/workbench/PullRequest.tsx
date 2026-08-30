//! The pull request, opened: what is on it right now, in the details pane.
//!
//! Fetched here rather than carried by the conversation, and for a stronger
//! version of the reason a commit's diff is: the server answers this by asking
//! GitHub through the host's `gh`, so a timeline that carried it would make an
//! API call every time the page heard the world moved.
//!
//! What it shows is a reading of GitHub as it stands — the commits the PR
//! carries, what GitHub is running against it and everything said on it — rather
//! than anything Verkstead wrote down. The three facts that *are* written down
//! are on the pinned event above: the number, the title and the way out to
//! GitHub itself.
//!
//! The checks are the part the card above has only one icon for. Here each of
//! them is named, marked with the same three shapes, and linked to its own run —
//! which is what a red suite is read by, the failure itself being on GitHub's
//! side of the wire.
//!
//! The comments arrive as HTML the server rendered and sanitized. They are
//! markdown from whoever can reach the repository, which is the strongest reason
//! on this page for the rendering to happen on the other side of the wire.

import { For, Match, Show, Switch, type JSX } from "solid-js";

import { PaneSticky } from "../Panes";
import { loadPullRequest } from "../api/client";
import type { ConversationView, PullRequestEvent } from "../api/types";
import { useReading } from "../freshness";
import { Empty, ErrorLine } from "../notices";
import { utcStamp } from "../set/when";
import { CheckMark, SAID } from "./Checks";
import { PaneHead } from "./PaneHead";
import styles from "./PullRequest.module.css";
import { ABBREVIATED } from "./Timeline";

export function PullRequest(props: {
  conversation: ConversationView;
  opened: PullRequestEvent;
  back: () => void;
}): JSX.Element {
  const carried = useReading(() => ({
    // The event is in the key, as a commit's diff is: opening another
    // conversation's pull request is another query rather than this one showing
    // the wrong commits for a moment.
    queryKey: ["pull-request", props.conversation.id, props.opened.id],
    queryFn: () => loadPullRequest(props.conversation.id, props.opened.id),

    // Merged rather than frozen: a pull request is the one payload here that
    // somebody else is still writing, so an open pane has to follow commits
    // landing, checks turning green and comments arriving. Keyed by the commit's
    // `sha`, the only identifier any of the three lists carries — a comment and
    // a check have none, so both are matched by position. What the merge saves
    // is the `innerHTML` below: a comment whose markup did not change keeps the
    // node it was rendered into.
    freshness: { reconcile: "sha" },
  }));

  return (
    <>
      <PaneSticky>
        <PaneHead back={{ to: "Timeline", go: props.back }} title="Pull request" />
      </PaneSticky>

      <div class={styles.summary}>
        <p class={styles.title}>{props.opened.title}</p>
        <p class={styles.where}>
          <span class={styles.number}>#{props.opened.number}</span>
          <a href={props.opened.url} target="_blank" rel="noreferrer">
            {props.opened.url}
          </a>
        </p>
      </div>

      <Switch>
        <Match when={carried.isPending}>
          <Empty>Loading…</Empty>
        </Match>
        {/* The server's own wording, which is the thing to act on: no `gh` on
            the machine, an account not logged in, and a GitHub that would not
            answer are three different afternoons. */}
        <Match when={carried.isError}>
          <ErrorLine>
            Could not read this pull request: {carried.error?.message}
          </ErrorLine>
        </Match>
        <Match when={carried.data}>
          {(read) => (
            <>
              <section class={styles.commits} aria-label="Commits">
                <h2>Commits</h2>
                <Show
                  when={read().commits.length > 0}
                  fallback={<Empty>Nothing is on it yet.</Empty>}
                >
                  <ol class={styles.carried}>
                    <For each={read().commits}>
                      {(commit) => (
                        <li>
                          <span class={styles.sha}>
                            {commit.sha.slice(0, ABBREVIATED)}
                          </span>
                          <span class={styles.subject}>{commit.subject}</span>
                        </li>
                      )}
                    </For>
                  </ol>
                </Show>
              </section>

              <section class={styles.checks} aria-label="Checks">
                <h2>Checks</h2>
                <Show
                  when={read().checks.length > 0}
                  fallback={<Empty>Nothing is running against it.</Empty>}
                >
                  <ul class={styles.ran}>
                    <For each={read().checks}>
                      {(check) => (
                        <li>
                          <CheckMark how={check.how} spoken={SAID[check.how]} />
                          {/* The name links to the run where GitHub gave one,
                              and is plain text where it gave none: a check with
                              nothing to follow is still a check to name. */}
                          <Show
                            when={check.link !== ""}
                            fallback={
                              <span class={styles.check}>{check.name}</span>
                            }
                          >
                            <a
                              class={styles.check}
                              href={check.link}
                              target="_blank"
                              rel="noreferrer"
                            >
                              {check.name}
                            </a>
                          </Show>
                        </li>
                      )}
                    </For>
                  </ul>
                </Show>
              </section>

              <section class={styles.comments} aria-label="Comments">
                <h2>Comments</h2>
                <Show
                  when={read().comments.length > 0}
                  fallback={<Empty>Nobody has said anything.</Empty>}
                >
                  <ol class={styles.said}>
                    <For each={read().comments}>
                      {(comment) => (
                        <li>
                          <p class={styles.saidBy}>
                            <span class={styles.author}>
                              {comment.author === ""
                                ? "somebody since gone"
                                : comment.author}
                            </span>
                            {/* The stamp GitHub gave it, said as the stamp on a
                                settled set is: one clock, in UTC, wherever the
                                comment was written. */}
                            <span>{utcStamp(comment.at)}</span>
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
