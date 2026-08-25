//! A Conversation's Timeline: everything that has happened to it, in order.
//!
//! The kinds of Event so far — the Brief, a move, what a session printed, a
//! Question Set, the handoff, the Notices Verkstead writes on its own account,
//! and the commits a session lands on the branch — drawn as a list of Events
//! rather than as a Brief with a list under it.
//!
//! Above the list are the pinned Events, which are a fixed set — the backlog
//! now, the stage list and the PR as those stages arrive. They are not on the
//! record and do not scroll with it: each is the current state of something the
//! work is against rather than a moment in it. More than one of them is a
//! carousel rather than a stack, because everything pinned is held above the
//! record and a stack of them is what the record is pushed down by.
//!
//! An Event that has a full self shows its summary here and is opened in the
//! details pane, which is why this takes a way of selecting one. Three of them
//! are documents — the frozen Brief, the handoff and a Manual Task's
//! instruction — and a document's summary is its own opening: the card shows
//! [`CLAMPED_LINES`] of it under a fade, and the pane holds the whole. The
//! Brief is also the one Event that is written here as well as read: while the
//! Conversation is drafting it is a field that saves itself rather than a card
//! to open, and it carries a Conversation's setup under it for as long as there
//! is a draft to set up.
//!
//! The Timeline is also where the work is moved on from, because that is where
//! the reason to move it is: a control sits at the end of everything that has
//! happened so far, which is exactly where the next thing to happen belongs.
//! Two of them live there — `Start grilling` under the Brief it will freeze,
//! and, on a conversation Verkstead has finished with, the press that opens a
//! second round with a Brief of its own.
//! Stopping the work is in neither place and not in the list: none of the three
//! ways of doing it — stop after this task, stop now, abort the conversation —
//! is a step in the work, so all three hang off the header behind a menu, where
//! what cannot be undone is not one stray click away.
//!
//! Nothing here ends the grilling, and nothing here chooses a direction. That is
//! the agent's own closing move — a Question Set carrying a proposal, with the
//! chooser drawn on the Set itself — so both happen on the page the Set is
//! answered on and land here as the answered Set.

import { useMutation, useQueryClient } from "@tanstack/solid-query";
import {
  For,
  Match,
  Show,
  Switch,
  createSignal,
  onCleanup,
  onMount,
  type JSX,
} from "solid-js";

import {
  abortConversation,
  forceStopConversation,
  listProfiles,
  reopenConversation,
  resume,
  saveBrief,
  startGrilling,
  startManualTask,
  stopConversation,
} from "../api/client";
import type {
  AgentOutputEvent,
  BriefEvent,
  BriefSaved,
  CommitEvent,
  ConversationAborted,
  ConversationReopened,
  ConversationStopped,
  ConversationView,
  GrillingStarted,
  HandoffEvent,
  Lifecycle,
  ManualTaskEvent,
  ManualTaskStarted,
  MovedEvent,
  NoticeEvent,
  PinnedEvent,
  PullRequestEvent,
  QuestionSetEvent,
  Resumed,
  StageListEvent,
  TaskListEvent,
  TimelineEvent,
  UnreadableSetEvent,
} from "../api/types";
import { Menu } from "../Menu";
import { useReading } from "../freshness";
import { Empty, ErrorLine, Note } from "../notices";
import * as pairing from "../pairing";
import { Picker } from "../picking";
import { Adoption } from "./Adoption";
import { Mark } from "./Mark";
import { PaneHead } from "./PaneHead";
import { Pause } from "./Pause";
import { Setup } from "./Setup";
import { keeping } from "./settling";

/// How much of a commit's hash the timeline shows.
///
/// Seven characters, which is what git prints and what everybody reads a commit
/// by. The whole hash travels on the wire — what it takes to be unambiguous
/// grows with a repository, and shortening for reading is a different thing
/// from recording one short.
export const ABBREVIATED = 7;

/// What each way of being refused a Brief says.
///
/// `Saved` is here for completeness of the mapping and never drawn: a save that
/// worked says nothing at all, because a field quietly keeping up is what the
/// human already expects of it.
export const BRIEF_REFUSAL: Record<BriefSaved, string> = {
  Saved: "",
  NoSuchConversation: "This conversation is gone.",
  NotDrafting:
    "The brief was frozen when grilling started, so it cannot be edited.",
};

/// And each way of being refused a start.
///
/// Every one of them is something different to go and do, which is the whole
/// reason the server names them separately rather than saying "cannot start".
export const GRILL_REFUSAL: Record<GrillingStarted, string> = {
  Started: "",
  NoSuchConversation: "This conversation is gone.",
  NotDrafting: "This conversation has already been started.",
  NoGrillingProfile: "Choose a grilling profile and model first, on the brief.",
  NoImplementationProfile:
    "Choose an implementation profile and model first, on the brief.",
  ProfileBroken:
    "A chosen profile's claude pair is not where it was left, so there is no account to run under.",
  EmptyBrief: "Write the brief first — it is what the grilling starts from.",
  NoBaseCommit: "The repo has nothing to branch from any more.",
  BranchExists: "That branch already exists, and Verkstead did not make it.",
  WorktreeRefused: "Git would not make the worktree. The server log says why.",
};

/// And each way of being refused a stop, whichever of the two was pressed.
///
/// The two that are not refusals map to nothing: a conversation that has
/// stopped says so by the badge and the notice the read after the press brings
/// back, and one that is still finishing its task says so in its own words
/// beside the button rather than as an error.
export const STOP_REFUSAL: Record<ConversationStopped, string> = {
  Stopped: "",
  Stopping: "",
  AlreadyHalted:
    "This conversation has already stopped. Resume is what gets it going again.",
  NotDriven:
    "Nothing is supposed to be driving this conversation, so there is nothing to stop.",
  NoSuchConversation: "This conversation is gone.",
};

/// And each way of being refused an abort.
export const ABORT_REFUSAL: Record<ConversationAborted, string> = {
  Aborted: "",
  AlreadyAborted: "",
  NoSuchConversation: "This conversation is gone.",
  WorktreeStuck:
    "The worktree could not be removed, so nothing was changed. The server log says why.",
};

/// And each way of being refused a manual task.
///
/// `AlreadyRunning` is the one worth reading twice: the composer is drawn
/// wherever nothing is running, so a submit that arrives to find something
/// running was pressed against a page a moment out of date. Nothing is queued,
/// because an instruction written against a worktree that has since moved may no
/// longer be the thing to do.
export const MANUAL_TASK_REFUSAL: Record<ManualTaskStarted, string> = {
  Started: "",
  NoSuchConversation: "This conversation is gone.",
  NowhereToWork:
    "This conversation has no worktree to run in — start the grilling first.",
  AlreadyRunning:
    "An agent is already running here, so nothing was started. Have a look at what it is doing and ask again after.",
  EmptyInstruction: "Say what to do — the instruction is the whole of the task.",
  NoSuchProfile: "That profile has been removed.",
  NoSuchModel: "That profile no longer lists that model.",
  NotStarted:
    "The instruction is on the timeline and no session could be started for it. The server log says why.",
};

/// And each way of being refused a resume.
///
/// Every one of them is the button doing the one thing it is for: saying what
/// there is to do about a conversation nothing is driving. A press that quietly
/// found nothing to start would leave the human exactly as stuck as they were,
/// which is why the server names these rather than logging them.
export const RESUME_REFUSAL: Record<Resumed, string> = {
  Resumed: "",
  NoSuchConversation: "This conversation is gone.",
  NotDriven:
    "Nothing is supposed to be driving this conversation, so there is nothing to start again.",
  AlreadyDriven:
    "Something is already driving this conversation. Have a look at what it is doing.",
  NowhereToWork:
    "This conversation has no worktree to work in, so there is nowhere to start.",
  WorktreeRefused:
    "This conversation's worktree is broken and git would not make it again from the branch. The server log says why.",
  NoDirection:
    "Nothing on the record says how this work is being built, so there is no run to pick up.",
  NothingToWork:
    "There is no backlog left to work — nothing was written, or it is finished with. Set the next thing going by hand.",
  NoGrillingPairing:
    "Choose a grilling profile and model first, on the brief.",
  NoImplementationPairing:
    "Choose an implementation profile and model first, on the brief.",
};

