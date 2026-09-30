//! Moving a saved draft onto another device of the cluster (ADR-0020,
//! *Drafting on a device*).
//!
//! The device select at the head of a draft's setup row is the compose page's
//! own control — one control drawn twice, like everything else in that row — and
//! this is what a pick *does* on this side of it. The compose page moves what it
//! is composing, which is a field of what the device is holding; here there is a
//! Conversation already, so a pick **replays the draft onto the other device and
//! closes the one here**. The human chose that over a select that reads settled:
//! a draft started on the wrong machine is worth moving rather than worth making
//! again.
//!
//! **A pick asks rather than acts**, which is the whole of why there is a card.
//! A Repo id is one Verkstead's own, so the work cannot arrive over there until
//! somebody has said which of that device's repositories it is in — and the move
//! is not a thing to undo, the draft here being closed by it. So the pick opens
//! this card, the card asks the one question the move cannot answer for itself,
//! and the press on it is what happens. Nothing at all happens until that press,
//! which is also what makes the select safe to be corrected by something other
//! than a hand — see [`DeviceSelect`](./Setup.tsx), whose own effect picks this
//! device back when the one it is showing has left the cluster.
//!
//! **The Repo select is the compose page's**, pointed at the picked device: the
//! same control, the same two rows at its foot, so a target with no such
//! repository yet is not a dead end — *Open repo* and *Create repo* reach over
//! there, as they do on the compose page. Which is one `Reaching.Provider`
//! around the card and nothing else, every control inside it already reading
//! which device the page it is drawn on is about.
//!
//! **And it reads settled wherever the Repo picker beside it does**, which is
//! the same question one level up: *which machine* is what *which repository* is
//! a fact about. A branch that has been cut settles both — a later round, steered
//! onto work that is already built, is a checkout and a record of what happened
//! in it, and a move makes a Conversation elsewhere and closes this one. So do
//! the two kinds of draft that adopt, their lists being this device's own.
//!
//! What travels and what does not is [`moveTo`]'s to say, and it says it in one
//! place; what is here is the words that tell the human before they press.

import { useMutation, useQueryClient } from "@tanstack/solid-query";
import { useNavigate } from "@solidjs/router";
import { Show, createSignal, type JSX } from "solid-js";

import type { BriefEvent, ConversationView, DeviceIdentity } from "../api/types";
import { deviceShown, useDevices } from "../devices";
import { Modal } from "../Modal";
import { ErrorLine } from "../notices";
import { Reaching, keyOf, useDevice, type Device } from "../reaching";
import { leaveRefusals, moveTo, type Target } from "./composing";
import styles from "./Moving.module.css";
import { pathOf } from "./openings";
import { DeviceSelect, RepoSelect, useConversationTicks } from "./Setup";

/// The device select on a saved draft's composer, and the move a pick makes.
export function Moving(props: {
  conversation: ConversationView;

  /// The round's Brief, which is one of the three things a move carries. Handed
  /// in rather than dug out of the Timeline: the composer above already knows
  /// which Brief the round is drafting, and a second reading of that would be a
  /// second opinion about it.
  ///
  /// What the *record* holds, which is what the field keeps itself to: it saves
  /// on a pause in the typing and on the way out of the field, and picking a
  /// device is a press somewhere else — so by the time the card below has been
  /// answered, what is here is what was written.
  brief: BriefEvent;
}): JSX.Element {
  // The device this draft is on, which is the device the move is *from*: every
  // call that reads it is addressed this way, and so is the close at the end.
  const device = useDevice();
  const devices = useDevices();

  /// Which device a pick has named, while the card is up — `undefined` where
  /// nothing has been picked and there is no card at all.
  ///
  /// Three states rather than two, `null` being a device like any other here:
  /// a member's draft moved back to the machine this browser opened is a move,
  /// and one this select has to be able to hold.
  const [picked, setPicked] = createSignal<Device | undefined>(undefined);

  const ticks = useConversationTicks(() => props.conversation);

  /// And what that device is, off the membership — the name for the card and
  /// for the words left behind, and `null` while the reading has not landed or
  /// the device has gone.
  const target = (): DeviceIdentity | null => {
    const to = picked();
    return to === undefined ? null : deviceShown(devices.data, to);
  };

  return (
    <>
      {/* Settled wherever the Repo picker beside it is settled, which is the
          same question one level up: *which machine* is what *which repository*
          is a fact about, so anything that settles the one settles the other.

          A branch that has been cut — a later round, steered onto work that is
          already built — settles it for good: a move starts a Conversation over
          there and closes this one, and this one is a checkout and a record of
          work that has happened. And a draft that adopts settles it because the
          list it came off is this device's own: a stage is in the roadmap's own
          repository. */}
      <DeviceSelect
        chosen={device()}
        // Where the Conversation *is*, rather than something this browser
        // remembered: a member unlinked while this page is up is a page that
        // cannot be read any more, and a correction sent up here would open the
        // card below over it — asking for a move nobody made.
        remembered={false}
        disabled={
          props.conversation.worktree !== null ||
          props.conversation.adopting !== null
        }
        pick={(to) => {
          if (to !== device()) setPicked(to);
        }}
        // And where its agent may take the work later, which is the human's
        // to say while drafting — see `Ticks`. Saved as each is touched, like
        // everything else on a draft's setup row.
        ticks={ticks}
      />

      <Show when={target()}>
        {(to) => (
          <MoveSheet
            conversation={props.conversation}
            brief={props.brief}
            from={device()}
            to={{ reaching: picked() ?? null, identity: to() }}
            close={() => setPicked(undefined)}
          />
        )}
      </Show>
    </>
  );
}

