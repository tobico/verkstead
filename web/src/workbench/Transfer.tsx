//! Moving a Conversation to another device of the cluster: the dialog the
//! actions menu's *Transfer to…* row opens (ADR-0020, *Transfer*).
//!
//! A card over the page rather than a page of its own, on the pattern the
//! close's confirm set: what is being asked is one question with one press
//! behind it, and the Conversation the human pressed on is very often not the
//! one they are reading.
//!
//! **Two things in it.** The device select stage 07 built — see `DeviceSelect`
//! in `Setup.tsx` — listing every device of the cluster but the one this
//! Conversation is already on; and, under it, what the **preflight** found
//! missing on whichever of them was picked.
//!
//! **The preflight is a reading of the far end**, made the moment a device is
//! picked and drawn as it arrives. What it holds is what would stop the move,
//! and **Go** is refused while it holds anything — by name rather than by count,
//! because each finding is an errand somewhere: a repository to clone or open
//! over there, a harness to install over there, or a machine to go and wake.
//! See `crate::preflight` on the server, which is where the questions are asked
//! and which end answers each of them.
//!
//! **No match offers the clone** (ADR-0024, *Clonable repos*). A repository
//! with an origin that the far device has no Repo for reads *clone it there*,
//! as a press: the far device clones it the way its own Repo dropdown would —
//! into the parent **Create repo** remembers there, or through the same path
//! card where none is — and once it lands the preflight is read again, Go
//! unlocking if nothing else is in the way. A refusal is said under that
//! finding, in the far end's words. A repository with no origin is not
//! clonable, and still points at **Open repo** there.
//!
//! **And *unreachable* is never *no match*.** A machine that did not answer is
//! said as that, named, and offers no clone — a device asleep will answer
//! perfectly well tomorrow, where no match is a clone onto a machine that may
//! already have the repository.
//!
//! **And Go writes the move down rather than making it.** The session's turn
//! ends first — whatever is running runs to its own end and nothing is started
//! after it — and the work goes then, so what comes back says the conversation
//! is *going*. The card shuts on it, and the head of the timeline reads
//! *Transferring to* that machine until it lands. See `crate::transfers`.
//!
//! **The press asks the preflight again**, because the reading here was drawn a
//! moment ago and a machine can go to sleep in a moment. Where it comes back
//! holding something, what the card draws is that reading in place of its own —
//! the same sentences, about the world the press arrived in.

import { For, Show, createSignal, createUniqueId, type JSX } from "solid-js";

import { useMutation, useQueryClient } from "@tanstack/solid-query";

import { Modal } from "../Modal";
import { AGENT_NAME, DEFAULT_PROFILE } from "../agents";
import { preflight, transfer } from "../api/client";
import type {
  ClonableRepo,
  ConversationView,
  Lacking,
  PairingRole,
  Preflight,
  Transferring,
} from "../api/types";
import { UNREACHABLE, harnessAbsentOn } from "../broken";
import { useDevices } from "../devices";
import { useReading } from "../freshness";
import { ErrorLine } from "../notices";
import { Reaching, keyOf, useDevice, type Device } from "../reaching";
import { repoParent } from "../remembered";
import { CloneRepo, useCloning } from "../repos/RepoList";
import { DeviceSelect, useConversationTicks } from "./Setup";
import styles from "./Transfer.module.css";

/// What each refusal of the press says, where it is a word rather than a
/// reading.
///
/// Every one of them is a page drawn against a conversation that has moved
/// since — which is what the sentences say, rather than reporting the word the
/// server used.
const REFUSAL: Record<string, string> = {
  NotTransferable:
    "This conversation can't be transferred. A draft moves by the device " +
    "select on its composer, and a closed one has no work left to move.",
  Elsewhere:
    "The work has already moved to another device. Transfer it again from " +
    "there.",
  NoSuchConversation: "This conversation is gone.",
};

/// What each of a Conversation's three roles is called, which is what a missing
/// harness is named against.
///
/// The words the pickers on the setup row are labelled with, because those are
/// the controls the human would go and change — see `PairingPicker` in
/// `Setup.tsx`.
const ROLE_NAME: Record<PairingRole, string> = {
  Grilling: "Grilling",
  Implementation: "Implementation",
  Review: "Review",
};

