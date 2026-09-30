//! The composer: where a Conversation is drafted, and the press that starts it.
//!
//! A details pane of its own rather than a card on the Timeline. What is
//! written here — the Brief, the branch, the target, the base, the companions,
//! the three pairings — is one act of setting a piece of work up, and the
//! record is a record of what has happened rather than the desk it is arranged
//! on. So the
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
//! At its measure and in the middle of whatever room the pane has, which is how
//! it stands in the widened pane too: a Conversation whose record is the one
//! Event has no Timeline drawn beside this, so the pane is the Timeline's column
//! and its own — and the box is the same box, centred in the wider room. The
//! measure is the pane's own doing, in `Panes.module.css`.
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
//! The files handed over with the Brief are drawn inside the box as a row of
//! pills, and put on it either through the paperclip beside the press or by
//! dropping them anywhere on the box — one piece doing both, drawn here and on
//! the compose page alike (see [`Attaching`](../Attaching.tsx)), with only what
//! becomes of a chosen file different between them.
//!
//! **And the paperclip is a menu**, here and on the compose page alike, an
//! answer sheet's being the one that stays a plain button: *Attach file* first,
//! and under it the MCP servers declared on the settings page. Picking one puts
//! it on the Conversation and draws a chip for it in that same row, and the
//! chip's × takes it off again and puts it back in the menu. With none declared
//! it is still a menu — *Attach file*, and the way to the settings section where
//! one is declared — because a control that changed shape with the settings
//! would be a control the human had to find twice. The rows are [`ServerRows`],
//! which both pages draw.
//!
//! What is this pane's alone is that a pick lands on the server as it is made:
//! there is a Conversation under this composer, so the chip comes back off the
//! record. The compose page holds its picks until a press, having nothing yet
//! to put them on.
//!
//! **What is attached is a name.** The Conversation records which servers it
//! has, and the declaration is looked up by that name whenever it is needed, so
//! a URL corrected in the settings is the one every Conversation then uses and a
//! declaration deleted there leaves a chip that says the server is gone rather
//! than one that quietly went. Which is why the chip is still removable: the
//! reference is the human's, and so is taking it back.
//!
//! **The servers are the Conversation's rather than the round's**, and they
//! freeze as the files do: attached and taken off while the Conversation
//! drafts, fixed once the work starts, and drawn read-only on the frozen
//! Brief's own pane. Which is what a later drafting round is drawn from too —
//! this pane serves whichever Brief is open, and the chips it draws are the ones
//! the Conversation already had, attached and the human's to take off.
//!
//! Nothing on this pane has a Save. The Brief keeps itself on a pause in the
//! typing and on the way out of the field, every setup field sends its own
//! change as it is made, and what a save cannot do is said in words where it
//! happened.
//!
//! [`Brief`]: ./Brief.tsx

import { A } from "@solidjs/router";
import { useMutation, useQueryClient } from "@tanstack/solid-query";
import {
  For,
  Match,
  Show,
  Switch as Choose,
  createSignal,
  type JSX,
} from "solid-js";

