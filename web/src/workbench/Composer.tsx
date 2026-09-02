//! The composer: where a Conversation is drafted, and the press that starts it.
//!
//! A details pane of its own rather than a card on the Timeline. What is
//! written here — the Brief, the branch, the base, the companions, the three
//! pairings — is one act of setting a piece of work up, and the record is a
//! record of what has happened rather than the desk it is arranged on. So the
//! Timeline's Brief card is the five-line rendering of the document at all
//! times, drafting or frozen, and pressing it opens this.
//!
//! **Drawn as the composer of a chat app**, because that is what it is: one box
//! at the app's own measure, centred in the pane, with the Brief written inside
//! it and the whole of the setup along the inside of its bottom edge — a row of
//! borderless dropdowns, each a dimmed label over its value, that read as part
//! of the box rather than as a form under it. The press that starts the work is
//! the one thing outside: under the box, against its right edge, because it is
//! what happens to what is in the box rather than something else to fill in.
//!
//! It serves a Conversation *while it drafts*, whatever the Brief's own freeze
//! — which is three shapes rather than one:
//!
//! - the ordinary draft, whose Brief is the human's to write, with the whole
//!   setup along the box's bottom edge and `Start work` under it;
//! - an adopting draft, whose only Brief is the stage's and arrives frozen, so
//!   the box is the rendering rather than a field and the Adoption control
//!   stands where the start button would;
//! - a later round opened by a steer, whose branch and base froze when the
//!   first round's work started — the setup draws what is still the human's,
//!   which is what it has always done.
//!
//! Past drafting there is nothing here to change and no press to make, so a
//! frozen Brief opens the read-only pane instead — see [`Brief`], which is
//! what a Conversation's configuration is read from once it is settled.
//!
//! And never in a share, which draws that pane for every Brief it carries
//! whatever the record it holds was in the middle of: what happens next is
//! nothing a reader holding a file has any part in, and a pane that ended on a
//! press would be asking them for one.
//!
//! Nothing on this pane has a Save. The Brief keeps itself on a pause in the
//! typing and on the way out of the field, every setup field sends its own
//! change as it is made, and what a save cannot do is said in words where it
//! happened.
//!
//! [`Brief`]: ./Brief.tsx

import { useMutation, useQueryClient } from "@tanstack/solid-query";
import { Show, createSignal, type JSX } from "solid-js";

import { saveBrief, startGrilling } from "../api/client";
import type {
  BriefEvent,
  BriefSaved,
  ConversationView,
  GrillingStarted,
} from "../api/types";
import app from "../App.module.css";
import { PaneSticky } from "../Panes";
import { Empty, ErrorLine, Note } from "../notices";
import { Adoption } from "./Adoption";
import styles from "./Composer.module.css";
import { PaneHead } from "./PaneHead";
import { Setup, SetupNotes } from "./Setup";
import { keeping } from "./settling";
import { BRIEF_REFUSAL, grillRefusal } from "./Timeline";

/// Whether this Brief is the one the composer is for: the round a Conversation
/// is drafting.
///
/// The Conversation's state and the newest Brief on its Timeline, which is the
/// server's own rule for which Brief is open — a Conversation gets one Brief
/// per round, and a round steered into puts a second on the same record. The
/// Brief's `frozen` flag is not the question: an adopting draft's stage brief
/// comes down frozen and is still what the human is arranging work around.
export function composing(
  conversation: ConversationView,
  brief: BriefEvent,
): boolean {
  if (conversation.state !== "Draft") {
    return false;
  }

  const rounds = conversation.timeline.filter(
    (event): event is { Brief: BriefEvent } => "Brief" in event,
  );

  return rounds.at(-1)?.Brief.id === brief.id;
}

