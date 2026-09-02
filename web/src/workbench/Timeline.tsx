//! A Conversation's Timeline: everything that has happened to it, in order.
//!
//! The kinds of Event so far — the Brief, a move, what a session printed, a
//! Question Set, the handoff, the Notices Verkstead writes on its own account,
//! and the commits a session lands on the branch — drawn as a list of Events
//! rather than as a Brief with a list under it.
//!
//! Above the list are the pinned Events, which are a fixed set — the backlog
//! now, the stage list and the PR as those stages arrive — and which are drawn
//! in one order however they arrived: the pull request, then the task list, then
//! the roadmap. They do not scroll with the record: each is the current state of
//! something the work is against, and is worth having on screen whichever part
//! of the record is being read.
//! More than one of them is a carousel rather than a stack, because everything
//! pinned is held above the record and a stack of them is what the record is
//! pushed down by.
//!
//! Each of them is a moment as well — the pull request the finish step opened,
//! and the backlog and the roadmap at the moment they landed on the branch — so
//! each is drawn in both places: the same card in the pinned block and on the
//! record where it happened. A second appearance rather than a move: what the
//! record says happened should stay on it.
//!
//! The two lists differ from the pull request in where the card's content comes
//! from. A PR is three facts the record holds; a backlog and a roadmap are read
//! off the worktree every time the conversation is, so what the record row fixes
//! is the position and the card at it says what the list holds now. Nothing is
//! backfilled: a conversation from before the rows existed has its cards in the
//! pinned block alone.
//!
//! The pane is read from the bottom, because that is where the work got to: it
//! opens at the end of the record and stays there as Events arrive, until the
//! human scrolls up to read something further back — and again once they come
//! down to the end. The same following a running session's output is read with,
//! and the same code (`../scrolling`).
//!
//! An Event that has a full self shows its summary here and is opened in the
//! details pane, which is why this takes a way of selecting one — and so do the
//! backlog and the roadmap, whose cards open the documents their entries name
//! and which are selected by a word rather than by an id, having no Event of
//! their own. Three of them are documents — the frozen Brief, the handoff and
//! the instruction a
//! steer carried — and a document's summary is its own opening: the card shows
//! [`CLAMPED_LINES`] of it under a fade, and the pane holds the whole. A Notice
//! is read the same way and cut differently: what it has to say is a sentence
//! rather than a document, so its card shows one line under an ellipsis. The
//! Brief is also the one Event that is written here as well as read: while the
//! Conversation is drafting it is a field that saves itself rather than a card
//! to open, and it carries a Conversation's setup under it for as long as there
//! is a draft to set up.
//!
//! Nothing is held against the foot of the pane. A strip for the session running
//! now used to be — the title and the liveness mark, a way back to a card that a
//! long record had scrolled away from — and the status button at the head of the
//! pane says what is running, in more words than the strip ever did and where
//! the eye lands rather than at the far end of the pane.
//!
//! The Timeline is also where the work is moved on from, because that is where
//! the reason to move it is: a control sits at the end of everything that has
//! happened so far, which is exactly where the next thing to happen belongs.
//! One lives there — `Start work` under the Brief it will freeze. What to
//! do about a conversation Verkstead has finished with is not there: a Steer is
//! the way back into one, and it is in the menu the status button drops with
//! everything else done to the conversation as a whole.
//! Getting a stopped conversation going again is in that menu too rather than at
//! the foot of the record: Resume is a row of it like the rest, so it is reached
//! from the thing that says nothing is driving this — and from the sidebar's
//! right-click, which drops the same rows.
//! Stopping the work is in neither place and not in the list: none of the three
//! ways of doing it — stop after this task, stop now, close the conversation —
//! is a step in the work, so all three are in that same menu, where what cannot
//! be undone is not one stray click away.
//!
//! Nothing here ends the grilling, and nothing here chooses a direction. That is
//! the agent's own closing move — a Question Set carrying a proposal, with the
//! chooser drawn on the Set itself — so both happen on the page the Set is
//! answered on and land here as the answered Set.

import { faShare } from "@fortawesome/free-solid-svg-icons";
import { useMutation, useQueryClient } from "@tanstack/solid-query";
import {
  For,
  Match,
  Show,
  Switch,
  createMemo,
  createSignal,
  onCleanup,
  onMount,
  type JSX,
} from "solid-js";

import { ran, reading } from "../agents";
import { listProfiles, saveBrief, startGrilling } from "../api/client";
import type {
  AgentOutputEvent,
  BriefEvent,
  BriefSaved,
  CommitEvent,
  CompanionRefusal,
  ConversationView,
  GrillingStarted,
  HandoffEvent,
  Lifecycle,
  ManualTaskEvent,
  MovedEvent,
  NoticeEvent,
  PinnedEvent,
  ProfileEntry,
  PullRequestEvent,
  QuestionSetEvent,
  StageListEvent,
  StageListReached,
  SteerEvent,
  TaskListEvent,
  TaskListReached,
  TimelineEvent,
  UnreadableSetEvent,
} from "../api/types";
import app from "../App.module.css";
import { CardButton } from "../CardButton";
import { IconButton } from "../IconButton";
import { PaneSticky } from "../Panes";
import { useReading } from "../freshness";
import { HarnessMark } from "../HarnessMark";
import { Empty, ErrorLine, Note } from "../notices";
import { followBottom } from "../scrolling";
// The badge and the sentence a Set this build cannot read is drawn with, taken
// from the page that draws the whole record rather than kept a second time
// here: the row and the page are one record read at two distances.
import unreadable from "../set/Unreadable.module.css";
import { Adoption } from "./Adoption";
import { Checks } from "./Checks";
import { Mark } from "./Mark";
// The marks themselves, for the one this file draws without a session behind
// it: a Set waiting on the human wears the disc the sidebar's card wears, and
// that vocabulary is the module's rather than any one component's.
import marks from "./Mark.module.css";
import { Conflict } from "./Merging";
import { PaneHead } from "./PaneHead";
import { Setup } from "./Setup";
import { StatusButton } from "./StatusButton";
import styles from "./Timeline.module.css";
import { titled } from "./naming";
import { STATE } from "./states";
import { opensRoadmap, type Opening } from "./openings";
import { keeping } from "./settling";
import { windowed } from "./windowing";

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

/// And each way of being refused a start, for the conversation's own repo.
///
/// Every one of them is something different to go and do, which is the whole
/// reason the server names them separately rather than saying "cannot start".
const GRILL_REFUSAL: Record<
  Exclude<GrillingStarted, { Companion: unknown }>,
  string