import {
  attachFile,
  attachServer,
  loadSettings,
  removeAttachment,
  removeServer,
  saveBrief,
  startGrilling,
  takeUpPullRequest,
} from "../api/client";
import type {
  Attached,
  AttachmentRemoved,
  AttachmentView,
  BriefEvent,
  BriefSaved,
  ConversationView,
  GrillingStarted,
  McpServer,
  Process,
  ServerAttached,
  ServerRemoved,
  TakenUp,
} from "../api/types";
import app from "../App.module.css";
import { attaching, type Attaching, type Chip, type Shown } from "../Attaching";
import { PaneSticky } from "../Panes";
import shell from "../Panes.module.css";
import { useReading } from "../freshness";
import { Empty, ErrorLine } from "../notices";
import { Adoption } from "./Adoption";
import styles from "./Composer.module.css";
import { refusedOnCreate } from "./composing";
import { PaneHead } from "./PaneHead";
import { DRAFT, chosen } from "./naming";
import { Setup, SetupNotes } from "./Setup";
import { TakeUpRefusal } from "./TakeUp";
import { needed, targeted } from "./processes";
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

  /// The way out of the pane, named as well as pressed — which is the one thing
  /// on this pane that is not the same on every Conversation. A record with
  /// something on it beyond this Brief is read on a Timeline, and that is what
  /// a narrow window walks back through; a record that is the Brief alone has
  /// no Timeline drawn at all, so the way out is the conversations themselves.
  /// Which of the two it is is the workbench's to say — see `Workbench.tsx`,
  /// where the same answer decides whether the middle pane is handed to the
  /// frame in the first place.
  back: { to: string; go: () => void };
}): JSX.Element {
  // The files on this Conversation: the record's own and the ones this device
  // is still sending, and the requests either of them makes.
  const sending = sendingOn({
    conversation: () => props.conversation,
    // Attachments freeze with the Brief, so a frozen round draws the row as a
    // record: no × on a pill, no paperclip, and no drop taken over the box.
    frozen: () => props.brief.frozen,
  });

  // And the MCP servers it has, which are the other half of the one control:
  // the chips in that same row, and the rows the menu offers under *Attach
  // file*. Frozen with the Brief exactly as the files are.
  const servers = serversOn({
    conversation: () => props.conversation,
    frozen: () => props.brief.frozen,
  });

  // And the one piece the whole of the attaching UI is drawn through, here and
  // on the compose page alike — the pills inside the box, the paperclip in the
  // row of presses, and the drop the box itself takes. See `Attaching.tsx`.
  const files = attaching({
    shown: sending.shown,
    chips: servers.chips,
    add: sending.send,
    offered: () => !props.brief.frozen,
    // Which is what makes this control a menu rather than a button — as it is
    // on the compose page, the other place a Brief is written; the answer
    // sheets offer nothing at theirs and keep the plain paperclip. Passed
    // whatever the Brief's freeze, because a frozen one draws no paperclip to
    // open it — the adopting draft's press is the Adoption, and a round that
    // has started has this whole pane replaced by the Brief's own.
    offering: (shut) => (
      <ServerRows
        attached={props.conversation.mcp_servers.map((one) => one.name)}
        attach={(name) => {
          // The menu first, because the press has done its work: the row that
          // was pressed is about to go — the server it named has a chip now —
          // and a card still hanging under the trigger would be a list
          // rearranging itself under the hand that had finished with it.
          shut();
          servers.attach(name);
        }}
      />
    ),
  });

  return (
    <>
      <PaneSticky>
        {/* The branch the work will be done on, which is what this draft is
            called everywhere else it is named — and *Draft* until somebody has
            named it, the invented name being nothing to read. The same reading
            the branch field inside the panel stands on: see `naming.ts`. */}
        <PaneHead
          back={props.back}
          title={chosen(props.conversation) || DRAFT}
        />
      </PaneSticky>

      <div class={`${styles.composer} ${shell.paneComposer}`}>
        {/* The box, which is the whole of what there is to fill in: the Brief
            inside it, and the setup as a row of dropdowns along the inside of
            its bottom edge.

            And the whole of it is what a file is dropped onto — text, pills and
            setup row alike — because what the human is dropping onto is the
            thing they are writing. It highlights while a drag carrying files is
            over it. */}
        <div
          class={styles.box}
          classList={{ [styles.over!]: files.over() }}
          {...files.dropping}
        >
          <Written conversation={props.conversation} brief={props.brief} />

          {/* And the files handed over with it, as a row of pills between the
              text and the setup row — inside the box, because they are part of
              what is being written rather than something under it. */}
          <files.Pills class={styles.attachments} />

          {/* What could not be taken off, under the row it happened in: one
              line for the whole of it rather than one per pill, a pill being a
              name on a line with nowhere in it to say a sentence. */}
          <Show when={sending.refusedRemoval()}>
            {(said) => <ErrorLine class={styles.failure}>{said()}</ErrorLine>}
          </Show>

          {/* And the same for a chip, said in the same place and for the same
              reason: a chip has nowhere in it to say a sentence either, and
              what went wrong with attaching one and with taking one off is the
              same sentence about the same row. */}
          <Show when={servers.refused()}>
            {(said) => <ErrorLine class={styles.failure}>{said()}</ErrorLine>}
          </Show>

          <Setup conversation={props.conversation} />
        </div>

        {/* What the setup has to say that is not a control, under the box
            rather than inside it. */}
        <SetupNotes conversation={props.conversation} />

        {/* And what the create that made this Conversation could not do, for
            the one draft that was made from the compose page rather than
            configured here. Said on this pane because this is where what was
            refused is: the field is drawn holding what the server kept, and
            this is why it is not holding what was composed. See
            `composing.ts`. */}
        <For each={refusedOnCreate(props.conversation.id)}>
          {(said) => <ErrorLine class={styles.failure}>{said}</ErrorLine>}
        </For>

        {/* And what could not be attached, said where every other refusal on
            this pane is said — under the box, and beside the paperclip that
            made the press. One line per file, because a choice is several
            files and each of them was refused for its own reason. */}
        <For each={sending.refusals()}>
          {(said) => <ErrorLine class={styles.failure}>{said}</ErrorLine>}
        </For>

        {/* And the press the whole pane is arranged for. Only one of the two is
            ever drawn — an adopting draft's Brief is the stage's and its press is
            the adoption — so they read as the one thing there is to do from
            here. */}
        <Show
          when={props.conversation.adopting}
          fallback={<Start conversation={props.conversation} files={files} />}
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
                  Nothing written yet — starting the stage is what puts its
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
            `.grow` in `App.module.css`, and `.field` in this pane's own module
            for the three lines it starts at. */}
        <div class={`${app.grow} ${styles.field}`} data-value={text()}>
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

/// What an unready start is waiting on, said in its tooltip — the clauses and
/// the roles counted off the Process's own table rather than written into the
/// sentence, so a Review says it waits on a target and a Develop does not.
const missing = (process: Process): string =>
  `This needs ${needed(process, { brief: true, target: true })}.`;

/// The press the whole pane is arranged for, in whichever of its two shapes
/// this Conversation's Process gives it.
///
/// One press on a composer, so both shapes read *Start work* and both stand in
/// the same row beside the paperclip. What differs is the endpoint behind it and
/// the words a refusal comes back in: a Process pointed at work that is already
/// somewhere else is a wrap-up over whatever the Target names, so its press is
/// the take-up — and every refusal the take-up already had keeps its name and
/// its sentence. See [`Starting`], which is the row both draw.
///
/// Asked of [`targeted`] rather than of the Process by name, that being
/// `processes.ts`'s list to keep: the Target field and the endpoint behind the
/// press are the one fact about a Process, so a stage giving one a target gives
/// it this press along with the field.
function Start(props: {
  conversation: ConversationView;
  files: Attaching;
}): JSX.Element {
  return (
    <Show
      when={targeted(props.conversation.process)}
      fallback={
        <StartGrilling conversation={props.conversation} files={props.files} />
      }
    >
      <StartTakeUp conversation={props.conversation} files={props.files} />
    </Show>
  );
}

/// The button that gives a Conversation somewhere to work, and the lines under
/// it.
///
/// Drawn whenever there is something to start, ready or not. `ready_to_grill`
/// decides how it *behaves* rather than whether it is there: an unready button
/// looks inert, answers a press with nothing, and carries what is missing in a
/// `title` for whoever has a pointer to read it with.
///
/// `aria-disabled` rather than `disabled` all the same, and for the tooltip:
/// a truly disabled button is one a browser will not hover, so the one way it
/// had of explaining itself would go with the press it never takes. What it
/// costs is that a phone reads nothing here — which was decided over keeping a
/// line of prose under the button for it, that line being read once and read
/// past forever after.
///
/// The server checks every one of the conditions again regardless — the page's
/// copy is only as fresh as its last read.
///
/// The markup rather than the press, which is each Process's own: what a start
/// *is* is the same row whichever endpoint it reaches, and two copies of it
/// would be the one that stopped being corrected.
function Starting(props: {
  conversation: ConversationView;
  files: Attaching;

  /// Whether a press is already in flight, which is the one thing the button is
  /// truly disabled for.
  pending: boolean;

  /// What the press does, where the Conversation is ready for one.
  press: () => void;

  /// What came back refused, drawn under the row — a node rather than a string,
  /// because one refusal has a way out of itself in it.
  refused?: JSX.Element;

  /// And what the request itself could not do.
  failed?: string;
}): JSX.Element {
  const ready = () => props.conversation.ready_to_grill;

  return (
    <div class={styles.startGrilling}>
      {/* The paperclip at the near edge and the start at the far one, in the
          row the compose page's two presses stand in — pushed apart by the
          paperclip's own margin, the way the roadmap dropdown is. */}
      <div class={styles.presses}>
        <props.files.Clip class={styles.attach} />
        <button
          type="button"
          class={styles.start}
          classList={{ [styles.inert!]: !ready() }}
          // Only ever `disabled` for a press already in flight. Not being
          // ready is the other thing entirely: the button is still hoverable,
          // which is what carries the explanation.
          disabled={props.pending}
          aria-disabled={!ready()}
          title={ready() ? undefined : missing(props.conversation.process)}
          onClick={() => ready() && props.press()}
        >
          {props.pending ? "Starting…" : "Start work"}
        </button>
      </div>

      <Show when={props.refused}>
        {(refused) => (
          <ErrorLine class={styles.failure}>{refused()}</ErrorLine>
        )}
      </Show>
      <Show when={props.failed}>
        {(failed) => (
          <ErrorLine class={styles.failure}>
            The work could not be started: {failed()}
          </ErrorLine>
        )}
      </Show>
    </div>
  );
}

/// The press on every Process that opens a round of its own: the grill start,
/// which cuts the branch and launches the first session in it.
function StartGrilling(props: {
  conversation: ConversationView;
  files: Attaching;
}): JSX.Element {
  const queries = useQueryClient();

  const [refused, setRefused] = createSignal<GrillingStarted | null>(null);

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
    <Starting
      conversation={props.conversation}
      files={props.files}
      pending={start.isPending}
      press={() => start.mutate()}
      refused={
        refused() === null ? undefined : grillRefusal(refused() as GrillingStarted)
      }
      failed={start.isError ? start.error?.message : undefined}
    />
  );
}

