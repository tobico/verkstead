//! The wizard's first step: what a session needs, what this machine is missing,
//! and the press that installs it.
//!
//! **Three screens over one reading**, and which of them is open is the whole of
//! this component's own state — see [`./steps.ts`](./steps.ts), where it is kept
//! beside the open step and for the same reason.
//!
//! **The checkbox screen** is where the step opens. Every row the server drew is
//! on it — the sandbox, `git`, the four harnesses and `gh` — and a row this
//! machine has not got carries a checkbox where its mark would be, with the
//! row's name as the label. The sandbox and `git` start ticked, those being the
//! two rows nothing can run without; the harnesses and `gh` start unticked,
//! because one harness is enough and GitHub is a choice. Next is pressable once
//! the rows that are there plus the rows that are ticked would be a machine a
//! session could start on, and what it does is start the run.
//!
//! **Nothing here says in advance which rows cannot be installed.** What a
//! package manager carries is a fact about the distribution rather than about
//! the row, so the run is what reports it — see
//! `crates/server/src/onboarding/install.rs`, which fails a row no archive
//! carries in the machine's own words.
//!
//! **The install screen** is the run: a bar of one unit per ticked row, the
//! status line the server writes, and Cancel. It moves on by itself when the
//! run is over — to the next step where every ticked row landed, and to the
//! hint screen where one did not.
//!
//! **The hint screen** is the step as it used to be, cut down to the rows that
//! are still missing: the eight tabs of instructions, the list of directories a
//! session looks in, and the note that the list was read once. Next is held
//! until every one of those rows is detected, with the count beside it, and
//! Back returns to the checkbox screen — which is how a row nobody wants after
//! all is unticked.
//!
//! **A row that is there says which file it is**, and a row that is not says
//! where the name was seen — the two halves this step gained when a session's
//! `PATH` became the human's own. A stale `claude` in `/usr/bin` shadowing the
//! current one under `~/.local/bin` is a tick either way and two different
//! programs, and a `claude` on the server's `PATH` that no session's holds is a
//! `PATH` to fix rather than a program to install. See [`Where`] and [`Seen`]
//! below, which are the words for each.
//!
//! **And where a session looks is the server's list**, drawn above the hint
//! screen's rows on every platform: a session's `PATH` is composed out of the
//! one Verkstead was started with, so no sentence written here could say which
//! directories those are. **Read once, at startup** — which is the other half of
//! that and the one thing somebody who has just installed something by hand has
//! to be told, so [`RESTART`] is under the list on every tab.
//!
//! **Which instruction is a tab rather than a fact.** The detected OS opens and
//! the other seven are a press away — see
//! [`./instructions.ts`](./instructions.ts) — because the detection comes off
//! `/etc/os-release` and a derivative names its parent. Nothing about the rows
//! changes with the tab: the tab is which machine the commands are *for*, and
//! whether a program is there is a fact about the machine the server is on.

import { useMutation, useQueryClient } from "@tanstack/solid-query";
import {
  For,
  Match,
  Show,
  Switch,
  createEffect,
  createSignal,
  onCleanup,
  type JSX,
} from "solid-js";

import { Copy } from "../Copy";
import { HarnessMark } from "../HarnessMark";
import { AGENT_NAME, type AgentType } from "../agents";
import { cancelInstall, startInstall } from "../api/client";
import type {
  Dependency,
  DependencyState,
  DependencyView,
  Distro,
  OnboardingView,
  RunView,
  Seen as SeenSomewhere,
} from "../api/types";
import { ErrorLine, Note } from "../notices";
import { Mark, type Standing } from "./Mark";
import { DISTROS, GUIDES, type Instruction } from "./instructions";
import {
  PROBE,
  RUNNING,
  keepScreen,
  openScreen,
  type Screen,
} from "./steps";
import styles from "./Dependencies.module.css";

