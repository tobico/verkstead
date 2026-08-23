//! A Conversation's Timeline: everything that has happened to it, in order.
//!
//! The kinds of Event so far — the Brief, a move, what a session printed, a
//! Question Set, the handoff, and the commits a
//! session lands on the branch — drawn as a list of Events rather than as a
//! Brief with a list under it. The stages after this one put interruptions on
//! the same list.
//!
//! Above the list are the pinned Events, which are a fixed set — the backlog
//! now, the stage list and the PR as those stages arrive. They are not on the
//! record and do not scroll with it: each is the current state of something the
//! work is against rather than a moment in it.
//!
//! An Event that has a full self shows its summary here and is opened in the
//! details pane, which is why this takes a way of selecting one. The Brief has
//! no full self beyond what is already drawn, so it is the one Event nothing
//! opens.
//!
//! The Timeline is also where the work is moved on from, because that is where
//! the reason to move it is: a control sits at the end of everything that has
//! happened so far, which is exactly where the next thing to happen belongs.
//! One of them lives there — `Start grilling` under the Brief it will freeze.
//! Aborting is in neither place and not in the list: it is not a step
//! in the work but a way of ending it, so it hangs off the header behind a menu,
//! where a destructive action is not one stray click away.
//!
//! Nothing here ends the grilling, and nothing here chooses a direction. That is
//! the agent's own closing move — a Question Set carrying a proposal, with the
//! chooser drawn on the Set itself — so both happen on the page the Set is
//! answered on and land here as the answered Set.

import { useMutation, useQueryClient } from "@tanstack/solid-query";
import { For, Match, Show, Switch, createSignal, type JSX } from "solid-js";

import {
  abortConversation,
  listProfiles,
  saveBrief,
  startGrilling,
  startManualTask,
} from "../api/client";
import type {
  AgentOutputEvent,
  BriefEvent,
  BriefSaved,
  CommitEvent,
  ConversationAborted,
  ConversationView,
  GrillingStarted,
  HandoffEvent,
  Lifecycle,
  ManualTaskEvent,
  ManualTaskStarted,
  MovedEvent,
  NoticeEvent,
  PullRequestEvent,
  QuestionSetEvent,
  StageListEvent,
  TaskListEvent,
} from "../api/types";
import { useReading } from "../freshness";
import { Picker } from "../picking";
import { Adoption } from "./Adoption";
import { Interruption } from "./Interruption";
import { Mark } from "./Mark";

/// How much of a commit's hash the timeline shows.
///
/// Seven characters, which is what git prints and what everybody reads a commit
/// by. The whole hash travels on the wire — what it takes to be unambiguous
/// grows with a repository, and shortening for reading is a different thing
/// from recording one short.
export const ABBREVIATED = 7;

/// What each way of being refused a Brief says.
///
/// `Saved` is here for completeness of the mapping and never drawn: nothing is
/// said about an edit that worked, because the Brief reading back as what was
/// written is what says it.
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
  NoGrillingProfile: "Choose a grilling profile first, in the details pane.",
  NoImplementationProfile:
    "Choose an implementation profile first, in the details pane.",
  ProfileBroken:
    "A chosen profile's claude pair is not where it was left, so there is no account to run under.",
  EmptyBrief: "Write the brief first — it is what the grilling starts from.",
  NoBaseCommit: "The repo has nothing to branch from any more.",
  BranchExists: "That branch already exists, and Verkstead did not make it.",
  WorktreeRefused: "Git would not make the worktree. The server log says why.",
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
  NotStarted:
    "The instruction is on the timeline and no session could be started for it. The server log says why.",
};