/// And the press on a Process pointed at a target — a **Review** or a **Fix
/// Merge Issues** — which opens no round at all: what the Target names is taken
/// up, and the Conversation lands in Wrapping with the wrap-up its Process has
/// running over it.
///
/// Everything it turns on is the server's — the field read for a pull request or
/// a branch, GitHub asked about the one and origin about the other, and the branch
/// settled against origin either way — so what this does with an answer that is
/// not *TakenUp* is say it and read the page again. A branch has one more step
/// behind the press and it is the server's too: the pull request nobody opened is
/// sent for, and the wrap-up runs over what comes back.
function StartTakeUp(props: {
  conversation: ConversationView;
  files: Attaching;
}): JSX.Element {
  const queries = useQueryClient();

  const [refused, setRefused] = createSignal<TakenUp | null>(null);

  const start = useMutation(() => ({
    mutationFn: () => takeUpPullRequest(props.conversation.id),
    onSuccess: (outcome: TakenUp) => {
      // Whatever it came back with, the page is read again: what the take-up
      // did is a conversation that has moved, and what refused it is a
      // repository — or a GitHub — that has moved, and reading it again is the
      // correction either way.
      setRefused(outcome === "TakenUp" ? null : outcome);

      void queries.invalidateQueries({ queryKey: ["conversation"] });
      void queries.invalidateQueries({ queryKey: ["conversations"] });
      void queries.invalidateQueries({ queryKey: ["profiles"] });
    },
  }));

  return (
    <Starting
      conversation={props.conversation}
      files={props.files}
      pending={start.isPending}
      press={() => start.mutate()}
      refused={
        refused() === null ? undefined : (
          <TakeUpRefusal outcome={refused() as TakenUp} />
        )
      }
      failed={start.isError ? start.error?.message : undefined}
    />
  );
}

