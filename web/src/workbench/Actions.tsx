//! What can be done to a Conversation as a whole, rather than to any one event:
//! the rows behind the status button that ends the sticky block over the
//! Conversation pane, and the same rows under the pointer on a card in the
//! sidebar.
//!
//! A menu rather than three buttons, because the last of them throws a worktree
//! away and a pane header is somewhere the human's cursor passes on the way to
//! everything else.
//!
//! Resume is first, because it is the one *go* among them: what the human most
//! often comes here for is a Conversation nothing is driving. It is drawn
//! exactly where the server says it is worth drawing — see `ready_to_resume`,
//! which is the state being one something ought to be driving and nothing
//! driving it. The page cannot work that out for itself: what drives a
//! Conversation is a register of running tasks, and a register lives in the
//! server. It carries nothing, either: what to start is recomputed from where
//! the work now stands at the moment of the press, which is the whole point of
//! one row rather than one per way of stopping — saying something else should
//! happen is what Steer is for.
//!
//! One stop is worth more than that row's standing sentence, and it says so
//! there: a run stopped by an exhausted usage window waits for this press *and*
//! for the account to come back, so the row leads with when that is — see
//! [`resuming`]. The only place it is said, the status button under the pinned
//! cards having one line and the work to say it about.
//!
//! Then, in order of what each costs: stop, which waits for the task the run is
//! on; force stop, which does not; steer, which moves the work somewhere else;
//! and the two closes, which are not a stop at all but the end of the
//! Conversation — one of them taking it off the list on the way out. Each
//! carries what it does *inside* it, under its own name — because *stop* and
//! *force stop* are two words apart and hours of work apart, and because a row
//! that is one press from its name to its last word is one thing to aim at
//! rather than a button with a sentence loose beside it. Each is drawn only
//! where it applies — the two stops need something to stop, which is the
//! server's rule and not this page's — and where close has already been
//! pressed, archive stands in their place.
//!
//! **Sharing is not among them, and was.** Four rows of it were: the download,
//! the publish, where the last one went, and the share to the pull requests.
//! What they had in common is that not one of them does anything to the
//! Conversation, which is what this menu is for — and four rows about handing a
//! copy to somebody made a menu the human had to read past to reach what they
//! came for. They are a pane of their own now, opened by the share
//! icon on the Timeline's header: see `Share.tsx`. What is left here is what
//! moves the work.
//!
//! Stop goes the same way once it has been pressed. A stop waits for the step
//! the run is on to finish, so from the press until it lands there is a
//! decision recorded and nothing more to ask: a row still offering it would
//! answer a second press by doing exactly what the first did. Force stop stays,
//! being the escalation from there rather than the same press again.
//!
//! Every refusal is said in front of the human, and so is a request that fell
//! over on the way out. These presses are not expected to fail in ordinary use —
//! nearly every refusal any of them has is a page drawn against a Conversation
//! that has since moved — and the re-read each press ends with is still the
//! correction: a row that stopped applying stops being drawn. But a press the
//! human made and nothing came of is owed an answer rather than a line in a
//! console nobody has open, so the refusal's own sentence opens as a card over
//! the page and the menu goes on the way — a dropdown left hanging behind a
//! modal is a menu nobody can see to close.
//!
//! Resume is why that stance changed. Its refusals are the whole of what the row
//! is for — a Conversation nothing is driving, and the reason nothing is — and
//! there was never a way to tell those sentences from the rest.
//!
//! **Four of these rows do not wait for the server, and answer differently.**
//! Close, Close and archive, Archive and Unarchive draw their outcome at the
//! press: the menu shuts, the pane and the sidebar read as closed or as put
//! away, and the request runs behind the page — see `eager.ts`, which holds
//! what has been said until a read of the server has landed since. Closing
//! is the one that made this worth doing, taking seconds inside the POST, and
//! the other three joined it so that the menu behaves the one way throughout.
//! A press of theirs that is refused is rolled back and said in a toast rather
//! than in the card above: there is nothing to decide, and a modal over an
//! outcome the human has already watched happen would be the page arguing with
//! itself.
//!
//! The rest do wait, and keep the card. Resume, the two stops and Steer each
//! end in a session actually starting or stopping, which is not something this
//! page can truthfully draw ahead of the server — so they keep the pending
//! label on the row and the refusal over the page.
//!
//! **And two of the four take the page with them.** Archive and Close and
//! archive put the Conversation off the conversations list, and a human who
//! pressed one of them on the Conversation they were reading was left reading
//! it still — on a page nothing in the list points at any more, with the switch
//! at the foot of the sidebar as the only way back to where they had been. So
//! the page goes where the eye already is: to the Conversation **above** the one
//! that has gone, and to the compose page where there was none above it, that
//! being the one thing there is to do from the top of the list. See [`leaving`].
//!
//! It replaces rather than pushes. The human pressed a row rather than
//! navigating, and a Back that landed on the Conversation they had just put away
//! would hand it straight back.
//!
//! And it happens only where the row really goes. With *Show archived* on, an
//! archived Conversation keeps its row and the human keeps their page: the
//! switch is what says they want to go on seeing these. Nor does a press on
//! some other card move anything — the sidebar's menu is very often about a
//! Conversation no pane is showing, and nothing about the page it is over has
//! changed.
//!
//! Two menus and one set of rows. The pane's is about the Conversation that is
//! open, and the sidebar's right-click is about the card under the pointer,
//! which is very often a different one — but *what there is to do about a
//! Conversation* does not depend on which of the two asked. So the presses live
//! in [`actions`], which is a factory rather than a component: it holds one set
//! of mutations and one modal, and hands back the rows to draw and the way to
//! shut whatever menu is drawing them.
//!
//! The sidebar's is a pointer affordance and nothing else. A touch device has no
//! right-click, and a long press on a card there already picks it up to be
//! dragged — so on a phone this menu simply is not there, and the status button
//! on the Conversation is the way to all of it.