/// And each way of being refused a second round.
export const REOPEN_REFUSAL: Record<ConversationReopened, string> = {
  Reopened: "",
  NoSuchConversation: "This conversation is gone.",
  NotDone:
    "Only a finished conversation can be reopened, and this one is not finished.",
  WorktreeRefused:
    "The worktree is gone and git would not check the branch out again. The server log says why.",
};

/// Whether the event the *blocked on you* badge points at has a details pane
/// behind it.
///
/// Every other thing that stops a run does — a halt opens the Notice saying what
/// stopped, a held session opens its screen — and a pause does not: what it has
/// to say is three short facts and they are drawn whole in the list, with the
/// press on them. So the badge selects it and stays put, rather than sending a
/// narrow window away from the very thing there is to press.
function opensAPane(conversation: ConversationView, event: number): boolean {
  return !conversation.timeline.some(
    (entry) => "Pause" in entry && entry.Pause.id === event,
  );
}

/// The state a move came *from*: the state the move before it went to, and
/// `Draft` where there is no move before it, since a Conversation starts
/// drafting and its first move is the one out of that.
///
/// A move records only the state it went to, so the other half of the
/// transition is the Timeline's own to work out by reading back up itself.
function movedFrom(timeline: TimelineEvent[], index: number): Lifecycle {
  for (const event of timeline.slice(0, index).reverse()) {
    if ("Moved" in event) {
      return event.Moved.state;
    }
  }

  return "Draft";
}

/// How many lines of a document a card shows before it is cut off.
///
/// Five: enough for the opening of a handoff or an instruction to say what it is
/// about, and not enough for either to push the record off the screen. Where the
/// fifth line ends is a fact about the laid-out box rather than about the
/// markdown — how wide the pane is decides it — so the clamp is a height in the
/// stylesheet and this is what that height is written from.
export const CLAMPED_LINES = 5;

/// A document's markdown on a card, cut off at [`CLAMPED_LINES`].
///
/// The fade over the last line is drawn only where the document goes on under
/// it: it says there is more, and a card that already shows the whole thing
/// would be saying something untrue. Whether it overflows is another fact about
/// the laid-out box, so it is measured rather than counted — the observer
/// watches the markdown inside the clamp, whose height is the document's own, so
/// a rendering that changed and a pane that was resized both come back through
/// it.
function Clamped(props: { class: string; html: string }): JSX.Element {
  let clamp: HTMLDivElement | undefined;
  let body: HTMLDivElement | undefined;

  const [cut, setCut] = createSignal(false);

  onMount(() => {
    const measure = () => {
      if (clamp) {
        // A pixel of slack: a line height that is not a whole number of pixels
        // rounds either way, and a card faded over the last pixel of a document
        // that fits would read as one with something after it.
        setCut(clamp.scrollHeight - clamp.clientHeight > 1);
      }
    };

    const watching = new ResizeObserver(measure);

    if (body) {
      watching.observe(body);
    }

    onCleanup(() => watching.disconnect());
  });

  return (
    <div class="clamp" classList={{ cut: cut() }} ref={clamp}>
      <div class={`${props.class} markdown`} innerHTML={props.html} ref={body} />
    </div>
  );
}

/// A card whose whole surface opens the details pane.
///
/// Every other openable Event is a `button`, and these three cannot be: what
/// they hold is rendered markdown, and a link inside a button is not something a
/// browser will have. So the affordance goes on the article instead — the press,
/// the keyboard and the role that says what it is — and it reads as the same
/// card either way.
///
/// `open` is nothing where the card is not openable, which is the Brief for as
/// long as it is a draft: a field is not a thing to press, and neither is the
/// setup standing under it.
function Openable(props: {
  kind: string;
  selected: boolean;
  open: (() => void) | null;
  children: JSX.Element;
}): JSX.Element {
  const press = () => props.open?.();

  return (
    <article
      class={props.kind}
      classList={{ openable: props.open !== null, selected: props.selected }}
      role={props.open === null ? undefined : "button"}
      tabindex={props.open === null ? undefined : 0}
      aria-pressed={props.open === null ? undefined : props.selected}
      onClick={press}
      onKeyDown={(ev) => {
        // What a button would do for nothing: Enter and Space press it.
        if (props.open !== null && (ev.key === "Enter" || ev.key === " ")) {
          ev.preventDefault();
          press();
        }
      }}
    >
      {props.children}
    </article>
  );
}