/// What could not be attached, one sentence per way of being refused.
///
/// Every one of them is something different to go and do, which is why the
/// server names them apart rather than saying that the file could not be
/// attached.
export const ATTACH_REFUSAL: Record<
  Exclude<Attached, { Attached: unknown }>,
  string
> = {
  NoSuchConversation: "This conversation is gone.",
  NotDrafting: "The work has started, so its files are settled.",
  TooLarge: "That file is larger than 32 MB.",
  NotAName:
    "That name cannot be a file here: no folders, and nothing starting with a dot.",
};

/// And taking one off again.
export const ATTACHMENT_REMOVAL_REFUSAL: Record<AttachmentRemoved, string> = {
  Removed: "",
  NoSuchConversation: "This conversation is gone.",
  NotDrafting: "The work has started, so its files are settled.",
};

/// What could not be done about an MCP server, one sentence per way of being
/// refused.
///
/// One table for both presses, unlike the files' two: attaching a server and
/// taking one off are refused for the same reasons, said in the same words, and
/// the one refusal only a press *on* can meet reads as a sentence about the
/// declaration rather than about the press.
export const SERVER_REFUSAL: Record<
  Exclude<ServerAttached | ServerRemoved, "Attached" | "Removed">,
  string
> = {
  NoSuchConversation: "This conversation is gone.",
  NotDrafting: "The work has started, so its servers are settled.",
  NoSuchServer: "That server is no longer declared in the settings.",
};

