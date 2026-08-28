//! The roadmap, opened: every stage brief of it in the details pane.
//!
//! The backlog pane one level up, and the same pane end to end — see
//! `Backlog.tsx`, whose module docs say why either of them is fetched rather
//! than carried, and `Documents.tsx`, which draws the stack for both. A stage
//! list's card is the entries — a number, a title and a box — and each entry
//! names a brief beside `ROADMAP.md` that says what the stage is for. This is
//! those briefs, in the roadmap's own order.
//!
//! One thing is the roadmap's own: it is named by the roadmap rather than by the
//! conversation, a worktree being allowed any number of roadmaps where it has
//! one `.tasks/`. Everything else it does, the backlog pane does too — every
//! stage has a document, done or not, and the done state is something the
//! section's heading says rather than the reason it is empty.

import { Match, Show, Switch, createMemo, type JSX } from "solid-js";

import { loadRoadmapPane } from "../api/client";
import type { ConversationView, StageDocument } from "../api/types";
import { useReading } from "../freshness";
import { Empty, ErrorLine } from "../notices";
import { Contents, navigation } from "../set/Contents";
import type { Section } from "../set/outline";
import { spied } from "../set/outline";
import { Documents, type DocumentSection } from "./Documents";
import styles from "./Documents.module.css";
import { PaneHead } from "./PaneHead";

/// What one stage's section is reached by. Its own prefix, as a task's is, and
/// a different one: the two panes are never open at once, but the anchors say
/// which kind of thing a link points at.
function anchor(stage: StageDocument): string {
  return `stage-${stage.number}`;
}

export function Roadmap(props: {
  conversation: ConversationView;

  /// Which roadmap: the directory name under `docs/roadmaps/`, off the card
  /// that was pressed.
  name: string;

  back: () => void;
}): JSX.Element {
  const opened = useReading(() => ({
    queryKey: ["roadmap", props.conversation.id, props.name],
    queryFn: () => loadRoadmapPane(props.conversation.id, props.name),

    // Merged rather than frozen, for the backlog pane's reason: this is the
    // worktree as it stands, and a stage ticking itself off moves it while this
    // is open.
    freshness: { reconcile: "number" },
  }));

  /// The briefs to stack, in the roadmap's own order — which is the order the
  /// effort goes through them.
  const documents = createMemo((): DocumentSection[] =>
    (opened.data?.stages ?? []).map((stage) => ({
      anchor: anchor(stage),
      number: stage.number,
      title: stage.title,
      html: stage.html,
      // A brief stays where it is, so nothing here is a file that has gone.
      // What it is instead is the roadmap pointing at a file nobody wrote,
      // which is the human's to fix — the same thing `/next-stage` refuses to
      // guess past.
      missing: "The roadmap names a brief that is not there to read.",
      // The done state on the heading rather than in the box, because a done
      // stage still has its brief — see `Backlog.tsx`, which says it the same
      // way for the same reason.
      mark: stage.done ? "done" : "to do",
    })),
  );

  // One line per stage, done or not: the whole roadmap is what the pane is.
  const sections = createMemo((): Section[] =>
    documents().map((stage) => ({
      anchor: stage.anchor,
      name: `${stage.number} ${stage.title}`,
      entries: [],
    })),
  );

  const watched = createMemo(() => spied(sections()));

  const nav = navigation();

  return (
    <>
      <PaneHead back={{ to: "Timeline", go: props.back }} title="Roadmap" />

      {/* Which roadmap this is, said the way the card says it: the heading
          `ROADMAP.md` wrote about itself, or the directory that is its identity
          where it wrote none. Once the pane has arrived rather than before it,
          so the line is not the name and then the heading a moment later. */}
      <Show when={opened.data}>
        {(pane) => (
          <p class={styles.feature}>{pane().title || props.name}</p>
        )}
      </Show>

      <Show when={sections().length > 0}>
        <Contents sections={sections()} watched={watched()} nav={nav} paned />
      </Show>

      <Switch>
        <Match when={opened.isPending}>
          <Empty>Loading…</Empty>
        </Match>
        <Match when={opened.isError}>
          <ErrorLine>
            Could not read this roadmap: {opened.error?.message}
          </ErrorLine>
        </Match>
        <Match when={opened.data}>
          {(pane) => (
            <Documents sections={documents()} diagrams={pane().diagrams} />
          )}
        </Match>
      </Switch>
    </>
  );
}
