//! The wizard's second step: the agent accounts already on this machine,
//! offered as Agent Profiles.
//!
//! **The accounts are the server's.** It looks in its own home for the four
//! shapes an account comes in — `crates/server/src/sandbox.rs` says what each is
//! made of, and the wizard reads that same list — so at most one per harness is
//! ever found, and each arrives here as the account the profile create would be
//! handed. Nothing on this side knows what a `.claude` is.
//!
//! **A row is offered ticked where its harness is on the machine and greyed
//! where it is not.** An account whose binary is missing is not something to
//! make a Profile of yet: the step before this one is where that is installed,
//! and this page ticks the row when it lands.
//!
//! **A ticked one is saved with no name and every model this build knows for
//! its harness.** Both are the settings page's to change afterwards; nothing
//! here asks anybody to type either, because an account found in a home is one
//! account and needs no name to tell it from another.
//!
//! **Next needs at least one Profile** — ticked here, or made in the form
//! under the rows. It is one of the three things the objective is and the only
//! one a step could walk past: clearing the mode with nothing saved would land
//! somebody on exactly the empty state a skip was rejected for, until the next
//! start put them through the wizard again.
//!
//! **And nothing found is a state rather than a gap.** It says what to run to
//! make an account, keeps probing on the frame's own ten-second cadence — see
//! [`./SetupPage.tsx`](./SetupPage.tsx) — and the link under it still reaches
//! the form for an account that lives somewhere else entirely.
//!
//! **That form is behind a link.** Naming an account by hand is the way in for
//! a machine whose accounts are kept somewhere other than this server's home,
//! which is the uncommon one: the ordinary machine ticks a row and presses
//! Next, and a form of paths and models standing open under those rows
//! reads as work the step is asking for. So the section starts as its own
//! heading drawn as a link, and a press puts the whole of it — heading,
//! explainer and form — where the link stood. It never shuts again: nothing on
//! this step wants the room back. And it starts shut in the nothing-found state
//! as well, that state already saying what to run and this page reading the
//! machine again every ten seconds, with the link right under it.

import { useMutation, useQueryClient } from "@tanstack/solid-query";
import { For, Show, createEffect, createSignal, type JSX } from "solid-js";

import { HarnessMark } from "../HarnessMark";
import { AGENT_NAME, type AgentType } from "../agents";
import { createProfile } from "../api/client";
import type {
  AccountView,
  OnboardingView,
  ProfileEdit,
  ProfileSaved,
} from "../api/types";
import { KNOWN_MODELS } from "../models";
import { ErrorLine, Note } from "../notices";
import { BLANK_PROFILE, PROFILE_REFUSAL, ProfileForm } from "../profiles/ProfileList";
import { Mark } from "./Mark";
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

