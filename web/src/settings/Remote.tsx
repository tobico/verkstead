//! Remote access, on the settings page: whether this machine can be reached
//! from a phone, and what stands between it and being.
//!
//! The Workbench Key is what makes this a section rather than a line in the
//! adoption docs (ADR-0015). A phone cannot reach the workbench until it holds
//! the key, and the address it would hold one against only exists once
//! `tailscale serve` is putting this machine's tailnet name in front of the
//! port Verkstead is listening on — which was a command somebody ran by hand.
//!
//! **Everything here is read rather than configured.** There is no saved half:
//! what the card and the pane draw is what two commands said a moment ago, so a
//! tailnet joined in a terminal and a serve set up by hand read here exactly as
//! ones set up from this page would. Which is why this section reads nothing of
//! the settings query every other section on the page shares — there is nothing
//! of it in either file.
//!
//! **Four things to say, because each wants something different done about it.**
//! No `tailscale` at all is an install, and nothing on this page can do it. The
//! binary with no daemon answering is a `tailscale up`, said in the machine's
//! own words because the line it prints names the service to start. Up is the
//! node's name and whether the workbench is served to the tailnet. And an
//! answer this build could not read is none of the three: Tailscale is whatever
//! the host has, so a shape nobody here has seen says so rather than being read
//! as the nearest state with room for it. The card says which of the four this
//! machine is, in a line; the pane draws only the ones with something to be
//! done about them.
//!
//! **And one thing to press**, which is the serve itself: the checkbox every
//! other section of this page is written in — see [`Check`]. Its position is
//! the reading rather than anything this page remembers, so a serve somebody set
//! up in a terminal reads as on and the box unticks *that*; a press answers with
//! the machine read again, and the box settles wherever the machine actually
//! ended up. It is drawn only where the serve state was readable, because a box
//! over *cannot tell* would be offering to turn on something that may already be
//! on — that state draws what Tailscale said instead.
//!
//! **And it will not lock the page out.** A browser that reached this pane over
//! the served address is here *because* the serve is on, and unticking the box
//! from there would take away the connection carrying the request. So the box is
//! disabled with a tooltip saying why whenever the page's own hostname is the
//! served address's — see [`arrivedOver`]. The client settles that on its own,
//! out of the address the reading already carries: the server sees the tailnet
//! and the loopback arrive on one port and could not tell them apart. From
//! localhost, or from the desktop app, the box unticks as it always did.
//!
//! **The operator grant is a sentence, not a button.** Tailscale refuses a
//! serve from a process that is neither root nor the tailnet's operator, and the
//! server has no privilege to raise. So a refused press draws the line that
//! lifts it — for this machine's own user, as it is to be typed — and the next
//! press is the re-try.
//!
//! **And the address by itself lets nobody in.** The workbench answers 401
//! without the Workbench Key, so what a phone is actually pointed at is the
//! login link — the address with the key on it — which is drawn here as a QR
//! code and offered to copy beside it. The code is drawn in the browser from an
//! encoder the viewer ships: a workbench standing behind a secret has no
//! business handing that secret to a third party to render, and an install on a
//! tailnet may have nowhere to fetch from — see [`Qr`].
//!
//! **Reset key stands under all of it**, because the key is not Tailscale's. It
//! gates a machine that has never heard of a tailnet exactly as it gates one
//! serving on it, and the daemon prints it in the startup line wherever it is
//! running — so it stands on every state of this pane rather than under the
//! code, and turning the serve off does not take away the press that would take
//! a link back. Re-issuing logs every other device out: the QR and the link
//! redraw on the new one out of the answer, and whatever was holding the old one
//! meets a 401 on its next request. The browser that pressed it stays in — a
//! reset made from the phone on the tailnet is a reset made from the only device
//! that could reach this server at all.
//!
//! Nothing about it is confirmed twice. This is the human's own machine and the
//! sentence beside the press is what says what it costs, the way the sandbox
//! binds say what widening one costs: a press somebody has to acknowledge twice
//! is one they stop reading.
//!
//! **Nothing here explains itself beyond that.** The two subheadings, the
//! readings of node and served address, and every note about what a tailnet is
//! and what a login link is worth have gone: what a phone opening this pane
//! needs is the box, the code, the link and the press. What stays under each
//! control is the one line that is the control's own — and, where the machine is
//! in a state somebody has to act on, what the machine said about it.
//!
//! **And Devices is a third section here rather than a pane of its own**
//! (ADR-0020). Linking two machines is how this one is reached as much as the
//! serve and the key are, so it is a section of this pane: nothing is added to
//! `WORDS`, there is no card and no route. It stands beside [`TheKey`] rather
//! than inside the Tailscale choice, because a machine that has never heard of
//! a tailnet has a device identity all the same — and a Devices list that went
//! away on such a machine would be a cluster feature that appeared to need
//! Tailscale.
//!
//! It reads nothing of the settings query either, for the same reason the two
//! sections above it do not: a device is not configured. The name is the
//! hostname, the word for the OS is the platform's own — *Linux (WSL)* where
//! the kernel says so, which is the one case a hostname cannot tell apart — and
//! the addresses are read off the machine's interfaces at the moment it
//! answers. The list holds this device and a row apiece for the devices linked
//! to it, drawn the same way — what is read off this machine now is what a
//! member last said of its own.
//!
//! **Every member's row carries an Unlink and this device's row does not**,
//! there being nothing to unlink this machine from itself. It is the second of
//! the two departures this pane makes from *nothing is confirmed twice* — Add
//! is the first — and it departs for the reason Remove on a Repo does: it
//! cannot be taken back, so it is asked once, over the page, naming the device
//! — see [`Confirm`]. What it does is not cut this device's own half of a link
//! but take the device out of the cluster for everybody, which is what the
//! card says in the sentence under the name.
//!
//! **A member the last dial could not reach is the same row, dimmed, reading
//! *unreachable*.** It is not taken off the list and nothing about it is left
//! out: a machine with its lid shut is still one of this cluster, and what the
//! row says is that a press on it would find nobody there. Which is this
//! device's own finding rather than anything the far end said, so it arrives
//! beside the identity rather than in it — and its Unlink is drawn and works
//! exactly as a live row's, the machine that is never coming back being most of
//! what the press is for.
//!
//! **And under those rows, the devices nobody has typed an address for** — see
//! [`Discovered`], which is the list a browse of the LAN and a probe of this
//! machine's tailnet fill between them. A row there is what another Verkstead
//! said about itself, over mDNS or when it was asked: its name, the mark for its
//! OS, the addresses it was found at, and where it was found — *LAN*,
//! *Tailscale*, or both words where both halves found it, one machine being one
//! row. So two machines are linked with nothing known about either one's address,
//! whether they share a network or only a tailnet.
//!
//! **A reading of its own rather than a field of the one above**, and that is
//! what keeps the two apart: a browse hears something every few seconds, and the
//! cluster's own rows are not re-read for any of it — see [`useDiscovered`]. Its
//! own kind of Nudge carries it, so a device turning up draws a row without a
//! reload and without a poll; and because a browse is cold when it starts, the
//! first answer is empty however many machines are out there. Which is why the
//! empty list reads as one still listening rather than as a network with nothing
//! on it.
//!
//! Members are not in it, nor is this device, nor is a device a press has already
//! been made on — the server leaves all three out, so a device moves from that
//! list to these rows rather than being drawn in both.
//!
//! **And a press on one of those rows is a Join with nothing typed.** It names
//! the device rather than one of its addresses: the row holds every place this
//! machine found that device, and the server works down them in the order it
//! found them. What it leaves is what the box below leaves — a pending row with a
//! fingerprint on it for two people to compare — and the row it was pressed on
//! has gone with it. A row that had gone stale is refused in the words the dial
//! put it in, naming the device, and dropped from the list for good measure: the
//! browse hears a device that is really there again within the minute.
//!
//! **And under both lists, the other control on this pane that configures rather
//! than reads**: Add, against an address somebody types — which is what covers
//! the devices a browse cannot reach, a Windows machine and the WSL on it among
//! them — see [`Add`]. Every other thing here is the machine said back, so the
//! box is the departure the Unlink above makes beside it and Remove on a Repo
//! made before either. A port is optional: every device answers on the peer port
//! unless its host was told another.
//!
//! What a press leaves is a pending row — see [`Waiting`] — reading *waiting for
//! confirmation on* the device that answered, with **this device's own
//! fingerprint** under it and a Cancel. This device's rather than the far end's,
//! deliberately: the modal over there draws the same string, and the pair exists
//! for one person reading a phone and another reading a screen to compare by
//! eye. Nothing has been agreed while that row is drawn, so it is not a member
//! and does not count as one. A request nobody answered inside ten minutes reads
//! expired, and one the far end came back and said no to reads refused; both are
//! dismissed by the same press that cancels a live one. What ends a row the
//! other way is the far end saying yes, which arrives as a member and takes the
//! pending row with it.