export function Timeline(props: {
  conversation: ConversationView;
  back: () => void;
  details: () => void;

  /// Which Event the details pane is showing, and how to change it.
  selected: number | null;
  select: (event: number) => void;
}): JSX.Element {
  return (
    <>
      {/* The header and the pinned block as one block, because that is how they
          stay: the stylesheet sticks this to the top edge of the pane and both
          of them travel with it, so there is no strip of scrolling record
          between the title and the pinned items and nothing to keep a pinned
          block's own offset in step with. */}
      <div class="pane-chrome">
        {/* The way back out of this level, which is the whole of what a narrow
            window offers instead of the pane beside it. Drawn always and hidden
            by the pane head where all three panes are on screen at once. */}
        <PaneHead
          back={{ to: "Conversations", go: props.back }}
          title={props.conversation.branch}
        >
          {/* What the work has stopped on, said where the conversation is named
              rather than only down in the list: a timeline is long by the time a
              run gets far enough to stop, and a badge the human had to go
              hunting behind would not be one. It points at the event that
              stopped it, which is what makes it worth pressing. */}
          <Show when={props.conversation.blocked_on}>
            {(event) => (
              <button
                type="button"
                class="blocked"
                onClick={() => {
                  props.select(event());

                  if (opensAPane(props.conversation, event())) {
                    props.details();
                  }
                }}
              >
                Blocked on you
              </button>
            )}
          </Show>
          <Actions conversation={props.conversation} />
          {/* And the way on to the next level, drawn only where there is a next
              level to reach: the details pane holds the selected Event and
              nothing else, so with nothing selected it is bare paper and a
              control that paged into it would page into nothing. Hidden by the
              stylesheet anyway where all three panes are on screen at once. */}
          <Show when={props.selected !== null}>
            <button type="button" class="pane-forward" onClick={props.details}>
              Details →
            </button>
          </Show>
        </PaneHead>

        <Pinned
          conversation={props.conversation}
          selected={props.selected}
          select={props.select}
          details={props.details}
        />
      </div>

      <ol class="timeline">
        <For each={props.conversation.timeline}>
          {(event, index) => (
            <li class="timeline-event">
              <Switch>
                <Match when={"Brief" in event && event.Brief}>
                  {(brief) => (
                    <Brief
                      conversation={props.conversation}
                      brief={brief()}
                      selected={props.selected === brief().id}
                      open={() => {
                        props.select(brief().id);
                        props.details();
                      }}
                    />
                  )}
                </Match>
                <Match when={"Moved" in event && event.Moved}>
                  {(moved) => (
                    <Moved
                      from={movedFrom(props.conversation.timeline, index())}
                      moved={moved()}
                    />
                  )}
                </Match>
                <Match when={"Handoff" in event && event.Handoff}>
                  {(handoff) => (
                    <Handoff
                      handoff={handoff()}
                      selected={props.selected === handoff().id}
                      open={() => {
                        props.select(handoff().id);
                        props.details();
                      }}
                    />
                  )}
                </Match>
                <Match when={"Notice" in event && event.Notice}>
                  {(notice) => <Notice notice={notice()} />}
                </Match>
                <Match when={"ManualTask" in event && event.ManualTask}>
                  {(manual) => (
                    <ManualTask
                      manual={manual()}
                      selected={props.selected === manual().id}
                      open={() => {
                        props.select(manual().id);
                        props.details();
                      }}
                    />
                  )}
                </Match>
                <Match when={"AgentOutput" in event && event.AgentOutput}>
                  {(output) => (
                    <AgentOutput
                      output={output()}
                      selected={props.selected === output().id}
                      open={() => {
                        props.select(output().id);
                        props.details();
                      }}
                    />
                  )}
                </Match>
                <Match when={"QuestionSet" in event && event.QuestionSet}>
                  {(asked) => (
                    <QuestionSet
                      asked={asked()}
                      selected={props.selected === asked().id}
                      open={() => {
                        props.select(asked().id);
                        props.details();
                      }}
                    />
                  )}
                </Match>
                <Match when={"UnreadableSet" in event && event.UnreadableSet}>
                  {(asked) => (
                    <UnreadableSet
                      asked={asked()}
                      selected={props.selected === asked().id}
                      open={() => {
                        props.select(asked().id);
                        props.details();
                      }}
                    />
                  )}
                </Match>
                {/* Drawn like a card with something to press inside it, and
                    with nothing behind a pane: what a pause has to say is a
                    profile, a time and the line the session printed, so there
                    is nothing to open. */}
                <Match when={"Pause" in event && event.Pause}>
                  {(waiting) => (
                    <Pause
                      conversation={props.conversation}
                      waiting={waiting()}
                      selected={props.selected === waiting().id}
                    />
                  )}
                </Match>
                <Match when={"Commit" in event && event.Commit}>
                  {(commit) => (
                    <Commit
                      commit={commit()}
                      selected={props.selected === commit().id}
                      open={() => {
                        props.select(commit().id);
                        props.details();
                      }}
                    />
                  )}
                </Match>
              </Switch>
            </li>
          )}
        </For>
      </ol>

      {/* After everything that has happened, because it is what happens next.
          Drawn outside the list: none of them is an event, and any of them would
          be an event that moved every time one landed. Only one is ever drawn —
          each is for a different state — so they read as the one thing there is
          to do from here. Adopting a stage is one state's, and the other two are
          the two ends of the ladder: starting a draft grilling, and opening a
          second round on a conversation that is finished. */}
      <Show
        when={props.conversation.adopting}
        fallback={
          <>
            <StartGrilling conversation={props.conversation} />
            <Reopen conversation={props.conversation} />
          </>
        }
      >
        {(adopting) => (
          <Adoption conversation={props.conversation} adopting={adopting()} />
        )}
      </Show>
      {/* And under that, the two ways to get a conversation moving again. Both
          are offered *whenever nothing is running*, which is a quiet moment
          between steps as much as it is a run that has stopped, so they sit
          below whichever of the two above is drawn rather than instead of it.

          Resume first, because it is the one that carries on what Verkstead was
          already doing: the other is for the thing it was never going to do. */}
      <Resume conversation={props.conversation} />
      <ManualTaskComposer conversation={props.conversation} />
    </>
  );
}

/// The one standing way to get Verkstead driving again: a button, and what it
/// refuses with when there is nothing to drive.
///
/// Drawn exactly where the server says it is worth drawing — see
/// `ready_to_resume`, which is the state being one something ought to be driving
/// and nothing driving it. The page cannot work that out for itself: what drives
/// a conversation is a register of running tasks, and a register lives in the
/// server.
///
/// It carries nothing. What to start is recomputed from the conversation's state
/// and its branch at the moment of the press, which is the whole point of one
/// button rather than one per way of stopping — steering the work is what the
/// manual task below is for.
function Resume(props: { conversation: ConversationView }): JSX.Element {
  const queries = useQueryClient();

  const [refused, setRefused] = createSignal<Resumed | null>(null);

  const press = useMutation(() => ({
    mutationFn: () => resume(props.conversation.id),
    onSuccess: (outcome: Resumed) => {
      setRefused(outcome === "Resumed" ? null : outcome);

      // Either way the page it was pressed on is out of date: driving has
      // started, or the world had moved under the button. Reading it again is
      // both the correction and the explanation.
      void queries.invalidateQueries({ queryKey: ["conversation"] });
      void queries.invalidateQueries({ queryKey: ["conversations"] });
    },
  }));

  return (
    <Show when={props.conversation.ready_to_resume}>
      <div class="resume">
        <h2>Nothing is driving this</h2>

        <button
          type="button"
          class="resume-conversation"
          disabled={press.isPending}
          onClick={() => press.mutate()}
        >
          {press.isPending ? "Resuming…" : "Resume"}
        </button>

        <Note>
          Verkstead works out what should be running from where the work now
          stands, and starts it.
        </Note>

        <Show when={refused()}>
          {(outcome) => (
            <ErrorLine class="failure">{RESUME_REFUSAL[outcome()]}</ErrorLine>
          )}
        </Show>
        <Show when={press.isError}>
          <ErrorLine class="failure">
            The conversation could not be resumed: {press.error?.message}
          </ErrorLine>
        </Show>
      </div>
    </Show>
  );
}

