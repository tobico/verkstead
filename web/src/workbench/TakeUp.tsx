//! What a take-up can be refused for, in the words of what to go and do about it.
//!
//! A **Review**'s Start is the take-up — the Target field read, a pull request or
//! a branch resolved out of it, and the branch settled against origin — and every
//! way that press can come back is something different for the human to go and
//! do. So each one is a sentence of its own rather than a shared *cannot take
//! up*, and they live here rather than in the composer because the composer is
//! not the only place one is read: the compose page's own create replay carries
//! them to the draft it made.
//!
//! One of them has a way out of itself inside it — a pull request another
//! Conversation is already on — so there are two readings here: the sentence, and
//! the sentence with the link. See [`takeUpRefusal`] and [`TakeUpRefusal`].

import { A } from "@solidjs/router";
import { Show, type JSX } from "solid-js";

import type { TakenUp } from "../api/types";
import { pathOf } from "./openings";
import { companionRefusal } from "./Timeline";

/// Each way of being refused a take-up, in the words of what to go and do about
/// it — for the conversation's own repo.
///
/// One line each rather than a single "cannot take up", for the reason the
/// adoption's own list is one line each: a profile to choose, a branch somebody
/// has pushed to and a branch somebody is standing on are three different jobs,
/// and only the human can tell which they are looking at.
export const TAKE_UP_REFUSAL: Record<
  Exclude<
    TakenUp,
    | { Companion: unknown }
    | { CheckedOutElsewhere: unknown }
    | { AnotherRepository: unknown }
    | { NoSuchPullRequest: unknown }
    | { GitHubRefused: unknown }
    | { AlreadyHeld: unknown }
  >,
  string
> = {
  TakenUp: "",
  NoSuchConversation: "This conversation is gone.",
  NotDrafting: "This conversation has already been started.",
  NotHoldingOne:
    "This conversation is not a review, so there is nothing for it to wrap up.",
  NoTarget:
    "Nothing is named in the Target field — put a pull request in it, by its link or as #number, or the branch to wrap up.",
  Fork:
    "That pull request's branch is in a fork, so nothing fixed here could be pushed to it.",
  NoImplementationProfile:
    "Choose an implementation profile and model first, on the brief.",
  NoReviewProfile: "Choose a review profile and model first, on the brief.",
  ProfileBroken:
    "A chosen profile's claude pair is not where it was left, so there is no account to run under.",
  FetchFailed:
    "Git could not fetch from the repo's remote, so nothing was started. The server log says why.",
  NoHeadBranch:
    "Origin has no branch by that name, so there is nowhere for a review to happen. Push it, or name another.",
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
/// The ones that carry something with them say it, because in each the thing
/// carried is the whole of what makes it actionable: which repository a
/// companion's failing was in, *where* the head branch is already checked out,
/// which repository a link named, which number GitHub had nothing open under,
/// and what `gh` itself said.
///
/// One of them carries a conversation instead, and this says the sentence
/// without the way there: the line under the press is where the link goes, and
/// it is drawn by [`TakeUpRefusal`].
export function takeUpRefusal(outcome: TakenUp): string {
  if (typeof outcome === "object") {
    if ("Companion" in outcome) {
      return `${outcome.Companion.repo}: ${companionRefusal(outcome.Companion.why)}`;
    }

    if ("AnotherRepository" in outcome) {
      return `That link is a pull request of ${outcome.AnotherRepository.named}, which is not the repo this conversation is on.`;
    }

    if ("NoSuchPullRequest" in outcome) {
      return `This repo has nothing open under #${outcome.NoSuchPullRequest.number} — it may have been merged, closed, or never opened.`;
    }

    if ("GitHubRefused" in outcome) {
      return `GitHub could not be asked about that pull request: ${outcome.GitHubRefused.why}.`;
    }

    if ("AlreadyHeld" in outcome) {
      return "That pull request is already another conversation's, and there is one conversation per piece of work.";
    }

    return `That branch is already checked out at ${outcome.CheckedOutElsewhere.at}, and git holds one checkout per branch.`;
  }

  return TAKE_UP_REFUSAL[outcome];
}

/// The same, as the line a composer draws under its press — which is the one
/// place a refusal has room for a way out of itself.
///
/// The pull request another conversation holds is what that is for: there is
/// one conversation per piece of work, so what this refusal offers is the one
/// that has it rather than a second wrap-up over the same branch. Every other
/// refusal is the sentence and nothing else.
export function TakeUpRefusal(props: { outcome: TakenUp }): JSX.Element {
  const held = (): number | null =>
    typeof props.outcome === "object" && "AlreadyHeld" in props.outcome
      ? props.outcome.AlreadyHeld.conversation
      : null;

  return (
    <Show when={held()} fallback={takeUpRefusal(props.outcome)}>
      {(conversation) => (
        <>
          That pull request is already{" "}
          <A href={pathOf(conversation())}>another conversation's</A>, and there
          is one conversation per piece of work.
        </>
      )}
    </Show>
  );
}