/// The card a pick opens: which repo over there, and the press that moves it.
function MoveSheet(props: {
  conversation: ConversationView;
  brief: BriefEvent;
  from: Device;
  to: Target;
  close: () => void;
}): JSX.Element {
  const id = "move-a-draft";

  const queries = useQueryClient();
  const navigate = useNavigate();

  /// Which of the target's Repos the work is in — nothing until it is asked,
  /// which is what the press waits on.
  const [repo, setRepo] = createSignal<number | null>(null);

  /// A Repo picked out of a list read a moment ago and gone by the time the
  /// press landed. Nothing was made, so what was composed stays exactly where
  /// it is and the list is read again.
  const [gone, setGone] = createSignal(false);

  const move = useMutation(() => ({
    mutationFn: () =>
      moveTo(
        props.from,
        props.conversation,
        props.brief.markdown,
        props.to,
        repo()!,
      ),
    onSuccess: (outcome) => {
      if (outcome === "NoSuchRepo") {
        setGone(true);
        void queries.invalidateQueries({
          queryKey: keyOf(props.to.reaching, "repos"),
        });
        return;
      }

      setGone(false);

      // What the replay could not do goes with the navigation, to be said on the
      // draft it is about — against that draft's device as well as its number,
      // ids colliding by construction. Which is also what leaves the old draft
      // open: a move that was refused anywhere closes nothing, so both ends are
      // there to be looked at.
      leaveRefusals(props.to.reaching, outcome.conversation, outcome.refused);

      // The sidebar is one list merged from the whole cluster and read off this
      // device whoever owns the rows in it, so the key is this device's own
      // however far away the work went — and both ends of the move moved.
      void queries.invalidateQueries({ queryKey: ["conversations"] });
      void queries.invalidateQueries({
        queryKey: keyOf(props.from, "conversation"),
      });

      navigate(pathOf(outcome.conversation, props.to.reaching));
    },
  }));

  return (
    <Modal
      class={styles.moving!}
      open
      close={props.close}
      labelledBy={id}
    >
      <p id={id} class={styles.title}>
        Move this draft to {props.to.identity.name}?
      </p>

      <p class={styles.why}>
        The brief, the branch name, the base and the files go with it, and this
        draft is closed once they have. The repos this one works alongside are
        left behind, and the pairings arrive as that machine remembers them.
      </p>

      {/* The one question the move cannot answer for itself, asked with the
          compose page's own control pointed at the picked device — its two rows
          reaching over there as well, so a machine with no such repository yet
          is not a dead end. */}
      <Reaching.Provider value={() => props.to.reaching}>
        <div class={styles.repo}>
          <RepoSelect
            chosen={repo() === null ? "" : String(repo())}
            disabled={move.isPending}
            pick={(repoId) => {
              setGone(false);
              setRepo(repoId);
            }}
          />
        </div>
      </Reaching.Provider>

      <Show when={gone()}>
        <ErrorLine class={styles.failure}>
          That repo is not registered on {props.to.identity.name} any more, so
          nothing was moved.
        </ErrorLine>
      </Show>
      <Show when={move.isError}>
        <ErrorLine class={styles.failure}>
          The draft could not be moved: {move.error?.message}
        </ErrorLine>
      </Show>

      <div class={styles.out}>
        {/* Both classes, as every confirm pair in the app carries them: the
            global one is the paint, and the module's is what the row stands the
            filled press out of. */}
        <button
          type="button"
          class={`${styles.secondary!} secondary`}
          disabled={move.isPending}
          onClick={() => props.close()}
        >
          Keep it here
        </button>
        <button
          type="button"
          disabled={repo() === null || move.isPending}
          onClick={() => move.mutate()}
        >
          {move.isPending ? "Moving…" : "Move"}
        </button>
      </div>
    </Modal>
  );
}