import { useNavigate, useParams } from "@solidjs/router";
import { useMutation, useQueryClient } from "@tanstack/solid-query";
import {
  Match,
  Show,
  Switch,
  createSignal,
  createUniqueId,
  type JSX,
} from "solid-js";

import { ContextMenu, Menu } from "../Menu";
import { Modal } from "../Modal";
import {
  archiveConversation,
  closeAndArchiveConversation,
  closeConversation,
  forceStopConversation,
  listConversations,
  loadConversation,
  resume,
  showingArchived,
  steerConversation,
  stopConversation,
  unarchiveConversation,
} from "../api/client";
import type {
  ConversationArchived,
  ConversationClosed,
  ConversationStopped,
  ConversationUnarchived,
  ConversationView,
  Resumed,
  SteerOpened,
} from "../api/types";
import { useReading } from "../freshness";
import { Empty, ErrorLine } from "../notices";
import styles from "./Actions.module.css";
import { eagerly, pressed, pressedRows, rowFor } from "./eager";
import { pathOf } from "./openings";
import { NO_SESSIONS, noSessions } from "./sessions";
import { Steer } from "./Steer";

/// Each way of being refused a stop, whichever of the two was pressed, in the
/// words the human is told them in.
///
/// The two that are not refusals map to nothing: a conversation that has
/// stopped, and one that is still finishing its task, are both the press having
/// landed — and what says so is the read that follows it.
export const STOP_REFUSAL: Record<ConversationStopped, string> = {
  Stopped: "",
  Stopping: "",
  AlreadyStopped:
    "This conversation has already stopped. Resume is what gets it going again.",
  NotDriven:
    "Nothing is supposed to be driving this conversation, so there is nothing to stop.",
  NoSuchConversation: "This conversation is gone.",
};

/// And each way of being refused a close, whichever of the two closing rows was
/// pressed.
///
/// One refusal between them: a conversation that has gone. A worktree the server
/// could not remove is not one of them — it is logged and left behind, and the
/// conversation closes around it.
export const CLOSE_REFUSAL: Record<ConversationClosed, string> = {
  Closed: "",
  AlreadyClosed: "",
  NoSuchConversation: "This conversation is gone.",
};

/// And each way of being refused an archive.
///
/// Both of the ones that say anything are a page drawn against a conversation
/// that has moved: the row is offered on a closed conversation, so a refusal
/// here means it stopped being one between the drawing and the press.
export const ARCHIVE_REFUSAL: Record<ConversationArchived, string> = {
  Archived: "",
  AlreadyArchived: "",
  NotClosed: "This conversation is not closed, so there is nothing to put away.",
  NoSuchConversation: "This conversation is gone.",
};

/// And the one way of being refused the way back out of it. There is no state
/// a conversation can be in that is the wrong one to put on the list again, so
/// the only thing left to refuse is a conversation that has gone.
export const UNARCHIVE_REFUSAL: Record<ConversationUnarchived, string> = {
  Unarchived: "",
  NotArchived: "",
  NoSuchConversation: "This conversation is gone.",
};