/// What each row is called where somebody reads it.
///
/// The harnesses are [`AGENT_NAME`]'s, which is what they are called everywhere
/// else in the app: a row saying *Claude* beside a Profile list saying *Claude
/// Code* would be two names for one backend.
const NAMES: Record<Dependency, string> = {
  Sandbox: "A sandbox",
  Git: "git",
  Claude: AGENT_NAME.Claude,
  Codex: AGENT_NAME.Codex,
  Grok: AGENT_NAME.Grok,
  OpenCode: AGENT_NAME.OpenCode,
  Gh: "gh",
};

/// And what each is for, which is also which of them hold the step up.
const WHY: Record<Dependency, string> = {
  Sandbox: "Every session runs inside one",
  Git: "Every Conversation is a branch and a worktree",
  Claude: "One harness is enough",
  Codex: "One harness is enough",
  Grok: "One harness is enough",
  OpenCode: "One harness is enough",
  Gh: "Optional: pull requests and reviews",
};

/// And what a row reads instead, where this platform has no such thing to have:
/// the Windows sandbox row, whose whole answer is the note under it.
const MOOT = "Not applicable";

/// What the list of directories above the hint screen's rows is.
///
/// The server's own, rather than a sentence about where a session looks on this
/// kind of machine: a session's `PATH` is composed out of the one Verkstead was
/// started with, so the list is a fact about this box and nothing written here
/// could stand in for it.
const LOOKS = "Where a session looks for a program, in order:";

/// And that the list is one reading rather than a live one.
///
/// Verkstead reads its `PATH` when it starts and composes every session's out of
/// it, so a directory put on a shell's `PATH` after that is a directory no
/// session has — which is the whole of what somebody whose fresh install has not
/// ticked needs to know. Said once here rather than eight times over: it is a
/// fact about Verkstead rather than about any one machine.
const RESTART =
  "Verkstead reads that PATH once, when it starts. A directory added to it " +
  "since — the one a new install landed in — is one no session has until " +
  "Verkstead is started again from a shell whose PATH names it.";

/// And the word over the one install a whole tab stands on — the Mac's
/// Homebrew, which is what every command under it is run with.
const FIRST = "What every command below is run with:";

/// Which rows are a harness, so that the row wears the same mark the rest of the
/// app draws that backend with.
const HARNESSES: Partial<Record<Dependency, AgentType>> = {
  Claude: "Claude",
  Codex: "Codex",
  Grok: "Grok",
  OpenCode: "OpenCode",
};

/// The four of them, as the objective reads them: one is enough.
const HARNESS = Object.keys(HARNESSES) as Dependency[];

/// And the two rows that start ticked: the ones a session cannot start without.
///
/// A harness is needed too, and no harness is: which of the four somebody wants
/// is theirs to say, and a wizard that ticked one for them would install an
/// agent nobody asked for.
const GATING: Dependency[] = ["Sandbox", "Git"];