import { useMutation, useQueryClient } from "@tanstack/solid-query";
import {
  For,
  Match,
  Show,
  Switch as Choose,
  createSignal,
  createUniqueId,
  type JSX,
} from "solid-js";

import { faHourglassHalf } from "@fortawesome/free-solid-svg-icons";

import { CardButton } from "../CardButton";
import { Check } from "../Check";
import { Copy } from "../Copy";
import { Icon } from "../Icon";
import { Modal } from "../Modal";
import { PaneSticky } from "../Panes";
import {
  addDevice,
  addFound,
  cancelJoin,
  loadDevices,
  loadDiscovered,
  loadRemote,
  pressServe,
  resetKey,
  unlinkDevice,
} from "../api/client";
import type {
  DeviceIdentity,
  DevicesView,
  DiscoveredDevice,
  LinkedDevice,
  PendingJoin,
  RemoteView,
  ServePress,
  ServeView,
} from "../api/types";
import { osIcon } from "../devices";
import { useReading } from "../freshness";
import { Empty, ErrorLine, Note } from "../notices";
import { PaneHead } from "../workbench/PaneHead";
import { Qr } from "./Qr";
import styles from "./Remote.module.css";

/// Where a machine with no Tailscale on it gets one. The one thing this section
/// has to say that nothing on this page can do.
const DOWNLOAD = "https://tailscale.com/download";

/// Why the box will not untick on a page that arrived over the tailnet.
///
/// The browser's own tooltip, because there is nowhere on a cut-down pane to put
/// a sentence that is only true on some of the devices reading it — and a
/// disabled box that gave no reason is one somebody presses twice and then goes
/// looking for the bug in.
const LOCKED =
  "This page was opened over the tailnet, so turning remote access off " +
  "here would disconnect it. Turn it off on the machine Verkstead runs on.";

/// The reading narrowed to one of its states, or `null` where it is in another.
///
/// Written once each rather than inline at every `when`, because that is what a
/// `Match` takes: a condition that is also the value the arm is drawn from, so
/// that the arm has the state's own fields in hand without asking again.
const absent = (told: RemoteView) => told.tailscale === "Absent";
const down = (told: RemoteView) => (told.tailscale === "Down" ? told : null);
const unreadable = (told: RemoteView) =>
  told.tailscale === "Unreadable" ? told : null;
const up = (told: RemoteView) => (told.tailscale === "Up" ? told : null);

/// And the serve state the same way.
const serving = (serve: ServeView) => (serve.serve === "On" ? serve : null);
const unread = (serve: ServeView) =>
  serve.serve === "Unreadable" ? serve : null;

/// And what a press came back with, where it was one of the two answers that is
/// not the machine read again. `undefined` before anything has been pressed.
const ungranted = (pressed: ServePress | undefined) =>
  pressed?.press === "Ungranted" ? pressed : null;
const troubled = (pressed: ServePress | undefined) =>
  pressed?.press === "Trouble" ? pressed : null;