export function Composer(props: {
  conversation: ConversationView;
  brief: BriefEvent;
  back: () => void;
}): JSX.Element {
  return (
    <>
      <PaneSticky>
        <PaneHead back={{ to: "Timeline", go: props.back }} title="Brief" />
      </PaneSticky>

      <div class={styles.composer}>
        {/* The box, which is the whole of what there is to fill in: the Brief
            inside it, and the setup as a row of dropdowns along the inside of
            its bottom edge. */}
        <div class={styles.box}>
          <Written conversation={props.conversation} brief={props.brief} />

          <Setup conversation={props.conversation} />
        </div>

        {/* What the setup has to say that is not a control, under the box
            rather than inside it. */}
        <SetupNotes conversation={props.conversation} />

        {/* And the press the whole pane is arranged for. Only one of the two is
            ever drawn — each is for a different kind of draft — so they read as
            the one thing there is to do from here. */}
        <Show
          when={props.conversation.adopting}
          fallback={<StartGrilling conversation={props.conversation} />}
        >
          {(adopting) => (
            <Adoption conversation={props.conversation} adopting={adopting()} />
          )}
        </Show>
      </div>
    </>
  );
}

/// The Brief itself: the field it is written in, or the document it already is.
///
/// A field whenever the round's Brief is the human's to write — raw markdown in
/// a textarea that is always there, growing with what is typed into it, saving
/// itself on a pause and on the way out. There is no Edit and no Save, because
/// a document that is only ever written in one state does not need a mode to be
/// written in.
///
/// And the rendering where it is not: an adopting Conversation's Brief is the
/// stage's own, so the box is locked to what the server rendered rather than
/// offering a field nothing could be saved from.
function Written(props: {
  conversation: ConversationView;
  brief: BriefEvent;
}): JSX.Element {
  const queries = useQueryClient();

  /// Whether the Brief is the human's to write here.
  const writing = () => !props.brief.frozen;

  // What has been typed, or nothing if nothing has been. The field follows the
  // Event until the first keystroke and follows itself after it, so a read of
  // the Conversation landing mid-sentence cannot take the sentence with it.
  const [typed, setTyped] = createSignal<string | null>(null);
  const text = () => typed() ?? props.brief.markdown;

  // What the record has, as far as this pane knows: what came down with the
  // Event, until a save of its own puts something else there.
  const [kept, setKept] = createSignal<string | null>(null);
  const recorded = () => kept() ?? props.brief.markdown;

  const [refused, setRefused] = createSignal<BriefSaved | null>(null);

  /// Whether the field is ahead of the record, which is the whole of what there
  /// is to save.
  const unsaved = () => text() !== recorded();

  /// Whether a refusal has come back, which stops the field for good: both of
  /// them are permanent — a Brief that has frozen does not thaw, and a
  /// Conversation that is gone does not come back. Trying again every time the
  /// typing paused would be a request a second for as long as the human went on
  /// writing, and the answer would be the one already on the pane.
  const settled = () => refused() !== null;

  const save = useMutation(() => ({
    mutationFn: (markdown: string) => saveBrief(props.conversation.id, markdown),
    onSuccess: (outcome: BriefSaved, markdown: string) => {
      if (outcome !== "Saved") {
        // What was typed stands: it is the only copy of it there is, and the
        // human is owed the chance to take it somewhere else. The commonest
        // refusal is the freeze landing mid-edit, which is why it is said in
        // words rather than left to a field that quietly stopped keeping up.
        setRefused(outcome);
        return;
      }

      setRefused(null);
      setKept(markdown);
      // The readiness verdict under this pane is a fact about the Brief, so it
      // is read again every time the Brief moves.
      void queries.invalidateQueries({ queryKey: ["conversation"] });
    },
    // Whatever became of it, the field may have been typed into while it was in
    // flight — so the moment one save is done the next is considered.
    onSettled: () => keeper.done(),
  }));

  const keeper = keeping({
    unsaved,
    settled,
    save: () => save.mutate(text()),
  });

  return (
    <>
      <Show
        when={writing()}
        fallback={
          <Show
            when={props.brief.markdown !== ""}
            fallback={
              <Empty>
                <Show
                  when={props.conversation.adopting}
                  fallback={<>Nothing was written.</>}
                >
                  Nothing written yet — adopting the stage is what puts its
                  brief here.
                </Show>
              </Empty>
            }
          >
            <div
              class={`${styles.written} markdown`}
              innerHTML={props.brief.html}
            />
          </Show>
        }
      >
        {/* A copy of what has been typed gives the field its height — see
            `.grow` in `App.module.css`. */}
        <div class={app.grow} data-value={text()}>
          <textarea
            rows="1"
            aria-label="Brief"
            placeholder="What is this piece of work?"
            value={text()}
            onInput={(ev) => {
              setTyped(ev.currentTarget.value);
              keeper.settle();
            }}
            onBlur={() => keeper.keep()}
          />
        </div>
      </Show>

      {/* Outside the field rather than under it, so a freeze that lands while
          the human was typing is still explained where it happened once the box
          has gone back to being a rendering. */}
      <Show when={refused()}>
        {(outcome) => (
          <ErrorLine class={styles.failure}>
            {BRIEF_REFUSAL[outcome()]}
          </ErrorLine>
        )}
      </Show>
      <Show when={save.isError}>
        <ErrorLine class={styles.failure}>
          The brief could not be saved: {save.error?.message}
        </ErrorLine>
      </Show>
    </>
  );
}