/// What a draft's composer does about its MCP servers, held once for the pane:
/// the chips it hands the attaching piece, the two presses, and whatever came
/// back refused from either of them.
type Servers = {
  /// The row of chips: what the Conversation holds, each with its × where there
  /// is still one to press.
  chips: () => Array<Chip>;

  /// Put one on, by name.
  attach: (name: string) => void;

  /// And what could not be done, one line for the whole row — a chip is a name
  /// on a line with nowhere in it to say a sentence, exactly as a pill is.
  refused: () => string | null;
};

/// The whole of that, made once by the composer.
///
/// Not a mutation, for the reason the files' sending is not one: the row is a
/// list and each press is its own request with its own answer, so what is in
/// flight is per server rather than one `isPending` for the row. The query it
/// invalidates is the one every press on this pane invalidates — the chip is
/// drawn from the Conversation, so the read is both how it arrives and how it
/// goes.
function serversOn(what: {
  conversation: () => ConversationView;

  /// Whether the round's Brief has frozen, which is what takes the × off every
  /// chip: the servers freeze with it, and a control that could only be refused
  /// is not drawn.
  frozen: () => boolean;
}): Servers {
  const queries = useQueryClient();

  const [refused, setRefused] = createSignal<string | null>(null);

  // Which names have a press in flight, either way: the one thing a row of the
  // menu and a × on a chip can be truly disabled for is a press already made.
  const [working, setWorking] = createSignal<Array<string>>([]);

  /// One press, whichever of the two it is.
  ///
  /// The same shape both ways down to the sentence it says, which is what
  /// [`SERVER_REFUSAL`] is one table for: what the human did is put a name on
  /// this Conversation or take it off, and what can be wrong with either is the
  /// state of the Conversation.
  const press = (
    name: string,
    made: Promise<ServerAttached | ServerRemoved>,
    said: string,
  ) => {
    setWorking((was) => [...was, name]);

    void made
      .then((outcome: ServerAttached | ServerRemoved) => {
        setRefused(
          outcome === "Attached" || outcome === "Removed"
            ? null
            : SERVER_REFUSAL[outcome],
        );

        // Either way: what came back is about a Conversation this pane read a
        // moment ago, so reading it again is both the correction and — where
        // the chip is simply there, or simply gone — the whole of what there
        // was to do.
        void queries.invalidateQueries({ queryKey: ["conversation"] });
      })
      .catch((error: unknown) =>
        setRefused(
          `${name} ${said}: ${
            error instanceof Error ? error.message : String(error)
          }`,
        ),
      )
      .finally(() => setWorking((was) => was.filter((one) => one !== name)));
  };

  const attach = (name: string) => {
    setRefused(null);
    press(name, attachServer(what.conversation().id, name), "could not be attached");
  };

  const forget = (name: string) =>
    press(name, removeServer(what.conversation().id, name), "could not be removed");

  const chips = (): Array<Chip> =>
    what.conversation().mcp_servers.map((server) => ({
      name: server.name,
      gone: !server.declared,
      remove: what.frozen() ? undefined : () => forget(server.name),
      removing: working().includes(server.name),
    }));

  return { chips, attach, refused };
}

