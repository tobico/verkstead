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
//! Overriding the base in the details pane is answered by the stage that is
//! next *there*: what this names is never what the notice happened to show, it
//! is what the press would actually start.
//!
//! The branch is not offered here or in the details pane. A stage is worked on
//! its own slug — `04-wrap-up.md` becomes `wrap-up` — so the name the server
//! invented when the row was made is discarded at the press, and what the
//! sidebar shows until then is that invented name.

import { type JSX, Show } from "solid-js";

import type { ConversationView } from "../api/types";

export function Adoption(props: {
  conversation: ConversationView;
  adopting: NonNullable<ConversationView["adopting"]>;
}): JSX.Element {
  return (
    <section class="adoption" aria-label="Adoption">
      <h2>Adopt a roadmap</h2>

      <p class="roadmap">
        <code>{props.adopting.roadmap}</code>
        <Show when={props.adopting.title}>
          {(title) => <span class="roadmap-title">{title()}</span>}
        </Show>
      </p>

      <Show
        when={props.adopting.stage}
        fallback={
          // Everything that can be wrong with a roadmap at a commit reads the
          // same way here: there is no stage to start. Which of them it is —
          // the roadmap finished, the roadmap not there, somebody already on
          // the next stage — is what the press says by name.
          <p class="empty">
            Nothing to adopt at this base commit: no stage of{" "}
            <code>{props.adopting.roadmap}</code> can be started from it.
          </p>
        }
      >
        {(stage) => (
          <>
            <p class="stage">
              Stage {stage().label}: {stage().title}
            </p>
            <p class="note">
              The brief at <code>{stage().brief_path}</code> becomes this
              conversation's brief, and the work is done on{" "}
              <code>{stage().branch}</code>, branched from the base commit.
            </p>
          </>
        )}
      </Show>

      <Show when={props.conversation.state === "Draft"}>
        <button type="button" class="adopt">
          Adopt
        </button>
        <p class="note">
          This creates the branch and its worktree, takes the stage brief as
          this conversation's brief, and starts the work on it. Both agent
          profiles have to be chosen first.
        </p>
      </Show>
    </section>
  );
}