/// The step, over the reading the page last had.
export function Dependencies(props: {
  /// How this machine stands, as the server last said.
  reading: OnboardingView;

  /// And the step after this one, opened by Next once there is nothing left to
  /// install.
  onwards: () => void;

  /// How often the frame is to read the machine again while this step is open,
  /// or `null` where it is to follow its own rule.
  ///
  /// The cadence is a fact about what this step is doing — a run going is read
  /// every two seconds, and the hint screen goes on reading after the objective
  /// is met — and the query is the frame's. See
  /// [`./SetupPage.tsx`](./SetupPage.tsx).
  poll: (every: number | null) => void;
}): JSX.Element {
  const queries = useQueryClient();

  // Which screen is open. Kept on the device, so a reload lands where the human
  // was — and overruled by a run that is going, which is the one thing about
  // this step that is on the wire.
  const [screen, setScreen] = createSignal<Screen>(opening(props.reading));

  // Which rows the human has changed their mind about, over the default a row
  // starts at. Kept per row rather than rebuilt from the reading, because the
  // reading arrives again every few seconds and a tick somebody has just taken
  // off is not something a re-read may put back.
  //
  // Seeded from the run where there is one: what was ticked is this device's own
  // and nowhere on the wire, so a reload in the middle of a run has only the
  // rows the run has touched to go on.
  const [chosen, setChosen] = createSignal<Partial<Record<Dependency, boolean>>>(
    props.reading.run === null ? {} : touched(props.reading),
  );

  // Which rows the hint screen is about: the ticked ones that were still
  // missing when the run ended. Fixed when that screen opens rather than read
  // off the reading, so that a row landing while somebody watches moves the
  // counter instead of leaving the list.
  const [hinted, setHinted] = createSignal<Dependency[]>(
    screen() === "hints" ? failing(props.reading) : [],
  );

  // And what the server said about a press it would not take.
  const [refused, setRefused] = createSignal<string | null>(null);

  /// Open a screen, and remember on this device that it is the one open.
  const show = (screen: Screen): void => {
    setScreen(screen);
    keepScreen(screen);
  };

  /// One row of the reading, by what it is about.
  const rowFor = (dependency: Dependency): DependencyView | undefined =>
    props.reading.dependencies.find((row) => row.dependency === dependency);

  /// Whether a row is ticked: what was said about it, or — where nothing has
  /// been — whether it is one of the two that start ticked.
  ///
  /// A row that is *there* is never ticked, whatever was said about it before
  /// it landed: the checkbox is what stands where a present row's mark is, so a
  /// row that has just been installed has no tick to carry.
  const ticked = (row: DependencyView): boolean =>
    absent(row.state) &&
    (chosen()[row.dependency] ?? GATING.includes(row.dependency));

  /// The rows a press would install — which, a tick being an absent row's, are
  /// also the ones the run has not delivered yet.
  const taking = (): DependencyView[] =>
    props.reading.dependencies.filter(ticked);

  /// Whether Next is pressable: the rows that are there, plus the rows that are
  /// ticked, are a sandbox, `git` and at least one harness.
  ///
  /// The viewer's own arithmetic rather than the server's verdict on the step,
  /// because what is being asked about is the machine this press would leave
  /// behind rather than the one in front of it.
  const would = (): boolean => {
    const settled = (dependency: Dependency): boolean => {
      const row = rowFor(dependency);

      return row !== undefined && (!absent(row.state) || ticked(row));
    };

    return settled("Sandbox") && settled("Git") && HARNESS.some(settled);
  };

  /// The rows the hint screen draws, as the reading has them now.
  const hinting = (): DependencyView[] =>
    props.reading.dependencies.filter((row) =>
      hinted().includes(row.dependency),
    );

  /// And how many of them this machine has since been found to have.
  const detected = (): number =>
    hinting().filter((row) => !absent(row.state)).length;

  const start = useMutation(() => ({
    mutationFn: (dependencies: Dependency[]) => startInstall(dependencies),
    onSuccess: (reading: OnboardingView) => {
      // Handed over rather than invalidated: the press already has the machine
      // read again with the run on it, and the install screen is drawn from
      // exactly that.
      queries.setQueryData(["onboarding"], reading);
      show("installing");
    },
    onError: (error: Error) => setRefused(error.message),
  }));

  const stop = useMutation(() => ({
    mutationFn: cancelInstall,
    onSuccess: (reading: OnboardingView) =>
      queries.setQueryData(["onboarding"], reading),
  }));

  /// What Next does on the checkbox screen: start the run over what is ticked,
  /// and — where nothing is ticked — go straight on, there being nothing to
  /// install.
  const press = (): void => {
    setRefused(null);

    const installing = taking().map((row) => row.dependency);

    if (installing.length === 0) {
      show("choosing");
      props.onwards();
      return;
    }

    // What was ticked is settled by the press: the run moves every row's state
    // under this page, and a default read off those states would be a different
    // set of rows a moment later.
    setChosen(
      Object.fromEntries(
        props.reading.dependencies.map((row) => [
          row.dependency,
          installing.includes(row.dependency),
        ]),
      ),
    );

    start.mutate(installing);
  };

  // How often the frame reads the machine again while this step is open, which
  // is what makes the install screen move and what keeps the hint screen
  // watching after the objective is met.
  createEffect(() => props.poll(cadence()));
  onCleanup(() => props.poll(null));

  /// Which of the two it is, or nothing where the frame's own rule is right.
  const cadence = (): number | null => {
    if (running(props.reading.run)) {
      return RUNNING;
    }

    return screen() === "hints" && detected() < hinted().length ? PROBE : null;
  };

  // The install screen moves on by itself once the run is over: to the next
  // step where every ticked row landed, and to the hint screen where one did
  // not. Nobody is asked to press anything about a run they watched finish.
  createEffect(() => {
    if (
      screen() !== "installing" ||
      props.reading.run === null ||
      running(props.reading.run)
    ) {
      return;
    }

    const left = taking().map((row) => row.dependency);

    if (left.length === 0) {
      show("choosing");
      props.onwards();
      return;
    }

    setHinted(left);
    show("hints");
  });

  return (
    <div class={styles.step}>
      <Switch>
        <Match when={screen() === "choosing"}>
          <Choosing
            reading={props.reading}
            ticked={ticked}
            tick={(row, on) =>
              setChosen({ ...chosen(), [row.dependency]: on })
            }
            ready={would()}
            pressing={start.isPending}
            press={press}
            refused={refused()}
          />
        </Match>

        <Match
          when={
            screen() === "installing" ? props.reading.run : null
          }
        >
          {(run) => (
            <Installing
              run={run()}
              stopping={stop.isPending}
              cancel={() => stop.mutate()}
            />
          )}
        </Match>

        <Match when={screen() === "hints"}>
          <Hints
            reading={props.reading}
            rows={hinting()}
            detected={detected()}
            back={() => show("choosing")}
            onwards={() => {
              show("choosing");
              props.onwards();
            }}
          />
        </Match>
      </Switch>
    </div>
  );
}

