//! The backlog, opened: every task document of it in the details pane.
//!
//! What the card cannot show. A task list's card is the entries — a number, a
//! title and a box — and each entry names a document in `.tasks/` that says what
//! the task is and what *done* means for it. This is those documents, stacked in
//! backlog order — see `Documents.tsx`, which draws them and is shared with the
//! roadmap pane beside this one.
//!
//! Fetched here rather than carried by the conversation, for the reason a
//! commit's diff is: the timeline is read again every time the page hears the
//! world moved, and a backlog is worth reading whole when somebody opens it.
//!
//! Fetched by the conversation alone, unlike the four panes around it. A backlog
//! is a reading of the worktree rather than a record, so there is no event to
//! name it by: there is one backlog per conversation, and this is it. A roadmap
//! is the one place that differs — a worktree may hold several — which is why
//! `Roadmap.tsx` is named by the roadmap.
//!
//! The list down its margin is the set page's own table of contents, one line
//! per task, jumping to the section that task's document is in. Which shape it
//! takes is the pane's width's answer — see `set/Contents`.

import { Match, Show, Switch, createMemo, type JSX } from "solid-js";

import { loadBacklogPane } from "../api/client";
import type { ConversationView, TaskDocument } from "../api/types";
import { useReading } from "../freshness";
import { Empty, ErrorLine } from "../notices";
import { Contents, navigation } from "../set/Contents";
import type { Section } from "../set/outline";
import { spied } from "../set/outline";
import { Documents, type DocumentSection } from "./Documents";
import styles from "./Documents.module.css";
import { PaneHead } from "./PaneHead";

/// What one task's section is reached by.
function anchor(task: TaskDocument): string {
  return `task-${task.number}`;
}

export function Backlog(props: {
  conversation: ConversationView;
  back: () => void;
}): JSX.Element {
  const opened = useReading(() => ({
    queryKey: ["backlog", props.conversation.id],
    queryFn: () => loadBacklogPane(props.conversation.id),

    // Merged rather than frozen, unlike a commit's pane: a commit cannot change
    // and `.tasks/` is the worktree as it stands, so a session finishing a task
    // takes its document away while this is open. Matched by the number, which
    // is what an entry answers to — a task whose document did not move keeps
    // the object it had, and the assignment of its HTML below, which compiles
    // to an unguarded effect over the data, is left alone with it.
    freshness: { reconcile: "number" },
  }));

  /// The documents to stack, in backlog order — which is the order they are
  /// read and the order the work goes through them.
  const documents = createMemo((): DocumentSection[] =>
    (opened.data?.tasks ?? []).map((task) => ({
      anchor: anchor(task),
      number: task.number,
      title: task.title,
      html: task.html,
      // A done task, its file gone from `.tasks/` — which is the done-signal the
      // whole task runner turns on. Said in words inside the box, so the section
      // reads as a finished task rather than as a document that would not load.
      missing: "Finished, and the document removed.",
    })),
  );

  // The sections this pane is made of. One line per task, whether or not its
  // document is still there: a finished task is part of what the backlog is,
  // and a nav that listed only what was left would be a shorter list than the
  // page.
  const sections = createMemo((): Section[] =>
    documents().map((task) => ({
      anchor: task.anchor,
      name: `${task.number} ${task.title}`,
      entries: [],
    })),
  );

  const watched = createMemo(() => spied(sections()));

  const nav = navigation();

  return (
    <>
      <PaneHead back={{ to: "Timeline", go: props.back }} title="Task list" />

      {/* What the backlog is called, where the list wrote itself a heading —
          the same words the card carries, so the pane walked into on a phone
          still says which backlog this is. */}
      <Show when={opened.data?.feature}>
        {(feature) => <p class={styles.feature}>{feature()}</p>}
      </Show>

      {/* Above the documents. The stylesheet takes it out of the flow and puts
          it in the pane's margin where there is room for one. */}
      <Show when={sections().length > 0}>
        <Contents sections={sections()} watched={watched()} nav={nav} paned />
      </Show>

      <Switch>
        <Match when={opened.isPending}>
          <Empty>Loading…</Empty>
        </Match>
        <Match when={opened.isError}>
          <ErrorLine>
            Could not read this backlog: {opened.error?.message}
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