> = {
  Started: "",
  NoSuchConversation: "This conversation is gone.",
  NotDrafting: "This conversation has already been started.",
  NoGrillingProfile:
    "Pick a grilling profile and model — or No grilling — first, on the brief.",
  NoImplementationProfile:
    "Choose an implementation profile and model first, on the brief.",
  NoReviewProfile:
    "Pick a review profile and model — or No review — first, on the brief.",
  ProfileBroken:
    "A chosen profile's claude pair is not where it was left, so there is no account to run under.",
  EmptyBrief: "Write the brief first — it is what the work starts from.",
  FetchFailed:
    "Git could not fetch from the repo's remote, so nothing was started. The server log says why.",
  NoBaseCommit: "The repo has nothing to branch from any more.",
  BranchExists: "That branch already exists, and Verkstead did not make it.",
  WorktreeRefused: "Git would not make the worktree. The server log says why.",
};

/// And the same four failings over a companion repo, which say the same things
/// about a different repository.
///
/// Exported because both presses that take a draft past drafting meet them:
/// starting a grilling and adopting a stage each check the companions out, so
/// each is refused by these four names — see `adoptRefusal` in
/// [`Adoption`](./Adoption.tsx).
export const COMPANION_REFUSAL: Record<CompanionRefusal, string> = {
  FetchFailed:
    "Git could not fetch from its remote, so nothing was started. The server log says why.",
  NoBaseCommit: "It has nothing to check out any more.",
  BranchExists:
    "The branch already exists there, and Verkstead did not make it.",
  WorktreeRefused: "Git would not make its worktree. The server log says why.",
};

/// What to say about a start that was refused.
///
/// A companion's refusal names the repository, because that is the whole of
/// what makes it different from the same failing on the conversation's own: the
/// thing to go and look at is one of several repos rather than the obvious one.
export function grillRefusal(outcome: GrillingStarted): string {
  if (typeof outcome === "object") {
    return `${outcome.Companion.repo}: ${COMPANION_REFUSAL[outcome.Companion.why]}`;
  }

  return GRILL_REFUSAL[outcome];
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
function Clamped(props: {
  /// The module's name for whichever document this is, put on the rendering
  /// rather than on the cut around it: the three of them are read at the
  /// measure, and the clamp is the same clamp for all three.
  class: string;
  html: string;
}): JSX.Element {
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
    <div class={styles.clamp} classList={{ [styles.cut!]: cut() }} ref={clamp}>
      <div class={`${props.class} markdown`} innerHTML={props.html} ref={body} />
    </div>
  );
}

/// A card whose whole surface opens the details pane, drawn as an `article`.
///
/// Every other openable Event is a `CardButton` drawing the button it is, and
/// the cards drawn through here cannot have one: the documents hold rendered
/// markdown, and a link inside a button is not something a browser will have;
/// the lists and the pull request hold a heading and rows, which a button would
/// flatten to one run of text. So the card is asked for as an article, and
/// `CardButton` puts the press, the keyboard and the role that says what it is
/// on that instead — it reads as the same card either way, because it is the
/// same card.
///
/// `open` is nothing where the card is not openable, which is the Brief for as
/// long as it is a draft: a field is not a thing to press, and neither is the
/// setup standing under it.
function Openable(props: {
  /// The module's name for which of the three documents this card is. The card
  /// is told rather than asking, because what a Brief is and what a Handoff is
  /// is the caller's to know.
  kind: string;
  selected: boolean;
  open: (() => void) | null;
  children: JSX.Element;
}): JSX.Element {
  return (
    <CardButton
      as="article"
      class={props.kind}
      open={props.selected}
      press={props.open}
    >
      {props.children}
    </CardButton>
  );
}

/// What this pane is called: the branch it is titled by, and the Repo that
/// branch is in understated beside it.
///
/// The two facts the sidebar's card says in the same order and the same voice,
/// so the card and the header of the pane it opens read as the one name said
/// twice — and the status button under them goes on in that voice with its own
/// two lines.
///
/// Drawn in every state, a Draft's included. A Conversation nobody has named is
/// called *Draft* on both, which is what it is; the Repo beside it is then the
/// only thing on the header that tells one draft from the next.
///
/// The space between the two is written out, and is the whole of what a screen
/// reader has to tell them apart: the heading is named by everything under it
/// run together, and two spans with nothing between them are read as one word.
/// It is not drawn — a run of white space makes no flex item — so the gap
/// between them on screen is still the stylesheet's.
function PaneName(props: { conversation: ConversationView }): JSX.Element {
  return (
    <>
      <span class={styles.paneTitle}>{titled(props.conversation)}</span>{" "}
      <span class={styles.paneRepo}>{props.conversation.repo.name}</span>
    </>
  );
}