/// The way to move a conversation by hand: an instruction, a pairing to run it
/// under, and a submit.
///
/// Drawn whenever there is a worktree to run in and no session is registered for
/// it. That is the literal rule and it is deliberate: the gaps between an
/// unattended run's steps, a wrapping lull, a grilling waiting on a pick, a
/// finished conversation, a reopened one being written a second brief, and a
/// conversation that has halted all show it, because the point of it is to get a
/// stuck conversation moving. After a server restart nothing is running anywhere,
/// so it shows everywhere, and that is wanted too.
///
/// A conversation that has never been grilled and one that was aborted have no
/// worktree, so neither is ever offered it — there is nowhere for a session to
/// run.
///
/// The pairing starts on the conversation's implementation one and picking
/// another is one-off: it is what this task runs under, and it never becomes the
/// conversation's own. Nothing here writes it back.
function ManualTaskComposer(props: {
  conversation: ConversationView;
}): JSX.Element {
  const queries = useQueryClient();

  const [instruction, setInstruction] = createSignal("");
  const [picked, setPicked] = createSignal<string | null>(null);
  const [refused, setRefused] = createSignal<ManualTaskStarted | null>(null);

  /// The profile list, read here rather than passed down, the way the setup's
  /// pickers read it: the control is whole wherever it is drawn.
  const profiles = useReading(() => ({
    queryKey: ["profiles"],
    queryFn: listProfiles,

    // And the same merge, for the same picker — see the setup on the brief
    // card. This one sits under a half-typed instruction while a session is
    // talking above it, which is the loudest a Nudge ever gets.
    freshness: { reconcile: "id" },
  }));

  /// Which pairing is selected: whatever the human picked, and the
  /// conversation's implementation one until they pick anything.
  ///
  /// The empty string is nothing selected, which is where the composer opens on
  /// a conversation whose implementation profile was chosen before models were
  /// paired with them: there is no default model anywhere, so the pick is the
  /// human's to make.
  const running = () =>
    picked() ?? pairing.chosen(props.conversation.implementation_pairing);

  /// Whether the composer belongs on this conversation at all.
  ///
  /// A worktree to run in and nothing running is the whole of it: a conversation
  /// that has never been grilled and one that was aborted have no worktree, so
  /// neither is ever offered it, and a reopened one being written a second brief
  /// has one and is exactly where the escape hatch belongs.
  const offered = () =>
    props.conversation.worktree !== null && !props.conversation.working;

  const submit = useMutation(() => ({
    mutationFn: (chosen: string) =>
      startManualTask(
        props.conversation.id,
        instruction(),
        pairing.choice(chosen),
      ),
    onSuccess: (outcome: ManualTaskStarted) => {
      if (outcome !== "Started") {
        setRefused(outcome);
        // Refused against a picture of the world this page read a moment ago:
        // reading it again is both the correction and the explanation.
        void queries.invalidateQueries({ queryKey: ["conversation"] });
        void queries.invalidateQueries({ queryKey: ["profiles"] });
        return;
      }

      // It is on the timeline now, which is where it is read back from: the box
      // is emptied so what is in it is always something not yet asked for.
      setRefused(null);
      setInstruction("");
      void queries.invalidateQueries({ queryKey: ["conversation"] });
      void queries.invalidateQueries({ queryKey: ["conversations"] });
    },
  }));

  return (
    <Show when={offered()}>
      <div class="manual-task-composer">
        <h2>Do something by hand</h2>

        <label for="manual-task">What should the agent do?</label>
        {/* A copy of what has been typed gives the field its height — see
            `.grow`, which the brief's field uses for the same reason. */}
        <div class="grow" data-value={instruction()}>
          <textarea
            id="manual-task"
            rows="1"
            placeholder="Rebase this onto main and force-push"
            value={instruction()}
            onInput={(ev) => {
              setInstruction(ev.currentTarget.value);
              setRefused(null);
            }}
          />
        </div>

        {/* Drawn only once the list is here, the way the setup's pickers are:
            a select whose value is set before its options exist is a select
            showing nothing. */}
        <div class="manual-task-profile">
          <label for="manual-task-pairing">Run it as</label>
          <Show
            when={profiles.data}
            fallback={
              <Note>
                {profiles.isError
                  ? `Could not read the agent profiles: ${profiles.error?.message}`
                  : "Reading the agent profiles…"}
              </Note>
            }
          >
            {(saved) => (
              /* A [`Picker`] rather than a `<select>`, the way the setup's
                 pickers are: what this shows and what the press below runs the
                 task as are the same pairing, list or no list — see
                 `src/picking.tsx`. */
              <Picker
                id="manual-task-pairing"
                options={pairing.pairings(saved())}
                value={pairing.value}
                label={pairing.label}
                chosen={running()}
                pick={setPicked}
                // The one-off pick is gone from the list: it is dropped, and
                // `running` falls back to the conversation's own implementation
                // pairing — which is where the composer opened.
                gone={() => setPicked(null)}
                disabled={submit.isPending}
              />
            )}
          </Show>
        </div>

        <button
          type="button"
          class="start-manual-task"
          disabled={
            submit.isPending || instruction().trim() === "" || running() === ""
          }
          onClick={() => {
            const chosen = running();
            if (chosen !== "") {
              submit.mutate(chosen);
            }
          }}
        >
          {submit.isPending ? "Starting…" : "Set it going"}
        </button>

        <Note>
          One session, outside the grilling and the implementation. Nothing about
          the conversation moves — what it leaves behind is what it commits.
        </Note>

        <Show when={refused()}>
          {(outcome) => (
            <ErrorLine class="failure">
              {MANUAL_TASK_REFUSAL[outcome()]}
            </ErrorLine>
          )}
        </Show>
        <Show when={submit.isError}>
          <ErrorLine class="failure">
            The manual task could not be started: {submit.error?.message}
          </ErrorLine>
        </Show>
      </div>
    </Show>
  );
}

/// The pinned events: what stays in view rather than scrolling past with the
/// record.
///
/// Above the list rather than in it, and held at the top of the pane by the
/// stylesheet. The rest of the timeline is a record of moments and reads in
/// order; each of these is the current state of something the work is against,
/// and is worth having on screen whichever part of the record is being read.
///
/// Pinning is the fixed set — a task list, a stage list and the pull request —
/// so there is nothing to pin, nothing to unpin, and no control for either.
///
/// One card at a time once there is more than one of them: they are held above
/// the record, so a stack of them is a stack the record is pushed down by, and
/// what is pinned is worth having in view rather than worth having all of at
/// once. One of them alone is drawn exactly as it always was — a carousel of one
/// is furniture around a card nothing can be turned to.
function Pinned(props: {
  conversation: ConversationView;
  selected: number | null;
  select: (event: number) => void;
  details: () => void;
}): JSX.Element {
  return (
    <Show when={props.conversation.pinned.length > 0}>
      <div class="pinned">
        <Show
          when={props.conversation.pinned.length > 1}
          fallback={
            <Card
              event={props.conversation.pinned[0]!}
              selected={props.selected}
              select={props.select}
              details={props.details}
            />
          }
        >
          <Carousel
            conversation={props.conversation}
            selected={props.selected}
            select={props.select}
            details={props.details}
          />
        </Show>
      </div>
    </Show>
  );
}

/// How far a finger has to travel across a card before it has swiped it, in the
/// pixels a touch reports.
///
/// Far enough that a press which slid a little is still a press — a dot and a
/// pull request's title are both pressed through this — and short enough that a
/// flick across a card in a phone-width pane counts.
export const SWIPE = 40;

/// The carousel: one pinned card showing, and the ways to the others.
///
/// Dots beneath saying how many there are and which is showing, arrows over the
/// card's edges where there is a pointer to reach them with, and a swipe across
/// the card where there is not. All three are the same move, which is why they
/// are one function between them.
///
/// It wraps: with two or three cards, an arrow that stopped at the end would be
/// a dead control most of the time.
///
/// Which card fronts is [`fronting`]'s to say, and it says it once — when the
/// conversation is opened and this is built. Nothing is remembered between
/// visits, and nothing moves the card under a reader afterwards: a re-read that
/// jumped the carousel back to where it started would be the page arguing with
/// whoever is holding it.
function Carousel(props: {
  conversation: ConversationView;
  selected: number | null;
  select: (event: number) => void;
  details: () => void;
}): JSX.Element {
  const cards = () => props.conversation.pinned;

  const [at, setAt] = createSignal(fronting(props.conversation));

  /// Never off the end of a list that shrank underneath it — a pull request is
  /// pinned as the run finishes, and a backlog stops being pinned as its last
  /// task file goes.
  const showing = () => Math.min(at(), cards().length - 1);

  /// Turn to a card, counting round both ends.
  const turn = (to: number) => {
    const many = cards().length;
    setAt(((to % many) + many) % many);
  };

  /// Where the finger went down, in the coordinates it will come back up in.
  let from: number | null = null;

  return (
    <div class="carousel">
      <div
        class="showing"
        onTouchStart={(event) => {
          from = event.changedTouches[0]?.clientX ?? null;
        }}
        onTouchEnd={(event) => {
          const to = event.changedTouches[0]?.clientX;
          const went = from;
          from = null;
          if (went === null || to === undefined) {
            return;
          }
          // Leftwards is onwards, the way a page turns.
          if (Math.abs(to - went) >= SWIPE) {
            turn(showing() + (to < went ? 1 : -1));
          }
        }}
      >
        <Card
          event={cards()[showing()]!}
          selected={props.selected}
          select={props.select}
          details={props.details}
        />
      </div>

      {/* The arrows, which the stylesheet draws only where there is a pointer:
          on a touch device the swipe is what these are, and two buttons lying
          over the card would be two buttons in the way of it. */}
      <button
        type="button"
        class="step back"
        aria-label="Previous pinned card"
        onClick={() => turn(showing() - 1)}
      >
        ‹
      </button>
      <button
        type="button"
        class="step on"
        aria-label="Next pinned card"
        onClick={() => turn(showing() + 1)}
      >
        ›
      </button>

      {/* And the dots: how many cards there are, which one is showing, and a way
          straight to any of them. Each is named for the card it turns to rather
          than numbered, because that is what a reader who cannot see the dots
          needs to know about it. */}
      <ol class="dots">
        <For each={cards()}>
          {(card, index) => (
            <li>
              <button
                type="button"
                aria-label={named(card)}
                aria-current={showing() === index() ? "true" : undefined}
                onClick={() => turn(index())}
              />
            </li>
          )}
        </For>
      </ol>
    </div>
  );
}