/// Which screen a reading opens on.
///
/// A run that is going is the install screen whatever this device was left on —
/// somebody pressed Next here a moment ago, or on the phone beside it. Anything
/// else is the checkbox screen, unless the device was left past it and there is
/// still something the run could not install to say.
function opening(reading: OnboardingView): Screen {
  if (running(reading.run)) {
    return "installing";
  }

  return openScreen() !== "choosing" && failing(reading).length > 0
    ? "hints"
    : "choosing";
}

/// Whether a run is still going: one that has been started and has not reached
/// its end.
export function running(run: RunView | null | undefined): boolean {
  return run !== null && run !== undefined && run.phase !== "Done";
}

/// Which rows the run could not install, as far as the reading says.
///
/// What was ticked is this device's and nowhere on the wire, so this is what a
/// reload has to go on: a row the run failed is a row somebody ticked, and it
/// is the only kind the hint screen ever has anything to say about.
function failing(reading: OnboardingView): Dependency[] {
  return reading.dependencies
    .filter((row) => absent(row.state) && row.install.install === "Failed")
    .map((row) => row.dependency);
}

/// And which rows a run that is still going has touched, as the ticks a reload
/// stands in with.
function touched(
  reading: OnboardingView,
): Partial<Record<Dependency, boolean>> {
  return Object.fromEntries(
    reading.dependencies.map((row) => [
      row.dependency,
      row.install.install !== "Idle",
    ]),
  );
}

/// Whether a row is one this machine has not got — which is the one state that
/// carries a checkbox, and the one the objective is not met by.
function absent(state: DependencyState): boolean {
  return state.state === "Absent";
}

/// The checkbox screen: every row, a tick to put on the ones that are missing,
/// and the press that installs them.
function Choosing(props: {
  reading: OnboardingView;
  ticked: (row: DependencyView) => boolean;
  tick: (row: DependencyView, on: boolean) => void;
  ready: boolean;
  pressing: boolean;
  press: () => void;
  refused: string | null;
}): JSX.Element {
  return (
    <>
      <p class={styles.standing}>
        A session runs inside a sandbox, works in a git worktree, and is one of
        four agents. Tick what this machine is missing and Verkstead installs
        it.
      </p>

      <ul class={styles.rows}>
        <For each={props.reading.dependencies}>
          {(row) => (
            <Row
              row={row}
              ticked={props.ticked(row)}
              tick={(on) => props.tick(row, on)}
            />
          )}
        </For>
      </ul>

      <div class={styles.onwards}>
        <button
          type="button"
          class={styles.next}
          disabled={!props.ready || props.pressing}
          onClick={() => props.press()}
        >
          Next
        </button>

        <Show when={!props.ready}>
          <Note>
            A sandbox, git and one of the four agents are what a session cannot
            start without. This page reads the machine again every ten seconds,
            so an install that lands ticks its row here.
          </Note>
        </Show>

        <Show when={props.refused}>
          {(why) => <ErrorLine class={styles.failure}>{why()}</ErrorLine>}
        </Show>
      </div>
    </>
  );
}

