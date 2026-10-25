//! The wizard's second step: the agent accounts already on this machine,
//! offered as Agent Profiles.
//!
//! **The accounts are the server's.** It looks in its own home for the four
//! shapes an account comes in — `crates/server/src/sandbox.rs` says what each is
//! made of, and the wizard reads that same list — so at most one per harness is
//! ever found, and each arrives here as the account the profile create would be
//! handed. Nothing on this side knows what a `.claude` is.
//!
//! **A found account is a card, offered ticked where its harness is on the
//! machine and drawn quiet where it is not.** The whole card is the press, and
//! a ticked one takes the ember's edge. An account whose binary is missing is
//! not something to make a Profile of yet: it has no box, a Not installed pill
//! after its name, and a Go to step 1 on its right — drawn only while step 1
//! is met, which is the rail's own rule and the only time there is a way back.
//! The page ticks the card when the harness lands.
//!
//! **A ticked one is saved with no name and every model this build knows for
//! its harness.** Both are the settings page's to change afterwards; nothing
//! here asks anybody to type either, because an account found in a home is one
//! account and needs no name to tell it from another.
//!
//! **The press names what it does: Keep N profile(s), or Next** once a Profile
//! stands saved and nothing is ticked. It needs at least one Profile — ticked
//! here, or made in the form. That is one of the three things the objective is
//! and the only one a step could walk past: clearing the mode with nothing
//! saved would land somebody on exactly the empty state a skip was rejected
//! for, until the next start put them through the wizard again. Held, its
//! reason stands beside it.
//!
//! **Nothing found is a state rather than a gap.** A card says so, with a
//! Watching dot over the frame's own ten-second cadence — see
//! [`./SetupPage.tsx`](./SetupPage.tsx) — and what to run to make an account.
//!
//! **The form is behind a dashed card.** Naming an account by hand is the way
//! in for a machine whose accounts are kept somewhere other than this server's
//! home, which is the uncommon one: the ordinary machine ticks a card and
//! presses on, and a form of paths and models standing open under those cards
//! reads as work the step is asking for. A press opens the card in place into
//! the form, and it never shuts again: nothing on this step wants the room
//! back. It starts shut in the nothing-found state as well.

import { faArrowRight, faPlus } from "@fortawesome/free-solid-svg-icons";
import { useMutation, useQueryClient } from "@tanstack/solid-query";
import { For, Show, createEffect, createSignal, type JSX } from "solid-js";

import { Copy } from "../Copy";
import { HarnessMark } from "../HarnessMark";
import { Icon } from "../Icon";
import { AGENT_NAME, type AgentType } from "../agents";
import { createProfile } from "../api/client";
import { harnessAbsent } from "../broken";
import type {
  AccountView,
  OnboardingView,
  ProfileEdit,
  ProfileSaved,
} from "../api/types";
import { KNOWN_MODELS } from "../models";
import { ErrorLine } from "../notices";
import { rowPress } from "../rows";
import {
  BLANK_PROFILE,
  PROFILE_REFUSAL,
  ProfileForm,
} from "../profiles/ProfileList";
import { Foot } from "./Foot";
import { PROBE } from "./steps";
import styles from "./Accounts.module.css";

/// What to run to make an account of each harness: the program itself, once, in
/// a terminal — logging in is what each of them does when it is first started.
///
/// The viewer's own words, like the install commands beside them: what the
/// server says is which accounts are *there*, and how one is made is the same
/// answer on every Verkstead.
const LOGIN: Record<AgentType, string> = {
  Claude: "claude",
  Codex: "codex",
  Grok: "grok",
  OpenCode: "opencode auth login",
};

/// Where each shape of account is kept, said as the paths a human would
/// recognise — the ones under the home the server itself runs as.
///
/// Drawn under a row so that what is about to be saved can be checked. The
/// server sends the resolved paths; these are the names they have.
const KEPT: Record<AgentType, string[]> = {
  Claude: ["~/.claude", "~/.claude.json"],
  Codex: ["~/.codex"],
  Grok: ["~/.grok"],
  OpenCode: ["~/.config/opencode", "~/.local/share/opencode"],
};

