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
//! Nothing on this pane has a Save. The Brief keeps itself on a pause in the
//! typing and on the way out of the field, every setup field sends its own
//! change as it is made, and what a save cannot do is said in words where it
//! happened.
//!
//! [`Brief`]: ./Brief.tsx

import { useMutation, useQueryClient } from "@tanstack/solid-query";
import { For, Show, createSignal, type JSX } from "solid-js";

import { faPaperclip } from "@fortawesome/free-solid-svg-icons";

import {
  attachFile,
  removeAttachment,
  saveBrief,
  startGrilling,
} from "../api/client";
import type {
  Attached,
  AttachmentRemoved,
  AttachmentView,
  BriefEvent,
  BriefSaved,
  ConversationView,
  GrillingStarted,
} from "../api/types";
import app from "../App.module.css";
import { IconButton } from "../IconButton";
import { PaneSticky } from "../Panes";
import shell from "../Panes.module.css";
import { Truncated } from "../Truncated";
import { Empty, ErrorLine } from "../notices";
import { Adoption } from "./Adoption";
import styles from "./Composer.module.css";
import { refusedOnCreate } from "./composing";
import { PaneHead } from "./PaneHead";
import { DRAFT, chosen } from "./naming";
import { NoSessions, noSessions } from "./sessions";
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
  // The files this device is sending, made once here and read in the two places
  // an attachment is drawn: the paperclip under the box makes the uploads, and
  // the row of pills inside it shows the ones that have not landed yet.
  const sending = attaching(() => props.conversation);

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
            its bottom edge. */}
        <div class={styles.box}>
          <Written conversation={props.conversation} brief={props.brief} />

          {/* And the files handed over with it, as a row of pills between the
              text and the setup row — inside the box, because they are part of
              what is being written rather than something under it. */}
          <Attachments
            conversation={props.conversation}
            attaching={sending}
            frozen={props.brief.frozen}
          />

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
            ever drawn — each is for a different kind of draft — so they read as
            the one thing there is to do from here. */}
        <Show
          when={props.conversation.adopting}
          fallback={
            <StartGrilling
              conversation={props.conversation}
              attaching={sending}
            />
          }
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

/// What an unready start is waiting on, said in its tooltip.
const MISSING = "This needs a brief, and every role picked and working.";