/// The install screen: how far the run has got, what it is doing, and the press
/// that stops it.
///
/// No rows on it. What the run has made of each of them is what the screen
/// after this one is about, and a list of seven moving under a progress bar
/// would be two accounts of the same thing.
function Installing(props: {
  run: RunView;
  stopping: boolean;
  cancel: () => void;
}): JSX.Element {
  return (
    <>
      <p class={styles.standing}>
        Verkstead is installing what you ticked. One password dialog covers the
        packages this machine's own package manager carries.
      </p>

      <div
        class={styles.run}
        data-done={props.run.done}
        data-total={props.run.total}
      >
        <progress
          class={styles.bar}
          value={props.run.done}
          max={props.run.total}
        />
        <span class={styles.count}>
          {props.run.done}/{props.run.total}
        </span>
      </div>

      <p class={styles.status} data-status>
        {props.run.status}
      </p>

      <div class={styles.onwards}>
        {/* A press cannot stop a package manager that is already unpacking — a
            half-installed machine is worse than a fully installed one — so
            Cancel skips what has not started and says so until the unit under
            way is over. */}
        <button
          type="button"
          class={styles.cancel}
          disabled={props.run.cancelling || props.stopping}
          onClick={() => props.cancel()}
        >
          {props.run.cancelling ? "Cancelling…" : "Cancel"}
        </button>
      </div>
    </>
  );
}

/// The hint screen: the rows Verkstead could not install here, what to run for
/// each of them, and the press that is held until they are detected.
function Hints(props: {
  reading: OnboardingView;
  rows: DependencyView[];
  detected: number;
  back: () => void;
  onwards: () => void;
}): JSX.Element {
  // Which OS the commands are for. The detected one to begin with, and the
  // human's after they have picked: a machine whose `ID_LIKE` sent them to the
  // wrong tab is exactly who this control is for, and the reading re-arriving
  // every ten seconds must not put them back.
  const [showing, setShowing] = createSignal<Distro>(props.reading.distro);

  const guide = () => GUIDES[showing()];

  /// Whether every row this screen is about has since been found.
  const met = (): boolean => props.detected === props.rows.length;

  return (
    <>
      <p class={styles.standing}>
        The rest has to be installed by hand on the machine Verkstead is running
        on. This page reads the machine again while you do it.
      </p>

      <div
        class={styles.tabs}
        role="group"
        aria-label="What these commands are for"
      >
        <For each={DISTROS}>
          {(distro) => (
            <button
              type="button"
              class={styles.tab}
              data-distro={distro}
              aria-pressed={showing() === distro}
              onClick={() => setShowing(distro)}
            >
              {GUIDES[distro].title}
            </button>
          )}
        </For>
      </div>

      <Show when={props.reading.path.length > 0}>
        <div class={styles.looks}>
          <p class={styles.looking}>{LOOKS}</p>
          <ol class={styles.path}>
            <For each={props.reading.path}>
              {(entry) => (
                <li>
                  <code>{entry}</code>
                </li>
              )}
            </For>
          </ol>
        </div>
      </Show>

      <Note class={styles.restart}>{RESTART}</Note>

      {/* What every command on this tab wants first, where they all want the
          same thing: the Mac's Homebrew, drawn once above the rows rather than
          under each of the five that are a `brew install`. */}
      <Show when={guide().before}>
        {(first) => (
          <div class={styles.before} data-before>
            <p class={styles.firstly}>{FIRST}</p>
            <Instructed of={first()} />
          </div>
        )}
      </Show>

      <ul class={styles.rows}>
        <For each={props.rows}>
          {(row) => (
            <Row row={row} instruction={guide().rows[row.dependency]} />
          )}
        </For>
      </ul>

      <div class={styles.onwards}>
        {/* Back rather than a way of unticking a row here: what a row nobody
            wants after all is, is a tick to take off, and the screen with the
            ticks on it is the one before this. */}
        <button type="button" class={styles.back} onClick={() => props.back()}>
          Back
        </button>

        <button
          type="button"
          class={styles.next}
          disabled={!met()}
          onClick={() => props.onwards()}
        >
          Next
        </button>

        <span class={styles.counter} data-detected>
          {props.detected}/{props.rows.length} detected
        </span>
      </div>
    </>
  );
}

