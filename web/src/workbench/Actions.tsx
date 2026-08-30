//! What can be done to a Conversation as a whole, rather than to any one event:
//! the rows behind the status button at the head of the Conversation pane, and
//! the same rows under the pointer on a card in the sidebar.
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
//! Then, in order of what each costs: share, which costs nothing and is a file
//! to take away rather than a press at all; publish, which puts that file in a
//! gist; share to pull request, which publishes it and says so in front of
//! whoever is reviewing the work; stop, which waits for the task the run is on;
//! force stop, which does not; steer, which moves the work somewhere else; and
//! the two closes, which are not a stop at all but the end of the Conversation
//! — one of them taking it off the list on the way out. Each carries what it
//! does *inside* it, under its own name — because *stop* and *force stop* are
//! two words apart and hours of work apart, and because a row that is one press
//! from its name to its last word is one thing to aim at rather than a button
//! with a sentence loose beside it. Each is drawn only where it applies — the
//! two stops need something to stop, which is the server's rule and not this
//! page's — and where close has already been pressed, archive stands in their
//! place.
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
//! The two that publish are the exception, and it is the reaching outside this
//! machine that makes them one: what GitHub refused is a thing to go and do
//! something about rather than a page out of date, and where a share went is
//! worth being told. Both are said in a **toast** — see `Toasts.tsx` — because a
//! publish that worked hands back a link to reach for, and a card with one way
//! out is a sentence to dismiss rather than something to take up.
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

import { A } from "@solidjs/router";
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
  loadConversation,
  publishShare,
  resume,
  sharePath,
  shareToPullRequests,
  steerConversation,
  stopConversation,
  unarchiveConversation,
} from "../api/client";
import type {
  CommentedOn,
  ConversationArchived,
  ConversationClosed,
  ConversationStopped,
  ConversationUnarchived,
  ConversationView,
  MissedOut,
  Resumed,
  ShareCommented,
  SharePublished,
  SteerOpened,
} from "../api/types";
import { toast } from "../Toasts";
import { useReading } from "../freshness";
import { Empty, ErrorLine } from "../notices";
import { utcStamp } from "../set/when";
import styles from "./Actions.module.css";
import { Steer, onAPullRequest } from "./Steer";

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
  NoGrillingPairing:
    "Choose a grilling profile and model first, on the brief.",
  NoImplementationPairing:
    "Choose an implementation profile and model first, on the brief.",
  NoFollowUpBrief:
    "Nothing on the record says what this follow-up was opened about. Steer it into Follow-up again with a fresh brief.",
};

/// What a publish came back with, as the toast it is said in.
///
/// The one press on this menu whose outcome is *not* a page drawn against a
/// conversation that has moved: publishing writes to GitHub as the token on the
/// settings page, and two of the three ways it can be refused are that token. So
/// each is a sentence and a way to the page it is fixed on, and the one that
/// worked is a sentence and the link it just made.
///
/// **A toast rather than the card the refusals around it open.** An outcome is a
/// moment and a menu row is a drawing of the conversation it is about — and the
/// link a publish just made is a thing to reach for rather than a sentence to
/// dismiss, which is what a card with one way out asks for. See `Toasts.tsx`.
function published(outcome: SharePublished): JSX.Element {
  if (outcome === "NoToken") {
    return (
      <>
        Verkstead has no GitHub token to publish as.{" "}
        <A href="/settings/github">Put one in on the settings page.</A>
      </>
    );
  }

  if (outcome === "NoGistScope") {
    return (
      <>
        The saved GitHub token may not write gists.{" "}
        <A href="/settings/github">
          Re-issue it with the gist scope and save it again.
        </A>
      </>
    );
  }

  if ("Refused" in outcome) {
    return <>GitHub would not take it: {outcome.Refused.why}</>;
  }

  // And the link it just made, because the menu it was pressed from is shut by
  // the time this is read: the row that draws where the last share went is
  // there when the menu is next opened, and this is the moment itself.
  //
  // Through the share viewer, which is what the server composed it as — see
  // `link` in `crates/server/src/sharing.rs`. The gist itself is what was
  // published; a link that draws it as a conversation is what is worth handing
  // to somebody, so that is what this opens and what the human copies.
  return (
    <>
      The share is published.{" "}
      <a href={outcome.Published.share.url} target="_blank" rel="noreferrer">
        Open it.
      </a>
    </>
  );
}