/// The rows an Attach menu holds under *Attach file*: one per declared MCP
/// server that is not on already.
///
/// **Both pages that write a Brief draw these**, which is why what is already
/// on arrives as a list of names rather than as the Conversation: the composer
/// reads them off the record and the compose page off what it is holding, and
/// the menu is the same menu either way — see [`Compose`](./Compose.tsx).
///
/// The declarations are read here rather than beside the Conversation, and read
/// when the menu opens rather than when the pane does — the rows of a menu are
/// built on the way open and thrown away on the way closed (see `Menu.tsx`), so
/// a composer nobody pressed Attach on reads no settings at all, and one that is
/// pressed reads them as they stand at that moment.
///
/// **A server already attached is not here**, having a chip of its own in the
/// row: the menu is what can still be put on, and the × on the chip is what puts
/// one back.
///
/// **And with nothing declared it is still a menu**, with the way to the
/// settings section where a declaration is made. A control that fell back to a
/// plain button would be one the human had to learn twice — and the answer to
/// an empty menu is to go and declare one, which is a thing the menu can say.
export function ServerRows(props: {
  /// The names that are on already, in whatever the page holds them in.
  attached: Array<string>;
  attach: (name: string) => void;
}): JSX.Element {
  const settings = useReading(() => ({
    queryKey: ["settings"],
    queryFn: loadSettings,
    freshness: { reconcile: "id" } as const,
  }));

  const attached = () => new Set(props.attached);

  /// The declarations this Conversation could still be given.
  const offerable = (): Array<McpServer> =>
    (settings.data?.mcp_servers ?? []).filter(
      (server) => !attached().has(server.name),
    );

  return (
    <Choose>
      {/* The two nothing can be picked out of yet, each a row that says which
          it is and cannot be pressed — a menu that came down empty would say
          neither. */}
      <Match when={settings.isPending}>
        <button type="button" role="menuitem" disabled>
          Reading the settings…
        </button>
      </Match>
      <Match when={settings.isError}>
        <button type="button" role="menuitem" disabled>
          The settings could not be read.
        </button>
      </Match>
      <Match when={settings.data}>
        <Show
          when={(settings.data?.mcp_servers.length ?? 0) > 0}
          fallback={
            <A href="/settings/mcp-servers" role="menuitem">
              Declare an MCP server…
            </A>
          }
        >
          <For each={offerable()}>
            {(server) => (
              <button
                type="button"
                role="menuitem"
                onClick={() => props.attach(server.name)}
              >
                {server.name}
              </button>
            )}
          </For>
        </Show>
      </Match>
    </Choose>
  );
}

/// One file this device is still sending.
///
/// Its own key rather than its name, because two files chosen together may
/// share one — and what the key is for is taking the right one out of the row
/// when the right request lands.
type Landing = { key: number; name: string };

/// What a draft's composer does about its files, held once for the pane: the
/// row of pills it hands the attaching piece, the requests a choice and a ×
/// make, and what came back refused from either of them.
type Sending = {
  /// The row: the files on the record, and the ones on their way up after them.
  shown: () => Array<Shown>;

  /// What could not be attached, one line per file that was refused.
  refusals: () => Array<string>;

  /// And what could not be taken off again — one line for the whole row rather
  /// than one per pill: a pill is a name on a line and there is nowhere inside
  /// one to say a sentence, and two failed removals are the same sentence
  /// twice.
  refusedRemoval: () => string | null;

  /// Send every file that was chosen, each on a request of its own.
  send: (files: Array<File>) => void;
};

