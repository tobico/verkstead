//! A Conversation's Timeline: everything that has happened to it, in order.
//!
//! The kinds of Event so far — the Brief, a move, what a session printed, a
//! Question Set, the direction the human chose, the handoff, and the commits a
//! session lands on the branch — drawn as a list of Events rather than as a
//! Brief with a list under it. The stages after this one put task lists, PRs
//! and interruptions on the same list.
//!
//! An Event that has a full self shows its summary here and is opened in the
//! details pane, which is why this takes a way of selecting one. The Brief has
//! no full self beyond what is already drawn, so it is the one Event nothing
//! opens.
//!
//! The Timeline is also where the work is moved on from, because that is where
//! the reason to move it is: a control sits at the end of everything that has
//! happened so far, which is exactly where the next thing to happen belongs.
//! Two of them live there, one per state — `Start grilling` under the Brief it
//! will freeze, and the direction chooser under the proposal that ended the
//! grilling. Aborting is in neither place and not in the list: it is not a step
//! in the work but a way of ending it, so it hangs off the header behind a menu,
//! where a destructive action is not one stray click away.
//!
//! Nothing here ends the grilling. That is the agent's own closing move — a
//! marked Question Set, answered — which is why the chooser appears without any
//! button on this page having been pressed.

import { useMutation, useQueryClient } from "@tanstack/solid-query";
import { For, Match, Show, Switch, createSignal, type JSX } from "solid-js";

import {
  abortConversation,
  chooseDirection,
  saveBrief,
  startGrilling,
} from "../api/client";
import type {
  AgentOutputEvent,
  BriefEvent,
  BriefSaved,
  CommitEvent,
  ConversationAborted,
  ConversationView,
  DirectedEvent,
  Direction,
  DirectionChosen,
  GrillingStarted,
  HandoffEvent,
  Lifecycle,
  MovedEvent,
  QuestionSetEvent,
} from "../api/types";

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

/// And each way of being refused a direction.
export const DIRECTION_REFUSAL: Record<DirectionChosen, string> = {
  Chosen: "",
  NoSuchConversation: "This conversation is gone.",
  NotChoosing:
    "This conversation is not choosing a direction — the grilling has not proposed wrapping up, or the work is past this point.",
  RoadmapNotYet: "Staged roadmaps are not built yet.",
};

/// What each direction is called, wherever one is named: on a button in the
/// chooser, and in the line the timeline gives the choice afterwards.
///
/// One record for both, so the thing the human pressed and the thing they read
/// back cannot come to be called different things.
export const DIRECTION: Record<Direction, string> = {
  inline: "Implement inline",
  "task-list": "Break into a task list",
  roadmap: "Stage a roadmap",
};

