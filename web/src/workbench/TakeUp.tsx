//! The pull request a draft is holding, and what taking it up would do.
//!
//! The other way work gets into the pipeline: a pull request Verkstead did not
//! open — by hand, by a contributor, by the old tools — taken up at its wrap-up,
//! with the ordinary Wrapping loop running from there. Nothing about that loop
//! knows or cares who opened the pull request; what was missing was a door into
//! it.
//!
//! **The card is what both composers draw**, the compose page's and the draft's
//! own, over the same box. Which is why it is here rather than in either of
//! them: what a human reads about a pull request they are about to take up
//! should not depend on whether the Conversation exists yet.
//!
//! And it stands *over* the box rather than in place of it, which is the whole
//! difference from the roadmap card beside it (see [`Adoption`](./Adoption.tsx)).
//! An adopted stage's brief is the repository's own and arrives with the
//! adoption, so there is nothing to write; a pull request brings words of its
//! own — a title and a description — and it is the one thing taken up that the
//! human is likeliest to have something to add to. So the box stays a box, and
//! what is left in it is the Brief.
//!
//! **The press is here too**, under the box where `Start grilling` stands on
//! every other draft and never beside it. What it does is the whole of taking
//! one up: the head branch checked out — cut off origin's where this checkout
//! has none, moved on to origin's where it has an older copy — the pull request
//! recorded, and the ordinary wrap-up running from there. Every way it can be
//! refused is a different thing to go and do about it, which is why they are
//! named one at a time.

import { useMutation, useQueryClient } from "@tanstack/solid-query";
import { createSignal, Show, type JSX } from "solid-js";

import { takeUpPullRequest } from "../api/client";
import type { ConversationView, TakenUp } from "../api/types";
import { ErrorLine, Note } from "../notices";
import { keyOf, useDevice } from "../reaching";
import { companionRefusal } from "./Timeline";
import styles from "./TakeUp.module.css";

/// The pull request being held, as either composer names it.
///
/// Read off what that composer is holding rather than off GitHub. On the compose
/// page that is the row that was pressed; on a draft's own it is what the create
/// wrote down — and neither is asked again, a re-read being a `gh` call to name
/// something already on the screen. What GitHub says *now* is the take-up's
/// question.
export function HeldPullRequest(props: {
  /// What the Repo is called, because a number alone names nothing: `#41` is a
  /// different pull request in every repository.
  repo: string;
  number: number;
  title: string;
  /// Where it is, for the way out to GitHub itself.
  url: string;
  /// The branch the work is on, which is the branch the take-up checks out.
  head: string;
  /// And the branch it goes into.
  base: string;

  /// Put it down, where there is anywhere to put it down to.
  ///
  /// The compose page's own: clearing gives the box back the text that was
  /// stowed when the pull request was loaded over it. A draft's page has no
  /// such control — the Conversation was created holding this, and the way out
  /// of one is to close it.
  clear?: () => void;
}): JSX.Element {
  return (
    <div class={styles.held}>
      <p class={styles.line}>
        <a class={styles.what} href={props.url} target="_blank" rel="noreferrer">
          {props.repo} #{props.number}
        </a>
        <span class={styles.title}>{props.title}</span>

        {/* A mark rather than a word, as the companion rows' own is: the line
            beside it is what says which pull request is being put down. The
            screen reader gets the sentence. */}
        <Show when={props.clear}>
          {(clear) => (
            <button
              type="button"
              class={styles.clear}
              aria-label={`Clear #${props.number}`}
              onClick={() => clear()()}
            >
              ×
            </button>
          )}
        </Show>
      </p>

      <p class={styles.branches}>
        <code>{props.head}</code> into <code>{props.base}</code>
      </p>
    </div>
  );
}

/// Each way of being refused a take-up, in the words of what to go and do about
/// it — for the conversation's own repo.
///
/// One line each rather than a single "cannot take up", for the reason the
/// adoption's own list is one line each: a profile to choose, a branch somebody
/// has pushed to and a branch somebody is standing on are three different jobs,
/// and only the human can tell which they are looking at.
export const TAKE_UP_REFUSAL: Record<
  Exclude<TakenUp, { Companion: unknown } | { CheckedOutElsewhere: unknown }>,
  string
