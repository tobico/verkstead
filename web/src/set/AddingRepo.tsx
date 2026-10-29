//! The card a question set's Attach menu opens for a repo: adding a registered
//! one alongside the work, or opening a read-only companion up (ADR-0028).
//!
//! **The steer's rows, on a card.** What it asks is what a ticked row of the
//! steer's companion section asks — read-write or not, off which branch, and
//! under what name — drawn by the same [`CompanionFields`], because it is the
//! same question asked at a third moment. It opens prefilled with the least a
//! human has to say: read-only, off the default branch.
//!
//! **Opening one up is the same card with the mode fixed.** Read-write is the
//! whole of what is being asked for, so there is no switch; the base and the
//! branch name are still the human's to say.
//!
//! **Confirmed is picked.** The card's press hands what it says to the caller,
//! which records it on the set the way a server row's press does. Nothing here
//! reaches the conversation: that is the answer's to do.

import { Show, createSignal, createUniqueId, type JSX } from "solid-js";

import { Modal } from "../Modal";
import type { Pick, RepoEntry } from "../api/types";
import { Note } from "../notices";
import {
  CompanionFields,
  PLAINEST,
  type Addition,
} from "../workbench/Steer";
import styles from "../workbench/AddServer.module.css";
import own from "./AddingRepo.module.css";

/// One opening of the card: the repo it is about, and whether it is being
/// added or opened up.
type Opened = { repo: RepoEntry; opening: boolean };

/// The card, made once by the sheet and drawn beside it — the menu's rows go
/// with the press that opened it, as *Add a server…*'s card outlives them.
export function addingRepo(
  /// What a confirmed card picks.
  pick: (pick: Pick) => void,
  /// What a cleared branch name comes to: the conversation's own branch.
  mirroring: () => string,
): { open: (repo: RepoEntry, opening: boolean) => void; modal: () => JSX.Element } {
  const [up, setUp] = createSignal<Opened | null>(null);

  return {
    open: (repo, opening) => setUp({ repo, opening }),
    modal: () => (
      <Show when={up()}>
        {(opened) => (
          <AddingRepo
            {...opened()}
            mirroring={mirroring()}
            close={() => setUp(null)}
            picked={(picked) => {
              setUp(null);
              pick(picked);
            }}
          />
        )}
      </Show>
    ),
  };
}

function AddingRepo(props: {
  repo: RepoEntry;
  opening: boolean;
  mirroring: string;
  close: () => void;
  picked: (pick: Pick) => void;
}): JSX.Element {
  const id = createUniqueId();

  const [addition, setAddition] = createSignal<Addition>(
    props.opening ? { ...PLAINEST, mode: "ReadWrite" } : PLAINEST,
  );

  /// The card as the record takes it: the empty base is the default branch,
  /// and the empty branch name is mirroring, both sent as nothing.
  const picking = (): Pick => {
    const { mode, base, branch } = addition();
    const base_ref = base.trim() === "" ? null : base;
    const named = branch.trim() === "" ? null : branch.trim();

    return props.opening
      ? { OpenedUp: { repo_id: props.repo.id, base_ref, branch: named } }
      : {
          Companion: {
            repo_id: props.repo.id,
            mode,
            base_ref,
            branch: mode === "ReadWrite" ? named : null,
          },
        };
  };

  const commit = (ev: SubmitEvent) => {
    ev.preventDefault();
    props.picked(picking());
  };

  return (
    <Modal class={styles.adding!} open close={props.close} labelledBy={id}>
      <h3 id={id}>
        {props.opening ? "Open Up" : "Add"} {props.repo.name}
      </h3>

      <form onSubmit={commit}>
        <Note class={styles.note}>
          {props.opening
            ? "Its branch is cut when this question set is answered, and replaces the read-only checkout."
            : "Checked out when this question set is answered, and kept for the rest of the conversation."}
        </Note>

        <CompanionFields
          class={own.fields}
          id={`${id}-companion`}
          repo={props.repo}
          addition={addition()}
          mirroring={props.mirroring}
          opening={props.opening}
          disabled={false}
          settle={setAddition}
        />

        <div class={styles.buttons}>
          <button type="submit">
            {props.opening ? "Open up" : "Add repo"}
          </button>
          {/* Drawn as well as the ways out the modal already has, for the thumb
              — the server card's reason. */}
          <button
            type="button"
            class={`${styles.cancel} secondary`}
            onClick={() => props.close()}
          >
            Cancel
          </button>
        </div>
      </form>
    </Modal>
  );
}
