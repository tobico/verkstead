//! The backlog, opened: every task document of it in the details pane.
//!
//! What the card cannot show. A task list's card is the entries — a number, a
//! title and a box — and each entry names a document in `.tasks/` that says what
//! the task is and what *done* means for it. This is those documents, stacked in
//! backlog order, each in the boxed section a set's Preface is drawn in: the
//! heading outside the box, the rendered markdown inside it.
//!
//! Fetched here rather than carried by the conversation, for the reason a
//! commit's diff is: the timeline is read again every time the page hears the
//! world moved, and a backlog is worth reading whole when somebody opens it.
//!
//! Fetched by the conversation alone, unlike the three panes around it. A
//! backlog is a reading of the worktree rather than a record, so there is no
//! event to name it by: there is one backlog per conversation, and this is it.
//!
//! Nothing here parses markdown. The documents arrive as HTML the server
//! rendered and sanitized, with any Diagram left as the source block the
//! client-side renderer draws over — the set page's own arrangement, and a
//! backlog whose documents hold none never asks for mermaid.
//!
//! The list down its margin is the set page's own table of contents, one line
//! per task, jumping to the section that task's document is in. Which shape it
//! takes is the pane's width's answer — see `set/Contents`.

import {
  For,
  Match,
  Show,
  Switch,
  createEffect,
  createMemo,
  on,
  onCleanup,
  type JSX,
} from "solid-js";

import app from "../App.module.css";
import { loadBacklogPane } from "../api/client";
import type { ConversationView, TaskDocument } from "../api/types";
import { useReading } from "../freshness";
import { Empty, ErrorLine } from "../notices";
import { Contents, navigation } from "../set/Contents";
import { drawDiagrams } from "../set/diagrams";
import type { Section } from "../set/outline";
import { spied } from "../set/outline";
import styles from "./Backlog.module.css";
import { PaneHead } from "./PaneHead";

/// What one task's section is reached by. Its own prefix rather than the bare
/// number, because a commit's pane and a set's page can be open at once and an
/// id names one element in a document.
function anchor(task: TaskDocument): string {
  return `task-${task.number}`;
}

/// One task document, put in the page and — where the backlog holds one — drawn.
///
/// The renderer is turned loose on the pane's own block rather than on the
/// document, because a set's page can be open behind the workbench and its
/// Diagrams are its own to draw.
function Documents(props: {
  tasks: TaskDocument[];
  diagrams: boolean;
}): JSX.Element {
  let block!: HTMLDivElement;

  // On the tasks that are in the block rather than on this component's mount:
  // opening a second conversation's backlog is not a second mount, and the
  // markup is assigned into the block the first one built. Following the tasks
  // because they are what is being drawn over — assigning the HTML is a render
  // effect, and those are all through before the first of these runs.
  createEffect(
    on(
      () => props.tasks,
      () => {
        if (!props.diagrams) {
          return;
        }

        // Stopped when the next backlog arrives as much as when the pane goes: a
        // drawing nobody stopped is still watching the colour scheme, and would
        // go on redrawing nodes this block no longer holds.
        onCleanup(drawDiagrams({ root: block }));
      },
    ),
  );

  return (
    <div ref={block}>
      <For each={props.tasks}>
        {(task) => (
          /* Named and anchored the way a set's Preface is: the heading is what a
             jump from the table of contents lands on, the id is what it jumps
             to, and the heading stays outside the box, which is what makes the
             two look alike. */
          <section id={anchor(task)} class={styles.task}>
            <h2 class={app.sectionHeading}>
              <span class={styles.n}>{task.number}</span>
              <span class={styles.what}>{task.title}</span>
            </h2>
            <Show
              when={task.html}
              fallback={
                /* A done task, its file gone from `.tasks/` — which is the
                   done-signal the whole task runner turns on. Said in words
                   inside the box, so the section reads as a finished task
                   rather than as a document that would not load. */
                <p class={styles.finished}>
                  Finished, and the document removed.
                </p>
              }
            >
              {(html) => (
                /* Marked as rendered markdown, so the headings, tables and code
                   a task document is written with get the same rules here as
                   they get in a Preface — the box around it is all that is this
                   section's own. */
                <div
                  class={`${styles.document} markdown`}
                  innerHTML={html()}
                />
              )}
            </Show>
          </section>
        )}
      </For>
    </div>
  );
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

  // The sections this pane is made of, in backlog order — which is the order it
  // is read and the order the work goes through it. One line per task, whether
  // or not its document is still there: a finished task is part of what the
  // backlog is, and a nav that listed only what was left would be a shorter list
  // than the page.
  const sections = createMemo((): Section[] =>
    (opened.data?.tasks ?? []).map((task) => ({
      anchor: anchor(task),
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
            <Documents tasks={pane().tasks} diagrams={pane().diagrams} />
          )}
        </Match>
      </Switch>
    </>
  );
}
