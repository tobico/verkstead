//! What can be done to a Conversation as a whole, rather than to any one event:
//! the rows behind the ⋯ at the head of the Conversation pane, and the same rows
//! under the pointer on a card in the sidebar.
//!
//! A menu rather than three buttons, because the last of them throws a worktree
//! away and a pane header is somewhere the human's cursor passes on the way to
//! everything else.
//!
//! In order of what each costs: stop, which waits for the task the run is on;
//! force stop, which does not; steer, which moves the work somewhere else; and
//! close, which is not a stop at all but the end of the Conversation. Each
//! carries what it does *inside* it, under its own name — because *stop* and
//! *force stop* are two words apart and hours of work apart, and because a row
//! that is one press from its name to its last word is one thing to aim at
//! rather than a button with a sentence loose beside it. Each is drawn only
//! where it applies — the two stops need something to stop, which is the
//! server's rule and not this page's — and where close has already been
//! pressed, archive stands in its place.
//!
//! Nothing here draws a failure. These presses are not expected to fail in
//! ordinary use — every refusal any of them has is a page drawn against a
//! Conversation that has since moved — and the re-read each press ends with is
//! both the correction and the answer: a row that stopped applying stops being
//! drawn. What is left for whoever is debugging is a `console.error`, which is
//! where a thing nobody is meant to see belongs.
//!
//! Two menus and one set of rows. The pane's ⋯ is about the Conversation that is
//! open, and the sidebar's right-click is about the card under the pointer,
//! which is very often a different one — but *what there is to do about a
//! Conversation* does not depend on which of the two asked. So the presses live
//! in [`actions`], which is a factory rather than a component: it holds one set
//! of mutations and one modal, and hands back the rows to draw and the way to
//! shut whatever menu is drawing them.
//!
//! The sidebar's is a pointer affordance and nothing else. A touch device has no
//! right-click, and a long press on a card there already picks it up to be
//! dragged — so on a phone this menu simply is not there, and the ⋯ on the
//! Conversation is the way to all of it.

import { useMutation, useQueryClient } from "@tanstack/solid-query";
import { Match, Show, Switch, createSignal, type JSX } from "solid-js";

import { ContextMenu, Menu } from "../Menu";
import {
  archiveConversation,
  closeConversation,
  forceStopConversation,
  loadConversation,
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
  SteerOpened,
} from "../api/types";
import { useReading } from "../freshness";
import { Empty, ErrorLine } from "../notices";
import styles from "./Actions.module.css";
import { Steer } from "./Steer";

/// Each way of being refused a stop, whichever of the two was pressed, in the
/// words the console is told them in.
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

/// And each way of being refused a close.
export const CLOSE_REFUSAL: Record<ConversationClosed, string> = {
  Closed: "",
  AlreadyClosed: "",
  NoSuchConversation: "This conversation is gone.",
  WorktreeStuck:
    "The worktree could not be removed, so nothing was changed. The server log says why.",
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

/// One row of this menu: what the press is called, and under it the sentence
/// saying what pressing it means.
///
/// Both inside the button, which is the whole of what makes it one row: the
/// name and the sentence are one thing to read and one thing to hit, on a phone
/// as much as under a pointer.
function Action(props: {
  /// Which row this is, for the paint and for the tests that look for it.
  class?: string;
  /// What it reads as, and what it reads as while its press is in flight.
  label: string;
  pressing: string;
  /// The sentence under the name.
  says: string;
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
      <span>{props.working ? props.pressing : props.label}</span>
      <span class={styles.says}>{props.says}</span>
    </button>
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
  /// And what the steer row opens, which outlives the menu that opened it.
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

  // The menu's own way to shut, held here because what closes it is the press
  // coming back rather than the press going out.
  let shut = (): void => {};

  /// What every press here leaves behind: a page drawn against a conversation
  /// that has moved. Reading it again is both the correction and, where the
  /// press was refused, the answer — the row goes.
  const reread = () => {
    void queries.invalidateQueries({ queryKey: ["conversation"] });
    void queries.invalidateQueries({ queryKey: ["conversations"] });
  };

  /// Both stops answer the same way, so both are pressed the same way: a
  /// refusal goes to the console and the menu stays where it is for the re-read
  /// to correct, and anything else is the press having landed.
  const pressing = (stopping: (id: number) => Promise<ConversationStopped>) => ({
    mutationFn: stopping,
    onSuccess: (outcome: ConversationStopped) => {
      if (STOP_REFUSAL[outcome]) {
        console.error(STOP_REFUSAL[outcome]);
      } else {
        shut();
      }

      reread();
    },
    onError: (error: Error) =>
      console.error("The conversation could not be stopped:", error),
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
        console.error("This conversation is gone.");
        reread();
        return;
      }

      setSteering({ conversation, working: outcome.Opened.working });
      shut();

      // The conversation has stopped, whatever the human goes on to decide, so
      // the page behind the modal is already out of date.
      reread();
    },
    onError: (error: Error) =>
      console.error("The conversation could not be stopped to steer it:", error),
  }));

  const close = useMutation(() => ({
    mutationFn: (id: number) => closeConversation(id),
    onSuccess: (outcome: ConversationClosed) => {
      if (CLOSE_REFUSAL[outcome]) {
        console.error(CLOSE_REFUSAL[outcome]);
        return;
      }

      // Closed or already closed: what was asked for holds either way.
      shut();
      reread();
    },
    onError: (error: Error) =>
      console.error("The conversation could not be closed:", error),
  }));

  /// And putting the closed conversation away, which reads the same way: both
  /// of its refusals are a page drawn against a conversation that has moved,
  /// and both of its successes mean it is off the list.
  const archive = useMutation(() => ({
    mutationFn: (id: number) => archiveConversation(id),
    onSuccess: (outcome: ConversationArchived) => {
      if (ARCHIVE_REFUSAL[outcome]) {
        console.error(ARCHIVE_REFUSAL[outcome]);
        return;
      }

      shut();
      reread();
    },
    onError: (error: Error) =>
      console.error("The conversation could not be archived:", error),
  }));

  /// And the way back out, which is the same press mirrored: its one refusal is
  /// a conversation that has gone, and either success means it is on the list
  /// again.
  const unarchive = useMutation(() => ({
    mutationFn: (id: number) => unarchiveConversation(id),
    onSuccess: (outcome: ConversationUnarchived) => {
      if (UNARCHIVE_REFUSAL[outcome]) {
        console.error(UNARCHIVE_REFUSAL[outcome]);
        return;
      }

      shut();
      reread();
    },
    onError: (error: Error) =>
      console.error("The conversation could not be unarchived:", error),
  }));

  return {
    closes: (close) => (shut = close),

    rows: (conversation) => (
      <>
        <Show when={conversation().ready_to_stop}>
          <Action
            class={styles.stop}
            label="Stop"
            pressing="Stopping…"
            says="Stop after the current task until you resume."
            working={stop.isPending}
            press={() => stop.mutate(conversation().id)}
          />

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
        </Show>
      </>
    ),

    // Outside the menu, because the press that opens it shuts the menu: what
    // the human is looking at from here is one card over the page.
    modal: () => (
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
    ),
  };
}

/// The ⋯ at the head of the Conversation pane: what there is to do about the
/// Conversation that is open.
export function Actions(props: { conversation: ConversationView }): JSX.Element {
  const acts = actions();

  return (
    <>
      <Menu
        class={styles.conversationActions!}
        label="Conversation actions"
        name="Conversation actions"
        closer={acts.closes}
        mark
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
