//! Turning notifications on for the device in front of you: the switch that says
//! where this device stands, and the one flip that changes it.
//!
//! Per device, like the drafts in `localStorage`: the phone being subscribed
//! says nothing about the laptop. What the switch shows is read out of the
//! browser on every load rather than remembered — see [`look`].
//!
//! **Inside the desktop app it is the app's own setting instead** (ADR-0020,
//! *Amended: notifications are the app's own*). Electron ships without a push
//! service, so a subscribe there is refused however the checks come out; the
//! app hears every notice from its sidecar and shows it itself, and this switch
//! is `notifications` in its settings file, read and written over the bridge
//! with no browser permission and no subscription anywhere in it. A page with
//! no bridge — a browser on the same machine, a phone — is the Web Push switch
//! it always was.

import type { JSX } from "solid-js";
import { createSignal, onMount } from "solid-js";

import { type Bridge, bridge } from "../settings/bridge";
import { Switch } from "../Switch";
import { disable, enable, look, said as saidAbout } from "./browser";
import styles from "./Notifications.module.css";
import type { Standing } from "./standing";
import { flippable, said } from "./standing";

/// What the line under the app's switch says on a desktop that cannot show a
/// notification at all — a Linux session with no notification daemon, chiefly.
/// Nothing is said where it can: a refusal in the OS's own settings is the OS's
/// to lift, and not something the app can see to guess at.
export const UNNOTIFIABLE =
  "This desktop cannot show notifications — there is nothing on it to show them.";

/// The switch on the Settings head: the app's own setting inside the app, and
/// this browser's push subscription everywhere else.
export function Notifications(): JSX.Element {
  const reach = bridge();

  return reach === null ? <Pushed /> : <Apps reach={reach} />;
}

/// The switch inside the desktop app: the app's `notifications` setting.
///
/// Moved by the answer to a set rather than by the press, as every control on
/// the Desktop section is — so a set the app refused is a switch that goes back
/// where it was.
function Apps(props: { reach: Bridge }): JSX.Element {
  // `null` until the app has answered, which is a switch that waits rather
  // than one that guesses.
  const [on, setOn] = createSignal<boolean | null>(null);
  const [notifiable, setNotifiable] = createSignal(true);
  const [wanted, setWanted] = createSignal<boolean | null>(null);

  onMount(() => {
    void props.reach.settings().then((stands) => setOn(stands.notifications));
    void props.reach.notifiable().then(setNotifiable);
  });

  const flip = (asked: boolean) => {
    if (wanted() !== null) {
      return;
    }
    setWanted(asked);

    void props.reach
      .set({ notifications: asked })
      .then((stands) => setOn(stands.notifications))
      .finally(() => setWanted(null));
  };

  return (
    <section class={styles.notifications}>
      <Switch
        label="Push notifications"
        on={wanted() ?? on() ?? false}
        // Not disabled where the desktop cannot show one: the app asks again at
        // every notice, so a daemon started later is one that is used.
        disabled={on() === null || wanted() !== null}
        flip={flip}
      />
      <p class={styles.state} aria-live="polite">
        {notifiable() ? "" : UNNOTIFIABLE}
      </p>
    </section>
  );
}

/// The switch everywhere but the app: where this device stands, and the one
/// flip that changes it.
function Pushed(): JSX.Element {
  const [standing, setStanding] = createSignal<Standing>("unknown");
  // Why a flip did not do what it offered to, in the browser's own words where
  // it had any. Cleared by the next flip: it describes that attempt and no
  // other.
  const [trouble, setTrouble] = createSignal<string | null>(null);
  // What a flip in flight is asking for, and `null` when none is. The
  // destination rather than a bare "busy", because that is what the switch shows
  // while it waits — subscribing is four round trips through the browser and a
  // push service, and a switch that snapped back to where it started for a
  // second of it would read as a flip that failed.
  const [wanted, setWanted] = createSignal<boolean | null>(null);

  // The browser's alone, which is why the control draws `unknown` first and this
  // is what replaces it.
  onMount(() => {
    void look().then(setStanding);
  });

  // Where the switch sits: what is being asked for while something is being
  // asked for, and otherwise where the device actually is. Every standing but
  // `on` reads as off — an unknown device is not a device that is on, and
  // neither is a blocked one.
  const on = () => wanted() ?? standing() === "on";

  // Nothing a flip could do, or nothing until the one in flight is done.
  const waiting = () => wanted() !== null || !flippable(standing());

  const flip = (on: boolean) => {
    if (wanted() !== null) {
      return;
    }
    setWanted(on);
    setTrouble(null);

    void (async () => {
      try {
        setStanding(await (on ? enable() : disable()));
      } catch (error) {
        setTrouble(saidAbout(error));
        // Where the device actually ended up, rather than where the flip meant
        // to leave it: a subscribe that failed halfway is exactly when the
        // switch must not be guessed at.
        setStanding(await look());
      } finally {
        setWanted(null);
      }
    })();
  };

  // The switch says on or off; the line under it says the things a switch cannot
  // — that this browser has none to offer, that permission is a dead end, that a
  // flip failed. It is always in the document, so that a screen reader has a live
  // region to announce into rather than a paragraph appearing beside it; the
  // stylesheet takes an empty one out of the layout.
  return (
    <section class={styles.notifications}>
      <Switch
        label="Push notifications"
        on={on()}
        disabled={waiting()}
        flip={flip}
      />
      <p class={styles.state} aria-live="polite">
        {trouble() ?? said(standing())}
      </p>
    </section>
  );
}