/// What a move reads as. The state moved *to*, said as something that happened.
const MOVED: Record<Lifecycle, string> = {
  Draft: "Went back to drafting",
  Grilling: "Started grilling",
  Implementing: "Started implementing",
  Wrapping: "Moved to wrapping up",
  Done: "Finished",
  Aborted: "Aborted",
};

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
        <div class="pane-head">
          {/* The way back out of this level, which is the whole of what a narrow
              window offers instead of the pane beside it. Drawn always and
              hidden by the stylesheet where all three panes are on screen at
              once. */}
          <button type="button" class="pane-back" onClick={props.back}>
            ← Conversations
          </button>
          <h1>{props.conversation.branch}</h1>
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
                  props.details();
                }}
              >
                Blocked on you
              </button>
            )}
          </Show>
          <Actions conversation={props.conversation} />
          <button type="button" class="pane-forward" onClick={props.details}>
            Details →
          </button>
        </div>

        <Pinned
          conversation={props.conversation}
          selected={props.selected}
          select={props.select}
          details={props.details}
        />
      </div>

      <ol class="timeline">
        <For each={props.conversation.timeline}>
          {(event) => (
            <li class="timeline-event">
              <Switch>
                <Match when={"Brief" in event && event.Brief}>
                  {(brief) => (
                    <Brief
                      id={props.conversation.id}
                      brief={brief()}
                      adopting={props.conversation.adopting !== null}
                    />
                  )}
                </Match>
                <Match when={"Moved" in event && event.Moved}>
                  {(moved) => <Moved moved={moved()} />}
                </Match>
                <Match when={"Handoff" in event && event.Handoff}>
                  {(handoff) => <Handoff handoff={handoff()} />}
                </Match>
                <Match when={"Notice" in event && event.Notice}>
                  {(notice) => <Notice notice={notice()} />}
                </Match>
                <Match when={"ManualTask" in event && event.ManualTask}>
                  {(manual) => <ManualTask manual={manual()} />}
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
                <Match when={"Interruption" in event && event.Interruption}>
                  {(stopped) => (
                    <Interruption
                      conversation={props.conversation}
                      stopped={stopped()}
                      selected={props.selected === stopped().id}
                      open={() => {
                        props.select(stopped().id);
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
          Drawn outside the list: neither is an event, and either would be an
          event that moved every time one landed. Only one of them is ever drawn
          — each is for a different state — so they read as the one thing there
          is to do from here. */}
      <Show
        when={props.conversation.adopting}
        fallback={<StartGrilling conversation={props.conversation} />}
      >
        {(adopting) => (
          <Adoption conversation={props.conversation} adopting={adopting()} />
        )}
      </Show>
      {/* And under that, the way to move the conversation by hand. It is not
          one of the two above — neither is it for one state, nor is it the one
          thing there is to do from here. It is what is offered *whenever
          nothing is running*, which is a quiet moment between steps as much as
          it is a run that has stopped, so it sits below whichever of the two is
          drawn rather than instead of it. */}
      <ManualTaskComposer conversation={props.conversation} />
    </>
  );
}

/// The way to move a conversation by hand: an instruction, a profile to run it
/// under, and a submit.
///
/// Drawn whenever there is a worktree to run in, the conversation is neither
/// drafting nor aborted, and no session is registered for it. That is the
/// literal rule and it is deliberate: the gaps between an unattended run's
/// steps, a wrapping lull, a grilling waiting on a pick, a finished
/// conversation and a run stopped on an interruption all show it, because the
/// point of it is to get a stuck conversation moving. After a server restart
/// nothing is running anywhere, so it shows everywhere, and that is wanted too.
///
/// Drafting and aborted are the two states with no worktree, so those are the
/// two it is never offered in — there is nowhere for a session to run.
///
/// The profile starts on the conversation's implementation one and picking
/// another is one-off: it is what this task runs under, and it never becomes the
/// conversation's own. Nothing here writes it back.
function ManualTaskComposer(props: {
  conversation: ConversationView;
}): JSX.Element {
  const queries = useQueryClient();

  const [instruction, setInstruction] = createSignal("");
  const [picked, setPicked] = createSignal<number | null>(null);
  const [refused, setRefused] = createSignal<ManualTaskStarted | null>(null);

  /// The profile list, read here rather than passed down, the way the details
  /// pane's pickers read it: the control is whole wherever it is drawn.
  const profiles = useReading(() => ({
    queryKey: ["profiles"],
    queryFn: listProfiles,

    // And the same merge, for the same picker — see the details pane. This one
    // sits under a half-typed instruction while a session is talking above it,
    // which is the loudest a Nudge ever gets.
    freshness: { reconcile: "id" },
  }));

  /// Which profile is selected: whatever the human picked, and the
  /// conversation's implementation one until they pick anything.
  const running = () =>
    picked() ?? props.conversation.implementation_profile?.id ?? null;

  /// Whether the composer belongs on this conversation at all.
  const offered = () =>
    props.conversation.worktree !== null &&
    props.conversation.state !== "Draft" &&
    props.conversation.state !== "Aborted" &&
    !props.conversation.working;

  const submit = useMutation(() => ({
    mutationFn: (profileId: number) =>
      startManualTask(props.conversation.id, instruction(), profileId),
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
            `.grow`, which the brief's field and an interruption's note use for
            the same reason. */}
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

        {/* Drawn only once the list is here, the way the details pane's pickers
            are: a select whose value is set before its options exist is a
            select showing nothing. */}
        <div class="manual-task-profile">
          <label for="manual-task-profile">Run it as</label>
          <Show
            when={profiles.data}
            fallback={
              <p class="note">
                {profiles.isError
                  ? `Could not read the agent profiles: ${profiles.error?.message}`
                  : "Reading the agent profiles…"}
              </p>
            }
          >
            {(saved) => (
              /* A [`Picker`] rather than a `<select>`, the way the details
                 pane's pickers are: what this shows and what the press below
                 runs the task as are the same profile, list or no list — see
                 `src/picking.tsx`. */
              <Picker
                id="manual-task-profile"
                options={saved()}
                value={(profile) => String(profile.id)}
                label={(profile) => `${profile.name} — ${profile.models.join(", ")}`}
                chosen={running() === null ? "" : String(running())}
                pick={(chosen) => setPicked(Number(chosen))}
                // The one-off pick is gone from the list: it is dropped, and
                // `running` falls back to the conversation's own implementation
                // profile — which is where the composer opened.
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
            submit.isPending ||
            instruction().trim() === "" ||
            running() === null
          }
          onClick={() => {
            const profileId = running();
            if (profileId !== null) {
              submit.mutate(profileId);
            }
          }}
        >
          {submit.isPending ? "Starting…" : "Set it going"}
        </button>

        <p class="note">
          One session, outside the grilling and the implementation. Nothing about
          the conversation moves — what it leaves behind is what it commits.
        </p>

        <Show when={refused()}>
          {(outcome) => <p class="error">{MANUAL_TASK_REFUSAL[outcome()]}</p>}
        </Show>
        <Show when={submit.isError}>
          <p class="error">
            The manual task could not be started: {submit.error?.message}
          </p>
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
/// One of them opens: a pull request has a full self, which is what is on it
/// right now. Neither list does — what a details pane would show of one is what
/// is already drawn here.
function Pinned(props: {
  conversation: ConversationView;
  selected: number | null;
  select: (event: number) => void;
  details: () => void;
}): JSX.Element {
  return (
    <Show when={props.conversation.pinned.length > 0}>
      <div class="pinned">
        <For each={props.conversation.pinned}>
          {(event) => (
            <Switch>
              <Match when={"TaskList" in event && event.TaskList}>
                {(tasks) => <TaskList tasks={tasks()} />}
              </Match>
              <Match when={"StageList" in event && event.StageList}>
                {(stages) => <StageList stages={stages()} />}
              </Match>
              <Match when={"PullRequest" in event && event.PullRequest}>
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
          )}
        </For>
      </div>
    </Show>
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
              <span class="n">{task.number}</span>
              <span class="what">{task.title}</span>
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
              <span class="n">{stage.number}</span>
              <span class="what">{stage.title}</span>
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
function Handoff(props: { handoff: HandoffEvent }): JSX.Element {
  return (
    <article class="handoff">
      <div class="event-head">
        <h2>Handoff</h2>
      </div>
      <div class="markdown" innerHTML={props.handoff.html} />
    </article>
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
function ManualTask(props: { manual: ManualTaskEvent }): JSX.Element {
  return (
    <article class="manual-task">
      <div class="event-head">
        <h2>Manual task</h2>
      </div>
      <div class="markdown" innerHTML={props.manual.html} />
    </article>
  );
}

/// A move: the Conversation changing hands, said in a line.
///
/// A line and not a card, because there is nothing to it but the fact and the
/// time — everything a move has to say is already in the two.
function Moved(props: { moved: MovedEvent }): JSX.Element {
  return (
    <p class="moved" classList={{ [props.moved.state.toLowerCase()]: true }}>
      {MOVED[props.moved.state]}
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
          fallback={
            <span class="empty">Nothing printed yet.</span>
          }
        >
          {props.output.latest}
        </Show>
      </span>
    </button>
  );
}

/// A Question Set the session put to the human: the table of what was asked
/// against what was decided.
///
/// A button, as a session's output is, and for the same reason: the whole
/// document is in the details pane, and this is how it is opened. What the row
/// shows is the design's summary — the number, the question and the answer —
/// which is what makes a Timeline readable as the record of a conversation
/// rather than as a list of things to go and open.
///
/// A Set still waiting says so instead of drawing a column of blanks: nothing
/// has been decided yet, and an empty answer column would read as a Set that was
/// answered with nothing.
function QuestionSet(props: {
  asked: QuestionSetEvent;
  selected: boolean;
  open: () => void;
}): JSX.Element {
  const waiting = () => "Waiting" in props.asked.standing;
  const archived = () => "ArchivedUnanswered" in props.asked.standing;

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
        <Show when={archived()}>
          <span class="closed">closed unanswered</span>
        </Show>
      </span>

      <table class="asked">
        <tbody>
          <For each={props.asked.rows}>
            {(row) => (
              <tr classList={{ nested: row.nested }}>
                <td class="n">{row.name}</td>
                <td class="question">{row.question}</td>
                <td class="answer">
                  <Show
                    when={row.answer !== ""}
                    fallback={
                      <span class="open">{waiting() ? "—" : "unanswered"}</span>
                    }
                  >
                    {row.answer}
                  </Show>
                </td>
              </tr>
            )}
          </For>
        </tbody>
      </table>
    </button>
  );
}

/// A commit a session landed on the branch: what it was called, and how much of
/// the repository it moved.
///
/// A button, as a session's output and a question set are, and for the same
/// reason: the whole of it — the diff — is in the details pane, and this is how
/// it is opened.
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
    </button>
  );
}

/// The button that gives a Conversation somewhere to work.
///
/// Drawn only while there is something to start. `ready_to_grill` decides
/// whether it is *offered* rather than whether it is enabled: a conversation
/// that has already started has nothing to press, and one that is not ready is
/// told what is missing rather than handed a dead control. The server checks
/// every one of the conditions again regardless — the page's copy is only as
/// fresh as its last read.
function StartGrilling(props: { conversation: ConversationView }): JSX.Element {
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
    <Show when={props.conversation.state === "Draft"}>
      <div class="start-grilling">
        <Show
          when={props.conversation.ready_to_grill}
          fallback={
            // Deliberately not the details pane's wording. That one is a
            // verdict on the conversation, drawn where the profiles are fixed;
            // this one stands in for the button, and says what would make it
            // appear.
            <p class="note">
              Write the brief and choose both agent profiles, and the grilling
              can start.
            </p>
          }
        >
          <button
            type="button"
            class="start"
            disabled={start.isPending}
            onClick={() => start.mutate()}
          >
            {start.isPending ? "Starting…" : "Start grilling"}
          </button>
          <p class="note">
            This creates the branch and its worktree, and freezes the brief.
          </p>
        </Show>

        <Show when={refused()}>
          {(outcome) => <p class="error">{GRILL_REFUSAL[outcome()]}</p>}
        </Show>
        <Show when={start.isError}>
          <p class="error">
            The grilling could not be started: {start.error?.message}
          </p>
        </Show>
      </div>
    </Show>
  );
}

/// What can be done to the conversation as a whole, rather than to any one
/// event: a menu on the header, holding abort.
///
/// A menu rather than a button, because aborting throws a worktree away and the
/// header is somewhere the human's cursor passes on the way to everything else.
/// Native `details`/`summary`, so it opens, closes and reaches the keyboard
/// without any of that being this component's to get right.
function Actions(props: { conversation: ConversationView }): JSX.Element {
  const queries = useQueryClient();

  const [open, setOpen] = createSignal(false);
  const [refused, setRefused] = createSignal<ConversationAborted | null>(null);

  const abort = useMutation(() => ({
    mutationFn: () => abortConversation(props.conversation.id),
    onSuccess: (outcome: ConversationAborted) => {
      if (outcome === "NoSuchConversation" || outcome === "WorktreeStuck") {
        setRefused(outcome);
        return;
      }

      // Aborted or already aborted: what was asked for holds either way.
      setRefused(null);
      setOpen(false);
      void queries.invalidateQueries({ queryKey: ["conversation"] });
      void queries.invalidateQueries({ queryKey: ["conversations"] });
    },
  }));

  return (
    <details
      class="conversation-actions"
      open={open()}
      onToggle={(ev) => setOpen(ev.currentTarget.open)}
    >
      <summary aria-label="Conversation actions">⋯</summary>
      <div class="menu">
        <Show
          when={props.conversation.state !== "Aborted"}
          fallback={<p class="note">This conversation has been aborted.</p>}
        >
          <button
            type="button"
            class="abort"
            disabled={abort.isPending}
            onClick={() => abort.mutate()}
          >
            {abort.isPending ? "Aborting…" : "Abort conversation"}
          </button>
          <p class="note">
            Removes the worktree. The branch stays where it is.
          </p>
        </Show>

        <Show when={refused()}>
          {(outcome) => <p class="error">{ABORT_REFUSAL[outcome()]}</p>}
        </Show>
        <Show when={abort.isError}>
          <p class="error">
            The conversation could not be aborted: {abort.error?.message}
          </p>
        </Show>
      </div>
    </details>
  );
}

/// The Brief: the markdown a Conversation starts from, read inline and written
/// inline.
///
/// Inline in the Timeline rather than in the details pane, because there is
/// nothing of it the Timeline does not already show — it *is* its own summary.
///
/// Read as the server rendered it and written as it was typed. The two are one
/// field's worth of markdown either way, and the Brief is the one document on
/// this wire that travels both ways for exactly that reason.
function Brief(props: {
  id: number;
  brief: BriefEvent;

  /// Whether this conversation is adopting a roadmap, in which case the brief
  /// is nobody here to write: it is the stage brief, and it arrives when the
  /// stage is adopted. So there is no editor, and nothing to open one on.
  adopting: boolean;
}): JSX.Element {
  const queries = useQueryClient();

  // Whether the Brief is being written rather than read. Its own signal and not
  // "is there a draft": an empty Brief is a perfectly ordinary thing to open the
  // field on, and it is the first thing anyone does with a new Conversation.
  const [editing, setEditing] = createSignal(false);

  // What is being typed. Seeded from the Brief when editing starts rather than
  // kept in step with it, so a Brief that changed underneath is the one that
  // opens in the field.
  const [draft, setDraft] = createSignal("");

  const [refused, setRefused] = createSignal<BriefSaved | null>(null);

  const write = () => {
    setDraft(props.brief.markdown);
    setEditing(true);
  };

  const stop = () => {
    setEditing(false);
    setRefused(null);
  };

  const save = useMutation(() => ({
    mutationFn: (markdown: string) => saveBrief(props.id, markdown),
    onSuccess: (outcome: BriefSaved) => {
      if (outcome !== "Saved") {
        // The draft stands: it is the only copy of what was written, and the
        // human is owed the chance to take it somewhere else.
        setRefused(outcome);
        return;
      }

      setRefused(null);
      setEditing(false);
      void queries.invalidateQueries({ queryKey: ["conversation"] });
    },
  }));

  return (
    <article class="brief">
      <div class="event-head">
        <h2>Brief</h2>
        <Show when={!editing() && !props.adopting}>
          <button type="button" class="edit-brief" onClick={write}>
            Edit
          </button>
        </Show>
      </div>

      <Show
        when={editing()}
        fallback={
          <Show
            when={props.brief.markdown !== ""}
            fallback={
              <p class="empty">
                <Show
                  when={props.adopting}
                  fallback={
                    <>
                      Nothing written yet — this is what the grilling starts
                      from.
                    </>
                  }
                >
                  Nothing written yet — adopting the stage is what puts its
                  brief here.
                </Show>
              </p>
            }
          >
            <div class="brief-body markdown" innerHTML={props.brief.html} />
          </Show>
        }
      >
        <form
          class="edit-brief-form"
          onSubmit={(ev) => {
            ev.preventDefault();
            save.mutate(draft());
          }}
        >
          {/* A copy of what has been typed gives the field its height — see
              `.grow`. */}
          <div class="grow" data-value={draft()}>
            <textarea
              rows="1"
              aria-label="Brief"
              placeholder="What is this piece of work?"
              value={draft()}
              onInput={(ev) => {
                setDraft(ev.currentTarget.value);
                setRefused(null);
              }}
            />
          </div>
          <div class="edit-brief-buttons">
            <button type="submit" disabled={save.isPending}>
              {save.isPending ? "Saving…" : "Save"}
            </button>
            <button type="button" class="cancel" onClick={stop}>
              Cancel
            </button>
          </div>
          <Show when={refused()}>
            {(outcome) => <p class="error">{BRIEF_REFUSAL[outcome()]}</p>}
          </Show>
          <Show when={save.isError}>
            <p class="error">
              The brief could not be saved: {save.error?.message}
            </p>
          </Show>
        </form>
      </Show>
    </article>
  );
}
