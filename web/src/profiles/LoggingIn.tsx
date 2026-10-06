//! A Profile's login, run by the server and drawn here (ADR-0022): the address
//! to log in at, and a box for the code that page hands back.
//!
//! **The server runs it, so any device can finish it.** `claude auth login`
//! runs in the Profile's own sandbox on the machine the account is at home on,
//! and what this modal does is show what it printed and hand it what was
//! pasted. The page that hands back the code can be opened on a phone while the
//! modal is open on a laptop, or the other way round.
//!
//! **One login per Profile.** A second device opening the modal joins the one
//! running and is shown the same address — see `crate::logins` on the server.
//! Each modal names itself as it opens, so that the server can tell which
//! device closed it: the login is killed when the last one does.

import {
  Match,
  Show,
  Switch,
  createSignal,
  createUniqueId,
  onCleanup,
  onMount,
} from "solid-js";
import type { JSX } from "solid-js";

import { useMutation, useQueryClient } from "@tanstack/solid-query";

import { Copy } from "../Copy";
import { Modal } from "../Modal";
import {
  closeLogin,
  loadLogin,
  openLogin,
  sendLoginCode,
} from "../api/client";
import type { LoginState } from "../api/types";
import { useReading } from "../freshness";
import styles from "./LoggingIn.module.css";

/// The modal, open over `profile` for as long as it is mounted.
export function LoggingIn(props: {
  profile: number;
  /// What the human asked to be called — the card's own reading.
  called: string;
  close: () => void;
}): JSX.Element {
  const queries = useQueryClient();
  const id = createUniqueId();

  // This modal's own name for itself — see the module note.
  const viewer = crypto.randomUUID();
  const key = () => ["login", props.profile] as const;

  const settled = (state: LoginState) => queries.setQueryData(key(), state);

  const open = useMutation(() => ({
    mutationFn: () => openLogin(props.profile, viewer),
    onSuccess: settled,
  }));

  // Read only once this device's open has been answered: a read racing it
  // could bring back the last login's ending over the one just started.
  const login = useReading(() => ({
    queryKey: key(),
    queryFn: () => loadLogin(props.profile),
    freshness: { reconcile: "state" },
    enabled: open.isSuccess,
  }));

  const send = useMutation(() => ({
    mutationFn: (code: string) => sendLoginCode(props.profile, code),
    onSuccess: settled,
  }));

  onMount(() => open.mutate());

  // Whichever way the modal goes — a press, Escape, the page moving on — the
  // server is told this device is no longer looking.
  onCleanup(() => {
    void closeLogin(props.profile, viewer).catch(() => {});
  });

  const [code, setCode] = createSignal("");

  const state = (): LoginState | null =>
    login.data ?? open.data ?? null;

  const refusal = (): string | null =>
    (open.error ?? send.error)?.message ?? null;

  return (
    <Modal class={styles.login!} open close={props.close} labelledBy={id}>
      <p id={id} class={styles.title}>
        Log in to Claude for {props.called}
      </p>

      <Switch fallback={<p class={styles.said}>Starting Claude…</p>}>
        <Match
          when={((s) => (s?.state === "Waiting" ? s : null))(state())}
        >
          {(waiting) => (
            <form
              class={styles.steps}
              onSubmit={(event) => {
                event.preventDefault();

                if (code().trim() !== "") {
                  send.mutate(code());
                }
              }}
            >
              <p class={styles.said}>
                1. Open this page and log in. It gives you a code.
              </p>
              <p class={styles.address}>
                <a href={waiting().url} target="_blank" rel="noreferrer">
                  {waiting().url}
                </a>
              </p>
              <Copy of={waiting().url} class={styles.copy} />

              <label class={styles.said} for={`${id}-code`}>
                2. Paste the code here.
              </label>
              <input
                id={`${id}-code`}
                class={styles.code}
                autocomplete="off"
                spellcheck={false}
                value={code()}
                onInput={(event) => setCode(event.currentTarget.value)}
              />

              <div class={styles.out}>
                <button
                  type="button"
                  class="secondary"
                  onClick={() => props.close()}
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  disabled={send.isPending || code().trim() === ""}
                >
                  Log in
                </button>
              </div>
            </form>
          )}
        </Match>

        <Match when={state()?.state === "Checking"}>
          <p class={styles.said}>Checking the code…</p>
        </Match>

        <Match when={state()?.state === "LoggedIn"}>
          <p class={styles.said}>
            Logged in. Sessions under this Profile use the new login.
          </p>
          <div class={styles.out}>
            <button type="button" onClick={() => props.close()}>
              Done
            </button>
          </div>
        </Match>

        <Match
          when={((s) => (s?.state === "Failed" ? s : null))(state())}
        >
          {(failed) => (
            <>
              <p class={styles.said}>{failed().reason}</p>
              <div class={styles.out}>
                <button
                  type="button"
                  class="secondary"
                  onClick={() => props.close()}
                >
                  Close
                </button>
                <button
                  type="button"
                  disabled={open.isPending}
                  onClick={() => {
                    setCode("");
                    open.mutate();
                  }}
                >
                  Try again
                </button>
              </div>
            </>
          )}
        </Match>
      </Switch>

      <Show when={refusal()}>
        {(why) => <p class={styles.refused}>{why()}</p>}
      </Show>
    </Modal>
  );
}