/// One finding of the preflight, said.
///
/// A node rather than a string for the two that point somewhere. **Open repo**
/// is a press on the far device's own Repo dropdown, and a sentence that named
/// it in the same voice as the rest of the words would be a sentence the human
/// has to find the press in; *clone it there* is a press of its own — see
/// [`CloneThere`] — and `cloned` is what it says once the clone has landed.
function lacking(
  said: Lacking,
  device: string,
  cloned: () => void,
): JSX.Element {
  if (said === "Unreachable") {
    return (
      `${device} is ${UNREACHABLE}, so it can't say what it has. ` +
      "It'll answer once it's awake."
    );
  }

  if ("Repo" in said) {
    const missing = (
      <>
        {said.Repo.companion ? "The companion repo " : "The repo "}
        {said.Repo.name} isn't on {device}.
      </>
    );

    const clonable = said.Repo.clonable;

    if (clonable === null) {
      return (
        <>
          {missing} <strong>Open repo</strong> there to register it.
        </>
      );
    }

    return (
      <>
        {missing} <CloneThere repo={clonable} cloned={cloned} />
      </>
    );
  }

  if ("RepoAuthor" in said) {
    const { name, companion, here, there } = said.RepoAuthor;
    return (
      <>
        {companion ? "The companion repo " : "The repo "}
        {name} works as {there} on {device} and as {here} here. Pick {here} for
        it under <strong>Repos</strong> in Settings on {device} to move the
        work.
      </>
    );
  }

  if ("RepoAuthorUnpicked" in said) {
    return (
      <>
        {said.RepoAuthorUnpicked.companion
          ? "The companion repo "
          : "The repo "}
        {said.RepoAuthorUnpicked.name} has no author picked on {device}. Pick
        one for it under <strong>Repos</strong> in Settings there.
      </>
    );
  }

  return `${ROLE_NAME[said.Harness.role]} is paired with ${
    said.Harness.profile ?? DEFAULT_PROFILE
  }, which runs ${AGENT_NAME[said.Harness.agent_type]}. ${harnessAbsentOn(
    said.Harness.agent_type,
    device,
  )}`;
}

/// *Clone it there*: a no-match finding's press, cloning the repository on the
/// device picked (ADR-0024, *Clonable repos*).
///
/// The Repo dropdown's own clone, run on the far device by the [`Reaching`] the
/// findings are drawn inside: straight into the parent **Create repo**
/// remembers there, or through [`CloneRepo`]'s card where none is. What lands
/// is not picked by anything here — the preflight read again is what notices
/// it, the far end's registry now holding a match.
///
/// A straight clone that is refused says so under the finding, in the far
/// end's words; one from the card says so on the card, where another parent
/// can be picked.
function CloneThere(props: {
  repo: ClonableRepo;
  cloned: () => void;
}): JSX.Element {
  const device = useDevice();
  const cloning = useCloning();

  /// Whether the parent card is up, for a device nothing is remembered on.
  const [asking, setAsking] = createSignal(false);

  /// What a straight clone was refused with, where one was.
  const [refused, setRefused] = createSignal<string | null>(null);

  const clone = () => {
    setRefused(null);

    const parent = repoParent(device());

    if (parent === "") {
      setAsking(true);
      return;
    }

    cloning.mutate(
      { origin: props.repo.origin, parent, author: props.repo.author },
      {
        onSuccess: (landing) => {
          if ("refused" in landing) {
            setRefused(landing.refused);
            return;
          }

          props.cloned();
        },
      },
    );
  };

  return (
    <>
      <button
        type="button"
        class={`${styles.clone!} secondary`}
        disabled={cloning.isPending}
        onClick={clone}
      >
        {cloning.isPending ? "Cloning…" : "Clone it there"}
      </button>

      <Show when={refused()}>
        {(why) => <ErrorLine class={styles.refused}>{why()}</ErrorLine>}
      </Show>
      <Show when={cloning.isError}>
        <ErrorLine class={styles.refused}>
          The repo couldn't be cloned. {cloning.error?.message}
        </ErrorLine>
      </Show>

      <Show when={asking()}>
        <CloneRepo
          repo={props.repo}
          close={() => setAsking(false)}
          landed={() => {
            setAsking(false);
            props.cloned();
          }}
        />
      </Show>
    </>
  );
}