> = {
  TakenUp: "",
  NoSuchConversation: "This conversation is gone.",
  NotDrafting: "This conversation has already been started.",
  NotHoldingOne:
    "This conversation is holding no pull request, so there is nothing for it to wrap up.",
  NoImplementationProfile:
    "Choose an implementation profile and model first, on the brief.",
  NoReviewProfile: "Choose a review profile and model first, on the brief.",
  ProfileBroken:
    "A chosen profile's claude pair is not where it was left, so there is no account to run under.",
  FetchFailed:
    "Git could not fetch from the repo's remote, so nothing was started. The server log says why.",
  NoHeadBranch:
    "Origin has no branch by the name GitHub gave for this pull request's head.",
  BranchAhead:
    "The local branch of that name holds commits origin does not. Push them, or take them off, and try again.",
  BranchDiverged:
    "The local branch of that name and origin's have gone different ways. Reconcile them and try again.",
  FastForwardFailed:
    "Git would not move the local branch on to origin's. The server log says why.",
  WorktreeRefused: "Git would not make the worktree. The server log says why.",
};

/// What to say about a take-up that was refused.
///
/// The two that carry something with them say it, because in each the thing
/// carried is the whole of what makes it actionable: which repository a
/// companion's failing was in, and *where* the head branch is already checked
/// out.
export function takeUpRefusal(outcome: TakenUp): string {
  if (typeof outcome === "object") {
    if ("Companion" in outcome) {
      return `${outcome.Companion.repo}: ${companionRefusal(outcome.Companion.why)}`;
    }

    return `That branch is already checked out at ${outcome.CheckedOutElsewhere.at}, and git holds one checkout per branch.`;
  }

  return TAKE_UP_REFUSAL[outcome];
}

/// The press that takes the held pull request up, where `Start grilling` stands
/// on every other draft.
///
/// Never both: a draft holding a pull request has no round to open. The work on
/// it is built and what it is waiting for is the wrap-up, so the one act this
/// page offers is the one that starts one.
export function TakingUp(props: {
  conversation: ConversationView;
  held: NonNullable<ConversationView["adopting_pull_request"]>;
}): JSX.Element {
  const queries = useQueryClient();
  const device = useDevice();

  const [refused, setRefused] = createSignal<TakenUp | null>(null);

  const take = useMutation(() => ({
    mutationFn: () => takeUpPullRequest(device(), props.conversation.id),
    onSuccess: (outcome: TakenUp) => {
      // Whatever it came back with, the page is read again: what the take-up
      // did is a conversation that has moved, and what refused it is a
      // repository that has moved — and reading it again is the correction
      // either way.
      setRefused(outcome === "TakenUp" ? null : outcome);

      void queries.invalidateQueries({
        queryKey: keyOf(device(), "conversation"),
      });
      void queries.invalidateQueries({ queryKey: ["conversations"] });
      void queries.invalidateQueries({
        queryKey: keyOf(device(), "open-pull-requests"),
      });
      void queries.invalidateQueries({ queryKey: keyOf(device(), "profiles") });
    },
  }));

  return (
    <section class={styles.takingUp} aria-label="Wrapping up a pull request">
      <h2>Wrap up a pull request</h2>

      <button
        type="button"
        class={styles.press}
        disabled={take.isPending}
        onClick={() => take.mutate()}
      >
        {take.isPending ? "Wrapping up…" : "Wrap up"}
      </button>
      <Note>
        This checks <code>{props.held.head}</code> out, fetching it from origin
        and moving a local copy on to it, and starts the wrap-up over the pull
        request: the branch is reviewed, red checks are fixed and what has been
        said on it is answered. Both agent profiles have to be chosen first.
        {/* And the companions, where any were configured while it drafted: the
            press checks them out beside the head branch, so it is worth saying
            that it is this press that makes them. */}
        <Show when={props.conversation.companions.length}>
          {" "}
          The repos alongside are checked out with it.
        </Show>
      </Note>

      <Show when={refused()}>
        {(outcome) => (
          <ErrorLine class={styles.failure}>
            {takeUpRefusal(outcome())}
          </ErrorLine>
        )}
      </Show>
      <Show when={take.isError}>
        <ErrorLine class={styles.failure}>
          The pull request could not be taken up: {take.error?.message}
        </ErrorLine>
      </Show>
    </section>
  );
}