/// Which card is showing when a conversation is opened: the one needing
/// attention, and otherwise the first.
///
/// The first is the fixed order — task list, then roadmap, then pull request —
/// because that is the order the server hands them over in, which is the order
/// the work goes through them in.
///
/// Needing attention is the conversation being blocked on the card, which only a
/// pull request can be: it is the one pinned event that is also on the record,
/// and the two lists are read off the worktree rather than being moments
/// anything could have stopped at. So a pull request with feedback waiting on it
/// fronts over the backlog beside it, which is what a reader opening the
/// conversation is being stopped for.
function fronting(conversation: ConversationView): number {
  const at = conversation.pinned.findIndex(
    (event) =>
      "PullRequest" in event && event.PullRequest.id === conversation.blocked_on,
  );

  return at === -1 ? 0 : at;
}

/// What a pinned card is called, in the words its own heading uses.
function named(event: PinnedEvent): string {
  if ("TaskList" in event) {
    return "Task list";
  }
  if ("StageList" in event) {
    return "Roadmap";
  }
  return "Pull request";
}

/// One pinned card, whichever of the three kinds it is.
///
/// One of them opens: a pull request has a full self, which is what is on it
/// right now. Neither list does — what a details pane would show of one is what
/// is already drawn here.
function Card(props: {
  event: PinnedEvent;
  selected: number | null;
  select: (event: number) => void;
  details: () => void;
}): JSX.Element {
  return (
    <Switch>
      <Match when={"TaskList" in props.event && props.event.TaskList}>
        {(tasks) => <TaskList tasks={tasks()} />}
      </Match>
      <Match when={"StageList" in props.event && props.event.StageList}>
        {(stages) => <StageList stages={stages()} />}
      </Match>
      <Match when={"PullRequest" in props.event && props.event.PullRequest}>
        {(opened) => (
          <PullRequest
            opened={opened()}
            selected={props.selected === opened().id}
            open={() => {
              props.select(opened().id);
              props.details();
            }}
          />
        )}
      </Match>
    </Switch>
  );
}

/// The pull request the finish step opened: what it is called, and its number.
///
/// A button, because what is *on* it — the commits and the comments — is in the
/// details pane, fetched from GitHub when this is opened. The link out is a link
/// rather than part of the button: merging is the human's act and it happens
/// over there, so getting there must not depend on this page's own panes.
function PullRequest(props: {
  opened: PullRequestEvent;
  selected: boolean;
  open: () => void;
}): JSX.Element {
  return (
    <article class="pull-request" classList={{ selected: props.selected }}>
      <div class="event-head">
        <h2>Pull request</h2>
        <span class="number">#{props.opened.number}</span>
        <a class="out" href={props.opened.url} target="_blank" rel="noreferrer">
          On GitHub
        </a>
      </div>

      <button type="button" class="open-pull-request" onClick={props.open}>
        {props.opened.title}
      </button>
    </article>
  );
}

/// Whether one entry of a list is finished, drawn the way the file it is read
/// out of writes it: an empty box, or a checked one.
///
/// The glyph and nothing else — it is the same fact the row's own `state` word
/// carries, so it is hidden from anything that reads rather than looks, and the
/// word is what those get.
function Box(props: { done: boolean }): JSX.Element {
  return (
    <span class="box" aria-hidden="true">
      {props.done ? "☑" : "☐"}
    </span>
  );
}

/// The backlog: every task of it, and how far through it the work has got.
///
/// The whole list rather than a summary, because the whole list is short and it
/// is the one thing a conversation being built from a backlog is *about*. There
/// is nothing to open: the design gives a task list no details pane, since what
/// a details pane would show is what is already drawn here.
///
/// Read out of `.tasks/` in the worktree every time the page reads the
/// conversation, so a task finishing moves this without anybody pressing
/// anything.
function TaskList(props: { tasks: TaskListEvent }): JSX.Element {
  const done = () => props.tasks.tasks.filter((task) => task.done).length;

  return (
    <article class="task-list">
      <div class="event-head">
        <h2>Task list</h2>
        <Show when={props.tasks.feature !== ""}>
          <span class="feature">{props.tasks.feature}</span>
        </Show>
        <span class="progress">
          {done()} of {props.tasks.tasks.length} done
        </span>
      </div>

      <ol class="tasks">
        <For each={props.tasks.tasks}>
          {(task) => (
            <li classList={{ done: task.done }}>
              <Box done={task.done} />
              <span class="what">{task.title}</span>
              {/* At the far end of the row, where it is out of the way of the
                  reading: what a backlog is scanned for is which titles are
                  left, and a number is what one is quoted by afterwards. */}
              <span class="n">{task.number}</span>
              {/* The word travels with the row rather than being drawn by the
                  stylesheet, so a list read aloud or copied out still says
                  which tasks are finished. */}
              <span class="state">{task.done ? "done" : "to do"}</span>
            </li>
          )}
        </For>
      </ol>
    </article>
  );
}

/// The roadmap: every stage of it, and how far through it the effort has got.
///
/// Beside the task list and drawn the same way, because it is the same kind of
/// thing one level up — and it is read out of `docs/roadmaps/` in the worktree
/// every time the page reads the conversation, so a stage finishing moves this
/// without anybody pressing anything. There is nothing to open here either.
///
/// Which roadmap this is, is the one this branch has written to: a repository
/// keeps its finished roadmaps, and a conversation is about the one it touched.
function StageList(props: { stages: StageListEvent }): JSX.Element {
  const done = () => props.stages.stages.filter((stage) => stage.done).length;

  return (
    <article class="stage-list">
      <div class="event-head">
        <h2>Roadmap</h2>
        <span class="feature">{props.stages.title || props.stages.name}</span>
        <span class="progress">
          {done()} of {props.stages.stages.length} done
        </span>
      </div>

      <ol class="stages">
        <For each={props.stages.stages}>
          {(stage) => (
            <li classList={{ done: stage.done }}>
              <Box done={stage.done} />
              <span class="what">{stage.title}</span>
              {/* At the far end of the row, as a task's is, and for the reason
                  a task's is. */}
              <span class="n">{stage.number}</span>
              {/* The word travels with the row rather than being drawn by the
                  stylesheet, for the reason a task's does: a list read aloud
                  or copied out still says which stages are finished. */}
              <span class="state">{stage.done ? "done" : "to do"}</span>
            </li>
          )}
        </For>
      </ol>
    </article>
  );
}

/// The handoff the grilling wrote on its way out.
///
/// A card and not a line, because it is a document: what the grilling settled,
/// written down for the session that builds it — and the human reads the same
/// copy the implementation is primed with, which is the whole point of it
/// landing here rather than being passed along out of sight.
///
/// Read-only, unlike the Brief beside it. It is the agent's account of a
/// conversation that is over, and a document the human could edit afterwards
/// would be a record of something that never happened.
///
/// Clamped, and opened by pressing it: a settled handoff runs to a page or two,
/// and a timeline that had to be scrolled past one to reach what happened next
/// is a record nobody reads.
function Handoff(props: {
  handoff: HandoffEvent;
  selected: boolean;
  open: () => void;
}): JSX.Element {
  return (
    <Openable kind="handoff" selected={props.selected} open={props.open}>
      <div class="event-head">
        <h2>Handoff</h2>
      </div>
      <Clamped class="handoff-body" html={props.handoff.html} />
    </Openable>
  );
}