/// And what became of the one-click share, in a toast of its own.
///
/// Three shapes and one sentence each. A share that was never published says
/// what the publish would have said — it is the same write to GitHub under the
/// same token, and the settings page is where two of the three are fixed. A
/// conversation on no pull request is a page drawn against one that has since
/// moved. And a share that went says where it went, naming whatever missed out
/// beside what worked: the file is up either way, and a human told which pull
/// request it never reached can paste the link there themselves.
function commented(outcome: ShareCommented): JSX.Element {
  if (outcome === "NoPullRequest") {
    return <>This conversation is on no pull request.</>;
  }

  if ("NotPublished" in outcome) {
    // In the publish's own words rather than said again here. What it holds is
    // always a refusal — a publish that worked is the other shape of this — so
    // what comes back is the sentence and the way to fix it.
    return published(outcome.NotPublished.why);
  }

  const { on, missed } = outcome.Commented;
  const said: string[] = [];

  if (on.length > 0) {
    said.push(`Commented on ${on.map(named).join(", ")}.`);
  }

  for (const miss of missed) {
    said.push(`Nothing could be said on ${named(miss)}: ${miss.why}`);
  }

  return <>{said.join(" ")}</>;
}

/// What one pull request is called in that sentence: its number, and the
/// repository it is in where that is not the conversation's own.
///
/// The same rule its card draws by — an unlabeled number means the repo the
/// work is in — because a conversation ends on one pull request per repository
/// it was worked in, and `#7` means something else in each of them.
function named(pull: CommentedOn | MissedOut): string {
  return pull.repo ? `#${pull.number} in ${pull.repo}` : `#${pull.number}`;
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
  label: string;
  pressing: string;
  /// A node rather than a string: most rows say one sentence, and the one that
  /// publishes says why it could not, with the way to the page that fixes it
  /// inside the sentence — see [`refusal`].
  says: JSX.Element;
  /// Whether that press is in flight, which is also what takes the row.
  working: boolean;
  press: () => void;
}): JSX.Element {
  return (
    <button
      type="button"
      role="menuitem"
      class={props.class}
      disabled={props.working}
      onClick={() => props.press()}
    >
      <span class={styles.title}>{props.working ? props.pressing : props.label}</span>
      <span class={styles.says}>{props.says}</span>
    </button>
  );
}

