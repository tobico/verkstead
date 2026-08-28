//! The adoption-shaped page: what a conversation started from the abandoned
//! roadmaps notice offers instead of a brief and a grilling.
//!
//! An adopting conversation has nothing to write. Its brief is the stage brief
//! and it arrives when the stage is adopted, so what stands where the brief and
//! `Start grilling` would be is the roadmap named, the stage that would be
//! started, and one `Adopt` press.
//!
//! What the roadmap and the stage are is the server's reading of the repository
//! at this conversation's base commit, redone every time the page is drawn.
//! Overriding the base on the brief card is answered by the stage that is
//! next *there*: what this names is never what the notice happened to show, it
//! is what the press would actually start.
//!
//! The branch is not offered here or on the brief card. A stage is worked on
//! its own slug — `04-wrap-up.md` becomes `wrap-up` — so the name the server
//! invented when the row was made is discarded at the press, and what the
//! sidebar shows until then is that invented name.

import { useMutation, useQueryClient } from "@tanstack/solid-query";
import { createSignal, type JSX, Show } from "solid-js";

import { adoptRoadmap } from "../api/client";
import type { Adopted, ConversationView } from "../api/types";
import { Empty, ErrorLine, Note } from "../notices";
import { COMPANION_REFUSAL } from "./Timeline";
import styles from "./Adoption.module.css";

/// Each way of being refused an adoption, in the words of what to go and do
/// about it — for the conversation's own repo.
///
/// One line each rather than a single "cannot adopt", because the server names
/// them separately for exactly this: a profile to choose, a box somebody
/// ticked, a branch somebody is on and a worktree git would not make are four
/// different jobs, and only the human can tell which they are looking at.
export const ADOPT_REFUSAL: Record<
  Exclude<Adopted, { Companion: unknown }>,
  string
> = {
  Adopted: "",
  NoSuchConversation: "This conversation is gone.",
  NotDrafting: "This conversation has already been adopted.",
  NotAdopting:
    "This conversation is adopting nothing, so it has no roadmap to take a stage from.",
  NoGrillingProfile: "Choose a grilling profile and model first, on the brief.",
  NoImplementationProfile:
    "Choose an implementation profile and model first, on the brief.",
  ProfileBroken:
    "A chosen profile's claude pair is not where it was left, so there is no account to run under.",
  FetchFailed:
    "Git could not fetch from the repo's remote, so nothing was adopted. The server log says why.",
  NoBaseCommit: "The repo has nothing to branch from any more.",
  NoRoadmap: "There is no roadmap by that name at the base commit.",
  RoadmapComplete:
    "Every stage of that roadmap is ticked off, so there is nothing left to start.",
  NoBrief:
    "The next stage names a brief that is not there at the base commit, which is the roadmap's own to fix.",
  StageInFlight:
    "The next stage is marked as in progress on a branch that still exists, so somebody is already on it.",
  BranchExists:
    "The stage's own branch already exists, and Verkstead did not make it.",
  WorktreeRefused: "Git would not make the worktree. The server log says why.",
};

/// What to say about an adoption that was refused.
///
/// A companion's refusal names the repository, because that is the whole of what
/// makes it different from the same failing on the conversation's own: the thing
/// to go and look at is one of several repos rather than the obvious one. The
/// four lines are the grill start's own, because the four failings are — see
/// `COMPANION_REFUSAL` in [`Timeline`](./Timeline.tsx).
export function adoptRefusal(outcome: Adopted): string {
  if (typeof outcome === "object") {
    return `${outcome.Companion.repo}: ${COMPANION_REFUSAL[outcome.Companion.why]}`;
  }

  return ADOPT_REFUSAL[outcome];
}

export function Adoption(props: {
  conversation: ConversationView;
  adopting: NonNullable<ConversationView["adopting"]>;
}): JSX.Element {
  const queries = useQueryClient();

  const [refused, setRefused] = createSignal<Adopted | null>(null);

  const adopt = useMutation(() => ({
    mutationFn: () => adoptRoadmap(props.conversation.id),
    onSuccess: (outcome: Adopted) => {
      // Whatever it came back with, the page is read again: what adopting did
      // is a conversation that has moved, and what refused it is a repository
      // that has moved — and reading it again is the correction either way.
      setRefused(outcome === "Adopted" ? null : outcome);

      void queries.invalidateQueries({ queryKey: ["conversation"] });
      void queries.invalidateQueries({ queryKey: ["conversations"] });
      void queries.invalidateQueries({ queryKey: ["abandoned-roadmaps"] });
      void queries.invalidateQueries({ queryKey: ["profiles"] });
    },
  }));

  return (
    <section class={styles.adoption} aria-label="Adoption">
      <h2>Adopt a roadmap</h2>

      <p class={styles.roadmap}>
        <code>{props.adopting.roadmap}</code>
        <Show when={props.adopting.title}>
          {(title) => <span class={styles.roadmapTitle}>{title()}</span>}
        </Show>
      </p>

      <Show
        when={props.adopting.stage}
        fallback={
          // Everything that can be wrong with a roadmap at a commit reads the
          // same way here: there is no stage to start. Which of them it is —
          // the roadmap finished, the roadmap not there, somebody already on
          // the next stage — is what the press says by name.
          <Empty>
            Nothing to adopt at this base commit: no stage of{" "}
            <code>{props.adopting.roadmap}</code> can be started from it.
          </Empty>
        }
      >
        {(stage) => (
          <>
            <p class={styles.stage}>
              Stage {stage().label}: {stage().title}
            </p>
            <Note>
              The brief at <code>{stage().brief_path}</code> becomes this
              conversation's brief, and the work is done on{" "}
              <code>{stage().branch}</code>, branched from the base commit.
            </Note>
          </>
        )}
      </Show>

      <Show when={props.conversation.state === "Draft"}>
        <button
          type="button"
          class={styles.adopt}
          disabled={adopt.isPending}
          onClick={() => adopt.mutate()}
        >
          {adopt.isPending ? "Adopting…" : "Adopt"}
        </button>
        <Note>
          This creates the branch and its worktree, takes the stage brief as
          this conversation's brief, and starts the work on it. Both agent
          profiles have to be chosen first.
          {/* And the companions, where any were configured while it drafted:
              the press checks them out beside the stage's own, so it is worth
              saying that it is this press that makes them. */}
          <Show when={props.conversation.companions.length}>
            {" "}
            The repos alongside are checked out with it.
          </Show>
        </Note>

        <Show when={refused()}>
          {(outcome) => (
            <ErrorLine class={styles.failure}>{adoptRefusal(outcome())}</ErrorLine>
          )}
        </Show>
        <Show when={adopt.isError}>
          <ErrorLine class={styles.failure}>
            The stage could not be adopted: {adopt.error?.message}
          </ErrorLine>
        </Show>
      </Show>
    </section>
  );
}