/// Whether this page is being read over the very serve it is offering to turn
/// off.
///
/// The address is the reading's own and the hostname is this document's, so the
/// question is answered in the browser and nothing new is asked of the server —
/// which could not answer it anyway: a request off the tailnet and one off the
/// loopback arrive on the same port, and the socket cannot tell the two apart.
///
/// False wherever there is nothing to compare: a machine serving nothing has no
/// address, and an address this browser cannot parse is not one to lock a
/// control over.
function arrivedOver(serve: ServeView): boolean {
  const on = serving(serve);
  if (!on) return false;

  try {
    return (
      new URL(on.address).hostname.toLowerCase() ===
      window.location.hostname.toLowerCase()
    );
  } catch {
    return false;
  }
}

/// What this machine's Tailscale is doing, read for the two panes that draw it.
///
/// A read of its own rather than a field of the settings, because none of it is
/// a setting: it is what the machine is doing, and the answer changes without
/// anybody having been to this page.
function useRemote() {
  return useReading(() => ({
    queryKey: ["remote"],
    queryFn: loadRemote,
    freshness: { reconcile: "tailscale" },
  }));
}

/// And what this Verkstead *is*, read for the two panes that draw that too: the
/// device this machine runs, and every other device in its cluster.
///
/// A read of its own beside the one above rather than a field of it, because
/// they are two different questions about this machine: what Tailscale is doing
/// changes when somebody runs a command in a terminal, and what device this is
/// changes when a link is made or an address moves. Neither is a setting, which
/// is why neither is in the settings query.
///
/// Merged by the device id: what says one row from another is the id, so a
/// re-read that found another device leaves the rows it already drew alone.
function useDevices() {
  return useReading(() => ({
    queryKey: ["devices"],
    queryFn: loadDevices,
    freshness: { reconcile: "device" },
  }));
}

/// And the devices out there that this one is *not* in a cluster with, which is
/// the Discovered list under those rows.
///
/// **A read of its own beside the one above rather than a field of it**, and that
/// is what it is for: a browse hears something every few seconds, and a list
/// arriving on the same answer as the membership would be the rows the pane had
/// already drawn replaced each time the LAN said anything. What a `discovered`
/// Nudge re-reads is this and nothing else.
///
/// **And this read is what holds the browse open.** The server starts browsing
/// when it is first asked and stops once nothing has asked for a spell — a phone
/// that closes a tab says nothing — so the first answer is empty or short and the
/// rows arrive over the seconds after it, each announced. Nothing polls.
///
/// **It is also what makes the probe of the tailnet happen**, which is the other
/// half of the list and is asked rather than heard: a tailnet has no multicast for
/// a device to announce itself over, so the server asks its peers as this is read.
/// Which is why the answer can take a moment where the LAN half is instant, and
/// why the tailnet rows are in the first answer rather than arriving after it.
///
/// Merged by the device id, like the reading above and for its reason: a browse
/// that heard one more device leaves the rows it already drew alone.
function useDiscovered() {
  return useReading(() => ({
    queryKey: ["discovered"],
    queryFn: loadDiscovered,
    freshness: { reconcile: "device" },
  }));
}

/// How many devices are linked to this one, as a sentence.
///
/// The clause the card carries beside what Tailscale is doing — *other*
/// devices, because this device is the row the list already holds and nothing
/// is linked to itself. Nought is a sentence rather than a silence: a workbench
/// that says nothing about devices reads as one that has not heard of them.
///
/// Counted off the rows the list draws rather than answered beside them, so the
/// sentence and the list cannot come to disagree about one membership.
function linked(count: number): string {
  if (count === 0) return "No other devices are linked.";
  if (count === 1) return "One other device is linked.";

  return `${count} other devices are linked.`;
}

/// How things stand, in the one line somebody scanning the page is after.
///
/// The card's line, and the card's alone: the pane below it is the controls
/// themselves, and a box that is ticked has already said that the workbench is
/// served.
function standing(told: RemoteView): JSX.Element {
  return (
    <Choose>
      <Match when={absent(told)}>
        No Tailscale on this machine, so the workbench is reachable only from
        the machine it runs on.
      </Match>
      <Match when={down(told)}>
        Tailscale is installed here and not up, so there is no tailnet address
        to reach the workbench at.
      </Match>
      <Match when={unreadable(told)}>
        Tailscale answered in a way this build could not read, so what it is
        doing cannot be told from here.
      </Match>
      <Match when={up(told)} keyed>
        {(here) => (
          <Choose>
            <Match when={serving(here.serve)} keyed>
              {(on) => (
                <>
                  Served to the tailnet at{" "}
                  <span class={styles.address}>{on.address}</span>.
                </>
              )}
            </Match>
            <Match when={here.serve.serve === "Off"}>
              On the tailnet as <span class={styles.address}>{here.node}</span>,
              with the workbench not served to it.
            </Match>
            <Match when={here.serve.serve === "Unreadable"}>
              On the tailnet as <span class={styles.address}>{here.node}</span>.
              What is served to it could not be read.
            </Match>
          </Choose>
        )}
      </Match>
    </Choose>
  );
}

/// How this machine is reached, as the card that opens the section.
export function RemoteCard(props: {
  /// Whether the pane beside this is the one that is open.
  open: boolean;
  /// What pressing it does, which is opening that pane.
  press: () => void;
}): JSX.Element {
  const remote = useRemote();
  const devices = useDevices();

  return (
    <Choose>
      <Match when={remote.isPending}>
        <Empty>Loading…</Empty>
      </Match>
      <Match when={remote.isError}>
        <ErrorLine>
          Could not read this machine's Tailscale: {remote.error?.message}
        </ErrorLine>
      </Match>
      <Match when={remote.data}>
        {(told) => (
          <CardButton
            as="article"
            class={styles.remoteCard}
            open={props.open}
            press={props.press}
          >
            <h2>Remote access</h2>

            {/* The Tailscale sentence and the devices clause in the one line,
                the clause after whichever of the six the machine turned out to
                be: it is true of every one of them, so it follows all of them
                rather than being written into one. Absent until the devices
                read lands, and absent for good if it fails — what Tailscale is
                doing is still worth saying on its own. */}
            <p class={styles.standing}>
              {standing(told())}{" "}
              <Show when={devices.data}>
                {(here) => <>{linked(here().members.length)}</>}
              </Show>
            </p>
          </CardButton>
        )}
      </Match>
    </Choose>
  );
}