/// The one row of this menu that is not a press: a file to take away.
///
/// A link rather than a button that fetches, because that is what a browser is
/// for. The server answers as an attachment and names the file, so the whole of
/// this row is where it points — nothing to hold in memory, nothing to fail
/// half-way with a megabyte in flight, and a right-click that offers *Save link
/// as* like every other download the human has ever made.
///
/// Which is also why it draws no failure, where the presses around it say
/// theirs: what goes wrong with a link is the browser's to say, in the place it
/// already says it.
export function Download(props: {
  /// Which row this is, for the paint and for the tests that look for it.
  class?: string;
  /// What it reads as, and the sentence under the name — as every row here has.
  label: string;
  says: string;
  /// Where the file is.
  href: string;
}): JSX.Element {
  return (
    <a role="menuitem" class={props.class} href={props.href} download="">
      <span class={styles.title}>{props.label}</span>
      <span class={styles.says}>{props.says}</span>
    </a>
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
  const reread = () => {
    void queries.invalidateQueries({ queryKey: ["conversation"] });
    void queries.invalidateQueries({ queryKey: ["conversations"] });
  };

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

      reread();
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

      reread();
    },
    onError: (error: Error) =>
      refuse(`The conversation could not be stopped: ${error.message}`),
  });

  const stop = useMutation(() => pressing(stopConversation));

  const force = useMutation(() => pressing(forceStopConversation));

  /// Publishing the share: the one press here that reaches outside this machine
  /// and the one that costs somebody an account.
  ///
  /// The menu stays open while it is in flight — a publish is a `gh`, a clone
  /// and a push, and the row saying *Publishing…* is the only thing that says
  /// anything is happening — and shuts as the answer arrives, which is where the
  /// toast takes over. See [`published`], and `Toasts.tsx` for why the outcome
  /// does not stay in the row.
  const publish = useMutation(() => ({
    mutationFn: (id: number) => publishShare(id),
    onSuccess: (outcome: SharePublished) => {
      shut();
      toast(() => published(outcome));
      reread();
    },
    onError: (error: Error) => {
      // The transport rather than the answer: a request that never landed has
      // no named outcome, so it is said in GitHub's place.
      shut();
      toast(() => published({ Refused: { why: error.message } }));
    },
  }));

  /// Sharing to the pull requests: the publish above and a comment on every one
  /// of them, which is the press that says something in front of other people.
  ///
  /// The same arrangement, for the same reasons — and one more: what comes back
  /// names each pull request it reached and each it did not, which is more than
  /// a menu row has room for and worth reading after the menu has gone.
  const comment = useMutation(() => ({
    mutationFn: (id: number) => shareToPullRequests(id),
    onSuccess: (outcome: ShareCommented) => {
      shut();
      toast(() => commented(outcome));
      reread();
    },
    onError: (error: Error) => {
      // The transport rather than the answer, exactly as the publish's is: a
      // request that never landed has no named outcome, and the publish is what
      // it would have failed at.
      shut();
      toast(() =>
        commented({ NotPublished: { why: { Refused: { why: error.message } } } }),
      );
    },
  }));

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
        reread();
        return;
      }

      // The menu first, as a refusal does it: shutting hands the focus back to
      // the button the press came from, and the modal opening after that is
      // what makes that button where the focus lands again when it is closed.
      shut();
      setSteering({ conversation, working: outcome.Opened.working });

      // The conversation has stopped, whatever the human goes on to decide, so
      // the page behind the modal is already out of date.
      reread();
    },
    onError: (error: Error) =>
      refuse(
        `The conversation could not be stopped to steer it: ${error.message}`,
      ),
  }));

  /// Both closing rows answer the same way, so both are pressed the same way:
  /// the one that only closes and the one that puts the conversation away as
  /// well are the same press with the same refusal behind it.
  const closing = (ending: (id: number) => Promise<ConversationClosed>) => ({
    mutationFn: ending,
    onSuccess: (outcome: ConversationClosed) => {
      if (CLOSE_REFUSAL[outcome]) {
        refuse(CLOSE_REFUSAL[outcome]);
        return;
      }

      // Closed or already closed: what was asked for holds either way.
      shut();
      reread();
    },
    onError: (error: Error) =>
      refuse(`The conversation could not be closed: ${error.message}`),
  });

  const close = useMutation(() => closing(closeConversation));

  /// The same press with the archive already made, which is one press rather
  /// than two because it is one intention: a conversation the human is finished
  /// with is usually one they are finished looking at.
  const closeAway = useMutation(() => closing(closeAndArchiveConversation));

  /// And putting the closed conversation away, which reads the same way: both
  /// of its refusals are a page drawn against a conversation that has moved,
  /// and both of its successes mean it is off the list.
  const archive = useMutation(() => ({
    mutationFn: (id: number) => archiveConversation(id),
    onSuccess: (outcome: ConversationArchived) => {
      if (ARCHIVE_REFUSAL[outcome]) {
        refuse(ARCHIVE_REFUSAL[outcome]);
        return;
      }

      shut();
      reread();
    },
    onError: (error: Error) =>
      refuse(`The conversation could not be archived: ${error.message}`),
  }));

  /// And the way back out, which is the same press mirrored: its one refusal is
  /// a conversation that has gone, and either success means it is on the list
  /// again.
  const unarchive = useMutation(() => ({
    mutationFn: (id: number) => unarchiveConversation(id),
    onSuccess: (outcome: ConversationUnarchived) => {
      if (UNARCHIVE_REFUSAL[outcome]) {
        refuse(UNARCHIVE_REFUSAL[outcome]);
        return;
      }

      shut();
      reread();
    },
    onError: (error: Error) =>
      refuse(`The conversation could not be unarchived: ${error.message}`),
  }));

  return {
    closes: (close) => (shut = close),

    rows: (conversation) => (
      <>
        {/* The one standing way to get Verkstead driving again, and the one row
            here that starts something: above the stops because it is the *go*
            among them, and drawn only where the server says there is something
            to start — see `ready_to_resume`. It carries nothing: what to start
            is worked out from where the work now stands at the moment of the
            press. */}
        <Show when={conversation().ready_to_resume}>
          <Action
            class={styles.resume}
            label="Resume"
            pressing="Resuming…"
            says="Work out what should be running from where the work stands, and start it."
            working={start.isPending}
            press={() => start.mutate(conversation().id)}
          />
        </Show>

        {/* A copy of the record to send somebody, which is the one row here
            that does nothing to the conversation at all — so it stands above
            everything that costs something. Offered in every state and on every
            conversation there is: a share is the record as it stands, and a
            record stands from the moment there is one. */}
        <Download
          class={styles.share}
          label="Share"
          says="Download the conversation as one file to send."
          href={sharePath(conversation().id)}
        />

        {/* And the same file put where a link reaches it, which is the other
            way to hand it over: a secret gist, published as the token on the
            settings page. Beside the download rather than instead of it — one
            is a file to attach and the other is a link to paste, and which of
            the two a colleague wants is not this menu's to decide. */}
        <Action
          class={styles.publish}
          label={conversation().shared ? "Publish again" : "Publish"}
          pressing="Publishing…"
          says="Publish it as a secret gist and get a link to send."
          working={publish.isPending}
          press={() => publish.mutate(conversation().id)}
        />

        {/* Where the last one went, on a Conversation somebody has published
            one of. A link out rather than a row that does anything: what the
            human came for is the URL, and a share already published is one they
            can send again without publishing a second snapshot.

            The URL is the share viewer's, composed by the server off the gist
            it recorded — so this row upgrades on its own where the viewer moved
            or arrived after the publish, and a share taken before there was one
            still opens as a conversation. See `link` in
            `crates/server/src/sharing.rs`. */}
        <Show when={conversation().shared}>
          {(shared) => (
            <a
              role="menuitem"
              class={styles.published}
              href={shared().url}
              target="_blank"
              rel="noreferrer"
            >
              <span class={styles.title}>Published share</span>
              <span class={styles.says}>
                Taken {utcStamp(shared().at)}. Opens it in the share viewer.
              </span>
            </a>
          )}
        </Show>

        {/* And the whole of it in one press, on a conversation whose work is on
            a pull request: the same publish, and a comment carrying the link
            and what is in the file on every pull request it holds. Under the
            two rows above rather than instead of them — it is the same share,
            and this is the one that says something in front of other people.

            Offered only where there is a pull request to say it on, which is
            what the pinned cards already say: a conversation with none has
            nowhere for this press to go. */}
        <Show when={onAPullRequest(conversation())}>
          <Action
            class={styles.comment}
            label="Share to pull request"
            pressing="Sharing…"
            says="Publish it and comment the link on every pull request."
            working={comment.isPending}
            press={() => comment.mutate(conversation().id)}
          />
        </Show>

        <Show when={conversation().ready_to_stop}>
          {/* Until the press has been made. A stop waits for the step the run
              is on to finish, and from then on the decision is recorded and the
              run halts the moment it lands — so a row still offering it would
              be asking for a decision Verkstead already has, and a second press
              would do nothing and say nothing. */}
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

          {/* And force stop stays where it is, being the escalation from there
              rather than the same press again: it is what the human presses
              when they turn out not to want to wait for the step after all. */}
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

        {/* And where Close was, on a conversation that has already had it: the
            way to put the record out of sight once there is nothing left to
            read on it. Reversible, so there is nothing to confirm — and on one
            already put away it is the unarchive that stands here instead,
            which is that same reversal made. */}
        <Show
          when={conversation().state !== "Closed"}
          fallback={
            <Show
              when={conversation().archived}
              fallback={
                <Action
                  class={styles.archive}
                  label="Archive"
                  pressing="Archiving…"
                  says="Take it off the conversations list. Its record stays where it is."
                  working={archive.isPending}
                  press={() => archive.mutate(conversation().id)}
                />
              }
            >
              <Action
                class={styles.unarchive}
                label="Unarchive"
                pressing="Unarchiving…"
                says="Put it back on the conversations list, where it stays."
                working={unarchive.isPending}
                press={() => unarchive.mutate(conversation().id)}
              />
            </Show>
          }
        >
          <Action
            class={styles.close}
            label="Close conversation"
            pressing="Closing…"
            says="Permanently end the conversation and delete the worktree. The branch stays where it is."
            working={close.isPending}
            press={() => close.mutate(conversation().id)}
          />

          {/* And the same press with the archive already made, which saves
              coming back to a conversation there is nothing left to read on.
              Under Close rather than over it, because it is Close and more:
              what it adds is the reversible half. */}
          <Action
            class={styles.closeAndArchive}
            label="Close and archive"
            pressing="Closing…"
            says="The same, and take it off the conversations list. Its record stays where it is."
            working={closeAway.isPending}
            press={() => closeAway.mutate(conversation().id)}
          />
        </Show>
      </>
    ),

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

/// The menu at the head of the Conversation pane: what there is to do about the
/// Conversation that is open.
///
/// The trigger is the caller's, which is the one thing that makes this menu
/// different from every other one at the head of a pane. What drops it is the
/// StatusButton — a two-line button saying where the work stands — so the mark
/// and the paint that the menu draws for a ⋯ would both be in the way, and the
/// caller hands in what its trigger reads as and a class to paint it by.
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

  const conversation = useReading(() => ({
    // The key the Conversation pane reads under, so the open one is already in
    // hand and any other is in hand for the pane that opens it next.
    queryKey: ["conversation", String(props.pointed?.id ?? "")],
    queryFn: () => loadConversation(String(props.pointed!.id)),
    enabled: props.pointed !== null,

    // Merged, as the pane's own read of this is: a Nudge landing while the menu
    // is open should not rebuild the row the human is about to press.
    freshness: { reconcile: "id" } as const,
  }));

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