/// One row: what it is, whether this machine has it, what the run made of it,
/// and — where the screen draws one — what to run by hand.
function Row(props: {
  row: DependencyView;

  /// Whether the row is ticked, and the press that changes that. Handed down by
  /// the checkbox screen alone, and drawn only on a row that is missing: a row
  /// that is there has its mark where the box would be.
  ticked?: boolean;
  tick?: (on: boolean) => void;

  /// And what to run by hand, which the hint screen draws and no other does.
  instruction?: Instruction;
}): JSX.Element {
  const state = (): DependencyState => props.row.state;

  /// Whether this row is one to tick: the screen offers it, and the machine has
  /// not got it.
  const choosing = (): boolean => props.tick !== undefined && absent(state());

  /// What a failed run said, which is only ever the Linux sandbox row's: a
  /// `bwrap` that is installed and will not make a namespace says why in its
  /// own words, and no sentence written here would be as useful.
  const trouble = (): string | null => {
    const said = state();

    return said.state === "Absent" ? said.trouble : null;
  };

  /// And what the install run made of it, where it failed: a dismissed password
  /// dialog, a package manager's own refusal, or a row no archive here carries.
  const failed = (): string | null => {
    const made = props.row.install;

    return made.install === "Failed" ? made.why : null;
  };

  /// Which file a row that is there found, where it named one at all.
  const found = (): Found | null => {
    const said = state();

    return said.state === "Present" && said.at
      ? { at: said.at, target: said.target }
      : null;
  };

  /// And where the name was seen, where a row that is not there saw one: a
  /// program on this machine that no session can open.
  const seen = (): SeenSomewhere | null => {
    const said = state();

    return said.state === "Absent" ? said.seen : null;
  };

  /// The head, whichever stands at the front of it — the mark, or the box that
  /// is the whole of what the checkbox screen asks for.
  const head = (leading: JSX.Element): JSX.Element => (
    <>
      {leading}
      <span class={styles.name}>
        <HarnessMark
          of={HARNESSES[props.row.dependency] ?? null}
          class={styles.harness}
        />
        {NAMES[props.row.dependency]}
      </span>
      <span class={styles.why}>
        {state().state === "NotApplicable" ? MOOT : WHY[props.row.dependency]}
      </span>
    </>
  );

  return (
    <li
      class={styles.row}
      data-dependency={props.row.dependency}
      data-state={state().state}
    >
      <Show
        when={choosing()}
        fallback={
          <div class={styles.head}>
            {head(<Mark standing={standing(state())} />)}
          </div>
        }
      >
        {/* The name labels the box by being inside it — the same wrapping the
            accounts step's rows use, and the whole of what a row is asking. */}
        <label class={styles.head}>
          {head(
            <input
              type="checkbox"
              checked={props.ticked ?? false}
              onChange={(ev) => props.tick?.(ev.currentTarget.checked)}
            />,
          )}
        </label>
      </Show>

      <Show when={failed()}>
        {(why) => (
          <p class={styles.failed} data-failed>
            {why()}
          </p>
        )}
      </Show>

      <Show when={trouble()}>
        {(said) => <pre class={styles.trouble}>{said()}</pre>}
      </Show>

      <Show when={found()}>{(file) => <Where of={file()} />}</Show>

      <Show when={seen()}>{(where) => <Seen at={where()} />}</Show>

      {/* No instruction under a row that is already there: what to install is
          all an instruction says, and this one is installed. */}
      <Show when={state().state === "Present" ? null : props.instruction}>
        {(instruction) => <Instructed of={instruction()} />}
      </Show>
    </li>
  );
}