/// And the controls the card opens: the box, the way in, and the key.
export function RemotePane(props: {
  /// The way back to the settings, which is the pane this one was entered from.
  back: () => void;
}): JSX.Element {
  const queries = useQueryClient();
  const remote = useRemote();

  /// The serve box, ticked.
  ///
  /// What a press that went through answers with is the machine read again, so
  /// it is written straight over the read the pane is drawn from: a second
  /// request would learn nothing the first one did not already say, and could
  /// only disagree with what is on screen while it was in flight.
  ///
  /// The other two answers are left in `press.data` and drawn from there. They
  /// are not errors — a refusal for want of the operator grant is a sentence
  /// with a command in it — so nothing here throws, and the next press replaces
  /// whichever of them is showing.
  const press = useMutation(() => ({
    mutationFn: (on: boolean) => pressServe({ on }),
    onSuccess: (pressed: ServePress) => {
      if (pressed.press === "Done") {
        queries.setQueryData(["remote"], pressed.reading);
      }
    },
  }));

  return (
    <>
      <PaneSticky>
        <PaneHead
          back={{ to: "Settings", go: props.back }}
          title="Remote access"
        />
      </PaneSticky>

      <Choose>
        <Match when={remote.isPending}>
          <Empty>Loading…</Empty>
        </Match>
        <Match when={remote.isError}>
          <ErrorLine>
            Could not read this machine's Tailscale: {remote.error?.message}
          </ErrorLine>
        </Match>
        <Match when={remote.data}>
          {(told) => (
            <div class={styles.remote}>
              <Choose>
                {/* The one state whose answer is somewhere else entirely: a
                    pointer at where to get one, because nothing on this page
                    can install it. */}
                <Match when={absent(told())}>
                  <Note>
                    Verkstead reaches a phone over a tailnet.{" "}
                    <a
                      class={styles.pointer}
                      href={DOWNLOAD}
                      target="_blank"
                      rel="noreferrer"
                    >
                      Install Tailscale
                    </a>{" "}
                    on this machine and join it to a tailnet.
                  </Note>
                </Match>

                {/* Installed and not up. What it printed is the useful half —
                    the line names the service to start — so it stands as it
                    came rather than reworded. */}
                <Match when={down(told())} keyed>
                  {(stopped) => (
                    <>
                      <Note>
                        Run <code>tailscale up</code> on this machine to join it
                        to a tailnet. This is what it said when it was asked:
                      </Note>
                      <p class={styles.trouble}>{stopped.trouble}</p>
                    </>
                  )}
                </Match>

                <Match when={unreadable(told())} keyed>
                  {(strange) => (
                    <>
                      <Note>
                        Tailscale is whatever this machine has, and this build
                        does not know the shape it answered in. This is what it
                        said:
                      </Note>
                      <p class={styles.trouble}>{strange.trouble}</p>
                    </>
                  )}
                </Match>

                <Match when={up(told())} keyed>
                  {(here) => (
                    <>
                      <Choose>
                        {/* A box over an answer this build could not read would
                            be offering to turn on something that may already be
                            on, so the state says so instead. */}
                        <Match when={unread(here.serve)} keyed>
                          {(strange) => (
                            <>
                              <Note>
                                What Tailscale is serving to the tailnet could
                                not be read, so nothing here offers to change
                                it. This is what it said:
                              </Note>
                              <p class={styles.trouble}>{strange.trouble}</p>
                            </>
                          )}
                        </Match>

                        <Match when={!unread(here.serve)}>
                          <Check
                            label="Allow remote access via Tailscale"
                            on={Boolean(serving(here.serve))}
                            // Never from the page the serve is carrying: that
                            // press would take away the connection making it.
                            disabled={
                              press.isPending || arrivedOver(here.serve)
                            }
                            title={arrivedOver(here.serve) ? LOCKED : undefined}
                            flip={(on) => press.mutate(on)}
                          />

                          <Note>
                            Allows secure remote access from other machines over
                            the internet.
                          </Note>
                        </Match>
                      </Choose>

                      {refused(press.data)}

                      <Show when={press.isError}>
                        <ErrorLine class={styles.failure}>
                          The serve could not be changed: {press.error?.message}
                        </ErrorLine>
                      </Show>

                      {/* And the way in, where there is an address to be let in
                          at. A machine serving nothing has none, and so has
                          nothing for a camera to be pointed at — the key those
                          links carry stands below, which every machine has. */}
                      <Show when={here.link} keyed>
                        {(link) => <Reach link={link} />}
                      </Show>
                    </>
                  )}
                </Match>
              </Choose>

              {/* And the key itself, under whichever of the four the machine
                  turned out to be. It is not Tailscale's — it gates a machine
                  that has never heard of a tailnet exactly as it gates one
                  serving on it — so the press that re-issues it must not be a
                  thing the serve box can take away. */}
              <TheKey />

              {/* And the devices, outside that choice for the key's own
                  reason said about a different thing: a machine with no
                  Tailscale at all still has an identity of its own, and a list
                  that vanished on one would be a cluster feature that appeared
                  to need a tailnet. */}
              <Devices />
            </div>
          )}
        </Match>
      </Choose>
    </>
  );
}

/// How a phone gets in: the login link as a code to point a camera at, and as
/// text to paste.
///
/// Two things about one string. The QR is for the phone in somebody's hand; the
/// copy is for every other way a link travels — a laptop on the same tailnet, a
/// note to oneself. Neither is labelled: a QR code on a page about reaching this
/// workbench from a phone is a thing everybody has already scanned once.
///
/// Drawn only where there is a served address to build a link on, because that
/// is what a link *is* here: a machine serving nothing has nothing for a camera
/// to be pointed at. The key those links carry stands below it, which every
/// machine has whether or not it is serving.
function Reach(props: { link: string }): JSX.Element {
  return (
    <section class={styles.reach}>
      <div class={styles.letIn}>
        <div class={styles.code}>
          <Qr of={props.link} label="The login link for this workbench" />
        </div>

        <div class={styles.beside}>
          <p class={styles.link}>{props.link}</p>
          <Copy of={props.link} class={styles.copy} />
        </div>
      </div>
    </section>
  );
}