/// Why the press is held, beside it.
const HELD = "Tick an account, or save one.";

/// The step, over the reading the page last had.
export function Accounts(props: {
  /// How this machine stands, as the server last said.
  reading: OnboardingView;

  /// And the step after this one, opened by the press once there is a Profile.
  onwards: () => void;

  /// The step before this one, opened again from an account whose harness is
  /// missing. Offered only while that step is met.
  reopen: () => void;
}): JSX.Element {
  const queries = useQueryClient();

  // Which cards the human has changed their mind about. The tick's own default
  // is the harness being there, and this is what has been said since — kept per
  // harness rather than per card, so a re-read arriving every ten seconds cannot
  // put a tick back that somebody has just taken off.
  const [chosen, setChosen] = createSignal<Partial<Record<AgentType, boolean>>>(
    {},
  );
  const [refused, setRefused] = createSignal<ProfileSaved | null>(null);

  // Whether the form for an account nothing found is open. Its own signal, and
  // for the reason `chosen` above is one: the page is re-read every ten seconds
  // while this step is unmet, and a card somebody has just opened is not
  // something a re-read may shut under them.
  const [naming, setNaming] = createSignal(false);

  // The card's title once it is open, so that the keyboard has somewhere to
  // land. Opening the card is the one press that takes its own control away,
  // and a focus dropped on the page would put the next Tab back at the top of
  // it.
  let named: HTMLParagraphElement | undefined;

  createEffect(() => {
    if (naming()) named?.focus();
  });

  /// Which harness one found account belongs to.
  const agent = (found: AccountView): AgentType => found.account.agent_type;

  /// Whether a card is ticked: what was said about it, or — where nothing has
  /// been — whether its harness is on the machine.
  ///
  /// An account with no binary can never be ticked, whatever was said about it
  /// before the harness was uninstalled: a Profile of it is one no session
  /// could be launched under.
  const ticked = (found: AccountView): boolean =>
    found.harness && (chosen()[agent(found)] ?? true);

  /// The cards a press would save.
  const taking = (): AccountView[] => props.reading.accounts.filter(ticked);

  /// Whether the press is pressable: a Profile is saved already, or this press
  /// would save one.
  const ready = (): boolean =>
    props.reading.steps.accounts || taking().length > 0;

  /// What the press says: what it keeps, or Next where it keeps nothing.
  const press = (): string => {
    const count = taking().length;

    if (count === 0) {
      return "Next";
    }

    return count === 1 ? "Keep 1 profile" : `Keep ${count} profiles`;
  };

  const save = useMutation(() => ({
    // One at a time and stopping at the first refusal, because each is its own
    // Profile: two saved and a third refused is two Profiles and a sentence
    // about the third, rather than nothing at all.
    mutationFn: async (accounts: AccountView[]): Promise<ProfileSaved> => {
      for (const found of accounts) {
        const outcome = await createProfile(asProfile(found));

        if (!took(outcome)) {
          return outcome;
        }
      }

      return "Saved";
    },
    onSuccess: (outcome: ProfileSaved) => {
      read();

      if (outcome !== "Saved") {
        setRefused(outcome);
        return;
      }

      props.onwards();
    },
  }));

  /// What the press does: save whatever is ticked, and go on once it is
  /// written down. A step reopened with everything already saved has nothing
  /// to save and goes straight on.
  const onwards = () => {
    setRefused(null);

    const saving = taking();

    if (saving.length === 0) {
      props.onwards();
      return;
    }

    save.mutate(saving);
  };

  /// Read the wizard and the profiles again, which is what a Profile having
  /// been written down changes: this step's met-ness is the server's answer
  /// about the store, and the settings page's list is the same list.
  const read = () => {
    void queries.invalidateQueries({ queryKey: ["onboarding"] });
    void queries.invalidateQueries({ queryKey: ["profiles"] });
  };

  return (
    <div class={styles.step}>
      <p class={styles.standing}>
        A session runs under an agent profile: an account on this machine and
        the models it can be launched on.
        <Show when={props.reading.accounts.length > 0}>
          {" "}
          Tick the ones to keep.
        </Show>
      </p>

      <Show
        when={props.reading.accounts.length > 0}
        fallback={<Nothing reading={props.reading} />}
      >
        <section class={styles.group}>
          <p class={styles.label}>
            Found in ~
            <span class={styles.gloss}>
              {props.reading.accounts.length === 1
                ? "1 account"
                : `${props.reading.accounts.length} accounts`}
            </span>
          </p>

          <ul class={styles.cards}>
            <For each={props.reading.accounts}>
              {(found) => (
                <Card
                  found={found}
                  ticked={ticked(found)}
                  tick={(on) => {
                    setChosen({ ...chosen(), [agent(found)]: on });
                    setRefused(null);
                  }}
                  reopen={
                    props.reading.steps.dependencies ? props.reopen : undefined
                  }
                />
              )}
            </For>
          </ul>
        </section>
      </Show>

      {/* And the way in for an account that is not in this server's home at
          all — one kept under another login, or on another disk. The same form
          the settings page saves a Profile with, because it is the same
          Profile.

          A dashed card until it is asked for: the press opens it in place
          into the form, and there is no way back — see `naming` above. */}
      <Show
        when={naming()}
        fallback={
          <button
            type="button"
            class={styles.naming}
            aria-expanded="false"
            data-naming
            onClick={() => setNaming(true)}
          >
            <Icon of={faPlus} class={styles.plus} />
            <span class={styles.text}>
              <span class={styles.title}>Name an account yourself</span>
              <span class={styles.quiet}>
                For an account kept outside this server's home.
              </span>
            </span>
          </button>
        }
      >
        <section class={styles.named}>
          {/* Reachable by the focus without standing in the tab order: it is
              what the press that opened the card left behind. */}
          <p ref={named} class={styles.title} tabindex="-1">
            Name an account yourself
          </p>
          <p class={styles.quiet}>
            For an account kept outside this server's home.
          </p>

          <ProfileForm
            initial={() => BLANK_PROFILE}
            submit="Save profile"
            save={createProfile}
            saved={read}
          />
        </section>
      </Show>

      <Show when={refused()}>
        {(outcome) => <ErrorLine>{PROFILE_REFUSAL[outcome()]}</ErrorLine>}
      </Show>
      <Show when={save.isError}>
        <ErrorLine>
          The profile couldn't be saved. {save.error?.message}
        </ErrorLine>
      </Show>

      <Foot>
        <Show when={!ready()}>
          <span class={styles.held}>{HELD}</span>
        </Show>

        <button
          type="button"
          class={styles.next}
          disabled={!ready() || save.isPending}
          onClick={onwards}
        >
          {press()}
          <Icon of={faArrowRight} class={styles.arrow} />
        </button>
      </Foot>
    </div>
  );
}