/// Which file a present row found: the path the name resolved to, and what it
/// lands on where that path is a link.
///
/// A row that named no file has none of this — the sandbox on a Mac and on
/// Windows is not a program anybody went looking for.
type Found = {
  at: string;
  target: string | null;
};

/// That file, under the row.
///
/// The second path only where the two differ, which the server has already
/// decided: a program that is no link has one path and would read as two.
function Where(props: { of: Found }): JSX.Element {
  return (
    <p class={styles.where} data-where>
      <code data-at>{props.of.at}</code>
      <Show when={props.of.target}>
        {(target) => (
          <span class={styles.into}>
            {" "}
            links to <code data-target>{target()}</code>
          </span>
        )}
      </Show>
    </p>
  );
}

/// And where a row that is *not* there saw the name: a program the human has
/// that no session can open.
///
/// Three sentences for the three ways that happens, because they are three
/// different things to do about it — a directory to put on the `PATH` Verkstead
/// is started with, an install to move somewhere a session can reach, and a
/// link left behind by an install that has gone. A row with none of them saw
/// the name nowhere at all, and what it says is the instruction under it.
function Seen(props: { at: SeenSomewhere }): JSX.Element {
  return (
    <Note class={styles.seen}>
      <span data-seen={props.at.seen}>
        <Show when={props.at.seen === "Beyond"}>
          Found at <code>{props.at.at}</code>, which is not on the PATH a session
          gets — so a session cannot open it.
        </Show>

        <Show when={props.at.seen === "Leading" ? props.at : null}>
          {(leading) => (
            <>
              <code>{leading().at}</code> is a link to{" "}
              <code>{leading().target}</code>, which a session cannot reach.
            </>
          )}
        </Show>

        <Show when={props.at.seen === "Dangling"}>
          <code>{props.at.at}</code> is a link with nothing at the end of it.
        </Show>
      </span>
    </Note>
  );
}

/// The word over an instruction's alternative, which is the whole of what has
/// to be said about it: the two are the same program, and which one a machine
/// wants is the reader's to decide.
const OR = "Or, from this machine's own package manager:";

/// What to run, where to get it, and what neither says for itself.
///
/// Once more under itself where the instruction keeps an alternative, which goes
/// one level deep and no further: the second install is a command with a note
/// exactly as the first is, so it is drawn by the same function rather than by a
/// smaller copy of it.
function Instructed(props: { of: Instruction }): JSX.Element {
  return (
    <div class={styles.instruction}>
      <Show when={props.of.command}>
        {(command) => (
          <div class={styles.command}>
            <pre>{command()}</pre>
            {/* Somebody is going to paste this into a terminal, and a phone
                cannot select a line of it usefully. */}
            <Copy of={command()} class={styles.copy} />
          </div>
        )}
      </Show>

      <Show when={props.of.link}>
        {(href) => (
          <p class={styles.vendor}>
            <a href={href()} target="_blank" rel="noreferrer">
              {href()}
            </a>
          </p>
        )}
      </Show>

      <Show when={props.of.note}>{(note) => <Note>{note()}</Note>}</Show>

      {/* And the other way to get the same program, under the one that leads:
          Claude Code's row is the vendor's installer with this machine's own
          package kept beneath it. */}
      <Show when={props.of.alternative}>
        {(other) => (
          <div class={styles.alternative}>
            <p class={styles.or}>{OR}</p>
            <Instructed of={other()} />
          </div>
        )}
      </Show>
    </div>
  );
}

/// How a row's state stands, as the mark beside its name.
function standing(state: DependencyState): Standing {
  switch (state.state) {
    case "Present":
      return "met";
    case "NotApplicable":
      return "moot";
    default:
      return "waiting";
  }
}