/// And the key every one of those links hands over, with the press that takes
/// it back.
///
/// **On every state of this pane**, because the key is not Tailscale's. It
/// gates a machine that has never heard of a tailnet exactly as it gates one
/// serving on it, and the daemon prints it in its startup line wherever it is
/// running — so a key that has gone somewhere it should not have has to be
/// re-issuable from the same place whether or not there is an address in front
/// of it. Drawn under the code where there is one, and on its own where there
/// is not.
///
/// Which also settles the awkward one: turning the serve *off* is exactly a
/// moment somebody might want the key back, and a Reset that went away with the
/// address would be the press removing itself.
///
/// The press answers with the machine read again, exactly as the serve box's
/// does, so it is written straight over the read this pane is drawn from: the
/// code above redraws on the new key out of the answer rather than out of a
/// second request.
function TheKey(): JSX.Element {
  const queries = useQueryClient();

  const reset = useMutation(() => ({
    mutationFn: resetKey,
    onSuccess: (reading: RemoteView) =>
      queries.setQueryData(["remote"], reading),
  }));

  return (
    <section class={styles.key}>
      <button
        type="button"
        class={styles.reset}
        disabled={reset.isPending}
        onClick={() => reset.mutate()}
      >
        {reset.isPending ? "Resetting…" : "Reset key"}
      </button>

      <Note>
        Resets the secret token used to access the UI. This will disconnect all
        other devices.
      </Note>

      <Show when={reset.isError}>
        <ErrorLine class={styles.failure}>
          The key could not be reset: {reset.error?.message}
        </ErrorLine>
      </Show>
    </section>
  );
}

/// The devices this workbench is one of: this one, and every other in its
/// cluster.
///
/// **A section of this pane rather than a settings section of its own**
/// (ADR-0020): linking is how this machine is reached as much as the serve and
/// the key are. So there is nothing added to `WORDS`, no card and no route —
/// what the pane grows is this and the reading behind it.
///
/// **On every state of the pane**, for the reason [`TheKey`] is on every state
/// of it: a machine that has never heard of a tailnet has a device identity all
/// the same, and a list that went away on one would say this needed Tailscale.
///
/// The name is the hostname the machine answers to and the mark beside it is
/// the word for its OS — a WSL wears the Linux mark and reads *Linux (WSL)*,
/// which is the one thing that tells it from the Windows it shares a hostname
/// with, and the case a cluster of two rows is drawn for. *This device* is what
/// marks which of them is this machine, and it is what stands where a member's
/// row carries its **Unlink**: there is nothing to unlink this machine from
/// itself.
///
/// A member the last dial found nothing at is dimmed and reads *unreachable*,
/// which is the one other way a member's row is drawn — and its Unlink works
/// exactly as any other's, a machine that is never coming back being most of
/// what the press is for.
///
/// **And under the rows, the one thing on this pane that is configured rather
/// than read**: Add, against an address somebody types. Everything else here is
/// the machine read again — which is what makes the box worth marking out, and
/// why it stands below the list rather than among it.
///
/// **The asking is this component's**, rather than each row's, for the reason
/// the Repos pane keeps it above its list: the card names the device it is
/// about, and one card over a list is one thing on the page at a time. The
/// device a press asked about is held here and given to it.
function Devices(): JSX.Element {
  const devices = useDevices();

  // The device a press on Unlink asked about, while the card asking is up — or
  // `null` while none is. The whole row rather than its id: the card names it,
  // and the press is made from the card rather than from the row.
  const [asking, setAsking] = createSignal<DeviceIdentity | null>(null);

  const queries = useQueryClient();

  // And the press itself, which answers with the section read again — so the
  // row goes out of this answer rather than out of a second request, the way
  // Add and Cancel below are answered.
  const drop = useMutation(() => ({
    mutationFn: (device: string) => unlinkDevice(device),
    onSuccess: (reading: DevicesView) =>
      queries.setQueryData(["devices"], reading),
  }));

  /// And what the press on the card that asked does, which is the unlink.
  const confirmed = () => {
    const device = asking();
    setAsking(null);

    if (device !== null) drop.mutate(device.device);
  };

  return (
    <section class={styles.devices}>
      <Choose>
        <Match when={devices.isPending}>
          <Empty>Loading…</Empty>
        </Match>
        <Match when={devices.isError}>
          <ErrorLine class={styles.failure}>
            Could not read this device: {devices.error?.message}
          </ErrorLine>
        </Match>
        <Match when={devices.data}>
          {(here) => (
            <List of={here()} unlink={setAsking} unlinking={drop.isPending} />
          )}
        </Match>
      </Choose>

      <Show when={drop.isError}>
        <ErrorLine class={styles.failure}>
          The device could not be unlinked: {drop.error?.message}
        </ErrorLine>
      </Show>

      {/* And what is out there rather than in the cluster, between the rows and
          the box: a press here needs nothing typed, and the box below is what is
          left for the devices a browse cannot reach. */}
      <Discovered />

      <Add />

      {/* Asked before anything happens, the way Remove on a Repo is: one press
          in a list, on a phone, that cannot be taken back. */}
      <Confirm
        asked={asking()}
        keep={() => setAsking(null)}
        unlink={confirmed}
      />
    </section>
  );
}