export function Timeline(props: {
  conversation: ConversationView;

  /// The way back out to the conversations, where there is a list to go back
  /// to. A share has none — it is one Conversation and nothing around it — so
  /// there is nothing to draw and nowhere to go.
  back?: () => void;

  details: () => void;

  /// Which Event the details pane is showing, and how to change it.
  selected: Opening | null;
  select: (opening: Opening) => void;

  /// Whether this is a record to read rather than a Conversation to work in,
  /// which is what a share is.
  ///
  /// What it takes off is everything that is not a moment on the record: the
  /// status button, which is where the work stands and what is running in it
  /// and the whole of the actions menu behind one press; the share icon, which
  /// offers a reader a publish of somebody else's Conversation; and the block
  /// that says what happens next. The cards themselves are untouched — a share
  /// is read by opening them, exactly as the workbench is.
  ///
  /// Nothing about a *card* is decided here, and that is deliberate: a shared
  /// Conversation arrives with every field a control is drawn from already
  /// saying nothing — see `shared` in `crates/render/src/sharing.rs` — so the
  /// record cannot express an action whatever draws it. This is the header and
  /// what stands after the record, which belong to the Conversation rather than
  /// to anything on it.
  readOnly?: boolean;
}): JSX.Element {
  /// The record itself, which is what says which box this pane scrolls in: a
  /// column of the page below the first breakpoint, and the pane above it.
  let record!: HTMLOListElement;

  // The saved Profiles, for the one thing the record needs them for: whether
  // the account a session ran as is the only one on its backend, and so whether
  // its name is said after the model. Once for the pane rather than once per
  // card, a record holding a session per resume being a column of them.
  //
  // Not in a share, which is what the gate is: a share fetches nothing, and an
  // Agent run never boards one anyway — so this is a read for a card that
  // cannot be there. A list that has not been read says the account's name,
  // which is the answer that can never misattribute a run.
  const profiles = useReading(() => ({
    queryKey: ["profiles"],
    queryFn: listProfiles,
    enabled: !props.readOnly,
    freshness: { reconcile: "id" },
  }));

  // And the pane follows the bottom of it, the way a running session's output
  // already does — the same code, because it is the same reading: a record still
  // being written, opened at its end because what is being said now is there.
  // Every arriving Event puts the view back at the bottom until the human
  // scrolls up to read something further back, and again once they come down to
  // the end — see [`../scrolling`].
  //
  // Followed whatever the conversation is doing, rather than only while a
  // session talks. A Timeline is never finished with: a resume, a steer, a
  // commit landing off a check all put something new on the end of one, and
  // there is no moment at which it is right to open a record anywhere but at
  // its end. A record nothing is being added to never grows, so the following
  // has nothing to do beyond landing the view where it opened.
  followBottom(
    () => record,
    () => true,
    // What growing means for a record that is only ever appended to.
    () => props.conversation.timeline.length,
  );

  return (
    <>
      {/* The header and the pinned block as one block, because that is how they
          stay: the stylesheet sticks this to the top edge of the pane and both
          of them travel with it, so there is no strip of scrolling record
          between the title and the pinned items and nothing to keep a pinned
          block's own offset in step with. */}
      <PaneSticky>
        {/* The way back out of this level, which is the whole of what a narrow
            window offers instead of the pane beside it. Drawn wherever there is
            a list to go back to, and hidden by the pane head where all three
            panes are on screen at once.

            Titled for its branch, or a Draft where nobody has named one — the
            same rule the sidebar row it was opened from draws, so the card and
            the header are the one name.

            And the Repo understated beside it, as the card says the same two
            facts: the header and the card are then the one name said twice.
            Drawn in every state, a Draft's included — two drafts against two
            repositories are both called Draft, and the Repo is the only thing
            on either header that tells them apart. */}
        <PaneHead
          back={
            props.back === undefined
              ? undefined
              : { to: "Conversations", go: props.back }
          }
          heading={styles.paneName}
          title={<PaneName conversation={props.conversation} />}
        >
          {/* The pane's own controls, in the slot the settings gear stands in
              at the head of the conversations. Both of them page into the
              details, so they are one group at the end of the row rather than
              two things the header holds apart. */}
          <div class={styles.paneControls}>
            {/* And the way on to the next level, drawn only where there is a
                next level to reach: the details pane holds the selected Event
                and nothing else, so with nothing selected it is bare paper and
                a control that paged into it would page into nothing. Hidden by
                the stylesheet anyway where all three panes are on screen at
                once. */}
            <Show when={props.selected !== null}>
              <button
                type="button"
                class={styles.paneForward}
                onClick={props.details}
              >
                Details →
              </button>
            </Show>

            {/* Sharing the Conversation, which was four rows of the actions
                menu and is now a pane of its own — see `Share.tsx`. Drawn
                exactly as the settings gear beside the Verkstead wordmark is:
                the same [`IconButton`](../IconButton.tsx), open while its pane
                is what the details are showing, because it is another thing
                standing in a pane that is selected and opened into the pane
                beside it.

                The press walks into the details as pressing a card does, which
                is what makes it work on a narrow window: the selection and the
                level are two acts, exactly as they are for every card below. */}
            <Show when={!props.readOnly}>
              <IconButton
                of={faShare}
                label="Share"
                open={props.selected === "share"}
                press={() => {
                  props.select("share");
                  props.details();
                }}
              />
            </Show>
          </div>
        </PaneHead>

        {/* And under the title, where the eye lands: where the work stands,
            what is running in it, and behind its press everything there is to
            do about it. Above the pinned cards because it is about the
            Conversation rather than about anything the work is against, and
            inside the same block so that it stays in view with them.

            Not in a share, where all three of those are answers about a
            workbench the reader is not sitting at: nothing is running in a
            file, everything the menu offers is done to a Conversation they do
            not hold, and where the work stands is what the record under it
            says. */}
        <Show when={!props.readOnly}>
          <StatusButton conversation={props.conversation} />
        </Show>

        <Pinned
          conversation={props.conversation}
          selected={props.selected}
          select={props.select}
          details={props.details}
        />
      </PaneSticky>

      <ol class={styles.timeline} ref={record}>
        <For each={props.conversation.timeline}>
          {(event, index) => (
            <Show when={drawable(event)}>
            <li class={styles.timelineEvent}>
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
                <Match when={"Steer" in event && event.Steer}>
                  {(steer) => (
                    <Steered
                      steer={steer()}
                      selected={props.selected === steer().id}
                      open={() => {
                        props.select(steer().id);
                        props.details();
                      }}
                    />
                  )}
                </Match>
                <Match
                  when={"ResolveConflicts" in event && event.ResolveConflicts}
                >
                  <ConflictsResolved />
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
                  {(notice) => (
                    <Notice
                      notice={notice()}
                      blocked={
                        props.conversation.blocked_on === notice().id
                      }
                      selected={props.selected === notice().id}
                      open={() => {
                        props.select(notice().id);
                        props.details();
                      }}
                    />
                  )}
                </Match>
                <Match when={"ManualTask" in event && event.ManualTask}>
                  {(manual) => <ManualTask manual={manual()} />}
                </Match>
                <Match when={"AgentOutput" in event && event.AgentOutput}>
                  {(output) => (
                    <AgentOutput
                      output={output()}
                      saved={profiles.data}
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
                {/* The pull request where it happened, which is the same card
                    the pinned block above holds still: one event drawn twice.
                    Selecting from either marks both, because both are the one
                    pull request and there is one details pane behind it. */}
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
                {/* And the two lists where they landed, drawn the same way and
                    from the same reading the pinned block is drawn from — so
                    the copy on the record ticks along with the work exactly as
                    the pinned one does. Nothing where the worktree has gone:
                    the row is a moment that happened, and what it showed is
                    read off a branch that is no longer there. */}
                <Match when={"TaskList" in event && event.TaskList}>
                  {(reached) => (
                    <TaskListRow
                      reached={reached()}
                      selected={props.selected === "backlog"}
                      open={() => {
                        props.select("backlog");
                        props.details();
                      }}
                    />
                  )}
                </Match>
                <Match when={"StageList" in event && event.StageList}>
                  {(reached) => (
                    <StageListRow
                      reached={reached()}
                      selected={props.selected}
                      select={props.select}
                      details={props.details}
                    />
                  )}
                </Match>
              </Switch>
            </li>
            </Show>
          )}
        </For>
      </ol>

      {/* After everything that has happened, because it is what happens next.
          Drawn outside the list: neither is an event, and either would be an
          event that moved every time one landed. Only one is ever drawn — each
          is for a different state — so they read as the one thing there is to do
          from here. What to do about a conversation that is finished is not
          here, and neither is getting a stopped one going again: a steer and a
          resume are both rows of the menu the status button drops, drawn
          whatever state the conversation is in.

          And none of it is drawn in a share: what happens next is nothing a
          reader holding a file has any part in, and a record that ended on a
          press would be asking them for one. */}
      <Show when={!props.readOnly}>
        <Show
          when={props.conversation.adopting}
          fallback={<StartGrilling conversation={props.conversation} />}
        >
          {(adopting) => (
            <Adoption
              conversation={props.conversation}
              adopting={adopting()}
            />
          )}
        </Show>
      </Show>
    </>
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
/// They come in one order and are drawn in it: the pull request, then the task
/// list, then the roadmap. The server is what puts them in it — see the pinned
/// block in `crates/server/src/ui.rs`.
///
/// One card at a time once there is more than one of them: they are held above
/// the record, so a stack of them is a stack the record is pushed down by, and
/// what is pinned is worth having in view rather than worth having all of at
/// once. One of them alone is drawn exactly as it always was — a carousel of one
/// is furniture around a card nothing can be turned to.
function Pinned(props: {
  conversation: ConversationView;
  selected: Opening | null;
  select: (opening: Opening) => void;
  details: () => void;
}): JSX.Element {
  return (
    <Show when={props.conversation.pinned.length > 0}>
      <div class={styles.pinned}>
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

/// How long a turn between cards takes, in milliseconds.
///
/// The stylesheet is what runs the slide and this is what clears up after it, so
/// the two are the same number written twice — see `.arriving` in
/// `Timeline.module.css`.
export const SLIDE = 200;

/// One card in the deck: which of the pinned cards it is, and what it is doing
/// there. `showing` is the ordinary state and the only one with nothing in
/// flight; the other two are the pair a turn holds side by side while it runs.
type Pane = {
  index: number;
  part: "showing" | "leaving" | "arriving";
  onward: boolean;
};

/// The carousel: one pinned card showing, and the ways to the others.
///
/// Dots above saying how many there are and which is showing, arrows over the
/// card's edges where there is a pointer to reach them with, and a swipe across
/// the card where there is not. All three are the same move, which is why they
/// are one function between them.
///
/// Above rather than beneath, because the cards are not the same height as each
/// other: dots under them would move every time the card changed, and they are
/// the one part of the carousel that has to hold still to be aimed at.
///
/// It wraps: with two or three cards, an arrow that stopped at the end would be
/// a dead control most of the time.
///
/// A turn slides. For as long as one runs the deck holds both cards — the one
/// leaving and the one arriving — and the stylesheet moves the pair the way the
/// deck is travelling. Whether they move at all is the stylesheet's to say too:
/// under `prefers-reduced-motion` the leaving card is not drawn and the swap is
/// the instant one this replaced.
///
/// Which card fronts is [`fronting`]'s to say, and it says it once — when the
/// conversation is opened and this is built. Nothing is remembered between
/// visits, and nothing moves the card under a reader afterwards: a re-read that
/// jumped the carousel back to where it started would be the page arguing with
/// whoever is holding it.
function Carousel(props: {
  conversation: ConversationView;
  selected: Opening | null;
  select: (opening: Opening) => void;
  details: () => void;
}): JSX.Element {
  const cards = () => props.conversation.pinned;

  const [at, setAt] = createSignal(fronting(props.conversation));

  /// Never off the end of a list that shrank underneath it — a pull request is
  /// pinned as the run finishes, and a backlog stops being pinned as its last
  /// task file goes.
  const showing = () => Math.min(at(), cards().length - 1);

  /// The turn that is running, where there is one: the card it left, and the
  /// way it is going. Nothing at all while the deck is at rest.
  const [turning, setTurning] = createSignal<{
    from: number;
    onward: boolean;
  } | null>(null);

  let running: ReturnType<typeof setTimeout> | undefined;
  onCleanup(() => clearTimeout(running));

  /// Turn to a card, counting round both ends.
  const turn = (to: number) => {
    const many = cards().length;
    const was = showing();
    const next = ((to % many) + many) % many;
    if (next === was) {
      return;
    }

    // Which way it travels is read before the count is folded back into the
    // list: turning on past the last card is still travelling onwards, and the
    // card it lands on is the first.
    setTurning({ from: was, onward: to > was });
    setAt(next);
    clearTimeout(running);
    running = setTimeout(() => setTurning(null), SLIDE);
  };

  /// What the deck is holding: the card showing, and — while a turn runs — the
  /// card it is leaving behind as well, in the order they travel in.
  ///
  /// Held still unless a turn changed one of them. The conversation is re-read
  /// the whole time it is open, and a re-read that rebuilt the deck would
  /// restart a slide that nothing asked for.
  const deck = createMemo<Pane[]>(
    () => {
      const going = turning();
      const now: Pane = { index: showing(), part: "showing", onward: true };
      if (!going || going.from > cards().length - 1) {
        return [now];
      }
      return [
        { index: going.from, part: "leaving", onward: going.onward },
        { ...now, part: "arriving", onward: going.onward },
      ];
    },
    [],
    {
      equals: (was, now) =>
        was.length === now.length &&
        was.every(
          (pane, index) =>
            pane.index === now[index]!.index &&
            pane.part === now[index]!.part &&
            pane.onward === now[index]!.onward,
        ),
    },
  );

  /// Where the finger went down, in the coordinates it will come back up in.
  let from: number | null = null;

  return (
    <div class={styles.carousel}>
      {/* The dots, above the card: how many cards there are, which one is
          showing, and a way straight to any of them. Each is named for the card
          it turns to rather than numbered, because that is what a reader who
          cannot see the dots needs to know about it. */}
      <ol class={styles.dots}>
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

      <div
        class={styles.deck}
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
        <For each={deck()}>
          {(pane) => (
            <Show when={cards()[pane.index]}>
              {(card) => (
                <div class={parting(pane)}>
                  <Card
                    event={card()}
                    selected={props.selected}
                    select={props.select}
                    details={props.details}
                  />
                </div>
              )}
            </Show>
          )}
        </For>

        {/* The arrows, which the stylesheet draws only where there is a
            pointer: on a touch device the swipe is what these are, and two
            buttons lying over the card would be two buttons in the way of it.

            Inside the deck rather than beside it, so that what they are
            centred against is the card rather than the card and its dots. */}
        <button
          type="button"
          class={`${styles.step} ${styles.back}`}
          aria-label="Previous pinned card"
          onClick={() => turn(showing() - 1)}
        >
          ‹
        </button>
        <button
          type="button"
          class={`${styles.step} ${styles.on}`}
          aria-label="Next pinned card"
          onClick={() => turn(showing() + 1)}
        >
          ›
        </button>
      </div>
    </div>
  );
}

/// What a card in the deck wears: nothing where the deck is at rest, and while a
/// turn runs the part it is playing and the way the deck is travelling — which
/// is between them everything the stylesheet needs to slide it.
function parting(pane: Pane): string | undefined {
  if (pane.part === "showing") {
    return undefined;
  }

  const part = pane.part === "leaving" ? styles.leaving : styles.arriving;
  return `${part} ${pane.onward ? styles.onward : styles.backward}`;
}

/// Which card is showing when a conversation is opened: the one needing
/// attention, and otherwise the first.
///
/// The first is the fixed order — pull request, then task list, then roadmap —
/// because that is the order the server hands them over in. The pull request
/// leads it as the one of the three with anything on it to answer, so a
/// conversation that has reached one opens on it whether or not it is stopped
/// there.
///
/// Needing attention is the conversation being blocked on the card, which only a
/// pull request can be: what a wrap-up stops for is the review, and a backlog or
/// a roadmap is a list read off the worktree with nothing on it to answer. It
/// still says something where there is more than one pull request: a
/// companion's with feedback waiting on it fronts over the work's own, which is
/// what a reader opening the conversation is being stopped for.
function fronting(conversation: ConversationView): number {
  const at = conversation.pinned.findIndex(
    (event) =>
      "PullRequest" in event && event.PullRequest.id === conversation.blocked_on,
  );

  return at === -1 ? 0 : at;
}

/// What a pinned card is called, in the words its own heading uses.
///
/// A pull request in a companion repo is named with that repository, because a
/// conversation ends on one per repository it was worked in: two dots both
/// reading "Pull request" would be two cards a reader who cannot see them could
/// not tell apart. The work's own stays unlabelled, by the rule the card itself
/// follows.
function named(event: PinnedEvent): string {
  if ("TaskList" in event) {
    return "Task list";
  }
  if ("StageList" in event) {
    return "Roadmap";
  }
  if ("PullRequest" in event && event.PullRequest.repo) {
    return `Pull request in ${event.PullRequest.repo}`;
  }
  return "Pull request";
}

/// One pinned card, whichever of the three kinds it is.
///
/// All three open. A pull request has a full self, which is what is on it right
/// now; a task list opens the documents its entries name and a roadmap the
/// briefs its stages name, which is each list read at a second depth.
///
/// Each of the three is on the record as well, at the moment it arrived there,
/// and the card drawn there is this same card — see the module docs.
function Card(props: {
  event: PinnedEvent;
  selected: Opening | null;
  select: (opening: Opening) => void;
  details: () => void;
}): JSX.Element {
  return (
    <Switch>
      <Match when={"TaskList" in props.event && props.event.TaskList}>
        {(tasks) => (
          <TaskList
            tasks={tasks()}
            selected={props.selected === "backlog"}
            open={() => {
              props.select("backlog");
              props.details();
            }}
          />
        )}
      </Match>
      <Match when={"StageList" in props.event && props.event.StageList}>
        {(stages) => (
          <StageList
            stages={stages()}
            selected={props.selected === opensRoadmap(stages().name)}
            open={() => {
              props.select(opensRoadmap(stages().name));
              props.details();
            }}
          />
        )}
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

/// The pull request the finish step opened: what it is called, its number, how
/// its checks are getting on, and whether it merges into its base at all.
///
/// The last two are the marks on the end of the head line, and neither is ever
/// guessed at: each is drawn only where something has asked GitHub, and the
/// conflict mark goes the moment a fresh reading says the conflict is gone. See
/// `Checks.tsx` and `Merging.tsx`.
///
/// The whole card is the press, as the two lists beside it are: what is *on* it
/// — the commits and the comments — is in the details pane, fetched from GitHub
/// when this is opened, and there is nothing else on the card to press. The way
/// out to GitHub went with the button it used to sit beside: a card with a link
/// on it has two targets and only one of them is the card, so the link lives in
/// the pane instead, which is where it was already written out in full.
///
/// Which repository it was opened in is drawn beside the number, and only where
/// that is not the conversation's own: an unlabelled card means the work's own
/// repo, and the label earns its place when the pinned block holds a companion
/// repo's pull request as well. The rule a commit's label follows — a
/// conversation ends on one pull request per repository it was worked in.
///
/// Drawn twice, from the pinned block and from the record, because it belongs in
/// both. The two are the same Event and so the same selection: opening either
/// opens the one details pane, and both read as selected while it is open.
function PullRequest(props: {
  opened: PullRequestEvent;
  selected: boolean;
  open: () => void;
}): JSX.Element {
  return (
    <Openable
      kind={styles.pullRequest!}
      selected={props.selected}
      open={props.open}
    >
      <div class={styles.eventHead}>
        <h2>Pull request</h2>
        <span class={styles.number}>#{props.opened.number}</span>
        <Show when={props.opened.repo}>
          {(repo) => <span class={styles.repo}>{repo()}</span>}
        </Show>
        {/* On the other end of the line, where every card's second thing sits:
            what is true of the pull request now, beside what it was called when
            it opened. Two marks rather than one — how the checks are, and
            whether it merges at all — and each is drawn only where something
            has asked GitHub about it. */}
        <span class={styles.marks}>
          <Conflict merging={props.opened.merging} />
          <Checks checks={props.opened.checks} />
        </span>
      </div>

      <p class={styles.pullRequestTitle}>{props.opened.title}</p>
    </Openable>
  );
}

/// Whether there is a card to draw at a row of the record.
///
/// True of every kind but the two lists, which are the only Events with no
/// content of their own: what is drawn at one is the worktree read live, and a
/// worktree that has been taken away — which is every closed conversation —
/// leaves the moment on the record with nothing to show for it.
///
/// The row goes with the card rather than standing empty. The record is a
/// column with a rem between its rows, so a row with nothing in it is not
/// nothing: it is two rems of blank paper where the backlog landed, which reads
/// as something missing rather than as something that never had a card.
function drawable(event: TimelineEvent): boolean {
  if ("TaskList" in event) {
    return event.TaskList.list !== null;
  }

  if ("StageList" in event) {
    return event.StageList.roadmaps.length > 0;
  }

  return true;
}

/// The backlog on the record, at the row that says it landed on the branch.
///
/// The same card the pinned block holds, and the same reading behind both — the
/// server hands the one it took over twice. Nothing at all where there is
/// nothing left to read: a worktree that has been taken away leaves the moment
/// on the record with no list to show for it, and [`drawable`] above takes the
/// row with it.
function TaskListRow(props: {
  reached: TaskListReached;
  selected: boolean;
  open: () => void;
}): JSX.Element {
  return (
    <Show when={props.reached.list}>
      {(tasks) => (
        <TaskList tasks={tasks()} selected={props.selected} open={props.open} />
      )}
    </Show>
  );
}

/// And the roadmap on the record, at the row that says it landed.
///
/// Every roadmap the branch wrote to rather than one, because the pinned block
/// holds every one of them too — ordinarily that is exactly one. Which is why
/// the selection travels down here whole rather than as a yes or no: each card
/// opens its own roadmap, and the one that is open is the one that is selected.
function StageListRow(props: {
  reached: StageListReached;
  selected: Opening | null;
  select: (opening: Opening) => void;
  details: () => void;
}): JSX.Element {
  return (
    <For each={props.reached.roadmaps}>
      {(stages) => (
        <StageList
          stages={stages}
          selected={props.selected === opensRoadmap(stages.name)}
          open={() => {
            props.select(opensRoadmap(stages.name));
            props.details();
          }}
        />
      )}
    </For>
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
    <span class={styles.box} aria-hidden="true">
      {props.done ? "☑" : "☐"}
    </span>
  );
}

/// The entries a windowed list is not showing, at the end they are hidden at.
///
/// An ellipsis rather than a count, because what it says is that the list goes
/// on and the card is not the place to read it in — the details pane the card
/// opens is, and it holds every entry. Nothing at all where the list runs to
/// that end already: a row saying none are hidden is a row about nothing.
///
/// The count itself is in words beside the glyph, out of the layout and still
/// in the document, the way a row's own state word is: an ellipsis read aloud
/// says nothing whatever.
function Hidden(props: { count: number }): JSX.Element {
  return (
    <Show when={props.count > 0}>
      <li class={styles.more}>
        <span aria-hidden="true">…</span>
        <span class={styles.state}>{props.count} more</span>
      </li>
    </Show>
  );
}

/// The backlog: where the work has got to in it, and how far through it that
/// is.
///
/// Five entries rather than the whole list — the ones around the task being
/// worked, with an ellipsis wherever the rest of them are. It is the one thing
/// a conversation being built from a backlog is *about*, so it is pinned above
/// the record for the whole of one, and a card that grew with the backlog would
/// push the record out from under itself. The progress line still counts the
/// whole list; see `windowing.ts` for where the five sit.
///
/// It opens, and what it opens is not the list again: each entry names a
/// document in `.tasks/` that says what that task is, and those are what the
/// details pane holds — see `Backlog.tsx`. The whole card is the press, as a
/// document's card is, because there is nothing else on it to press.
///
/// Read out of `.tasks/` in the worktree every time the page reads the
/// conversation, so a task finishing moves this without anybody pressing
/// anything — in the pinned block and at the row on the record where the
/// backlog landed alike, both being drawn from the one reading. Drawn twice and
/// selected once for the reason the pull request beside it is: the two are one
/// backlog, so opening either opens the one details pane and both read as
/// selected while it is open.
function TaskList(props: {
  tasks: TaskListEvent;
  selected: boolean;
  open: () => void;
}): JSX.Element {
  const done = () => props.tasks.tasks.filter((task) => task.done).length;

  // What the card draws: five entries around the one being worked, and the
  // count of what is out of sight at either end. The progress line above them
  // keeps counting the whole list, which is what it is there for.
  const shown = createMemo(() =>
    windowed(props.tasks.tasks, (task) => task.done),
  );

  return (
    <Openable
      kind={styles.taskList!}
      selected={props.selected}
      open={props.open}
    >
      <div class={styles.eventHead}>
        <h2>Task list</h2>
        <Show when={props.tasks.feature !== ""}>
          <span class={styles.feature}>{props.tasks.feature}</span>
        </Show>
        <span class={styles.progress}>
          {done()} of {props.tasks.tasks.length} done
        </span>
      </div>

      <ol class={styles.tasks}>
        <Hidden count={shown().before} />
        <For each={shown().entries}>
          {(task) => (
            <li classList={{ [styles.done!]: task.done }}>
              <Box done={task.done} />
              <span class={styles.what}>{task.title}</span>
              {/* At the far end of the row, where it is out of the way of the
                  reading: what a backlog is scanned for is which titles are
                  left, and a number is what one is quoted by afterwards. */}
              <span class={styles.n}>{task.number}</span>
              {/* The word travels with the row rather than being drawn by the
                  stylesheet, so a list read aloud or copied out still says
                  which tasks are finished. */}
              <span class={styles.state}>{task.done ? "done" : "to do"}</span>
            </li>
          )}
        </For>
        <Hidden count={shown().after} />
      </ol>
    </Openable>
  );
}

/// The roadmap: where the effort has got to in it, and how far through it that
/// is.
///
/// Beside the task list and drawn the same way, windowed to five and all,
/// because it is the same kind of thing one level up — and it is read out of `docs/roadmaps/` in the worktree
/// every time the page reads the conversation, so a stage finishing moves this
/// without anybody pressing anything, in both of the places it is drawn.
///
/// It opens the same way too, and what it opens is not the list again: each
/// entry names a brief beside `ROADMAP.md` that says what that stage is for, and
/// those are what the details pane holds — see `Roadmap.tsx`. The whole card is
/// the press, as a document's card is, because there is nothing else on it to
/// press.
///
/// Which roadmap this is, is the one this branch has written to: a repository
/// keeps its finished roadmaps, and a conversation is about the one it touched.
/// It is also what the card opens *by* — a branch that touched two roadmaps has
/// two cards, and each of them opens its own.
function StageList(props: {
  stages: StageListEvent;
  selected: boolean;
  open: () => void;
}): JSX.Element {
  const done = () => props.stages.stages.filter((stage) => stage.done).length;

  // The same window the task list draws, because it is the same card one level
  // up — see `windowing.ts`.
  const shown = createMemo(() =>
    windowed(props.stages.stages, (stage) => stage.done),
  );

  return (
    <Openable
      kind={styles.stageList!}
      selected={props.selected}
      open={props.open}
    >
      <div class={styles.eventHead}>
        <h2>Roadmap</h2>
        <span class={styles.feature}>
          {props.stages.title || props.stages.name}
        </span>
        <span class={styles.progress}>
          {done()} of {props.stages.stages.length} done
        </span>
      </div>

      <ol class={styles.stages}>
        <Hidden count={shown().before} />
        <For each={shown().entries}>
          {(stage) => (
            <li classList={{ [styles.done!]: stage.done }}>
              <Box done={stage.done} />
              <span class={styles.what}>{stage.title}</span>
              {/* At the far end of the row, as a task's is, and for the reason
                  a task's is. */}
              <span class={styles.n}>{stage.number}</span>
              {/* The word travels with the row rather than being drawn by the
                  stylesheet, for the reason a task's does: a list read aloud
                  or copied out still says which stages are finished. */}
              <span class={styles.state}>{stage.done ? "done" : "to do"}</span>
            </li>
          )}
        </For>
        <Hidden count={shown().after} />
      </ol>
    </Openable>
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
    <Openable
      kind={styles.handoff!}
      selected={props.selected}
      open={props.open}
    >
      <div class={styles.eventHead}>
        <h2>Handoff</h2>
      </div>
      <Clamped class={styles.handoffBody!} html={props.handoff.html} />
    </Openable>
  );
}

/// Something Verkstead did on its own account: the stage it started, where the
/// branch went, a roadmap with nothing left to run — or a stop, which is what
/// stopped the run, why, and what the evidence was.
///
/// A card that opens, like every other event with a full self. It was the one
/// that did not: what a notice had to say was drawn whole where it stood, which
/// works for the line about a stage starting and not at all for a stop — that
/// one is a paragraph and two blocks of terminal output, and a record with a
/// couple of them on it was a record to be scrolled past rather than read. So
/// the card is a line and the whole of it is a press away, which is what the
/// handoff and the instruction beside it already do.
///
/// One line rather than the five a document's card shows, because a notice is
/// not a document: what it has to say is a sentence, and the sentence is what
/// tells one notice from another down a column of them. It is cut with an
/// ellipsis rather than faded, which is the difference between a line that goes
/// on and a document that does.
///
/// It is rendered markdown, because what it names — a branch, a stage, a file
/// the repository records its process in — reads better set apart from the
/// prose around it. Which is also why it is an `Openable`: a link inside a
/// button is not something a browser will have.
///
/// Marked while it is what the conversation is blocked on, which is now the
/// whole of how a stop is found on the record: the status button says the work
/// has stopped and the mark says where. Read off `blocked_on` rather than off
/// being the open one, because the two say different things — a notice being
/// read is not a notice the work stopped at.
function Notice(props: {
  notice: NoticeEvent;
  blocked: boolean;
  selected: boolean;
  open: () => void;
}): JSX.Element {
  return (
    <Openable
      kind={
        props.blocked ? `${styles.notice} ${styles.blocking}` : styles.notice!
      }
      selected={props.selected}
      open={props.open}
    >
      <div class={styles.eventHead}>
        <h2>Notice</h2>
      </div>
      <div
        class={`${styles.noticeBody} markdown`}
        innerHTML={props.notice.html}
      />
    </Openable>
  );
}

/// What somebody asked for by hand, once: the instruction a manual task was set
/// going with.
///
/// Nothing sets another going. A steer into implementing carries the instruction
/// now, and the session it starts drives the conversation rather than standing
/// beside it — so this is a record of something that happened, kept and read
/// rather than rewritten.
///
/// A card like the notice above it, and read the same way: it is what was asked
/// for, in somebody's own words, and there is nothing to open and nothing to
/// answer. It is rendered markdown, because that is what was typed.
///
/// What the session it started went on to do is not drawn here. That arrives as
/// the events any work arrives as — what it printed, what it asked, what it
/// committed — under this one and in the order it happened.
function ManualTask(props: { manual: ManualTaskEvent }): JSX.Element {
  return (
    <div class={`${styles.manualTask} markdown`} innerHTML={props.manual.html} />
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
    <p
      class={styles.moved}
      classList={{ [styles[props.moved.state.toLowerCase()]!]: true }}
    >
      {STATE[props.from]} → {STATE[props.moved.state]}
    </p>
  );
}

/// A steer: the human saying where the work goes.
///
/// A line and not a card, like the move directly under it, and drawn as the pair
/// they are — this says who decided, and the move says what came of it. Which is
/// the whole reason there are two: a timeline of moves alone could never be read
/// back for the difference between the pipeline arriving somewhere and somebody
/// putting it there.
///
/// Named rather than arrowed, unlike the move: where it came *from* is the move
/// above this one and is already on the page, and what a steer adds is the
/// deciding.
///
/// **A card where it carries a document**, which is a steer into implementing
/// that wrote an instruction, or one into follow-up, which always writes a
/// brief: either is what a session was sent off to do, so it is a document like
/// the brief and the handoff and is read the same way — clamped here, whole in
/// the details pane. A steer that carried nothing written stays the line it
/// always was, there being nothing to open.
function Steered(props: {
  steer: SteerEvent;
  selected: boolean;
  open: () => void;
}): JSX.Element {
  const line = () => (
    <p
      class={styles.steered}
      classList={{ [styles[props.steer.target.toLowerCase()]!]: true }}
    >
      You steered this into {STATE[props.steer.target]}
    </p>
  );

  return (
    <Show when={props.steer.html} fallback={line()}>
      {(html) => (
        <Openable
          kind={styles.steeredWith!}
          selected={props.selected}
          open={props.open}
        >
          {line()}
          <Clamped class={styles.steerBody!} html={html()} />
        </Openable>
      )}
    </Show>
  );
}

/// The press that asks for a finished conversation's conflict to be resolved.
///
/// A line and not a card, drawn like the steer beside it and the move directly
/// under it, and read as the same pair: this says who decided, and the move says
/// what came of it.
///
/// **Its own line rather than a steer's**, because the two are not the same act.
/// A steer into wrapping opens the branch to be read again — the review goes
/// with it and the whole of the work is read afresh — and this deliberately
/// leaves the review that carried the work to done standing, asking only that
/// the conflict be resolved. Both are the human sending a finished conversation
/// back into its wrap-up, and a record that drew them the same line could never
/// be read back for which of them happened.
///
/// It says nothing about which pull request, for the reason the steer says
/// nothing about which branch: the cards above it are what a conflict is drawn
/// on, and this is the moment somebody acted on one.
function ConflictsResolved(): JSX.Element {
  return <p class={styles.pressed}>You asked for the conflict to be resolved</p>;
}

/// What a session has printed: who ran it, how much of it there is, and the
/// last thing it said.
///
/// A button, because the whole of it is in the details pane and this is how it
/// is opened — the summary is a line, and a grilling session's Capture is an
/// hour of terminal output nobody wants in the middle pane.
///
/// The head names the run rather than the kind of thing it is. *Agent run* was
/// the same three words over every card on a record that may hold a dozen of
/// them, and what tells one from another is what it was run under — so the head
/// is the shared reading in [`../agents`], off the three facts the session wrote
/// down as it started, with the harness's mark in front of it. A session from
/// before Verkstead wrote any of them down has nothing to be named by, and keeps
/// the words — and no mark, there being no harness to draw one for.
///
/// It moves while the session runs, which is the point: the page hears the world
/// moved and reads this back, so a session that has just asked something says so
/// here rather than at the end of an hour.
function AgentOutput(props: {
  output: AgentOutputEvent;
  /// The Profiles as they stand, which says whether the account's own name is
  /// worth saying — `undefined` until the list has been read.
  saved: ProfileEntry[] | undefined;
  selected: boolean;
  open: () => void;
}): JSX.Element {
  return (
    <CardButton
      class={styles.agentOutput!}
      open={props.selected}
      press={props.open}
    >
      <span class={styles.eventHead}>
        <span class={styles.what}>
          {/* The harness's own mark in front of the words, which is what makes a
              column of these scannable: a reader picks the Claude run out of
              five by its shape before reading a word of any of them. Inside the
              heading rather than beside it, so the two stay together where the
              head wraps. */}
          <HarnessMark of={props.output.agent_type} class={styles.harness!} />
          {reading(ran(props.output), props.saved) || "Agent run"}
        </span>
        {/* How far the conversation has got. A session with no Transcript to
            count has no metric at all rather than a zero: there is nothing here
            that took turns, and a `0 turns` would be a claim about it. */}
        <Show when={props.output.turns !== null}>
          <span class={styles.turns}>
            {props.output.turns} {props.output.turns === 1 ? "turn" : "turns"}
          </span>
        </Show>
        {/* And whether anything is still writing it, at the right edge — the
            same mark a sidebar card says the same thing with. */}
        <Mark
          running={props.output.running}
          idle={props.output.idle}
          class={styles.rowMark}
        />
      </span>
      <span class={styles.latest}>
        <Show
          when={props.output.latest !== ""}
          fallback={<Empty inline>Nothing printed yet.</Empty>}
        >
          {props.output.latest}
        </Show>
      </span>
    </CardButton>
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
/// One still waiting on the human is the one thing on a Timeline asking for
/// something rather than recording it, and it says so in the disc — the same
/// disc, in the same place, that a Conversation waiting on the human wears in
/// the sidebar. It said it three ways at once before: an accent border round
/// the card, the words *waiting on you* in the head, and nothing at the edge
/// where every other mark on this page lives. The border and the words have
/// gone the way the sidebar's did, and for the same reason — one thing said
/// once, in the alphabet the rest of the app already reads.
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
  const locked = () => "LockedUnanswered" in standing();

  /// Whether the Set nobody has answered yet was a Deferred Ask. Read off the
  /// standing, which is where the fact lives while it matters: an answered Set
  /// held the work up or did not, and that is over.
  const deferred = () => {
    const how = standing();
    return "Waiting" in how && how.Waiting === "deferred";
  };

  return (
    <CardButton
      class={styles.questionSet!}
      open={props.selected}
      press={props.open}
    >
      <span class={styles.eventHead}>
        <span class={styles.what}>Question set</span>
        <span class={styles.setTitle}>{props.asked.title}</span>
        <Show when={deferred()}>
          <span class={styles.deferred}>deferred</span>
        </Show>
        <Show when={locked()}>
          <span class={styles.closed}>closed unanswered</span>
        </Show>
        {/* And a Set still waiting says so in the disc the sidebar says it in,
            at the right edge of the title's line — the same mark, in the same
            place, as the session running beside it puts its ring. It carries
            its own words, the card having no label of its own and the badge
            that used to say them having gone with the border. */}
        <Show when={waiting()}>
          <span
            class={`${marks.mark} ${marks.waiting} ${styles.rowMark}`}
            role="img"
            aria-label="waiting on you"
          />
        </Show>
      </span>

      {/* Spans rather than the blocks this reads as, laid out as blocks by the
          stylesheet: everything here is inside a button, and a button holds
          phrasing. */}
      <span class={styles.asked}>
        <For each={props.asked.rows}>
          {(row) => (
            <span
              class={styles.ask}
              classList={{ [styles.nested!]: row.nested }}
            >
              <span class={styles.n}>{row.name}</span>
              <span class={styles.question}>{row.question}</span>
              <span class={styles.answer}>
                <Show
                  when={row.answer !== ""}
                  fallback={
                    <span class={styles.open}>
                      {waiting() ? "—" : "unanswered"}
                    </span>
                  }
                >
                  {row.answer}
                </Show>
              </span>
            </span>
          )}
        </For>
      </span>
    </CardButton>
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
    <CardButton
      class={`${styles.questionSet} ${styles.unreadable}`}
      open={props.selected}
      press={props.open}
    >
      <span class={styles.eventHead}>
        <span class={styles.what}>Question set</span>
        <span class={unreadable.unreadableBadge}>cannot be read</span>
      </span>

      <span class={unreadable.unreadableWhy}>{props.asked.why}</span>
    </CardButton>
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
/// Which repository it landed in is drawn beside the word, and only where that
/// is not the conversation's own: an unlabeled card means the work's own repo,
/// and the label earns its place when a timeline carries the commits of a
/// companion repo as well.
///
/// And whether it is a merge is drawn beside the hash, in that same register. A
/// merge is where a resolution session brought the base branch in and settled
/// its conflicts, and what it carries is the hunks the agent resolved — so its
/// counts and its diff are an ordinary small commit's, and without the word it
/// would read as one. Every other commit is unlabelled, which is the ordinary
/// card, and so is every commit recorded before the record could say.
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
    <CardButton
      class={styles.commit!}
      open={props.selected}
      press={props.open}
    >
      <span class={styles.eventHead}>
        <span class={styles.what}>Commit</span>
        <Show when={props.commit.repo}>
          {(repo) => <span class={styles.repo}>{repo()}</span>}
        </Show>
        <span class={styles.sha}>{props.commit.sha.slice(0, ABBREVIATED)}</span>
        <Show when={props.commit.merge}>
          <span class={styles.merge}>Merge</span>
        </Show>
      </span>

      <span class={styles.subject}>{props.commit.subject}</span>

      <span class={styles.changed}>
        <span class={styles.files}>{files()}</span>
        {/* The signs travel with the numbers rather than being drawn by the
            stylesheet, so a row read aloud or copied out still says which way
            each of them went. */}
        <span class={styles.added}>+{props.commit.insertions}</span>
        <span class={styles.removed}>−{props.commit.deletions}</span>
      </span>

      {/* Under the counts, because the counts are how much moved and this is
          what the moving was for: the eye reads the line, then the size of it,
          then the account. Clamped to CLAMPED_LINES, as a document's card is,
          and by the stylesheet alone — plain prose is lines of one height, so
          there is nothing here for an observer to measure. */}
      <Show when={props.commit.snippet}>
        {(snippet) => <span class={styles.snippet}>{snippet()}</span>}
      </Show>
    </CardButton>
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
              <Note>
                This needs a brief, and every role picked and working.
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
      </div>
    </Show>
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
/// branch, the base commit and the pairings — because setting the work up
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
  /// The Brief's own flag rather than the Conversation's state, because what
  /// froze is the round and a Conversation steered into a second one has a Brief
  /// per round on the same Timeline: the server is what says which round each
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
      kind={styles.brief!}
      selected={props.selected}
      open={frozen() ? props.open : null}
    >
      {/* The heading alone: a field that keeps itself needs no word beside it
          saying so, and a line that changed as fast as this one was read past
          on a card the eye is meant to be typing into. What a save cannot do
          is still said, under the field, in words. */}
      <div class={styles.eventHead}>
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
                <div
                  class={`${styles.briefBody} markdown`}
                  innerHTML={props.brief.html}
                />
              }
            >
              <Clamped class={styles.briefBody!} html={props.brief.html} />
            </Show>
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
          the human was typing is still explained on the card it happened to
          once the card has gone back to being a rendering. */}
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

      {/* Under the brief, and only while the conversation is drafting: the
          branch, the base commit and the pairings all freeze when grilling
          starts, so past that moment there is nothing here that could be
          changed.

          Under the brief being written, which on an adopting conversation is
          one that is frozen: the stage brief is nobody here's to write, and its
          setup is still the human's. */}
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