/// Something Verkstead did on its own account: the stage it started and where
/// the branch went, or a roadmap with nothing left to run.
///
/// A line and not a card, unlike the handoff above it: it is a sentence rather
/// than a document, and there is nothing to open and nothing to answer. It is
/// rendered markdown all the same, because what it names — a branch, a stage, a
/// file the repository records its process in — reads better set apart from the
/// prose around it.
function Notice(props: { notice: NoticeEvent }): JSX.Element {
  return <div class="notice markdown" innerHTML={props.notice.html} />;
}

/// What the human asked for by hand: the instruction a Manual Task was set
/// going with.
///
/// A card and not a line, unlike the notice above it: it is what somebody asked
/// for in their own words, and the words are the whole of it. Read-only, like
/// the handoff — it is a moment on the record rather than a document to go back
/// to, and what a second thought produces is a second Manual Task.
///
/// What the session it started went on to do is not drawn here. That arrives as
/// the events any work arrives as — what it printed, what it asked, what it
/// committed — under this one and in the order it happened.
///
/// Clamped and openable, as the handoff is: an instruction is as long as whoever
/// typed it made it, and the events it set going belong directly under it.
function ManualTask(props: {
  manual: ManualTaskEvent;
  selected: boolean;
  open: () => void;
}): JSX.Element {
  return (
    <Openable kind="manual-task" selected={props.selected} open={props.open}>
      <div class="event-head">
        <h2>Manual task</h2>
      </div>
      <Clamped class="manual-task-body" html={props.manual.html} />
    </Openable>
  );
}

/// A move: the Conversation changing hands, said as the transition itself.
///
/// A line and not a card, because there is nothing to it but the fact and the
/// time — everything a move has to say is already in the two. Centred, so the
/// run of cards is what the eye follows and the moves read as the joins between
/// them.
///
/// Both states, `Grilling → Implementing`, rather than a verb phrase for the one
/// that was moved to: where the work has got to is a step from somewhere, and a
/// line saying only where it arrived leaves the reader to remember where it
/// was. The state it came from is [`movedFrom`]'s to say.
function Moved(props: { from: Lifecycle; moved: MovedEvent }): JSX.Element {
  return (
    <p class="moved" classList={{ [props.moved.state.toLowerCase()]: true }}>
      {props.from} → {props.moved.state}
    </p>
  );
}

/// What a session has printed: how much of it there is, and the last thing it
/// said.
///
/// A button, because the whole of it is in the details pane and this is how it
/// is opened — the summary is a line, and a grilling session's Capture is an
/// hour of terminal output nobody wants in the middle pane.
///
/// It moves while the session runs, which is the point: the page hears the world
/// moved and reads this back, so a session that has just asked something says so
/// here rather than at the end of an hour.
function AgentOutput(props: {
  output: AgentOutputEvent;
  selected: boolean;
  open: () => void;
}): JSX.Element {
  return (
    <button
      type="button"
      class="agent-output"
      classList={{
        selected: props.selected,
        running: props.output.running,
      }}
      aria-pressed={props.selected}
      onClick={props.open}
    >
      <span class="event-head">
        <span class="what">Agent output</span>
        {/* How far the conversation has got. A session with no Transcript to
            count has no metric at all rather than a zero: there is nothing here
            that took turns, and a `0 turns` would be a claim about it. */}
        <Show when={props.output.turns !== null}>
          <span class="turns">
            {props.output.turns} {props.output.turns === 1 ? "turn" : "turns"}
          </span>
        </Show>
        {/* And whether anything is still writing it, at the right edge — the
            same mark a sidebar card says the same thing with. */}
        <Mark running={props.output.running} idle={props.output.idle} />
      </span>
      <span class="latest">
        <Show
          when={props.output.latest !== ""}
          fallback={<Empty inline>Nothing printed yet.</Empty>}
        >
          {props.output.latest}
        </Show>
      </span>
    </button>
  );
}

/// A Question Set the session put to the human, read as the interview it was:
/// a question line and the answer line under it, pair after pair.
///
/// Not a table any more. Three columns never fitted the middle pane, and what
/// they were holding apart is two lines of one exchange: the label leads the
/// question the way the detail page has it lead one, and the answer is set in
/// far enough to clear the label, so the two texts share a left edge and the
/// card reads down rather than across.
///
/// Every pair is drawn — a long Set earns a long card. No document card's clamp
/// here, which would keep four of the questions and hide the rest: what is cut
/// is each line rather than the list, so every question is on the card and the
/// long ones end in an ellipsis. The whole of any of them is a press away.
///
/// A button, as a session's output is, and for the same reason: the whole
/// document is in the details pane, and this is how it is opened.
///
/// A Set still waiting says so instead of drawing a column of blanks: nothing
/// has been decided yet, and an empty answer would read as a Set that was
/// answered with nothing.
function QuestionSet(props: {
  asked: QuestionSetEvent;
  selected: boolean;
  open: () => void;
}): JSX.Element {
  const standing = () => props.asked.standing;
  const waiting = () => "Waiting" in standing();
  const archived = () => "ArchivedUnanswered" in standing();

  /// Whether the Set nobody has answered yet was a Deferred Ask. Read off the
  /// standing, which is where the fact lives while it matters: an answered Set
  /// held the work up or did not, and that is over.
  const deferred = () => {
    const how = standing();
    return "Waiting" in how && how.Waiting === "deferred";
  };

  return (
    <button
      type="button"
      class="question-set"
      classList={{ selected: props.selected, waiting: waiting() }}
      aria-pressed={props.selected}
      onClick={props.open}
    >
      <span class="event-head">
        <span class="what">Question set</span>
        <span class="set-title">{props.asked.title}</span>
        <Show when={waiting()}>
          <span class="live">waiting on you</span>
        </Show>
        <Show when={deferred()}>
          <span class="deferred">deferred</span>
        </Show>
        <Show when={archived()}>
          <span class="closed">closed unanswered</span>
        </Show>
      </span>

      {/* Spans rather than the blocks this reads as, laid out as blocks by the
          stylesheet: everything here is inside a button, and a button holds
          phrasing. */}
      <span class="asked">
        <For each={props.asked.rows}>
          {(row) => (
            <span class="ask" classList={{ nested: row.nested }}>
              <span class="n">{row.name}</span>
              <span class="question">{row.question}</span>
              <span class="answer">
                <Show
                  when={row.answer !== ""}
                  fallback={
                    <span class="open">{waiting() ? "—" : "unanswered"}</span>
                  }
                >
                  {row.answer}
                </Show>
              </span>
            </span>
          )}
        </For>
      </span>
    </button>
  );
}

/// A Question Set whose stored body this build cannot read: a row saying so,
/// and the reason.
///
/// A row rather than a gap. The ask happened and it is on the record; a Timeline
/// that quietly left it out would be this build deciding a decision never
/// occurred. There is no interview because there is nothing to draw one from,
/// and no standing because nobody is going to answer a Set nobody here can read.
///
/// A button all the same, as every Event with a full self is: what it opens is
/// the stored body, which is what there is of the Set.
function UnreadableSet(props: {
  asked: UnreadableSetEvent;
  selected: boolean;
  open: () => void;
}): JSX.Element {
  return (
    <button
      type="button"
      class="question-set unreadable"
      classList={{ selected: props.selected }}
      aria-pressed={props.selected}
      onClick={props.open}
    >
      <span class="event-head">
        <span class="what">Question set</span>
        <span class="unreadable-badge">cannot be read</span>
      </span>

      <span class="unreadable-why">{props.asked.why}</span>
    </button>
  );
}