/// And each way of being refused a resume.
///
/// Every one of them is the row doing the one thing it is for: saying what there
/// is to do about a conversation nothing is driving. A press that quietly found
/// nothing to start would leave the human exactly as stuck as they were, which
/// is why the server names these rather than logging them — and why the rows
/// around it now say their refusals the same way.
export const RESUME_REFUSAL: Record<Resumed, string> = {
  Resumed: "",
  NoSuchConversation: "This conversation is gone.",
  NotDriven:
    "Nothing is supposed to be driving this conversation, so there is nothing to start again.",
  // The one refusal here that is about this Verkstead rather than about this
  // conversation. The row is not drawn on a build with no session to start —
  // see `sessions.tsx` — so this is what a page drawn before the read answers
  // a press with.
  NotOnWindowsYet: NO_SESSIONS,
  AlreadyDriven:
    "Something is already driving this conversation. Have a look at what it is doing.",
  NowhereToWork:
    "This conversation has no worktree to work in, so there is nowhere to start.",
  WorktreeRefused:
    "This conversation's worktree is broken and git would not make it again from the branch. The server log says why.",
  NoDirection:
    "Nothing on the record says how this work is being built, so there is no run to pick up.",
  NothingToWork:
    "There is no backlog left to work and nothing was ever written on this branch, so there is nothing built here to carry anywhere. Set the next thing going by hand.",
  // Steer rather than the brief, which is where the drafting presses send
  // somebody: a conversation's pairings are fixed when its work starts, so the
  // pickers on the brief are past changing by the time Resume is a row at all.
  // Which is how a conversation arrives here — the agent profile it was set to
  // run under was removed, and steering is how another is picked.
  NoGrillingPairing:
    "The account this conversation grills under is gone. Steer it into Grilling and pick another.",
  NoImplementationPairing:
    "The account this conversation builds under is gone. Steer it into Implementing and pick another.",
  NoFollowUpBrief:
    "Nothing on the record says what this follow-up was opened about. Steer it into Follow-up again with a fresh brief.",
};

/// What the Resume row says under its name: what the press does, and — on the
/// one stop that waits for something the press cannot supply — when the account
/// it was spending comes back.
///
/// The window fact leads, because it is the half the human does not already
/// know: every row here says what its press does, and only this one has anything
/// to say about *when*. Nothing waits on it and nothing counts down to it — no
/// stop resumes itself, so a run stopped by a spent window waits for the same
/// press as every other, and the words are there to say whether pressing now is
/// worth it.
///
/// Here rather than on the status button, where it used to stand: the button
/// says where the *work* is and has one line to say it in, and this row is the
/// press the fact is about. As the session printed it — `3pm` stays `3pm` — the
/// wording being the backend's and the reader's own clock being what it is read
/// against.
export function resuming(conversation: ConversationView): string {
  const standing =
    "Work out what should be running from where the work stands, and start it.";

  return conversation.resets === null
    ? standing
    : `Out of window until ${conversation.resets}. ${standing}`;
}

/// One row of this menu: what the press is called, and under it the sentence
/// saying what pressing it means.
///
/// Both inside the button, which is the whole of what makes it one row: the
/// name and the sentence are one thing to read and one thing to hit, on a phone
/// as much as under a pointer.
///
/// Exported for the escape hatch in `Hatch.tsx`, which draws a row of this menu
/// without the reading the rest of it needs — the row a Conversation whose page
/// will not load is ended by is the same row, and should look and read as one.
export function Action(props: {
  /// Which row this is, for the paint and for the tests that look for it.
  class?: string;
  /// What it reads as, and what it reads as while its press is in flight.
  ///
  /// The eager rows say neither of the last two: their outcome is on the page
  /// before the request goes out, so there is no waiting to report and nothing
  /// to take the row for — a row that disabled itself over a press that has
  /// already happened would be the one thing on the page still waiting.
  label: string;
  pressing?: string;
  /// A node rather than a string: most rows say one sentence, and the one that
  /// publishes says why it could not, with the way to the page that fixes it
  /// inside the sentence — see [`refusal`].
  says: JSX.Element;
  /// Whether that press is in flight, which is also what takes the row.
  working?: boolean;
  press: () => void;
}): JSX.Element {
  return (
    <button
      type="button"
      role="menuitem"
      class={props.class}
      disabled={props.working ?? false}
      onClick={() => props.press()}
    >
      <span class={styles.title}>
        {props.working ? props.pressing : props.label}
      </span>
      <span class={styles.says}>{props.says}</span>
    </button>
  );
}