/// The button that gives a Conversation somewhere to work.
///
/// Drawn whenever there is something to start, ready or not. `ready_to_grill`
/// decides how it *behaves* rather than whether it is there: an unready button
/// looks inert and, pressed, says what is missing instead of starting. So it is
/// `aria-disabled` rather than `disabled` — a truly disabled button takes no
/// press to answer, and its only way of explaining itself is a `title` that a
/// phone will never show. The explanation is on hover as well, for whoever has a
/// pointer to hover with.
///
/// The server checks every one of the conditions again regardless — the page's
/// copy is only as fresh as its last read.
function StartGrilling(props: { conversation: ConversationView }): JSX.Element {
  const queries = useQueryClient();

  const [refused, setRefused] = createSignal<GrillingStarted | null>(null);

  // Whether the explanation is out: pressed, it stays out, because it was asked
  // for; hovered, it comes and goes with the pointer.
  const [asked, setAsked] = createSignal(false);
  const [hovered, setHovered] = createSignal(false);

  const ready = () => props.conversation.ready_to_grill;
  const missing = () => !ready() && (asked() || hovered());

  const start = useMutation(() => ({
    mutationFn: () => startGrilling(props.conversation.id),
    onSuccess: (outcome: GrillingStarted) => {
      if (outcome !== "Started") {
        setRefused(outcome);
        // Refused against a picture of the world this page read a moment ago:
        // reading it again is both the correction and the explanation.
        void queries.invalidateQueries({ queryKey: ["conversation"] });
        void queries.invalidateQueries({ queryKey: ["profiles"] });
        return;
      }

      setRefused(null);
      void queries.invalidateQueries({ queryKey: ["conversation"] });
      void queries.invalidateQueries({ queryKey: ["conversations"] });
    },
  }));

  return (
    <div class={styles.startGrilling}>
      <button
        type="button"
        class={styles.start}
        classList={{ [styles.inert!]: !ready() }}
        // Only ever `disabled` for a press already in flight. Not being ready
        // is the other thing entirely: that press has an answer to give.
        disabled={start.isPending}
        aria-disabled={!ready()}
        onClick={() => (ready() ? start.mutate() : setAsked(true))}
        onMouseEnter={() => setHovered(true)}
        onMouseLeave={() => setHovered(false)}
      >
        {start.isPending ? "Starting…" : "Start work"}
      </button>
      <Show
        when={ready()}
        fallback={
          <Show when={missing()}>
            <Note>This needs a brief, and every role picked and working.</Note>
          </Show>
        }
      >
        <Note>
          This creates the branch and its worktree, and freezes the brief.
        </Note>
      </Show>

      <Show when={refused()}>
        {(outcome) => (
          <ErrorLine class={styles.failure}>{grillRefusal(outcome())}</ErrorLine>
        )}
      </Show>
      <Show when={start.isError}>
        <ErrorLine class={styles.failure}>
          The work could not be started: {start.error?.message}
        </ErrorLine>
      </Show>
    </div>
  );
}