/// What an Unlink is answered with before anything has happened: which device
/// it would take out of the cluster, what that means for the rest of it, and
/// the two ways out.
///
/// The device is named on the card because the list it was pressed in is behind
/// it — the shape the Repos pane's confirm takes, for its reason: a card reading
/// *Unlink this device?* over a list of them would be asking about whichever row
/// the human remembers pressing.
///
/// And the sentence under the name is what makes this press different from
/// every other undoing in the app: it is not this device's own half of a link
/// being cut, it is the device leaving the cluster for everybody.
function Confirm(props: {
  /// The device the press asked about, or `null` while nothing is being asked.
  asked: DeviceIdentity | null;
  /// The way back, which is what Escape and a press on the backdrop come to as
  /// well: every way out of this card but the one button leaves the cluster
  /// alone.
  keep: () => void;
  /// And the press it asked about, made.
  unlink: () => void;
}): JSX.Element {
  // The heading's own id, for the `aria-labelledby` that names the card by it.
  const id = createUniqueId();

  return (
    <Modal
      class={styles.confirming!}
      open={props.asked !== null}
      close={props.keep}
      labelledBy={id}
    >
      <p id={id} class={styles.confirmingTitle}>
        Unlink {props.asked?.name}?
      </p>
      <p class={styles.confirmingWhy}>
        It leaves the cluster for every device in it, not just this one, and it
        is told to forget the rest. Linking it again means somebody allowing it
        on one of them.
      </p>
      <div class={styles.confirmingOut}>
        {/* Both classes, as every confirm pair in the app carries them: the
            global one is the paint, and the module's is what the row stands the
            filled press out of. */}
        <button
          type="button"
          class={`${styles.secondary!} secondary`}
          onClick={() => props.keep()}
        >
          Keep it
        </button>
        <button type="button" onClick={() => props.unlink()}>
          Unlink
        </button>
      </div>
    </Modal>
  );
}

/// The list: this device first, then a row apiece for the devices linked to it,
/// and last the joins this one has asked for and not been answered on.
///
/// This device leads rather than being sorted among them — it is the row
/// somebody opening this pane is looking for, and it is the only one read off
/// the machine the page is served from. The members follow in the order the
/// reading gives them, which is by the name each is shown under.
///
/// The pending rows come last, under the box that made them: they are not
/// devices — nothing has been agreed — so they sit below the cluster rather than
/// inside it, where the eye goes after pressing Add.
///
/// **This device's row is the one without an Unlink**, which is why the press is
/// given to the members alone rather than to [`Row`] for every row to decide:
/// there is nothing to unlink this machine from itself, and a row that has no
/// press is a row that is handed none.
function List(props: {
  of: DevicesView;
  /// What a press on a member's row asks about, which is the whole of what a
  /// row does with one: the card above this list is what acts.
  unlink: (device: DeviceIdentity) => void;
  /// And whether one is going through, which greys every press on the list
  /// while it is: the list is about to be read again and a second press would
  /// be about the list before it.
  unlinking: boolean;
}): JSX.Element {
  return (
    <ul class={styles.list}>
      <Row of={props.of.this} here />

      <For each={props.of.members}>
        {(member: LinkedDevice) => (
          <Row
            of={member.identity}
            unreachable={!member.reachable}
            unlink={() => props.unlink(member.identity)}
            unlinking={props.unlinking}
          />
        )}
      </For>

      <For each={props.of.pending}>
        {(join: PendingJoin) => (
          // This device's own fingerprint rather than the far end's, and that is
          // the whole point of it: the modal on the other machine draws the same
          // string, and the two are there for one person reading a phone and
          // another reading a screen to compare by eye.
          <Waiting on={join} fingerprint={props.of.this.fingerprint} />
        )}
      </For>
    </ul>
  );
}

/// One row: the OS mark, the name, and every address a peer could reach that
/// device on — with *this device* beside the name on the one that is this
/// machine.
///
/// One component for both because they are one thing drawn: what the pane shows
/// of a device is the same three facts whether it is this one or another, and
/// the answer behind them is the same shape either way. What differs is only
/// where it was read — this machine now, or the far end at the last exchange.
///
/// The addresses are drawn only where there are any. A device on neither a
/// tailnet nor a network answers with none, which is an answer rather than a
/// failure — it still has a name and a mark, and an empty line under them would
/// say something went wrong.
///
/// *unreachable* stands where *this device* stands on the row above, and dims
/// the whole of the row with it: the two are the same kind of thing said, a fact
/// about the row rather than something to press. Everything about the device
/// stays drawn — the mark, the name and the addresses — because none of it has
/// stopped being true, and the row is where the Unlink is pressed.
///
/// **And the Unlink is drawn wherever one was given**, which is every member's
/// row and no other: this device's row has none, there being nothing to unlink
/// this machine from itself. A dimmed row's press is drawn exactly as a live
/// one's and works exactly as one — the machine that is never coming back is
/// most of what an Unlink is for, and a press that greyed itself out on the one
/// row somebody needs it on would be the feature refusing its own case.
function Row(props: {
  of: DeviceIdentity;
  here?: boolean;
  unreachable?: boolean;
  /// What a press asks about, where this row has one to press.
  unlink?: () => void;
  /// And whether one is already going through.
  unlinking?: boolean;
}): JSX.Element {
  return (
    <li
      class={styles.device}
      classList={{ [styles.unreachable!]: props.unreachable }}
    >
      <Icon of={osIcon(props.of.os)} label={props.of.os} class={styles.os} />

      <div class={styles.about}>
        <p class={styles.deviceName}>
          {props.of.name}
          <Show when={props.here}>
            {" "}
            <span class={styles.here}>this device</span>
          </Show>
          <Show when={props.unreachable}>
            {" "}
            <span class={styles.away}>unreachable</span>
          </Show>
        </p>

        <Show when={props.of.addresses.length > 0}>
          <p class={styles.addresses}>{props.of.addresses.join(", ")}</p>
        </Show>
      </div>

      <Show when={props.unlink} keyed>
        {(ask) => (
          <button
            type="button"
            class={styles.unlink}
            disabled={props.unlinking}
            onClick={() => ask()}
          >
            Unlink
          </button>
        )}
      </Show>
    </li>
  );
}

