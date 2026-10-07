//! The Log in press on a signed-out stop's Notice (ADR-0022): the run stopped
//! because its account's login is gone, and this opens the same modal the
//! Profile's card does.
//!
//! Only on a Notice the server put a press on, which is a stop that still
//! stands on an account at home on this device. A mirrored Profile's Notice
//! names the device to log in on instead.

import { Show, createSignal, type JSX } from "solid-js";

import { AGENT_NAME } from "../agents";
import type { NoticeLogIn } from "../api/types";
import { LoggingIn } from "../profiles/LoggingIn";
import styles from "./LogIn.module.css";

export function LogIn(props: { press: NoticeLogIn }): JSX.Element {
  const [open, setOpen] = createSignal(false);

  // Read the way the Profile's card reads it: the harness, and the name the
  // human gave the account where they gave it one.
  const called = () =>
    props.press.name
      ? `${AGENT_NAME.Claude} — ${props.press.name}`
      : AGENT_NAME.Claude;

  return (
    <>
      <button type="button" class={styles.logIn} onClick={() => setOpen(true)}>
        Log in
      </button>
      <Show when={open()}>
        <LoggingIn
          profile={props.press.profile}
          called={called()}
          close={() => setOpen(false)}
        />
      </Show>
    </>
  );
}