/// One account that was found, as a card: whose it is, where it is kept, and
/// whether it is being taken on.
function Card(props: {
  found: AccountView;
  ticked: boolean;
  tick: (on: boolean) => void;

  /// Opens step 1, where this account's harness is installed — offered only
  /// while that step is met.
  reopen: (() => void) | undefined;
}): JSX.Element {
  const agent = (): AgentType => props.found.account.agent_type;

  /// The mark, the name and where the account is kept: the card's middle,
  /// whether or not it is a press.
  const reading = (): JSX.Element => (
    <>
      <HarnessMark of={agent()} class={styles.harness} />
      <span class={styles.text}>
        <span class={styles.name}>
          {AGENT_NAME[agent()]}
          <Show when={!props.found.harness}>
            <span class={styles.pill}>Not installed</span>
          </Show>
        </span>
        <span class={styles.kept}>
          <For each={KEPT[agent()]}>{(path) => <code>{path}</code>}</For>
        </span>
        {/* An account whose harness is missing is one there is nothing to
            run: the card is drawn so that it is not a surprise later, and
            step 1 is where it is put right.

            The finding is said in the words every other site says it in — a
            mirror of a member's profile whose harness is not here reads broken
            in this sentence too, see [`../broken.ts`] — and what is added
            after it is this step's own. */}
        <Show when={!props.found.harness}>
          <span class={styles.quiet}>
            {harnessAbsent(agent())} Install it and this account can be taken
            on.
          </span>
        </Show>
      </span>
    </>
  );

  return (
    <li
      class={styles.card}
      classList={{ [styles.chosen!]: props.ticked }}
      data-account={agent()}
      data-harness={props.found.harness ? "yes" : "no"}
    >
      <Show
        when={props.found.harness}
        fallback={
          <div class={styles.inner}>
            {reading()}
            <Show when={props.reopen}>
              {(reopen) => (
                <button
                  type="button"
                  class={styles.reopen}
                  onClick={() => reopen()()}
                >
                  Go to step 1
                </button>
              )}
            </Show>
          </div>
        }
      >
        {/* The whole card is what is ticked, so the card answers the press
            rather than the box a label forwards one to — and stops forwarding
            the moment a hand slides. See `rowPress`. */}
        <label
          class={styles.inner}
          onClick={rowPress(() => props.tick(!props.ticked))}
        >
          {reading()}
          <input
            type="checkbox"
            class={styles.box}
            checked={props.ticked}
            onChange={(ev) => props.tick(ev.currentTarget.checked)}
          />
        </label>
      </Show>
    </li>
  );
}