/// What a move reads as. The state moved *to*, said as something that happened.
const MOVED: Record<Lifecycle, string> = {
  Draft: "Went back to drafting",
  Grilling: "Started grilling",
  Direction: "Moved to choosing a direction",
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
      <div class="pane-head">
        {/* The way back out of this level, which is the whole of what a narrow
            window offers instead of the pane beside it. Drawn always and hidden
            by the stylesheet where all three panes are on screen at once. */}
        <button type="button" class="pane-back" onClick={props.back}>
          ← Conversations
        </button>
        <h1>{props.conversation.branch}</h1>
        <Actions conversation={props.conversation} />
        <button type="button" class="pane-forward" onClick={props.details}>
          Details →
        </button>
      </div>

      <ol class="timeline">
        <For each={props.conversation.timeline}>
          {(event) => (
            <li class="timeline-event">
              <Switch>
                <Match when={"Brief" in event && event.Brief}>
                  {(brief) => (
                    <Brief id={props.conversation.id} brief={brief()} />
                  )}
                </Match>
                <Match when={"Moved" in event && event.Moved}>
                  {(moved) => <Moved moved={moved()} />}
                </Match>
                <Match when={"Directed" in event && event.Directed}>
                  {(directed) => <Directed directed={directed()} />}
                </Match>
                <Match when={"Handoff" in event && event.Handoff}>
                  {(handoff) => <Handoff handoff={handoff()} />}
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
      <StartGrilling conversation={props.conversation} />
      <DirectionChooser conversation={props.conversation} />
    </>
  );
}

/// The direction the human chose, said in a line.
///
/// A line and not a card, like a move, and for the same reason: there is nothing
/// to it but the fact and the time. It sits below the move into Direction rather
/// than replacing it — the move says the choosing began and this says how it came
/// out, and a human who changed their mind has both on the record.
function Directed(props: { directed: DirectedEvent }): JSX.Element {
  return (
    <p class="directed" classList={{ [props.directed.direction]: true }}>
      Chose to {DIRECTION[props.directed.direction].toLowerCase()}
    </p>
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
/// is opened — the summary is a line, and a grilling session's transcript is an
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
        <span class="lines">
          {props.output.lines} {props.output.lines === 1 ? "line" : "lines"}
        </span>
        <Show when={props.output.running}>
          <span class="live">running</span>
        </Show>
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

/// The three ways the work can be built, in the order the design names them.
///
/// Roadmap is here and not runnable: the choice exists so the shape of the
/// decision is visible, and the stage that executes one has not landed. Drawn
/// disabled rather than left out, because a chooser that grew a third button
/// later would be a decision the human had never been shown.
const DIRECTIONS: { direction: Direction; note: string; ready: boolean }[] = [
  {
    direction: "inline",
    note: "One fresh session under the implementation profile, primed with the handoff. Starts as soon as you choose it.",
    ready: true,
  },
  {
    direction: "task-list",
    note: "Broken into .tasks/ in the worktree, one fresh session per task.",
    ready: true,
  },
  {
    direction: "roadmap",
    note: "Staged under docs/roadmaps/, a feature per stage. Arriving in a later stage.",
    ready: false,
  },
];

/// Where the grilling hands over: what the agent proposed, and the human's
/// choice of how the work gets built.
///
/// Drawn only while the conversation is choosing. What moved it here was the
/// grilling's closing question set being answered — there is no button for that,
/// which is the whole point of the agent proposing rather than the human
/// declaring.
///
/// The recommendation is marked rather than preselected. Nothing is chosen until
/// the human presses something, and a control that arrived already set would be
/// the agent deciding in their place.
function DirectionChooser(props: { conversation: ConversationView }): JSX.Element {
  const queries = useQueryClient();

  const [refused, setRefused] = createSignal<DirectionChosen | null>(null);

  const choose = useMutation(() => ({
    mutationFn: (direction: Direction) =>
      chooseDirection(props.conversation.id, direction),
    onSuccess: (outcome: DirectionChosen) => {
      if (outcome !== "Chosen") {
        setRefused(outcome);
        // Refused against a picture of the world this page read a moment ago:
        // reading it again is both the correction and the explanation.
        void queries.invalidateQueries({ queryKey: ["conversation"] });
        return;
      }

      setRefused(null);
      void queries.invalidateQueries({ queryKey: ["conversation"] });
    },
  }));

  /// Whether this direction is the one the grilling recommended.
  const recommended = (direction: Direction) =>
    props.conversation.proposal?.direction === direction;

  return (
    <Show when={props.conversation.state === "Direction"}>
      <div class="direction-chooser">
        <h2>How should this be built?</h2>

        {/* The agent's reasoning, which is what the human is deciding against.
            A conversation that reached Direction some other way has none, and
            says so rather than drawing an empty box. */}
        <Show
          when={props.conversation.proposal}
          fallback={
            <p class="empty">
              The grilling left no recommendation, so this is an open choice.
            </p>
          }
        >
          {(proposal) => (
            <div
              class="proposal markdown"
              innerHTML={proposal().rationale_html}
            />
          )}
        </Show>

        <ul class="directions">
          <For each={DIRECTIONS}>
            {(offered) => (
              <li
                classList={{
                  recommended: recommended(offered.direction),
                  chosen: props.conversation.direction === offered.direction,
                }}
              >
                <button
                  type="button"
                  class="direction"
                  disabled={!offered.ready || choose.isPending}
                  aria-pressed={props.conversation.direction === offered.direction}
                  onClick={() => choose.mutate(offered.direction)}
                >
                  {DIRECTION[offered.direction]}
                </button>
                <Show when={recommended(offered.direction)}>
                  <span class="mark">Recommended</span>
                </Show>
                <p class="note">{offered.note}</p>
              </li>
            )}
          </For>
        </ul>

        {/* What was chosen, said where it was chosen — which is only ever a
            task list by the time this is read: choosing inline starts the work,
            and a conversation that has started is past this chooser and no
            longer drawing it. Saying that nothing has run yet is better than a
            button that looks broken. */}
        <Show when={props.conversation.direction}>
          {(direction) => (
            <p class="note chosen-note">
              Chosen: {DIRECTION[direction()].toLowerCase()}. Nothing runs off
              this yet.
            </p>
          )}
        </Show>

        <Show when={refused()}>
          {(outcome) => <p class="error">{DIRECTION_REFUSAL[outcome()]}</p>}
        </Show>
        <Show when={choose.isError}>
          <p class="error">
            The direction could not be chosen: {choose.error?.message}
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
function Brief(props: { id: number; brief: BriefEvent }): JSX.Element {
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
        <Show when={!editing()}>
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
                Nothing written yet — this is what the grilling starts from.
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
