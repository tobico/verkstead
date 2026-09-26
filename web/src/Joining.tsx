//! The question another device asks this one: a modal raised over whatever the
//! human is reading, with **Allow** and **Deny** on it (ADR-0020, *The join*).
//!
//! **It belongs to no page, which is why it is drawn here.** Every other use of
//! the one modal in this app is a page's — a Remove on a Repo, a form on a pane
//! — and each of them is opened by a press the human just made. This one is
//! opened by another machine, while they are reading a Transcript or nothing at
//! all, so it is drawn in the shell every page sits inside, beside the toast
//! layer, which is the one place in the tree something is already drawn over
//! every page and is there once. See `App.tsx`.
//!
//! **What it says is what the other device said about itself**: the name of the
//! machine, the mark for its operating system, the first address it advertised,
//! and the fingerprint of the certificate it is presenting. That last is the
//! whole point of the pair — the asking device's own pending row draws the same
//! string, so two people, one at each screen, are comparing one certificate by
//! eye. The name, the OS and the address are what says *which* machine before
//! anybody bothers with thirty-two bytes of hex.
//!
//! **One at a time.** Two devices may be asking at once; the list is read
//! oldest first and the first of it is what is up. Answering it raises the next,
//! out of the same read — a stack of modals over one another would be two
//! irreversible questions with one Escape between them.
//!
//! **And it goes when the question does, whoever answered it.** The press
//! answers with the list read again, so the workbench that pressed redraws out
//! of that; every other open workbench is Nudged and reads the same list, which
//! is what takes the modal down on the one that did not press. A request nobody
//! answered inside its ten minutes is not in that list either — the server
//! leaves an expired one out — so the modal goes by itself at the moment the
//! question does.

import { Show, createMemo, createUniqueId } from "solid-js";
import type { JSX } from "solid-js";

import { useMutation, useQueryClient } from "@tanstack/solid-query";

import { Icon } from "./Icon";
import styles from "./Joining.module.css";
import { Modal } from "./Modal";
import { allowJoin, denyJoin, loadAsking } from "./api/client";
import type { AskingDevice } from "./api/types";
import { useReading } from "./freshness";
import { osIcon } from "./devices";

/// The modal, where a device is asking, and nothing at all where none is.
///
/// Mounted once in the shell and reading on its own: the Remote access pane may
/// never have been opened, and a question that only arrived when somebody had
/// the settings up would be a question nobody ever saw.
///
/// Merged by the request, which is what says one question from another: a
/// re-read that found a second device asking leaves the card already up alone.
export function Joining(): JSX.Element {
  const asking = useReading(() => ({
    queryKey: ["joins"],
    queryFn: loadAsking,
    freshness: { reconcile: "request" },
  }));

  // The oldest, which is the one this device was asked first. A second is
  // raised by the same read once this one has been answered — see the module
  // note.
  //
  // A read that failed draws nothing rather than saying so: there is nobody
  // being asked anything, and a page cannot put a refusal about a question
  // nobody knows was asked in front of somebody reading a Transcript. The
  // failure is the server's own line in its own log.
  const first = createMemo<AskingDevice | null>(() => asking.data?.[0] ?? null);

  return <Asked by={first()} />;
}

/// The card itself: who is asking, and the two ways out.
///
/// Its own component so that a test can put a device in front of it without a
/// server, the way the Repo list's confirm card is written — and so that what
/// is drawn is built afresh each time the modal opens, over whichever device is
/// asking now.
export function Asked(props: { by: AskingDevice | null }): JSX.Element {
  const queries = useQueryClient();

  // The heading's own id, for the `aria-labelledby` that names the card by it.
  const id = createUniqueId();

  /// What either press comes back with: the list read again.
  ///
  /// Written straight over this query's data rather than invalidating it, the
  /// way every press on the Remote access pane answers — the modal goes, or the
  /// next device asking takes its place, out of the answer rather than out of a
  /// second request. And the Devices list with it, because an Allow has just
  /// written a member into it.
  const settled = (left: AskingDevice[]) => {
    queries.setQueryData(["joins"], left);
    void queries.invalidateQueries({ queryKey: ["devices"] });
  };

  const allow = useMutation(() => ({
    mutationFn: (request: string) => allowJoin(request),
    onSuccess: settled,
  }));

  const deny = useMutation(() => ({
    mutationFn: (request: string) => denyJoin(request),
    onSuccess: settled,
  }));

  /// Whether a press is on its way, which is what disables both: the two are
  /// one decision, and a Deny pressed while an Allow is in flight would be the
  /// human answering twice.
  const pressing = () => allow.isPending || deny.isPending;

  /// The address the human is looking for: the first the device advertised,
  /// which is the one it would rather be reached on — the tailnet name before
  /// the LAN, in the order the list came in. A device on neither answers with
  /// none, which is a machine with a name and a fingerprint and no line under
  /// them.
  const at = () => props.by?.identity.addresses[0];

  return (
    <Modal
      class={styles.asking!}
      open={props.by !== null}
      // Neither Escape nor a press on the backdrop is a way out of this one —
      // see `insist` on the Modal, which exists for this card. Nobody opened
      // it, so there is nowhere for closing it to go back to, and a question
      // pressed away from would run out ten minutes later with nothing on the
      // page ever having said it was asked.
      insist
      // Which leaves nothing for this to be told about: the card is drawn from
      // the list and goes when the list stops carrying the request.
      close={() => {}}
      labelledBy={id}
    >
      <Show when={props.by} keyed>
        {(device) => (
          <>
            <p id={id} class={styles.title}>
              <Icon
                of={osIcon(device.identity.os)}
                label={device.identity.os}
                class={styles.os}
              />{" "}
              {device.identity.name} is asking to link with this device.
            </p>

            <p class={styles.why}>
              It joins this device's cluster, and every device linked to this one
              is told about it. Allow it only if you know the machine below —
              compare the fingerprint with the one shown on its own screen.
            </p>

            <dl class={styles.about}>
              <dt>Operating system</dt>
              <dd>{device.identity.os}</dd>

              <Show when={at()}>
                {(address) => (
                  <>
                    <dt>Address</dt>
                    <dd>{address()}</dd>
                  </>
                )}
              </Show>

              <dt>Fingerprint</dt>
              <dd class={styles.fingerprint}>{device.identity.fingerprint}</dd>
            </dl>

            {/* Both classes, as every confirm pair in the app carries them: the
                global one is the paint, and the module's is what the row stands
                the filled press out of. */}
            <div class={styles.out}>
              <button
                type="button"
                class={`${styles.secondary!} secondary`}
                disabled={pressing()}
                onClick={() => deny.mutate(device.request)}
              >
                Deny
              </button>
              <button
                type="button"
                disabled={pressing()}
                onClick={() => allow.mutate(device.request)}
              >
                Allow
              </button>
            </div>
          </>
        )}
      </Show>
    </Modal>
  );
}
