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
//! Two of them have a way out of themselves inside them — a pull request another
//! Conversation is still at work on, and the Conversations a start would close
//! that are holding uncommitted changes — so there are two readings here: the
//! sentence, and the sentence with the links. See [`takeUpRefusal`] and
//! [`TakeUpRefusal`].
//!
//! And one of them is not a refusal at all. `WouldDiscard` is the press stopping
//! to ask: the start goes ahead on the press after it, which is why the press
//! reads as going ahead while it is drawn. See [`goingAhead`].

import { A } from "@solidjs/router";
import { For, Show, type JSX } from "solid-js";

import type { TakenUp, Uncommitted } from "../api/types";
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
    | { WouldDiscard: unknown }
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
      return "Another conversation is still at work on that pull request, so the way on is that conversation rather than a second one over the same branch.";
    }

    if ("WouldDiscard" in outcome) {
      return `${held(outcome.WouldDiscard.uncommitted)} ${closingThem(outcome.WouldDiscard.uncommitted.length)}`;
    }

    return `That branch is already checked out at ${outcome.CheckedOutElsewhere.at}, and git holds one checkout per branch.`;
  }

  return TAKE_UP_REFUSAL[outcome];
}

/// What every drawing of `WouldDiscard` ends on: what would be lost, and what to
/// do about it.
///
/// One sentence rather than two readings of it, because the way out of this one
/// is the press itself rather than anywhere to go: the links say who, and this
/// says what pressing again does.
///
/// Agreeing with how many were named, because a stack is asked about in one list:
/// every conversation standing on a link of it is named by the one press, so
/// several of them and one worktree between them is the sentence a chain of five
/// would have read.
function closingThem(count: number): string {
  return count === 1
    ? "would be closed to make way, and the uncommitted changes in its worktree would go with it. Press start again to go ahead."
    : "would each be closed to make way, and the uncommitted changes in their worktrees would go with them. Press start again to go ahead.";
}

/// Who is holding something, as the plain sentence names them: by branch, in the
/// order the server gave, and as one clause whether there is one of them or four.
function held(uncommitted: Uncommitted[]): string {
  const branches = uncommitted.map((one) => one.branch).join(", ");

  return uncommitted.length === 1
    ? `The conversation on ${branches}`
    : `The conversations on ${branches}`;
}

/// The conversations a press was stopped over, or `null` where it was not
/// stopped at all.
///
/// The one reading of that outcome here, because everything the composer wants
/// of it is this list: what the button reads, what the line under it draws, and
/// what the press after it sends back.
function wouldDiscard(outcome: TakenUp | null): Uncommitted[] | null {
  return (
      outcome !== null &&
        typeof outcome === "object" &&
        "WouldDiscard" in outcome
    ) ?
      outcome.WouldDiscard.uncommitted
    : null;
}

/// Whether an outcome is the press stopping to ask rather than refusing.
///
/// What the composer reads it for is the button: a start that has been stopped
/// over what a close would discard is one press away from going ahead, so the
/// press says so. Every refusal answers `false` — there is nothing to press
/// through, only something to go and fix.
export function goingAhead(outcome: TakenUp | null): boolean {
  return wouldDiscard(outcome) !== null;
}

/// And which conversations a press that goes ahead is agreeing to lose, for the
/// body of that press. Empty for everything else.
export function discarding(outcome: TakenUp | null): number[] {
  return (wouldDiscard(outcome) ?? []).map((one) => one.conversation);
}

/// The same, as the line a composer draws under its press — which is the one
/// place these have room for a way out of themselves.
///
/// Two of them have one. The pull request another conversation is still at work
/// on offers that conversation: there is one open conversation per pull request,
/// so the way on is the one that has it rather than a second wrap-up over the
/// same branch. And the conversations a start would close with something
/// uncommitted in them are each a link too — the human is being asked whether to
/// throw that away, and the way to answer it is to go and look.
///
/// Every other refusal is the sentence and nothing else.
///
/// A conversation that has finished with the pull request and left nothing behind
/// is neither of them: the start closes it and takes the pull request up, so
/// there is nothing to refuse and nothing to draw.
export function TakeUpRefusal(props: { outcome: TakenUp }): JSX.Element {
  const stillAtWork = (): number | null =>
    typeof props.outcome === "object" && "AlreadyHeld" in props.outcome
      ? props.outcome.AlreadyHeld.conversation
      : null;

  const holding = (): Uncommitted[] | null => wouldDiscard(props.outcome);

  return (
    <Show
      when={holding()}
      fallback={
        <Show when={stillAtWork()} fallback={takeUpRefusal(props.outcome)}>
          {(conversation) => (
            <>
              <A href={pathOf(conversation())}>Another conversation</A> is still
              at work on that pull request, so the way on is that conversation
              rather than a second one over the same branch.
            </>
          )}
        </Show>
      }
    >
      {(uncommitted) => (
        <>
          {uncommitted().length === 1 ?
            "The conversation on "
          : "The conversations on "}
          <For each={uncommitted()}>
            {(one, at) => (
              <>
                <Show when={at() > 0}>, </Show>
                <A href={pathOf(one.conversation)}>{one.branch}</A>
              </>
            )}
          </For>{" "}
          {closingThem(uncommitted().length)}
        </>
      )}
    </Show>
  );
}