/// The whole of that, made once by the composer.
///
/// Not a mutation, which is what every other press on this pane is. A choice is
/// several files at once and each of them is its own request with its own
/// answer — a mutation is one observer with one `isPending` and one `error`,
/// and three uploads through it would be two answers thrown away. So the state
/// is held here, per file, and the query it invalidates is the same one every
/// mutation on this pane invalidates.
function sendingOn(what: {
  conversation: () => ConversationView;

  /// Whether the round's Brief has frozen, which is what takes the × off every
  /// pill: attachments freeze with the Brief, and a control that could only be
  /// refused is not drawn.
  frozen: () => boolean;
}): Sending {
  const queries = useQueryClient();

  const [landing, setLanding] = createSignal<Array<Landing>>([]);
  const [refusals, setRefusals] = createSignal<Array<string>>([]);
  const [refusedRemoval, setRefusedRemoval] = createSignal<string | null>(null);

  // Which of the record's files have a removal in flight, by id: the one thing
  // a × can be truly disabled for is a press already made.
  const [removing, setRemoving] = createSignal<Array<number>>([]);

  let keys = 0;

  const send = (files: Array<File>) => {
    // A fresh choice clears what the last one had to say: the lines are about
    // files the human has moved on from, and leaving them under a row that has
    // changed would be the pane explaining something that is no longer there.
    setRefusals([]);

    for (const file of files) {
      const key = (keys += 1);
      setLanding((held) => [...held, { key, name: file.name }]);

      void attachFile(what.conversation().id, file)
        .then(async (outcome) => {
          if (typeof outcome === "string") {
            setRefusals((said) => [
              ...said,
              `${file.name}: ${ATTACH_REFUSAL[outcome]}`,
            ]);
            return;
          }

          // What came back is the record it made, and the pill is drawn from
          // the Conversation — so the read is both how the pill arrives and
          // how it arrives under the name the server renamed it to.
          //
          // Waited on, because the dimmed pill comes off the row in the
          // `finally` below and the pill that replaces it is the one this read
          // brings back: firing the read and carrying straight on would be the
          // file blinking out of the row and back into it.
          await queries.invalidateQueries({ queryKey: ["conversation"] });
        })
        .catch((error: unknown) => {
          setRefusals((said) => [
            ...said,
            `${file.name} could not be attached: ${
              error instanceof Error ? error.message : String(error)
            }`,
          ]);
        })
        .finally(() =>
          setLanding((held) => held.filter((one) => one.key !== key)),
        );
    }
  };

  /// And taking one off the record, which is the × on a pill.
  ///
  /// Held here beside the sending for the same reason: the row is a list of
  /// files and each × is its own request, so what is in flight is per file
  /// rather than one `isPending` for the row.
  const forget = (attachment: AttachmentView) => {
    setRemoving((was) => [...was, attachment.id]);

    void removeAttachment(what.conversation().id, attachment.id)
      .then((outcome: AttachmentRemoved) => {
        setRefusedRemoval(
          outcome === "Removed" ? null : ATTACHMENT_REMOVAL_REFUSAL[outcome],
        );

        // Either way: what came back is about a conversation this pane read a
        // moment ago, so reading it again is both the correction and — where
        // the pill is simply gone — the whole of what there was to do.
        void queries.invalidateQueries({ queryKey: ["conversation"] });
      })
      .catch((error: unknown) =>
        setRefusedRemoval(
          `${attachment.name} could not be removed: ${
            error instanceof Error ? error.message : String(error)
          }`,
        ),
      )
      .finally(() =>
        setRemoving((was) => was.filter((id) => id !== attachment.id)),
      );
  };

  /// The row itself: what the record holds, and what this device is still
  /// sending drawn dimmed at the end of it — a file chosen with no pill until
  /// the record came back would be a press with no answer.
  const shown = (): Array<Shown> => [
    ...what.conversation().attachments.map((attachment) => ({
      name: attachment.name,
      remove: what.frozen() ? undefined : () => forget(attachment),
      removing: removing().includes(attachment.id),
    })),
    ...landing().map((one) => ({ name: one.name, landing: true })),
  ];

  return { shown, refusals, refusedRemoval, send };
}