/// The button that gives a Conversation somewhere to work.
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
/// **Except on a Verkstead with no session to start**, where there is no button
/// at all and the state stands in its place: not being ready is something to go
/// and fix, and this is not — see `sessions.tsx`. The press is refused by name
/// regardless, which is what `grillRefusal` is filled in for. The paperclip
/// goes with it, being the near edge of a row whose far edge is that press:
/// where there is nothing to start, the pane says so rather than offering the
/// half of the row that is left.
function StartGrilling(props: {
  conversation: ConversationView;
  attaching: Attaching;
}): JSX.Element {
  const queries = useQueryClient();

  const [refused, setRefused] = createSignal<GrillingStarted | null>(null);

  const ready = () => props.conversation.ready_to_grill;

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
      <Show
        when={!noSessions(props.conversation)}
        fallback={<NoSessions class={styles.noSessions} />}
      >
        {/* The paperclip at the near edge and the start at the far one, in
            the row the compose page's two presses stand in — pushed apart by
            the paperclip's own margin, the way the roadmap dropdown is. */}
        <div class={styles.presses}>
          <Attach attaching={props.attaching} />
          <button
            type="button"
            class={styles.start}
            classList={{ [styles.inert!]: !ready() }}
            // Only ever `disabled` for a press already in flight. Not being
            // ready is the other thing entirely: the button is still hoverable,
            // which is what carries the explanation.
            disabled={start.isPending}
            aria-disabled={!ready()}
            title={ready() ? undefined : MISSING}
            onClick={() => ready() && start.mutate()}
          >
            {start.isPending ? "Starting…" : "Start work"}
          </button>
        </div>

        <Show when={refused()}>
          {(outcome) => (
            <ErrorLine class={styles.failure}>
              {grillRefusal(outcome())}
            </ErrorLine>
          )}
        </Show>
        <Show when={start.isError}>
          <ErrorLine class={styles.failure}>
            The work could not be started: {start.error?.message}
          </ErrorLine>
        </Show>
      </Show>
    </div>
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

/// One file this device is still sending.
///
/// Its own key rather than its name, because two files chosen together may
/// share one — and what the key is for is taking the right pill away when the
/// right request lands.
type Landing = { key: number; name: string };

/// The uploading, held once for the pane and read in two places: the paperclip
/// under the box makes them, and the pills inside it draw the ones that have
/// not landed yet.
type Attaching = {
  /// The files this device is sending, in the order they were chosen.
  landing: () => Array<Landing>;

  /// What could not be attached, one line per file that was refused.
  refusals: () => Array<string>;

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
function attaching(conversation: () => ConversationView): Attaching {
  const queries = useQueryClient();

  const [landing, setLanding] = createSignal<Array<Landing>>([]);
  const [refusals, setRefusals] = createSignal<Array<string>>([]);

  let keys = 0;

  const send = (files: Array<File>) => {
    // A fresh choice clears what the last one had to say: the lines are about
    // files the human has moved on from, and leaving them under a row that has
    // changed would be the pane explaining something that is no longer there.
    setRefusals([]);

    for (const file of files) {
      const key = (keys += 1);
      setLanding((held) => [...held, { key, name: file.name }]);

      void attachFile(conversation().id, file)
        .then((outcome) => {
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
          void queries.invalidateQueries({ queryKey: ["conversation"] });
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

  return { landing, refusals, send };
}

/// The paperclip, at the near edge of the row the start press is at the far
/// edge of.
///
/// A button over the browser's own file picker rather than the picker itself:
/// an `<input type="file">` draws a control of the platform's choosing with a
/// word beside it, and what belongs in that row is an icon. So the input is
/// there and hidden, and the button is what reaches it.
///
/// Several files at once, each uploaded on its own request. A folder cannot be
/// chosen through a picker at all — that is what a drop can hand over, and
/// where one is skipped.
function Attach(props: { attaching: Attaching }): JSX.Element {
  let picker!: HTMLInputElement;

  return (
    <>
      <IconButton
        of={faPaperclip}
        label="Attach a file"
        open={false}
        press={() => picker.click()}
        class={styles.attach}
      />
      <input
        ref={picker}
        class={styles.picker}
        type="file"
        multiple
        onChange={(ev) => {
          props.attaching.send(Array.from(ev.currentTarget.files ?? []));
          // Emptied on the way out, so that choosing the same file again is a
          // change: an input still holding what was chosen last fires nothing.
          ev.currentTarget.value = "";
        }}
      />
    </>
  );
}

/// The files on the Conversation, as a row of pills between the Brief text and
/// the setup row inside the box.
///
/// Nothing at all where there are none and none on their way, rather than an
/// empty row with a heading over it — which is most drafts, the way most
/// conversations have no companions.
///
/// The ones this device is still sending are drawn at the end and dimmed: the
/// file is chosen and the record is not back yet, and a row that showed nothing
/// until it was would be a press with no answer.
function Attachments(props: {
  conversation: ConversationView;
  attaching: Attaching;

  /// Whether the Brief this row belongs to has frozen, which is what takes the
  /// remove presses off: attachments freeze with the Brief, and a control that
  /// cannot do anything is not drawn.
  frozen: boolean;
}): JSX.Element {
  // One line for the whole row rather than one per pill: a pill is a name on a
  // line and there is nowhere inside one to say a sentence, and two failed
  // removals are the same sentence twice.
  const [refused, setRefused] = createSignal<string | null>(null);

  return (
    <Show
      when={
        props.conversation.attachments.length ||
        props.attaching.landing().length
      }
    >
      <ul class={styles.attachments} aria-label="Attached files">
        <For each={props.conversation.attachments}>
          {(attachment) => (
            <Attachment
              conversation={props.conversation}
              attachment={attachment}
              frozen={props.frozen}
              refuse={setRefused}
            />
          )}
        </For>
        <For each={props.attaching.landing()}>
          {(one) => (
            <li class={`${styles.attachment} ${styles.landing}`}>
              <Truncated text={one.name} class={styles.attachmentName} />
            </li>
          )}
        </For>
      </ul>

      <Show when={refused()}>
        {(said) => <ErrorLine class={styles.failure}>{said()}</ErrorLine>}
      </Show>
    </Show>
  );
}

/// One of them: the name, cut to a line, and the × that takes it away.
///
/// Cut at the front, which is how every other name in the app is cut — see
/// [`Truncated`](../Truncated.tsx). On a file name that is the half worth
/// keeping too: the extension is what says what the thing is, and the whole
/// name is under the pointer either way.
function Attachment(props: {
  conversation: ConversationView;
  attachment: AttachmentView;
  frozen: boolean;
  refuse: (said: string | null) => void;
}): JSX.Element {
  const queries = useQueryClient();

  const forget = useMutation(() => ({
    mutationFn: () =>
      removeAttachment(props.conversation.id, props.attachment.id),
    onSuccess: (outcome: AttachmentRemoved) => {
      props.refuse(
        outcome === "Removed" ? null : ATTACHMENT_REMOVAL_REFUSAL[outcome],
      );

      // Either way: what came back is about a conversation this pane read a
      // moment ago, so reading it again is both the correction and — where the
      // pill is simply gone — the whole of what there was to do.
      void queries.invalidateQueries({ queryKey: ["conversation"] });
    },
    onError: (error: Error) =>
      props.refuse(
        `${props.attachment.name} could not be removed: ${error.message}`,
      ),
  }));

  return (
    <li class={styles.attachment}>
      <Truncated text={props.attachment.name} class={styles.attachmentName} />

      {/* A mark rather than a word, the way a companion row's is, and named for
          this file: the row is a line of names and the × on its own says
          nothing about which one it takes. */}
      <Show when={!props.frozen}>
        <button
          type="button"
          class={styles.forget}
          aria-label={`Remove ${props.attachment.name}`}
          disabled={forget.isPending}
          onClick={() => forget.mutate()}
        >
          ×
        </button>
      </Show>
    </li>
  );
}