/// The dialog itself, over whichever Conversation the row was pressed on.
///
/// `conversation` is `null` while nothing is being transferred, which is what
/// keeps the whole of it — the select, the pick and the reading behind it — off
/// the page until the row is pressed: a `Modal` builds its contents afresh each
/// time it opens, so the pick starts empty every time rather than holding
/// whatever the last press left.
export function Transfer(props: {
  conversation: ConversationView | null;
  /// The way back, which is what Escape and a press on the backdrop come to as
  /// well: every way out of this card but Go leaves the work where it is.
  close: () => void;
}): JSX.Element {
  // Generated rather than written, for the reason the menu's other two cards'
  // are: two of these stand on a page at once, the Conversation pane's menu and
  // the sidebar's right-click each holding one.
  const id = createUniqueId();

  return (
    <Modal
      class={styles.transferring!}
      open={props.conversation !== null}
      close={props.close}
      labelledBy={id}
    >
      <p id={id} class={styles.title}>
        Transfer to another device
      </p>

      {/* The Conversation as it stood when the row was hit, frozen: the card is
          asking about the world the human pressed in. */}
      <Show when={props.conversation}>
        {(conversation) => (
          <Picking conversation={conversation()} close={props.close} />
        )}
      </Show>
    </Modal>
  );
}