/// A home with no account in it: a card saying so, that this page is watching
/// for one, and what to run to make one.
///
/// One line per harness that is on the machine, because those are the ones that
/// can be run at all — and every one of them where none is, since a machine
/// with no harness has the step before to finish first and nothing here to
/// choose between.
function Nothing(props: { reading: OnboardingView }): JSX.Element {
  return (
    <div class={styles.nothing}>
      <div class={styles.top}>
        <p class={styles.title}>No account in ~ yet</p>
        {/* The frame's own cadence behind the word, for whoever wants to
            know how often "watching" is. */}
        <span
          class={styles.watching}
          title={`Reading ${props.reading.machine} every ${PROBE / 1000} s`}
        >
          Watching
        </span>
      </div>
      <p class={styles.quiet}>
        Log one in on {props.reading.machine} and it appears here.
      </p>

      <ul class={styles.logins}>
        <For each={runnable(props.reading)}>
          {(agent) => (
            <li class={styles.login} data-login={agent}>
              <span class={styles.name}>
                <HarnessMark of={agent} class={styles.harness} />
                {AGENT_NAME[agent]}
              </span>
              <span class={styles.command}>
                <span class={styles.prompt}>$</span>
                <pre>{LOGIN[agent]}</pre>
                <Copy of={LOGIN[agent]} class={styles.copy} />
              </span>
            </li>
          )}
        </For>
      </ul>
    </div>
  );
}

/// The harnesses a login could be run under here: those on the machine, or all
/// four where none is.
///
/// Read off the dependency rows rather than said again, which is what keeps
/// this list and the step above from disagreeing about what is installed. A
/// harness row and an agent type are written in the same four words on purpose
/// — see `crates/render/src/profiles.rs` — so one is read as the other rather
/// than through a table nobody would maintain.
function runnable(reading: OnboardingView): AgentType[] {
  const harnesses: AgentType[] = ["Claude", "Codex", "Grok", "OpenCode"];
  const there = harnesses.filter((agent) =>
    reading.dependencies.some(
      (row) => row.dependency === agent && row.state.state === "Present",
    ),
  );

  return there.length > 0 ? there : harnesses;
}

/// One found account as the Profile it would be saved as: no name, memory shared,
/// and every model this build knows for its harness.
///
/// Unnamed because a home holds one account per harness, so there is nothing to
/// tell it from — and the models because a Profile with none is refused, while
/// which of them a session runs is picked when the session is set up. Both are
/// the settings page's to change.
function asProfile(found: AccountView): ProfileEdit {
  return {
    name: null,
    account: found.account,
    models: KNOWN_MODELS.filter(
      (model) => model.agent === found.account.agent_type,
    ).map((model) => model.id),
    memory: true,
  };
}

/// Whether saving one account got as far as there being a Profile of it.
///
/// `DefaultTaken` counts: it says that harness already has the unnamed Profile
/// this press would have made, which is what a step opened again and pressed
/// twice is. Every other refusal is about the account itself and is said.
function took(outcome: ProfileSaved): boolean {
  return outcome === "Saved" || outcome === "DefaultTaken";
}