/// What a press that did nothing is answered with: the refusal's own sentence,
/// under a heading saying that is what it is, with one way out.
///
/// A card over the page rather than a line under the row, because the menu the
/// row was in has gone by the time there is anything to say — and because every
/// one of these sentences is the human being told the world moved under what
/// they were looking at.
///
/// One way out rather than two, which is the whole of what tells this card from
/// a confirm sheet: nothing is being decided here. The press has already been
/// refused, and reading why is all there is left to do.
///
/// Its own component because the two menus below are not the only place a row
/// of this menu is drawn. The escape hatch draws one for the Conversation whose
/// page will not load — the same `Action`, off the same refusal maps — and a
/// press refused one way there and another way here would be two answers to the
/// same press.
export function Refusal(props: {
  /// The sentence, or `null` while there is nothing to answer.
  said: string | null;
  /// Said once it has been read.
  close: () => void;
}): JSX.Element {
  // The heading's own id, for the `aria-labelledby` naming the card by it.
  // Generated rather than written, because more than one of these stands on a
  // page at once — the Conversation pane's and the sidebar's right-click each
  // hold one — and an id is the page's to keep unique.
  const id = createUniqueId();

  return (
    <Modal
      class={styles.refused!}
      open={props.said !== null}
      close={props.close}
      labelledBy={id}
    >
      <p id={id} class={styles.refusedTitle}>
        Nothing happened
      </p>
      <p class={styles.refusedWhy}>{props.said}</p>
      <div class={styles.refusedOut}>
        <button type="button" onClick={() => props.close()}>
          OK
        </button>
      </div>
    </Modal>
  );
}