/// The step, over the reading the page last had.
export function Accounts(props: {
  /// How this machine stands, as the server last said.
  reading: OnboardingView;

  /// And the step after this one, opened by Next once there is a Profile.
  onwards: () => void;
}): JSX.Element {
  const queries = useQueryClient();

  // Which rows the human has changed their mind about. The tick's own default
  // is the harness being there, and this is what has been said since — kept per
  // harness rather than per row, so a re-read arriving every ten seconds cannot
  // put a tick back that somebody has just taken off.
  const [chosen, setChosen] = createSignal<Partial<Record<AgentType, boolean>>>(
    {},
  );
  const [refused, setRefused] = createSignal<ProfileSaved | null>(null);

  // Whether the form for an account nothing found is open. Its own signal, and
  // for the reason `chosen` above is one: the page is re-read every ten seconds
  // while this step is unmet, and a section somebody has just opened is not
  // something a re-read may shut under them.
  const [naming, setNaming] = createSignal(false);

  // The heading that stands where the link stood, so that the keyboard has
  // somewhere to land. Opening the section is the one press that takes its own
  // control away, and a focus dropped on the page would put the next Tab back
  // at the top of it.
  let named: HTMLHeadingElement | undefined;

  createEffect(() => {
    if (naming()) named?.focus();
  });

  /// Which harness one found account belongs to.
  const agent = (found: AccountView): AgentType => found.account.agent_type;

  /// Whether a row is ticked: what was said about it, or — where nothing has
  /// been — whether its harness is on the machine.
  ///
  /// An account with no binary can never be ticked, whatever was said about it
  /// before the harness was uninstalled: a Profile of it is one no session
  /// could be launched under.
  const ticked = (found: AccountView): boolean =>
    found.harness && (chosen()[agent(found)] ?? true);

  /// The rows a press of Next would save.
  const taking = (): AccountView[] => props.reading.accounts.filter(ticked);

  /// Whether Next is pressable: a Profile is saved already, or this press
  /// would save one.
  const ready = (): boolean =>
    props.reading.steps.accounts || taking().length > 0;

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

  /// What Next does: save whatever is ticked, and go on once it is written
  /// down. A step reopened with everything already saved has nothing to save
  /// and goes straight on.
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
        A session runs under an Agent Profile: an account on this machine, and
        the models it can be launched on. What was found in this server's home
        is offered below.
      </p>

      <Show
        when={props.reading.accounts.length > 0}
        fallback={<Nothing reading={props.reading} />}
      >
        <ul class={styles.rows}>
          <For each={props.reading.accounts}>
            {(found) => (
              <Row
                found={found}
                ticked={ticked(found)}
                tick={(on) => {
                  setChosen({ ...chosen(), [agent(found)]: on });
                  setRefused(null);
                }}
              />
            )}
          </For>
        </ul>
      </Show>

      <div class={styles.onwards}>
        <button
          type="button"
          class={styles.next}
          disabled={!ready() || save.isPending}
          onClick={onwards}
        >
          Next
        </button>

        <Show when={!ready()}>
          <Note>
            Verkstead runs nothing without an Agent Profile. Tick an account
            above, or save one from the form below.
          </Note>
        </Show>

        <Show when={refused()}>
          {(outcome) => (
            <ErrorLine class={styles.failure}>
              {PROFILE_REFUSAL[outcome()]}
            </ErrorLine>
          )}
        </Show>
        <Show when={save.isError}>
          <ErrorLine class={styles.failure}>
            The profile could not be saved: {save.error?.message}
          </ErrorLine>
        </Show>
      </div>

      {/* And the way in for an account that is not in this server's home at
          all — one kept under another login, or on another disk. The same form
          the settings page saves a Profile with, because it is the same
          Profile.

          Behind a link until it is asked for, and the heading is the link: the
          press replaces it with the section it names, and there is no way back
          — see `naming` above. */}
      <section class={styles.byHand}>
        <Show
          when={naming()}
          fallback={
            <button
              type="button"
              class={styles.naming}
              aria-expanded="false"
              onClick={() => setNaming(true)}
            >
              Or name an account yourself
            </button>
          }
        >
          {/* Reachable by the focus without standing in the tab order: it is
              what the press that opened the section left behind. */}
          <h3 ref={named} tabindex="-1">
            Or name an account yourself
          </h3>
          <p class={styles.standing}>
            An account kept somewhere other than this server's own home is named
            here, the way the settings page names one.
          </p>

          <ProfileForm
            initial={() => BLANK_PROFILE}
            submit="Save"
            save={createProfile}
            saved={read}
          />
        </Show>
      </section>
    </div>
  );
}

/// One account that was found: whose it is, where it is kept, and whether it is
/// being taken on.
function Row(props: {
  found: AccountView;
  ticked: boolean;
  tick: (on: boolean) => void;
}): JSX.Element {
  const agent = (): AgentType => props.found.account.agent_type;

  return (
    <li
      class={styles.row}
      data-account={agent()}
      data-harness={props.found.harness ? "yes" : "no"}
    >
      <label class={styles.head}>
        <input
          type="checkbox"
          checked={props.ticked}
          disabled={!props.found.harness}
          onChange={(ev) => props.tick(ev.currentTarget.checked)}
        />
        <Mark standing={props.ticked ? "met" : "waiting"} />
        <span class={styles.name}>
          <HarnessMark of={agent()} class={styles.harness} />
          {AGENT_NAME[agent()]}
        </span>
      </label>

      <p class={styles.kept}>
        <For each={KEPT[agent()]}>{(path) => <code>{path}</code>}</For>
      </p>

      {/* An account whose harness is missing is one there is nothing to run:
          the row is drawn so that it is not a surprise later, and the step
          before this one is where it is put right. */}
      <Show when={!props.found.harness}>
        <Note>
          {AGENT_NAME[agent()]} is not on this machine. Install it in the step
          above and this account can be taken on.
        </Note>
      </Show>
    </li>
  );
}

/// A home with no account in it: what to run to make one, and the promise that
/// this page will notice.
///
/// One line per harness that is on the machine, because those are the ones that
/// can be run at all — and every one of them where none is, since a machine
/// with no harness has the step above to finish first and nothing here to
/// choose between.
function Nothing(props: { reading: OnboardingView }): JSX.Element {
  return (
    <div class={styles.nothing}>
      <p class={styles.standing}>
        No agent account was found in this server's home. Log one in and it is
        offered here — this page reads the machine again every ten seconds.
      </p>

      <ul class={styles.logins}>
        <For each={runnable(props.reading)}>
          {(agent) => (
            <li class={styles.login} data-login={agent}>
              <span class={styles.name}>
                <HarnessMark of={agent} class={styles.harness} />
                {AGENT_NAME[agent]}
              </span>
              <pre>{LOGIN[agent]}</pre>
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

/// One found account as the Profile it would be saved as: no name, and every
/// model this build knows for its harness.
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