/// A commit a session landed on the branch: what it was called, how much of the
/// repository it moved, and the opening of what it said about itself.
///
/// A button, as a session's output and a question set are, and for the same
/// reason: the whole of it — the summary drawn out and the diff — is in the
/// details pane, and this is how it is opened.
///
/// Which is also why the snippet is text rather than a rendering, where the
/// three document cards are rendered markdown: markdown cannot live inside a
/// button, and this card is a button first. The server sends the prose with the
/// Diagram already taken out — see `to_prose` — so what is clamped here is what
/// the summary says rather than the fence it opens with. A commit that said
/// nothing draws the card that has always been drawn, with nothing marking the
/// absence.
///
/// Nothing here asks the human for anything. Commits are viewable and have no
/// state of their own: the design gives them no per-commit review, because
/// feedback about the work consolidates in the wrap-up phase.
function Commit(props: {
  commit: CommitEvent;
  selected: boolean;
  open: () => void;
}): JSX.Element {
  const files = () =>
    `${props.commit.files} ${props.commit.files === 1 ? "file" : "files"}`;

  return (
    <button
      type="button"
      class="commit"
      classList={{ selected: props.selected }}
      aria-pressed={props.selected}
      onClick={props.open}
    >
      <span class="event-head">
        <span class="what">Commit</span>
        <span class="sha">{props.commit.sha.slice(0, ABBREVIATED)}</span>
      </span>

      <span class="subject">{props.commit.subject}</span>

      <span class="changed">
        <span class="files">{files()}</span>
        {/* The signs travel with the numbers rather than being drawn by the
            stylesheet, so a row read aloud or copied out still says which way
            each of them went. */}
        <span class="added">+{props.commit.insertions}</span>
        <span class="removed">−{props.commit.deletions}</span>
      </span>

      {/* Under the counts, because the counts are how much moved and this is
          what the moving was for: the eye reads the line, then the size of it,
          then the account. Clamped to CLAMPED_LINES, as a document's card is,
          and by the stylesheet alone — plain prose is lines of one height, so
          there is nothing here for an observer to measure. */}
      <Show when={props.commit.snippet}>
        {(snippet) => <span class="snippet">{snippet()}</span>}
      </Show>
    </button>
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
    <Show when={props.conversation.state === "Draft"}>
      <div class="start-grilling">
        <button
          type="button"
          class="start"
          classList={{ inert: !ready() }}
          // Only ever `disabled` for a press already in flight. Not being ready
          // is the other thing entirely: that press has an answer to give.
          disabled={start.isPending}
          aria-disabled={!ready()}
          onClick={() => (ready() ? start.mutate() : setAsked(true))}
          onMouseEnter={() => setHovered(true)}
          onMouseLeave={() => setHovered(false)}
        >
          {start.isPending ? "Starting…" : "Start grilling"}
        </button>
        <Show
          when={ready()}
          fallback={
            <Show when={missing()}>
              <Note class="wanting">
                This needs a brief, and both pairings chosen and working.
              </Note>
            </Show>
          }
        >
          <Note>
            This creates the branch and its worktree, and freezes the brief.
          </Note>
        </Show>

        <Show when={refused()}>
          {(outcome) => (
            <ErrorLine class="failure">{GRILL_REFUSAL[outcome()]}</ErrorLine>
          )}
        </Show>
        <Show when={start.isError}>
          <ErrorLine class="failure">
            The grilling could not be started: {start.error?.message}
          </ErrorLine>
        </Show>
      </div>
    </Show>
  );
}

/// The button that opens a second round on a conversation Verkstead has finished
/// with.
///
/// Drawn on `Done` and nowhere else. Aborted is off the ladder and stays there,
/// and every other state is somewhere the work has got to — there is nothing to
/// reopen about work that is still going on.
///
/// Where `Start grilling` sits, and for the same reason: it is the next thing to
/// do about this conversation, and the end of everything that has happened is
/// where the next thing belongs. What it leaves is a brief with nothing in it and
/// a conversation drafting again, so the button pressed after this one is that
/// one.
function Reopen(props: { conversation: ConversationView }): JSX.Element {
  const queries = useQueryClient();

  const [refused, setRefused] = createSignal<ConversationReopened | null>(null);

  const reopen = useMutation(() => ({
    mutationFn: () => reopenConversation(props.conversation.id),
    onSuccess: (outcome: ConversationReopened) => {
      setRefused(outcome === "Reopened" ? null : outcome);

      // Either way: reopened is a timeline that has moved, and refused is a
      // picture of the world this page read a moment ago — reading it again is
      // both the correction and the explanation.
      void queries.invalidateQueries({ queryKey: ["conversation"] });
      void queries.invalidateQueries({ queryKey: ["conversations"] });
    },
  }));

  return (
    <Show when={props.conversation.state === "Done"}>
      <div class="reopen">
        <button
          type="button"
          class="reopen-conversation"
          disabled={reopen.isPending}
          onClick={() => reopen.mutate()}
        >
          {reopen.isPending ? "Reopening…" : "Reopen with a new brief"}
        </button>
        <Note>
          A second round on the same branch. The brief above stays where it is —
          this adds one to write, and the worktree comes back if it has gone.
        </Note>

        <Show when={refused()}>
          {(outcome) => (
            <ErrorLine class="failure">{REOPEN_REFUSAL[outcome()]}</ErrorLine>
          )}
        </Show>
        <Show when={reopen.isError}>
          <ErrorLine class="failure">
            The conversation could not be reopened: {reopen.error?.message}
          </ErrorLine>
        </Show>
      </div>
    </Show>
  );
}