/// Every press this menu can make, and the rows that make them.
///
/// A factory rather than a component, because the rows are drawn in two menus of
/// two different shapes and everything behind them is the same: one set of
/// mutations, one steer modal, one way to shut whatever is open. A component
/// would have had to be the menu as well as the presses, and there are two
/// menus.
///
/// The Conversation arrives as an accessor rather than a value, and per call
/// rather than once: the sidebar's menu is about whichever card was
/// right-clicked, and the pane's is about a Conversation that is re-read while
/// the menu hangs open — a row that stopped being applicable mid-press should
/// stop being drawn.
function actions(): {
  /// Handed the way to shut the menu the rows are in, as [`Menu`] hands it out.
  closes: (close: () => void) => void;
  /// The rows, for the Conversation given.
  rows: (conversation: () => ConversationView) => JSX.Element;
  /// And what the rows open over the page — the steer form, and the card a
  /// refused press is answered with. Both outlive the menu that opened them.
  modal: () => JSX.Element;
} {
  const queries = useQueryClient();
  const navigate = useNavigate();
  /// Which Conversation the page is about, which is what says whether a press
  /// here is about the page at all. Off the URL rather than handed in: these
  /// rows are drawn beside every page the sidebar stands on.
  const opened = useParams();

  /// The sidebar's own list, read under the key the sidebar reads it under — so
  /// this is the answer already in hand rather than a second fetch of it.
  ///
  /// What it is for is [`leaving`]: the row above the one a press is about is a
  /// fact about the list the human is looking at, and the list is the one thing
  /// on the screen throughout — the sidebar stands beside every Conversation
  /// and beside the settings.
  const conversations = useReading(() => ({
    queryKey: ["conversations"],
    queryFn: listConversations,

    // Merged as the sidebar's is: the list is re-read constantly, and this is
    // the same cache entry.
    freshness: { reconcile: "id" } as const,
  }));

  /// And whether the ones put away are drawn among them, read the same way
  /// under the key the switch at the foot of the pane reads it under.
  ///
  /// The same rule the sidebar filters its own list by, asked for the same
  /// reason: it is what decides whether an archive takes a row off the list at
  /// all, and so whether there is any page for the human to be left on.
  const archived = useReading(() => ({
    queryKey: ["conversations", "archived"],
    queryFn: showingArchived,
    freshness: { reconcile: "id" } as const,
  }));

  /// A press has put this Conversation off the conversations list, so the page
  /// goes if the page was about it.
  ///
  /// Where it goes is where the eye already is: the row **above** the one that
  /// has gone, and the compose page where there was none above it — the top of
  /// the list having nothing over it but the one thing there is to do from
  /// there. The list as the human is looking at it, presses and all, rather
  /// than as the server last sent it: a row taken off a moment ago by another
  /// press of theirs is not somewhere to send them back to.
  ///
  /// Nothing at all in the two cases where nothing has moved under anybody: a
  /// press on some other card, which is most of what the sidebar's menu is for,
  /// and an archive made while the archived ones are shown — that row stays in
  /// the list, and going on reading it is what the switch is for.
  ///
  /// Called before the press writes what it says, because what it reads is the
  /// list with this Conversation still on it. See `eager.ts`.
  const leaving = (conversation: ConversationView): void => {
    if (String(conversation.id) !== opened.id) return;
    if (archived.data ?? false) return;

    const rows = pressedRows(conversations.data ?? [], false);
    const at = rows.findIndex((row) => row.id === conversation.id);
    const above = at > 0 ? rows[at - 1] : undefined;

    // Replacing rather than pushing: the human pressed a row rather than
    // navigating, and a Back that landed on the Conversation they have just put
    // away would hand it straight back.
    navigate(above === undefined ? "/compose" : pathOf(above.id), {
      replace: true,
    });
  };

  /// What the click found, once it has answered, and `null` while the modal is
  /// shut. Held out here rather than in the menu's rows, because the menu's rows
  /// are built when it opens and thrown away when it closes — and the modal
  /// outlives the menu that opened it by design: the press shuts the menu.
  ///
  /// The Conversation is kept beside it as it stood when the press went out,
  /// which is the world the steer was decided in: the press stops the drive, so
  /// nothing the modal is drawn against moves under it.
  const [steering, setSteering] = createSignal<{
    conversation: ConversationView;
    working: boolean;
  } | null>(null);

  /// The sentence a refused press is answered with, and `null` while there is
  /// nothing to answer. Held out here for the reason the steer is: what opens
  /// over the page outlives the menu the press was made in.
  const [refused, setRefused] = createSignal<string | null>(null);

  // The menu's own way to shut, held here because what closes it is the press
  // coming back rather than the press going out.
  let shut = (): void => {};

  /// What a press that did nothing comes to: the menu goes, and the sentence
  /// saying why opens over the page.
  ///
  /// The menu goes first because the card is drawn over it — a dropdown left
  /// hanging behind a modal is a menu nobody can see to close. And because the
  /// row that was pressed goes with it: shutting the menu hands the focus back
  /// to the button it was dropped from, which is then where the card hands it
  /// back to when it is answered. Opened the other way round, the card would
  /// come up over a document body with the focus on it and put it back there.
  const refuse = (sentence: string): void => {
    shut();
    setRefused(sentence);
  };

  /// What every press here leaves behind: a page drawn against a conversation
  /// that has moved. Reading it again is both the correction and, where the
  /// press was refused, the answer — the row goes.
  ///
  /// Awaitable, for the eager rows: what lets go of what one of those said is
  /// the read behind it landing, so this has to be something to wait for rather
  /// than something set going — see `eager.ts`.
  const reread = (): Promise<unknown> =>
    Promise.all([
      queries.invalidateQueries({ queryKey: ["conversation"] }),
      queries.invalidateQueries({ queryKey: ["conversations"] }),
    ]);

  /// Getting Verkstead driving again, which is the one row here that starts
  /// something rather than ending it. Its refusals are the whole of what it is
  /// for, so a press that found nothing to start says so in as many words.
  const start = useMutation(() => ({
    mutationFn: (id: number) => resume(id),
    onSuccess: (outcome: Resumed) => {
      if (RESUME_REFUSAL[outcome]) {
        refuse(RESUME_REFUSAL[outcome]);
      } else {
        shut();
      }

      void reread();
    },
    onError: (error: Error) =>
      refuse(`The conversation could not be resumed: ${error.message}`),
  }));

  /// Both stops answer the same way, so both are pressed the same way: a
  /// refusal opens as a card over the page and the re-read behind it corrects
  /// what was drawn, and anything else is the press having landed.
  const pressing = (stopping: (id: number) => Promise<ConversationStopped>) => ({
    mutationFn: stopping,
    onSuccess: (outcome: ConversationStopped) => {
      if (STOP_REFUSAL[outcome]) {
        refuse(STOP_REFUSAL[outcome]);
      } else {
        shut();
      }

      void reread();
    },
    onError: (error: Error) =>
      refuse(`The conversation could not be stopped: ${error.message}`),
  });

  const stop = useMutation(() => pressing(stopConversation));

  const force = useMutation(() => pressing(forceStopConversation));

  /// Clicking Steer, which is a press before it is a modal: it stops the drive,
  /// so that nothing new is launched while the human composes and the world the
  /// modal is drawn against is the world the submit arrives in.
  ///
  /// The menu shuts on the way through — what opens over it is the modal, and a
  /// dropdown left hanging behind one is a menu nobody can see to close.
  const click = useMutation(() => ({
    mutationFn: (conversation: ConversationView) =>
      steerConversation(conversation.id),
    onSuccess: (outcome: SteerOpened, conversation: ConversationView) => {
      if (outcome === "NoSuchConversation") {
        refuse("This conversation is gone.");
        void reread();
        return;
      }

      // The menu first, as a refusal does it: shutting hands the focus back to
      // the button the press came from, and the modal opening after that is
      // what makes that button where the focus lands again when it is closed.
      shut();
      setSteering({ conversation, working: outcome.Opened.working });

      // The conversation has stopped, whatever the human goes on to decide, so
      // the page behind the modal is already out of date.
      void reread();
    },
    onError: (error: Error) =>
      refuse(
        `The conversation could not be stopped to steer it: ${error.message}`,
      ),
  }));

  /// Both closing rows answer the same way, so both are pressed the same way:
  /// the one that only closes and the one that puts the conversation away as
  /// well are the same press with the same refusal behind it.
  ///
  /// Eager, unlike everything above it. The menu goes and the page reads as
  /// closed at once, and the close — which stops a session, gives back a
  /// worktree per repository and sweeps a directory, all inside the POST —
  /// runs behind it. A refusal rolls that back and says why in a toast. See
  /// `eager.ts`.
  const closing = (
    conversation: ConversationView,
    ending: (id: number) => Promise<ConversationClosed>,
    away: boolean,
  ) => {
    shut();

    // The archiving half takes the row off the list, so it takes the page with
    // it — before the press says so, the list this reads being the one the row
    // is still on. See [`leaving`].
    if (away) leaving(conversation);

    eagerly({
      conversation: conversation.id,
      // The archive is the half the two rows differ by, and it is said here
      // rather than in a second press: the server makes both in one request,
      // which is what stops a dropped connection leaving the pair half made.
      says: away ? { closed: true, archived: true } : { closed: true },
      post: () => ending(conversation.id),
      refusal: (outcome: ConversationClosed) => CLOSE_REFUSAL[outcome],
      fell: (error: Error) =>
        `The conversation could not be closed: ${error.message}`,
      reread,
    });
  };

  /// And putting the closed conversation away, which reads the same way: both
  /// of its refusals are a page drawn against a conversation that has moved,
  /// and both of its successes mean it is off the list.
  ///
  /// Eager for the reason the closes are, rather than for the wait — this one
  /// is a single row written to the store. What it buys is that the menu
  /// behaves the one way throughout: every row here that ends a conversation is
  /// a press and it has happened.
  const archiving = (conversation: ConversationView) => {
    shut();
    leaving(conversation);

    eagerly({
      conversation: conversation.id,
      says: { archived: true },
      post: () => archiveConversation(conversation.id),
      refusal: (outcome: ConversationArchived) => ARCHIVE_REFUSAL[outcome],
      fell: (error: Error) =>
        `The conversation could not be archived: ${error.message}`,
      reread,
    });
  };

  /// And the way back out, which is the same press mirrored: its one refusal is
  /// a conversation that has gone, and either success means it is on the list
  /// again.
  ///
  /// The one press here that has to say what the row *is*. A conversation
  /// unarchived while the archived ones are hidden is one the server's own list
  /// carries no row for, so the row it goes back on the list as is built from
  /// the reading this press was made against — see `rowFor`.
  const unarchiving = (conversation: ConversationView) => {
    shut();

    eagerly({
      conversation: conversation.id,
      says: { archived: false, row: rowFor(conversation) },
      post: () => unarchiveConversation(conversation.id),
      refusal: (outcome: ConversationUnarchived) => UNARCHIVE_REFUSAL[outcome],
      fell: (error: Error) =>
        `The conversation could not be unarchived: ${error.message}`,
      reread,
    });
  };

  return {
    closes: (close) => (shut = close),

    rows: (given) => {
      /// The Conversation as this menu is drawing it: what the server said,
      /// with whatever a press here has already said about it laid over. Which
      /// is what stands Archive where Close was the instant Close is pressed,
      /// rather than a round trip later — see `eager.ts`.
      const conversation = () => pressed(given());

      return (
        <>
          {/* The one standing way to get Verkstead driving again, and the one
              row here that starts something: above the stops because it is the
              *go* among them, and drawn only where the server says there is
              something to start — see `ready_to_resume`. It carries nothing:
              what to start is worked out from where the work now stands at the
              moment of the press.

              What it says is the one thing that differs between two stops: an
              exhausted usage window is the stop that waits for an account as
              well as for this press, and the row is where that is said — see
              [`resuming`]. */}
          {/* And never on a Verkstead with no session to start: what Resume
              works out is which session should be running, so a row offering it
              there is a press that could only be refused. What says so is on
              the conversation — see `sessions.tsx` — and the refusal is still
              mapped above, a page being only as fresh as its last read. */}
          <Show
            when={conversation().ready_to_resume && !noSessions(conversation())}
          >
            <Action
              class={styles.resume}
              label="Resume"
              pressing="Resuming…"
              says={resuming(conversation())}
              working={start.isPending}
              press={() => start.mutate(conversation().id)}
            />
          </Show>

          <Show when={conversation().ready_to_stop}>
            {/* Until the press has been made. A stop waits for the step the run
                is on to finish, and from then on the decision is recorded and
                the run halts the moment it lands — so a row still offering it
                would be asking for a decision Verkstead already has, and a
                second press would do nothing and say nothing. */}
            <Show when={!conversation().stop_asked}>
              <Action
                class={styles.stop}
                label="Stop"
                pressing="Stopping…"
                says="Stop after the current task until you resume."
                working={stop.isPending}
                press={() => stop.mutate(conversation().id)}
              />
            </Show>

            {/* And force stop stays where it is, being the escalation from
                there rather than the same press again: it is what the human
                presses when they turn out not to want to wait for the step
                after all. */}
            <Show when={conversation().working}>
              <Action
                class={styles.forceStop}
                label="Force stop"
                pressing="Stopping…"
                says="End any running task and stop immediately."
                working={force.isPending}
                press={() => force.mutate(conversation().id)}
              />
            </Show>
          </Show>

          {/* Drawn whatever state the conversation is in, unlike everything
              around it: every state is somewhere to steer *from* — a draft
              nothing has run in, a run in flight, work Verkstead has finished
              with — and which states it can be steered *to* is the modal's to
              offer. */}
          <Action
            class={styles.steer}
            label="Steer"
            pressing="Stopping…"
            says="Stop the run and move this conversation somewhere else."
            working={click.isPending}
            press={() => click.mutate(conversation())}
          />

          {/* And where Close was, on a conversation that has already had it:
              the way to put the record out of sight once there is nothing left
              to read on it. Reversible, so there is nothing to confirm — and on
              one already put away it is the unarchive that stands here instead,
              which is that same reversal made.

              None of these four says anything about being pressed, and none of
              them takes itself: the outcome is on the page before the request
              goes out, so there is nothing left here to wait for. */}
          <Show
            when={conversation().state !== "Closed"}
            fallback={
              <Show
                when={conversation().archived}
                fallback={
                  <Action
                    class={styles.archive}
                    label="Archive"
                    says="Take it off the conversations list. Its record stays where it is."
                    press={() => archiving(conversation())}
                  />
                }
              >
                <Action
                  class={styles.unarchive}
                  label="Unarchive"
                  says="Put it back on the conversations list, where it stays."
                  press={() => unarchiving(conversation())}
                />
              </Show>
            }
          >
            <Action
              class={styles.close}
              label="Close conversation"
              says="Permanently end the conversation and delete the worktree. The branch stays where it is."
              press={() => closing(conversation(), closeConversation, false)}
            />

            {/* And the same press with the archive already made, which saves
                coming back to a conversation there is nothing left to read on.
                Under Close rather than over it, because it is Close and more:
                what it adds is the reversible half. */}
            <Action
              class={styles.closeAndArchive}
              label="Close and archive"
              says="The same, and take it off the conversations list. Its record stays where it is."
              press={() =>
                closing(conversation(), closeAndArchiveConversation, true)
              }
            />
          </Show>
        </>
      );
    },

    // Outside the menu, because the press that opens either of these shuts the
    // menu: what the human is looking at from here is one card over the page.
    modal: () => (
      <>
        {/* What a press that did nothing is answered with — see [`Refusal`],
            which the escape hatch draws the same card from. */}
        <Refusal said={refused()} close={() => setRefused(null)} />

        <Show when={steering()}>
          {(opened) => {
            // Read on the way in rather than left as a getter on the modal's
            // props: what the steer is about was settled by the press, and a
            // `Show`'s accessor goes stale the moment the modal is closed —
            // which is precisely when the modal is still tearing its own memos
            // down over what it was drawn against.
            const { conversation, working } = opened();

            return (
              <Steer
                conversation={conversation}
                working={working}
                close={() => setSteering(null)}
              />
            );
          }}
        </Show>
      </>
    ),
  };
}