/// **Discovered**: the devices out there that this one is not in a cluster with,
/// each with one press to link it (ADR-0020, *Discovery*).
///
/// **The other half of the box under it.** A device on the same LAN says what it
/// is over mDNS, a node of the same tailnet says what it is when this one asks,
/// and a row here is either of those said back — the name, the mark for its OS,
/// where it was found and how — so that linking two machines needs nobody to know
/// either one's address. What the typed box is left for is the cases neither half
/// can cross: a Windows machine and the WSL on it, which is NAT rather than a bug,
/// a device listening on a port nothing assumes, and whatever else is on a network
/// of its own.
///
/// **Its own reading rather than a field of the one above**, which is what keeps
/// the two apart: a browse hears something every few seconds, and the cluster's
/// own rows are not re-read for any of it — see [`useDiscovered`].
///
/// **Empty reads as looking rather than as nothing there**, because a browse is
/// cold when it starts: the first answer is empty however many machines are on
/// the LAN, and the rows arrive over the seconds after it. A line saying nothing
/// was found would be a line that is wrong for the first second of every visit
/// to this pane.
///
/// **Members are not here**, nor is this device, nor is one a press has already
/// been made on — the server leaves all three out, so a device that has just been
/// pressed moves from this list to the rows above rather than being drawn twice.
///
/// **The press is this component's rather than each row's**, for the reason the
/// asking above the cluster's list is that component's: one thing on the page at
/// a time. A press names the device it was made on, the answer redraws the
/// section, and a refusal is drawn once under the list in the words it came back
/// in — which name the device themselves, a row being stale by the time somebody
/// presses it being exactly what they are about.
function Discovered(): JSX.Element {
  const found = useDiscovered();
  const queries = useQueryClient();

  // Which row is being asked about, so that the press on it reads *Asking…*
  // while the others are only held. `null` while nothing is in flight.
  const [pressed, setPressed] = createSignal<string | null>(null);

  const ask = useMutation(() => ({
    mutationFn: (device: string) => addFound(device),

    // The membership comes out of the answer, the way the typed press's does —
    // and the list this row was on is re-read rather than reasoned about here:
    // the server leaves out the device a join is now pending for, and leaves out
    // a row it has forgotten for want of an answer, which is the same re-read
    // either way.
    onSuccess: (reading: DevicesView) =>
      queries.setQueryData(["devices"], reading),
    onSettled: () => void queries.invalidateQueries({ queryKey: ["discovered"] }),
  }));

  const press = (device: string) => {
    setPressed(device);
    ask.mutate(device);
  };

  return (
    <div class={styles.found}>
      <p class={styles.foundTitle}>Discovered</p>

      <Choose>
        <Match when={found.isError}>
          <ErrorLine class={styles.failure}>
            Could not read what this device has heard: {found.error?.message}
          </ErrorLine>
        </Match>

        <Match when={found.data?.length}>
          <ul class={styles.list}>
            <For each={found.data}>
              {(device: DiscoveredDevice) => (
                <Heard
                  of={device}
                  add={press}
                  asking={ask.isPending ? pressed() : null}
                />
              )}
            </For>
          </ul>
        </Match>

        <Match when={true}>
          <Note>
            Devices on this network and on this tailnet appear here as they are
            found, with nothing to type. A machine that never appears is linked by
            its address below.
          </Note>
        </Match>
      </Choose>

      <Show when={ask.isError}>
        <ErrorLine class={styles.failure}>
          The device could not be asked: {ask.error?.message}
        </ErrorLine>
      </Show>
    </div>
  );
}

/// One device heard of: the mark for its OS, the name, where it was found, and
/// the press that links it.
///
/// Drawn as the rows above it are, because it is the same three facts said — a
/// name, a mark and the addresses under them. What is different is the word
/// beside the name, which stands where *this device* and *unreachable* stand on
/// those: where this device was heard, which is a fact about the row rather than
/// something to press.
///
/// **And the press is an Add rather than a link being made.** Nothing has been
/// agreed with a device that has only advertised itself, or only answered that it
/// is there: what the press does is the same Join the typed box makes, and what it
/// leaves is the same pending row with a fingerprint on it for two people to
/// compare — after which this row is gone, a device a join is pending for being
/// one the list leaves out.
///
/// **What it names is the device rather than an address.** The row holds every
/// place this device found it, and the server works down them in that order: a
/// row that sent one of its addresses would be the page choosing which of them is
/// the live one.
function Heard(props: {
  of: DiscoveredDevice;
  add: (device: string) => void;
  asking: string | null;
}): JSX.Element {
  return (
    <li class={styles.device}>
      <Icon of={osIcon(props.of.os)} label={props.of.os} class={styles.os} />

      <div class={styles.about}>
        <p class={styles.deviceName}>
          {props.of.name} <span class={styles.source}>{where(props.of)}</span>
        </p>

        <p class={styles.addresses}>{props.of.addresses.join(", ")}</p>
      </div>

      {/* The row names a device, and what is dialled is every address the
          discovery found for it. Every press is held while one is in flight, and
          the one that was made says so. */}
      <button
        type="button"
        class={styles.add}
        disabled={props.asking !== null}
        onClick={() => props.add(props.of.device)}
      >
        {props.asking === props.of.device ? "Asking…" : "Add"}
      </button>
    </li>
  );
}

/// The word for each way a device was heard of.
///
/// A table rather than the words in the markup, because the answer is a list: a
/// device on one LAN and one tailnet is found twice and is one row, and what the
/// row says is both.
const WHERE: Record<DiscoveredDevice["found"][number], string> = {
  Lan: "LAN",
  Tailscale: "Tailscale",
};

/// Where a device was heard, as the row reads it: *LAN*, *Tailscale*, and both
/// words where both found it.
function where(device: DiscoveredDevice): string {
  return device.found.map((on) => WHERE[on]).join(" and ");
}