/// What can be done to the conversation as a whole, rather than to any one
/// event: a menu on the header, holding the three ways of ending what it is
/// doing.
///
/// A menu rather than three buttons, because the last of them throws a worktree
/// away and the header is somewhere the human's cursor passes on the way to
/// everything else. The [`Menu`](../Menu.tsx) every dropdown here is, so it
/// opens, closes and reaches the keyboard without any of that being this
/// component's to get right.
///
/// In order of what each costs: stop, which waits for the task the run is on;
/// force stop, which does not; and abort, which is not a stop at all but the end
/// of the conversation. Each says what it does under it, because *stop* and
/// *force stop* are two words apart and hours of work apart.
///
/// Each is drawn only where it applies. The two stops need something to stop —
/// see `ready_to_stop`, which is the server's rule and not this page's — and
/// force stop needs a session to end, which is what `working` says.
function Actions(props: { conversation: ConversationView }): JSX.Element {
  const queries = useQueryClient();

  const [refused, setRefused] = createSignal<ConversationAborted | null>(null);
  const [halting, setHalting] = createSignal<ConversationStopped | null>(null);

  // The menu's own way to shut, held here because what closes this one is the
  // press coming back rather than the press going out.
  let shut = (): void => {};

  /// What every press here leaves behind: a page drawn against a conversation
  /// that has moved. Reading it again is both the correction and, where the
  /// press was refused, the explanation.
  const reread = () => {
    void queries.invalidateQueries({ queryKey: ["conversation"] });
    void queries.invalidateQueries({ queryKey: ["conversations"] });
  };

  /// Both stops answer the same way, so both are pressed the same way: the
  /// outcome is kept whatever it is — it is either what happened or why nothing
  /// did — and the menu closes only on a conversation that has actually
  /// stopped. A stop that is still waiting for a step to finish has something
  /// left to say, and says it where it was pressed.
  const pressing = (stopping: () => Promise<ConversationStopped>) => ({
    mutationFn: stopping,
    onSuccess: (outcome: ConversationStopped) => {
      setHalting(outcome);

      if (outcome === "Stopped") {
        shut();
      }

      reread();
    },
  });

  const stop = useMutation(() =>
    pressing(() => stopConversation(props.conversation.id)),
  );

  const force = useMutation(() =>
    pressing(() => forceStopConversation(props.conversation.id)),
  );

  const abort = useMutation(() => ({
    mutationFn: () => abortConversation(props.conversation.id),
    onSuccess: (outcome: ConversationAborted) => {
      if (outcome === "NoSuchConversation" || outcome === "WorktreeStuck") {
        setRefused(outcome);
        return;
      }

      // Aborted or already aborted: what was asked for holds either way.
      setRefused(null);
      shut();
      reread();
    },
  }));

  return (
    <Menu
      class="conversation-actions"
      label="Conversation actions"
      name="Conversation actions"
      closer={(close) => (shut = close)}
      trigger="⋯"
    >
      {() => (
        <>
          <Show when={props.conversation.ready_to_stop}>
            <div class="action">
              <button
                type="button"
                role="menuitem"
                class="stop"
                disabled={stop.isPending}
                onClick={() => stop.mutate()}
              >
                {stop.isPending ? "Stopping…" : "Stop"}
              </button>
              <Note>Pause after the current task until you resume.</Note>
              <Show when={halting() === "Stopping"}>
                <Note class="waiting">
                  The session running now finishes its task first. Nothing will
                  be started after it.
                </Note>
              </Show>
            </div>

            <Show when={props.conversation.working}>
              <div class="action">
                <button
                  type="button"
                  role="menuitem"
                  class="force-stop"
                  disabled={force.isPending}
                  onClick={() => force.mutate()}
                >
                  {force.isPending ? "Stopping…" : "Force stop"}
                </button>
                <Note>Halt any running tasks and stop immediately.</Note>
              </div>
            </Show>
          </Show>

          <Show
            when={props.conversation.state !== "Aborted"}
            fallback={<Note>This conversation has been aborted.</Note>}
          >
            <div class="action">
              <button
                type="button"
                role="menuitem"
                class="abort"
                disabled={abort.isPending}
                onClick={() => abort.mutate()}
              >
                {abort.isPending ? "Aborting…" : "Abort conversation"}
              </button>
              <Note>
                Permanently end the conversation and delete the worktree. The
                branch stays where it is.
              </Note>
            </div>
          </Show>

          <Show when={halting() && STOP_REFUSAL[halting()!]}>
            <ErrorLine class="failure">{STOP_REFUSAL[halting()!]}</ErrorLine>
          </Show>
          <Show when={stop.isError || force.isError}>
            <ErrorLine class="failure">
              The conversation could not be stopped:{" "}
              {stop.error?.message ?? force.error?.message}
            </ErrorLine>
          </Show>

          <Show when={refused()}>
            {(outcome) => (
              <ErrorLine class="failure">{ABORT_REFUSAL[outcome()]}</ErrorLine>
            )}
          </Show>
          <Show when={abort.isError}>
            <ErrorLine class="failure">
              The conversation could not be aborted: {abort.error?.message}
            </ErrorLine>
          </Show>
        </>
      )}
    </Menu>
  );
}

/// The Brief: the markdown a Conversation starts from, read inline and written
/// inline.
///
/// Inline in the Timeline rather than in the details pane, because there is
/// nothing of it the Timeline does not already show — it *is* its own summary.
///
/// While the Conversation is drafting the Brief *is* a field: raw markdown in a
/// textarea that is always there, growing with what is typed into it, saving
/// itself on a pause and on the way out of it. There is no Edit and no Save,
/// because a document that is only ever written in one state does not need a
/// mode to be written in — and a card that swapped a rendering for a field
/// would cost a tap before every correction.
///
/// Once grilling starts it freezes, and from then on it is read as the server
/// rendered it. The two are one field's worth of markdown either way, and the
/// Brief is the one document on this wire that travels both ways for exactly
/// that reason.
///
/// The setup rides under it while the Conversation is still drafting — the
/// branch, the base commit and the two pairings — because setting the work up
/// and kicking it off are one act, and this is where it is kicked off. Once
/// grilling starts the card is the Brief alone: everything under it froze at
/// that moment, so there is nothing there to draw.
///
/// Which is also when it becomes a card to press: frozen, it is a document like
/// the handoff, clamped to [`CLAMPED_LINES`] with the whole of it in the details
/// pane. While it is a draft it is not openable at all — the card is a field
/// with a setup under it, and every press on it belongs to one of those.
function Brief(props: {
  conversation: ConversationView;
  brief: BriefEvent;
  selected: boolean;
  open: () => void;
}): JSX.Element {
  const queries = useQueryClient();

  /// Whether the Brief has frozen, which is the one thing that decides what kind
  /// of card this is.
  ///
  /// Frozen, it is a document like the handoff: clamped, and pressed to read the
  /// whole of it. Still a draft, it is a card with things on it to press — the
  /// field, or the setup that stands under a Brief arriving with an adoption —
  /// so it neither opens nor clamps. The two go together deliberately: a card
  /// that cut a document off without offering the rest would be one that had
  /// hidden it.
  ///
  /// The Brief's own flag rather than the Conversation's state, because a
  /// reopened one has a frozen Brief and an open one on the same Timeline: what
  /// froze is the round, and the server is what says which round each Brief
  /// belongs to. An adopting Conversation's stage brief comes down frozen from
  /// the start — it is nobody here's to write.
  const frozen = () => props.brief.frozen;

  /// Whether the Brief is the human's to write here, which is the same question
  /// the other way round.
  const writing = () => !frozen();

  // What has been typed, or nothing if nothing has been. The field follows the
  // Event until the first keystroke and follows itself after it, so a read of
  // the Conversation landing mid-sentence cannot take the sentence with it.
  const [typed, setTyped] = createSignal<string | null>(null);
  const text = () => typed() ?? props.brief.markdown;

  // What the record has, as far as this card knows: what came down with the
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
  /// writing, and the answer would be the one already on the card.
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
      // The readiness verdict under this card is a fact about the Brief, so it
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
    <Openable
      kind="brief"
      selected={props.selected}
      open={frozen() ? props.open : null}
    >
      {/* The heading alone: a field that keeps itself needs no word beside it
          saying so, and a line that changed as fast as this one was read past
          on a card the eye is meant to be typing into. What a save cannot do
          is still said, under the field, in words. */}
      <div class="event-head">
        <h2>Brief</h2>
      </div>

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
            <Show
              when={frozen()}
              fallback={
                <div class="brief-body markdown" innerHTML={props.brief.html} />
              }
            >
              <Clamped class="brief-body" html={props.brief.html} />
            </Show>
          </Show>
        }
      >
        {/* A copy of what has been typed gives the field its height — see
            `.grow`. */}
        <div class="grow" data-value={text()}>
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
          the human was typing is still explained on the card it happened to
          once the card has gone back to being a rendering. */}
      <Show when={refused()}>
        {(outcome) => (
          <ErrorLine class="failure">{BRIEF_REFUSAL[outcome()]}</ErrorLine>
        )}
      </Show>
      <Show when={save.isError}>
        <ErrorLine class="failure">
          The brief could not be saved: {save.error?.message}
        </ErrorLine>
      </Show>

      {/* Under the brief, and only while the conversation is drafting: the
          branch, the base commit and the two pairings all freeze when grilling
          starts, so past that moment there is nothing here that could be
          changed.

          Under *one* brief, though. A reopened conversation is drafting with a
          frozen brief above its open one, and the setup belongs under the round
          being set up rather than under both — while an adopting one is drafting
          with a brief that is frozen from the start, because the stage brief is
          nobody here's to write, and its setup is still the human's. */}
      <Show
        when={
          props.conversation.state === "Draft" &&
          (writing() || props.conversation.adopting !== null)
        }
      >
        <Setup conversation={props.conversation} />
      </Show>
    </Openable>
  );
}