/// The menu on the Conversation pane: what there is to do about the
/// Conversation that is open.
///
/// The trigger is the caller's, which is the one thing that makes this menu
/// different from every other one on a pane. What drops it is the StatusButton
/// — a button saying where the work stands, at the foot of the sticky block —
/// so the mark and the paint that the menu draws for a ⋯ would both be in the
/// way, and the caller hands in what its trigger reads as and a class to paint
/// it by.
///
/// The class is handed to the anchor *beside* this menu's own, rather than in
/// place of it: what the card the rows come down as looks like belongs with the
/// rows, and it is the same card the sidebar's right-click drops.
export function Actions(props: {
  conversation: ConversationView;
  /// What the button reads as, which is the whole of the caller's half.
  trigger: JSX.Element;
  /// And the caller's class on the anchor, for painting that button. Styled by
  /// whoever passes it, never here.
  class: string;
}): JSX.Element {
  const acts = actions();

  return (
    <>
      <Menu
        class={`${styles.conversationActions!} ${props.class}`}
        name="Conversation actions"
        closer={acts.closes}
        trigger={props.trigger}
      >
        {() => acts.rows(() => props.conversation)}
      </Menu>

      {acts.modal()}
    </>
  );
}

/// And the same menu on a card in the sidebar, opened by a right-click and put
/// where the pointer was.
///
/// It acts on the card that was right-clicked rather than on the Conversation
/// that is open, which is the whole reason it is worth having: the list is where
/// the human is when they want to close, archive or steer something that is not
/// what they are reading.
///
/// One of these for the whole list rather than one per card. What a card carries
/// is a right-click that says which one it was — see `Conversations.tsx` — and
/// the Conversation itself is read here, whole, from the same query the
/// Conversation pane reads: a card is seven fields and the rows need a good deal
/// more than seven, and reading it again is what keeps *the rows the pane would
/// show* a fact rather than a resemblance. The one that is already open costs
/// nothing to read, its answer being in hand.
export function CardActions(props: {
  /// Which card was right-clicked and where the pointer was, or `null` while
  /// nothing is open.
  pointed: { id: number; x: number; y: number } | null;
  /// Said whenever the menu should go.
  close: () => void;
}): JSX.Element {
  const acts = actions();
  acts.closes(() => props.close());

  const conversation = useReading(() => {
    /// Which card this is a read of, taken here rather than asked for inside
    /// the fetch below.
    ///
    /// It is the Conversation pane's own cache entry, so whatever this observer
    /// last said a fetch of it is stands on that entry after this menu has gone
    /// — and a fetch that asked the prop again would be asking one that has
    /// since moved. Which is exactly what a press here does: the menu shuts,
    /// `pointed` goes to `null`, and the re-read every press ends with lands on
    /// this function. Asked live it threw where the id should have been, and
    /// what the human was shown was that failure drawn over the Conversation
    /// they were reading. Read off the key instead, the fetch is what the key
    /// says it is, whoever asks for it and whenever.
    const of = props.pointed === null ? "" : String(props.pointed.id);

    return {
      // The key the Conversation pane reads under, so the open one is already
      // in hand and any other is in hand for the pane that opens it next.
      queryKey: ["conversation", of],
      queryFn: () => loadConversation(of),
      enabled: of !== "",

      // Merged, as the pane's own read of this is: a Nudge landing while the
      // menu is open should not rebuild the row the human is about to press.
      freshness: { reconcile: "id" } as const,
    };
  });

  return (
    <>
      <ContextMenu
        class={styles.conversationActions!}
        name="Conversation actions"
        at={props.pointed}
        close={props.close}
      >
        {() => (
          <Switch fallback={<Empty>Loading…</Empty>}>
            {/* What was read, ahead of a read that failed: a refetch that fell
                over behind rows already in hand is no reason to take the rows
                away. */}
            <Match when={conversation.data}>
              {(view) => acts.rows(view)}
            </Match>
            <Match when={conversation.isError}>
              <ErrorLine class={styles.failure}>
                The conversation could not be read:{" "}
                {conversation.error?.message}
              </ErrorLine>
            </Match>
          </Switch>
        )}
      </ContextMenu>

      {acts.modal()}
    </>
  );
}
