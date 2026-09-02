//! The pull request, opened: what is on it right now, in the details pane.
//!
//! Fetched here rather than carried by the conversation, and for a stronger
//! version of the reason a commit's diff is: the server answers this by asking
//! GitHub through the host's `gh`, so a timeline that carried it would make an
//! API call every time the page heard the world moved.
//!
//! What it shows is a reading of GitHub as it stands — the commits the PR
//! carries, what GitHub is running against it and everything said on it — rather
//! than anything Verkstead wrote down. The facts that *are* written down are on
//! the pinned event above: the number, the title, the way out to GitHub itself,
//! and whether the last look found the branch conflicting with its base.
//!
//! That last one is said here in words. It is the same recorded fact the card
//! draws its mark from, and this is the place with room to say what follows from
//! it — nothing lands until somebody resolves it.
//!
//! And, on a conversation Verkstead has finished with, the one press that gets
//! it resolved: back into the wrap-up, where a conflict is something the machine
//! already knows how to have a go at. Only there, and only while the fact says
//! the branch conflicts — a wrapping conversation is having that go as fast as
//! it can, and a button offering what is already happening would be theatre.
//!
//! The checks are the part the card above has only one icon for. Here each of
//! them is named, marked with the same three shapes, and linked to its own run —
//! which is what a red suite is read by, the failure itself being on GitHub's
//! side of the wire.
//!
//! The comments arrive as HTML the server rendered and sanitized. They are
//! markdown from whoever can reach the repository, which is the strongest reason
//! on this page for the rendering to happen on the other side of the wire.

import { useMutation, useQueryClient } from "@tanstack/solid-query";
import { For, Match, Show, Switch, createSignal, type JSX } from "solid-js";

import { PaneSticky } from "../Panes";
import { loadPullRequest, resolveConflicts } from "../api/client";
import type {
  ConversationView,
  PullRequestEvent,
  Resolved,
} from "../api/types";
import { useReading } from "../freshness";
import { Empty, ErrorLine } from "../notices";
import { utcStamp } from "../set/when";
import { CheckMark, SAID } from "./Checks";
import { IN_WORDS } from "./Merging";
import { PaneHead } from "./PaneHead";
import { NO_SESSIONS, noSessions } from "./sessions";
import styles from "./PullRequest.module.css";
import { ABBREVIATED } from "./Timeline";

/// Each way of being refused the press, in the words of what stopped it.
///
/// Two of them are the record having moved on under a page somebody left open —
/// the button is drawn off the record and the press is answered from it — and
/// neither is anything for the human to go and do, which is why the re-read
/// behind the sentence is as much of the answer as the sentence: the row goes
/// with it.
///
/// The other two are about the checkout the resolution session would work in. A
/// conversation stays finished with for as long as nobody merges its pull
/// request, which is time enough for a directory to go — and the press sees to
/// that before it moves anything, so what it says here is why nothing moved.
export const RESOLVE_REFUSAL: Record<Resolved, string> = {
  Resolving: "",
  NoSuchConversation: "This conversation is gone.",
  // The one refusal here that is about this Verkstead rather than about this
  // conversation: what the press starts again is the resolution session, and
  // there is none to start. The button is not drawn on such a build — see
  // `sessions.tsx` — so this is what a page drawn before the read answers a
  // press with.
  NotOnWindowsYet: NO_SESSIONS,
  NotDone:
    "This conversation is not finished with any more, so whatever is driving it has the conflict in hand.",
  NothingConflicts:
    "This pull request merges again, so there is nothing left to resolve.",
  NowhereToWork:
    "This conversation has no worktree recorded, so there is nowhere to resolve the conflict.",
  WorktreeRefused:
    "The worktree this conversation was done in has gone, and git would not make it again. The server's log says why.",
};

export function PullRequest(props: {
  conversation: ConversationView;
  opened: PullRequestEvent;
  back: () => void;
}): JSX.Element {
  const queries = useQueryClient();

  /// What the press was refused with, and `null` while nothing has been.
  const [refused, setRefused] = createSignal<Resolved | null>(null);

  /// The press itself. Whatever it came back with, the page is read again: what
  /// it did is a conversation that has moved into a wrap-up, and what refused it
  /// is a conversation that had moved already — and reading it again is the
  /// correction either way. The button goes with the state, so a press that
  /// landed takes its own row off the page.
  const resolving = useMutation(() => ({
    mutationFn: (id: number) => resolveConflicts(id),
    onSuccess: (outcome: Resolved) => {
      setRefused(outcome === "Resolving" ? null : outcome);

      void queries.invalidateQueries({ queryKey: ["conversation"] });
      void queries.invalidateQueries({ queryKey: ["conversations"] });
    },
  }));

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
        {/* And the conflict, where the last look at GitHub found one: the same
            recorded fact the card draws a mark for, said here in the words a
            mark has no room for. Off the pinned Event rather than the reading
            below, which is what makes the two one fact drawn twice — see
            `Merging.tsx`. In words alone and with no mark beside them: the mark
            on the card exists because a card has no room to say this, and an
            icon here would be the same sentence read out twice.

            Nothing at all where the pull request merges, where GitHub has not
            worked the answer out, and where nothing has asked. */}
        <Show when={props.opened.merging === "Conflicting"}>
          <p class={styles.conflict}>{IN_WORDS}</p>
        </Show>
        {/* And the press that gets it resolved, under the words saying there is
            a conflict — on a conversation Verkstead has finished with, which is
            the one state nothing is watching this pull request on behalf of. A
            wrapping conversation has the watchers on it already and is
            resolving the conflict as fast as it can; a press there would be a
            button offering what is happening anyway.

            It sends the conversation back into its wrap-up with the review's
            settle left standing, so the resolution session goes out and nothing
            reads the branch a second time. Which is why it is not a steer: a
            steer into wrapping deliberately reads it again. */}
        <Show
          when={
            props.opened.merging === "Conflicting" &&
            props.conversation.state === "Done" &&
            // And never on a Verkstead with no session to resolve it: the press
            // would only be refused, and the line above it already says the
            // branch conflicts. See `sessions.tsx`.
            !noSessions(props.conversation)
          }
        >
          <button
            type="button"
            class={styles.resolve}
            disabled={resolving.isPending}
            onClick={() => resolving.mutate(props.conversation.id)}
          >
            {resolving.isPending ? "Resolving…" : "Resolve conflicts"}
          </button>

          {/* What a press that did nothing says. Both refusals are the record
              having moved on under a page that was drawn a while ago, so the
              re-read behind them is as much of the answer as the sentence is. */}
          <Show when={refused()}>
            {(outcome) => (
              <ErrorLine class={styles.refused}>
                {RESOLVE_REFUSAL[outcome()]}
              </ErrorLine>
            )}
          </Show>
          <Show when={resolving.isError}>
            <ErrorLine class={styles.refused}>
              The conflict could not be sent back to the wrap-up:{" "}
              {resolving.error?.message}
            </ErrorLine>
          </Show>
        </Show>
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