/// One join this device has asked for and not been answered on: where it
/// knocked, who answered, and what the person at the other machine is being
/// asked to compare.
///
/// **A row that is not a device.** It wears no OS mark and lists no addresses,
/// because nothing has been agreed: what is known is that somebody typed an
/// address, something answered, and a human over there has been asked. The
/// glass stands where an OS mark stands on the rows above — the one thing this
/// row has to say is that it is waiting.
///
/// **The fingerprint is this device's own.** The modal on the far end draws the
/// same string, and the pair exists to be read aloud between two people at two
/// screens; a row that showed the far end's would be each of them reading the
/// other's back.
///
/// **Three things a row can say, and two of them are over.** It is waiting; or
/// the human at the other machine said no, which came back over the link the
/// join was posted on; or nobody answered inside ten minutes. A refusal outranks
/// an expiry, because it is the thing that really happened — a request refused
/// three minutes in stays a refusal however long the row is left up.
///
/// **Cancel, and Dismiss on one that is over**, which are the same press: the
/// human is done with a request that came to nothing, and which word it wears is
/// a fact about the row. A row that is over is left drawn rather than taken away
/// — somebody pressed Add and is owed the answer.
function Waiting(props: { on: PendingJoin; fingerprint: string }): JSX.Element {
  const queries = useQueryClient();

  const take = useMutation(() => ({
    mutationFn: () => cancelJoin(props.on.request),
    onSuccess: (reading: DevicesView) =>
      queries.setQueryData(["devices"], reading),
  }));

  /// Whether there is anything left to wait for, which is the one question the
  /// whole row is drawn off: the fingerprint is worth comparing only while
  /// somebody is standing in front of it, and the press reads Dismiss once
  /// nobody is.
  const over = () => props.on.refused || props.on.expired;

  return (
    <li class={styles.device} classList={{ [styles.unreachable!]: over() }}>
      <Icon of={faHourglassHalf} label="waiting" class={styles.os} />

      <div class={styles.about}>
        <p class={styles.deviceName}>
          <Choose>
            <Match when={props.on.refused}>
              {props.on.name} refused the request.
            </Match>

            <Match when={props.on.expired}>
              The request to {props.on.name} expired.
            </Match>

            <Match when={true}>
              Waiting for confirmation on {props.on.name}.
            </Match>
          </Choose>
        </p>

        <p class={styles.addresses}>{props.on.address}</p>

        {/* Drawn only while somebody could still be comparing it: a request
            that is over is one nobody is standing in front of any more. */}
        <Show when={!over()}>
          <p class={styles.addresses}>{props.fingerprint}</p>
        </Show>

        <button
          type="button"
          class={styles.take}
          disabled={take.isPending}
          onClick={() => take.mutate()}
        >
          {over() ? "Dismiss" : "Cancel"}
        </button>

        <Show when={take.isError}>
          <ErrorLine class={styles.failure}>
            The request could not be taken back: {take.error?.message}
          </ErrorLine>
        </Show>
      </div>
    </li>
  );
}

/// **Add**: the address of another Verkstead, and the press that asks it.
///
/// **The one control on this pane that configures rather than reads.** The three
/// sections above it are the machine said back — what Tailscale is doing, what
/// the key is, what devices there are — and this is the one place somebody types
/// something in. Which is the same departure Unlink will make beside it, and the
/// one Remove on a Repo made before either.
///
/// **A port is optional and usually absent**: every device answers on the peer
/// port unless its host was told another, and the human should not have to know
/// which that is. One typed with a port is asked at it.
///
/// What the press answers with is the section read again, so the pending row
/// arrives out of this answer rather than out of a second request. A refusal is
/// drawn in the words the dial put it in: a machine that is off, an address
/// nobody is at and a Verkstead that would not have it are three different
/// things to do something about.
function Add(): JSX.Element {
  const queries = useQueryClient();
  const [typed, setTyped] = createSignal("");

  const ask = useMutation(() => ({
    mutationFn: (address: string) => addDevice(address),
    onSuccess: (reading: DevicesView) => {
      queries.setQueryData(["devices"], reading);
      setTyped("");
    },
  }));

  const press = (ev: Event) => {
    ev.preventDefault();

    const address = typed().trim();
    if (address) ask.mutate(address);
  };

  return (
    <form class={styles.adding} onSubmit={press}>
      <label for="link-address">Link another device</label>

      <div class={styles.field}>
        <input
          id="link-address"
          type="text"
          autocapitalize="off"
          autocorrect="off"
          spellcheck={false}
          placeholder="hostname or address"
          value={typed()}
          disabled={ask.isPending}
          onInput={(ev) => setTyped(ev.currentTarget.value)}
        />
        <button
          type="submit"
          disabled={ask.isPending || typed().trim().length === 0}
        >
          {ask.isPending ? "Asking…" : "Add"}
        </button>
      </div>

      <Note>
        Asks that device to link with this one. Somebody has to allow it there,
        and the fingerprint shown here is what they will be comparing against.
      </Note>

      <Show when={ask.isError}>
        <ErrorLine class={styles.failure}>
          The device could not be asked: {ask.error?.message}
        </ErrorLine>
      </Show>
    </form>
  );
}

/// What a press said, where what it said was not the machine read again.
///
/// Two answers and neither of them an error the page can retry for the human.
/// The operator grant is a line to run in a terminal, and the next press is the
/// re-try — so it is drawn as the command it is, with what Tailscale said under
/// it. Anything else is the machine's own words and nothing to add to them.
///
/// Nothing at all before the first press, and nothing after one that went
/// through: the reading it answered with is already on the page.
function refused(pressed: ServePress | undefined): JSX.Element {
  return (
    <Choose>
      <Match when={ungranted(pressed)} keyed>
        {(denied) => (
          <>
            <Note>
              Tailscale will not set up a serve for the user this server runs
              as. Run this on the machine, then press the box again:
            </Note>
            <p class={styles.command}>{denied.grant}</p>
            <p class={styles.trouble}>{denied.trouble}</p>
          </>
        )}
      </Match>
      <Match when={troubled(pressed)} keyed>
        {(bad) => (
          <>
            <Note>
              The serve could not be changed. This is what Tailscale said:
            </Note>
            <p class={styles.trouble}>{bad.trouble}</p>
          </>
        )}
      </Match>
    </Choose>
  );
}
