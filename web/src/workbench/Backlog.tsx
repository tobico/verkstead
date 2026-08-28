//! The backlog, opened: every task document of it in the details pane.
//!
//! What the card cannot show, which is two things. A task list's card is the
//! entries — a number, a title and a box — five of them around the one being
//! worked, and each entry names a document in `.tasks/` that says what the task
//! is and what *done* means for it. This is those documents, every one of them
//! whether the card had room for its entry or not, stacked in backlog order —
//! see `Documents.tsx`, which draws them and is shared with the roadmap pane
//! beside this one.
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
//! Every task has a document, done or not: a task file stays in `.tasks/` until
//! the feature is finished with, so the done state is something the section's
//! heading says rather than the reason it is empty — the same way round as the
//! roadmap pane beside this one. What says a task is done is the checkbox in
//! `TODO.md`, which is what the card is drawn from too.
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
    // ticks its entry off while this is open. Matched by the number, which is
    // what an entry answers to — a task whose document did not move keeps the
    // object it had, and the assignment of its HTML below, which compiles to an
    // unguarded effect over the data, is left alone with it.
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
      // Not the ordinary end of a task's life: a document stays in `.tasks/`
      // until the feature is finished with, so nothing here is a file that has
      // gone. What it is instead is the list naming a file nobody wrote, which
      // is the human's to fix — the same thing the runner refuses to put a
      // session at.
      missing: "The list names a task document that is not there to read.",
      // The done state on the heading rather than in the box, because a done
      // task still has its document — the roadmap pane beside this one has said
      // it this way all along. The word travels with the section rather than
      // being drawn by the stylesheet, for the reason the card's rows do: a page
      // read aloud or copied out still says which tasks are finished.
      mark: task.done ? "done" : "to do",
    })),
  );

  // The sections this pane is made of. One line per task, done or not: a
  // finished task is part of what the backlog is, and a nav that listed only
  // what was left would be a shorter list than the page.
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
