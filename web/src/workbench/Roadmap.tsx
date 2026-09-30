//! The roadmap, opened: every stage brief of it in the details pane.
//!
//! The backlog pane one level up, and the same pane end to end — see
//! `Backlog.tsx`, whose module docs say why either of them is fetched rather
//! than carried, and `Documents.tsx`, which draws the stack for both. A stage
//! list's card is the entries — a number, a title, a box and where the stage is
//! — five of them around the stages in flight, and each entry names a brief
//! beside `ROADMAP.md` that says what the stage is for. This is those briefs,
//! every one of them, in the roadmap's own order.
//!
//! Two things are the roadmap's own. It is named by the roadmap rather than by
//! the conversation, a worktree being allowed any number of roadmaps where it has
//! one `.tasks/`; and where each of its stages *is* comes over as a state the
//! server read off its own record and the roadmap's own declarations — see
//! `stages.ts`, which keeps the words — so this pane says where a stage is, and
//! which of its neighbours it is behind, where the backlog's says one of two
//! words. It says it twice, on the section's heading and on the line the table
//! of contents reaches that section by, which is what makes the way around the
//! roadmap an answer to where the effort has got to.
//!
//! Everything else it does, the backlog pane does too — every stage has a
//! document, over or not, and its state is something the section's heading says
//! rather than the reason it is empty.

import { Match, Show, Switch, createMemo, type JSX } from "solid-js";

import { PaneSticky } from "../Panes";
import { loadRoadmapPane } from "../api/client";
import type { ConversationView, StageDocument } from "../api/types";
import { useReading } from "../freshness";
import { Empty, ErrorLine } from "../notices";
import { keyOf, useDevice } from "../reaching";
import { Contents, navigation } from "../set/Contents";
import type { Section } from "../set/outline";
import { spied } from "../set/outline";
import { Documents, type DocumentSection } from "./Documents";
import styles from "./Documents.module.css";
import { PaneHead } from "./PaneHead";
import { stageState } from "./stages";

/// What one stage's section is reached by. Its own prefix, as a task's is, and
/// a different one: the two panes are never open at once, but the anchors say
/// which kind of thing a link points at.
function anchor(stage: StageDocument): string {
  return `stage-${stage.number}`;
}

/// What the stage's own line in `ROADMAP.md` declared, in words: what it stands
/// on, and the platform it wants where it names one.
///
/// Nothing at all where the line declared neither, which is every line of every
/// roadmap written before there was anything to declare — the server judges the
/// whole file before it reads a line of it, so such a roadmap arrives with
/// nothing declared on any line and its pane reads exactly as it always did.
///
/// A platform on its own is drawn on its own: within a roadmap that declares,
/// such a line is undeclared, and saying so is the refusal's business rather than
/// this pane's.
function declares(stage: StageDocument): string | undefined {
  const said: string[] = [];

  if (stage.stands_on) {
    // The empty list is the root — `no dependencies` on the line — and an
    // `after` naming nobody arrives as the same thing, that being a roadmap
    // nothing will run rather than a shape to draw differently.
    said.push(
      stage.stands_on.length > 0
        ? `Stands on ${stage.stands_on.join(", ")}`
        : "Stands on nothing",
    );
  }

  // As the line named it, whatever word that was: the three there are is what
  // the refusal knows, and a pane that quietly dropped a fourth would be hiding
  // the thing the human has to fix.
  if (stage.platform) {
    said.push(`on ${stage.platform}`);
  }

  return said.length > 0 ? said.join(" · ") : undefined;
}

export function Roadmap(props: {
  conversation: ConversationView;

  /// Which roadmap: the directory name under `docs/roadmaps/`, off the card
  /// that was pressed.
  name: string;

  back: () => void;
}): JSX.Element {
  const device = useDevice();

  const opened = useReading(() => ({
    queryKey: keyOf(device(), "roadmap", props.conversation.id, props.name),
    queryFn: () => loadRoadmapPane(device(), props.conversation.id, props.name),

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
      // Where the stage is, on the heading rather than in a box, because a
      // stage that is over still has its brief — see `Backlog.tsx`, which says
      // its own two words the same way for the same reason. The server's
      // reading, in the words `stages.ts` keeps: what the card's row says about
      // this stage, said again here.
      mark: stageState(stage.state),
      // What the roadmap's own line said about it, which is the one thing here
      // that comes off the list rather than out of the brief.
      declares: declares(stage),
    })),
  );

  // One line per stage, wherever it has got to: the whole roadmap is what the
  // pane is. Each carries the state as well, so the way around the roadmap says
  // where every stage of it is without the reader scrolling the briefs — which is
  // the nav answering the question the pane was opened with.
  const sections = createMemo((): Section[] =>
    (opened.data?.stages ?? []).map((stage) => ({
      anchor: anchor(stage),
      name: `${stage.number} ${stage.title}`,
      entries: [],
      mark: stageState(stage.state),
    })),
  );

  const watched = createMemo(() => spied(sections()));

  const nav = navigation();

  return (
    <>
      <PaneSticky>
        <PaneHead back={{ to: "Timeline", go: props.back }} title="Roadmap" />
      </PaneSticky>

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
        <Contents sections={sections()} watched={watched()} nav={nav} />
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