/// The contents, which exist only while the card is up.
///
/// Its own component so that the pick and the reading over it are made when the
/// card opens and thrown away when it shuts — a signal held outside would be
/// last week's choice waiting under the next press.
function Picking(props: {
  conversation: ConversationView;
  close: () => void;
}): JSX.Element {
  /// Which device this Conversation is on, which is the page's own — the menu
  /// acts on the Conversation being read — or the card's own where the press
  /// came from the sidebar. See `reaching.ts`.
  const here = useDevice();

  /// Where the agent may take the work itself, which the card changes without
  /// moving anything.
  const ticks = useConversationTicks(() => props.conversation);

  // This device's own reading of its cluster, for the one thing the select
  // cannot say: which Device Id *this* machine goes by, where the Conversation
  // is on a member and this device is somewhere it could be moved to.
  const devices = useDevices();

  /// What has been picked — `undefined` until something is, and `null` for this
  /// device, which is a device like any other here.
  const [picked, setPicked] = createSignal<Device | undefined>(undefined);

  /// And the same pick as a Device Id, which is what the reading is asked with:
  /// the far end is named in the path, and *this device* has an id like every
  /// other machine once it is the far end.
  const onto = (): string | undefined => {
    const chosen = picked();
    if (chosen === undefined) return undefined;

    return chosen ?? devices.data?.this.device;
  };

  /// What that device lacks, asked of the device the Conversation is on.
  ///
  /// Merged on the device it is about, which is the only thing that says one
  /// answer from another: a Nudge landing while the card is open must not
  /// rebuild the list the human is reading.
  const reading = useReading(() => {
    const target = onto();

    return {
      queryKey: keyOf(
        here(),
        "conversation",
        String(props.conversation.id),
        "preflight",
        target ?? "",
      ),
      queryFn: () => preflight(here(), props.conversation.id, target!),
      enabled: target !== undefined,
      freshness: { reconcile: "device" } as const,
    };
  });

  /// The reading the press came back holding, where it was refused over one —
  /// which is this card's own reading taken again as the press arrived.
  ///
  /// Drawn in place of it while it stands, and cleared the moment another
  /// device is picked: what it is about is the machine that was pressed on.
  const [again, setAgain] = createSignal<Preflight | null>(null);

  /// And what a press that was refused in a word says, or a press that never
  /// reached the machine holding the work.
  const [refused, setRefused] = createSignal<string | null>(null);

  /// What the findings under the select are: the press's own reading where it
  /// brought one back, and otherwise this card's.
  const found = (): Preflight | undefined => again() ?? reading.data;

  /// Whether the work could go: something picked, a reading in, and nothing in
  /// the way.
  ///
  /// A reading that has not landed is not a device that is ready — what a
  /// preflight is for is that Go is never drawn over a question nobody has
  /// answered yet.
  const ready = (): boolean => (found()?.lacks.length ?? -1) === 0;

  const queries = useQueryClient();

  /// A clone a finding offered has landed on the device picked: the preflight
  /// is read again, which is what finds the far end holding a match now and
  /// unlocks Go if nothing else is in the way.
  ///
  /// The press's own reading goes with it, being about a world before the
  /// clone.
  const cloned = () => {
    setAgain(null);
    void queries.invalidateQueries({
      queryKey: keyOf(
        here(),
        "conversation",
        String(props.conversation.id),
        "preflight",
      ),
    });
  };

  /// Pressing Go: the move is written down, and the card shuts on it.
  ///
  /// What is read again afterwards is the conversation and the list, because
  /// both say something new from here — the head of the timeline reads
  /// *Transferring to* that machine, and the row goes on being this device's
  /// until the work has actually gone.
  const going = useMutation(() => ({
    mutationFn: (target: string) =>
      transfer(here(), props.conversation.id, target),

    onSuccess: (outcome: Transferring) => {
      if (typeof outcome === "object" && "Lacking" in outcome) {
        setAgain(outcome.Lacking);
        return;
      }

      if (outcome !== "Transferring") {
        setRefused(REFUSAL[outcome] ?? "The conversation couldn't be moved.");
        return;
      }

      void queries.invalidateQueries({
        queryKey: keyOf(here(), "conversation"),
      });
      void queries.invalidateQueries({ queryKey: ["conversations"] });

      props.close();
    },

    onError: (error: Error) =>
      setRefused(`The conversation couldn't be moved. ${error.message}`),
  }));

  return (
    <>
      <p class={styles.says}>
        The session's turn ends, and the record, the branch, and the working
        changes go over. This device keeps a read-only copy.
      </p>

      <DeviceSelect
        id="transfer-device"
        chosen={picked()}
        // The Conversation is already on one machine of the cluster, and that is
        // the one thing this select is not for: a row offering it would be a
        // press with nothing behind it.
        without={here()}
        nothing="Select"
        // Nothing to correct: what this holds is a choice being made rather than
        // a pick this browser remembers — see `DeviceSelect`.
        remembered={false}
        // And what the last press said goes with the machine it was about: a
        // reading of one device drawn under the name of another would be the
        // card saying something untrue about both.
        pick={(chosen) => {
          setAgain(null);
          setRefused(null);
          setPicked(() => chosen);
        }}
        // The same ticks the draft was given, at the foot of the same rows:
        // which devices the agent may move the work to itself. Ticking one
        // moves nothing — see `Ticks` in `Setup.tsx`.
        ticks={ticks}
      />

      <Show
        when={onto() !== undefined}
        fallback={
          <p class={styles.waiting}>Pick the device to move the work to.</p>
        }
      >
        <Show
          when={!reading.isPending}
          fallback={<p class={styles.waiting}>Checking the device…</p>}
        >
          <Show
            when={found()}
            fallback={
              <ErrorLine class={styles.failure}>
                That device couldn't be checked. {reading.error?.message}
              </ErrorLine>
            }
          >
            {(reading) => (
              <Show
                when={reading().lacks.length > 0}
                fallback={
                  <p class={styles.ready}>
                    {reading().device} has everything this conversation needs.
                  </p>
                }
              >
                {/* By name rather than by count: every one of these is
                    something to go and put right somewhere, and the somewhere
                    is in the sentence. */}
                <ul class={styles.lacks}>
                  {/* Inside the picked device's reaching, so that a clone
                      offered by a finding runs over there — with that
                      device's remembered parent, its token and its git
                      author. */}
                  <Reaching.Provider value={() => picked() ?? null}>
                    <For each={reading().lacks}>
                      {(said) => (
                        <li>{lacking(said, reading().device, cloned)}</li>
                      )}
                    </For>
                  </Reaching.Provider>
                </ul>
              </Show>
            )}
          </Show>
        </Show>
      </Show>

      {/* And a press that was refused in a word, or one that never reached the
          machine holding the work: said under the findings, where the reading's
          own failure is said. */}
      <Show when={refused()}>
        {(said) => <ErrorLine class={styles.failure}>{said()}</ErrorLine>}
      </Show>

      <div class={styles.out}>
        <button
          type="button"
          class={`${styles.secondary!} secondary`}
          onClick={() => props.close()}
        >
          Cancel
        </button>
        {/* What the card is for: the move written down, and the card shut on
            it. Refused while the reading holds anything and while the press is
            in flight — a second press would be a second move asked for. */}
        <button
          type="button"
          class={styles.go}
          disabled={!ready() || going.isPending}
          onClick={() => {
            const target = onto();
            if (target === undefined) return;

            setRefused(null);
            going.mutate(target);
          }}
        >
          {going.isPending ? "Transferring…" : "Transfer"}
        </button>
      </div>
    </>
  );
}
