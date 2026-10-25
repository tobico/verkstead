//! The wizard's first step: what a session needs, what this machine is missing,
//! and the press that installs it.
//!
//! **Three screens over one reading**, and which of them is open is the whole of
//! this component's own state — see [`./steps.ts`](./steps.ts), where it is kept
//! beside the open step and for the same reason.
//!
//! **The checkbox screen** is where the step opens. Every row the server drew is
//! on it, in three groups — *Required*, the sandbox and `git`; *An agent*, the
//! four harnesses as tiles; and *Optional*, `gh` and, on Windows, WSL — and a
//! row this machine has not got carries a checkbox, with the row's name as the
//! label. A row that is there carries a tick and an *Installed* pill instead,
//! and nothing else: which file it found is the by-hand screen's to say. The sandbox and `git` start ticked,
//! those being the two rows nothing can run without; the harnesses and `gh`
//! start unticked, because one harness is enough and GitHub is a choice. WSL
//! starts unticked too, being a large download and maybe a restart, until the
//! machine is part of the way to it — see [`ticksItself`] — and a machine whose
//! firmware has virtualization off draws it refused, with no box. Next is
//! pressable once the rows that are there plus the rows that are ticked would
//! be a machine a session could start on, and what it does is start the run —
//! which is why it is called *Install N* for the rows it would install, and
//! *Next* where there are none. *Install by hand instead* beside it opens the
//! by-hand screen over every row this machine is missing, with no run.
//!
//! **Nothing here says in advance which rows cannot be installed.** What a
//! package manager carries is a fact about the distribution rather than about
//! the row, so the run is what reports it — see
//! `crates/server/src/onboarding/install.rs`, which fails a row no archive
//! carries in the machine's own words.
//!
//! **The install screen** is the run, titled *Installing*: how many of the
//! ticked rows are in over a bar of one unit each, the status line the server
//! writes — in the ember callout while it is waiting on a password dialog, the
//! one moment there is something to do and somewhere else to do it — and the
//! ticked rows, the one the run is on saying so. Cancel is its only press. It
//! moves on by itself when the run is over — to the next step where every
//! ticked row landed, and to the by-hand screen where one did not.
//!
//! **The by-hand screen** is titled *Install the Rest by Hand*: the rows still
//! missing — the ticked ones the run failed, or every missing one where the
//! human chose to install by hand — each a block with what the run said behind
//! its *Failed* pill, the command to paste, and any other way to get it behind
//! a link. A select says which machine the commands are for, and the box under
//! the rows is where a session looks, read once. Next is released once the step's
//! objective is met, whichever way the screen was reached, with how many of the
//! rows have since been found beside it; after a run it reads *Skip* while one
//! the run failed is still missing, which is what pressing it then does. Back
//! returns to the checkbox screen — which is how a row nobody wants after all is
//! unticked.
//!
//! **A row that is there says which file it is**, on the by-hand screen, and a
//! row that is not says
//! where the name was seen — the two halves this step gained when a session's
//! `PATH` became the human's own. A stale `claude` in `/usr/bin` shadowing the
//! current one under `~/.local/bin` is a tick either way and two different
//! programs, and a `claude` on the server's `PATH` that no session's holds is a
//! `PATH` to fix rather than a program to install. See [`Where`] and [`Seen`]
//! below, which are the words for each.
//!
//! **And where a session looks is the server's list**, drawn under the
//! by-hand screen's rows on every platform: a session's `PATH` is composed out
//! of the one Verkstead was started with, so no sentence written here could say
//! which directories those are. **Read once, at startup** — which is the other
//! half of that and the one thing somebody who has just installed something by
//! hand has to be told, so [`RESTART`] is in the box with the list.
//!
//! **Which instruction is a choice rather than a fact.** The detected OS is
//! chosen and the other eight are in the same select — see
//! [`./instructions.ts`](./instructions.ts) — because the detection comes off
//! `/etc/os-release` and a derivative names its parent. Nothing about the rows
//! changes with the choice: it is which machine the commands are *for*, and
//! whether a program is there is a fact about the machine the server is on.

import {
  faArrowRight,
  faCaretDown,
  faCaretUp,
  faChevronDown,
  faChevronRight,
} from "@fortawesome/free-solid-svg-icons";
import { useMutation, useQueryClient } from "@tanstack/solid-query";
import {
  For,
  Index,
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
import { Icon } from "../Icon";
import { AGENT_NAME, type AgentType } from "../agents";
import { cancelInstall, startInstall } from "../api/client";
import type {
  Barred,
  Dependency,
  DependencyState,
  DependencyView,
  Distro,
  OnboardingView,
  Platform,
  RunView,
  Seen as SeenSomewhere,
} from "../api/types";
import { ErrorLine, Note } from "../notices";
import { rowPress } from "../rows";
import { Foot } from "./Foot";
import { Mark, type Standing } from "./Mark";
import {
  DISTROS,
  GUIDES,
  instructionFor,
  type Instruction,
} from "./instructions";
import {
  PROBE,
  RUNNING,
  keepReached,
  keepScreen,
  openScreen,
  reached,
  type Screen,
} from "./steps";
import styles from "./Dependencies.module.css";

/// What each row is called where somebody reads it.
///
/// The harnesses are [`AGENT_NAME`]'s, which is what they are called everywhere
/// else in the app: a row saying *Claude* beside a Profile list saying *Claude
/// Code* would be two names for one backend.
export const NAMES: Record<Dependency, string> = {
  Sandbox: "A sandbox",
  Git: "git",
  Claude: AGENT_NAME.Claude,
  Codex: AGENT_NAME.Codex,
  Grok: AGENT_NAME.Grok,
  OpenCode: AGENT_NAME.OpenCode,
  Gh: "gh",
  Wsl: "Linux development (WSL)",
};

/// And what each is for, which is also which of them hold the step up.
const WHY: Record<Dependency, string> = {
  Sandbox: "Every session runs inside one.",
  Git: "Every conversation is a branch and a worktree.",
  Claude: "One harness is enough.",
  Codex: "One harness is enough.",
  Grok: "One harness is enough.",
  OpenCode: "One harness is enough.",
  Gh: "Pull requests and reviews",
  Wsl: "Sessions on Linux, on this PC",
};

/// The command a row is, in mono beside its name: what a harness tile names
/// under the harness. The sandbox row's is its package, and only on Linux —
/// see `Choice` — it being Apple's own on a Mac and an account on Windows; `git`
/// and `gh` are named by their command already.
const COMMANDS: Partial<Record<Dependency, string>> = {
  Claude: "claude",
  Codex: "codex",
  Grok: "grok",
  OpenCode: "opencode",
};

/// The command beside a row's name, where the name is not one already: the
/// harnesses', and the Linux sandbox's package.
function called(
  dependency: Dependency,
  platform: Platform,
): string | undefined {
  return dependency === "Sandbox"
    ? platform === "Linux"
      ? "bubblewrap"
      : undefined
    : COMMANDS[dependency];
}

/// What a row that cannot be had here says, in place of a box.
///
/// The WSL row's alone: a firmware with virtualization off is nothing a press
/// here can change, so the row says what is wrong and where it is put right —
/// and the Components pane says it of its row in the same words.
export const BARRED: Record<Barred, string> = {
  Virtualization:
    "Virtualization is off in this PC's firmware settings, so WSL can't run. " +
    "Turn it on there, then restart.",
};

/// That the list of directories under the by-hand screen's rows is one
/// reading rather than a live one.
///
/// The list is the server's own, rather than a sentence about where a session
/// looks on this kind of machine: a session's `PATH` is composed out of the one
/// Verkstead was started with, so it is a fact about this box. And Verkstead
/// reads that `PATH` when it starts, so a directory put on a shell's `PATH`
/// after that is a directory no session has — which is the whole of what
/// somebody whose fresh install has not ticked needs to know. Said once, in the
/// box with the list, rather than nine times over: it is a fact about
/// Verkstead rather than about any one machine.
const RESTART =
  "Read once, when Verkstead starts. A program installed somewhere new " +
  "reaches a session once Verkstead is started again from a shell whose PATH " +
  "names it.";

/// What the paper is called on the two screens past the ticks, in place of the
/// step's own title: each is titled for what it is doing.
const INSTALLING = "Installing";
const BY_HAND = "Install the Rest by Hand";

/// What the callout says under the server's status line while the run waits on
/// a password dialog: the dialog is on the machine Verkstead runs on, and the
/// human may be reading this from a phone.
const ANSWER = "Answer it on the machine Verkstead runs on, not here.";

/// And the word over the one install a tab's own commands stand on — the
/// Apple-silicon Mac's Homebrew, which is what the `brew` lines under it are
/// run with. Not every line under it: the two vendors' own installers there
/// want no Homebrew, and the other Mac has no `brew` line at all — what that
/// tab says about itself instead is its own `about`, see `instructions.ts`.
const FIRST = "What the brew commands below are run with:";

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

/// The three groups of the checkbox screen, in the order they are drawn: the
/// two rows nothing runs without, the four a session needs one of, and the
/// rest. A row the server drew that is in neither of the first two is the
/// third's, which is where WSL goes on Windows — its own label already says it
/// is optional.
const GROUPS: Array<{
  label: string;
  gloss: string | null;
  holds: (dependency: Dependency) => boolean;
  tiled: boolean;
}> = [
  {
    label: "Required",
    gloss: "Every session needs both.",
    holds: (dependency) => GATING.includes(dependency),
    tiled: false,
  },
  {
    label: "An agent",
    gloss: "At least one",
    holds: (dependency) => HARNESS.includes(dependency),
    tiled: true,
  },
  {
    label: "Optional",
    gloss: null,
    holds: (dependency) =>
      !GATING.includes(dependency) && !HARNESS.includes(dependency),
    tiled: false,
  },
];

/// What the press is held for, said beside it.
const HELD = "A sandbox, git, and one agent are needed.";

/// Whether a row starts ticked: one of those two, or a row this machine is
/// already part of the way to — WSL turned on, or answering with no Annex in it
/// yet, which is somebody who ticked it once.
function ticksItself(row: DependencyView): boolean {
  return GATING.includes(row.dependency) || row.state.state === "Owed";
}

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
  /// every two seconds, and the by-hand screen goes on reading after the objective
  /// is met — and the query is the frame's. See
  /// [`./SetupPage.tsx`](./SetupPage.tsx).
  poll: (every: number | null) => void;

  /// What the paper is to be called in place of the step's title, or `null`
  /// for the title. The paper is the frame's, and which screen is open is this
  /// step's.
  name: (title: string | null) => void;
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

  // Which rows the by-hand screen is about: the ticked ones that were still
  // missing when the run ended, or every missing one where the human chose to
  // install by hand. Fixed when that screen opens rather than read off the
  // reading, so that a row landing while somebody watches moves the counter
  // instead of leaving the list.
  const [hinted, setHinted] = createSignal<Dependency[]>(
    screen() === "hints" ? hints(props.reading) : [],
  );

  // And whether that screen was reached by a run, which is what makes leaving
  // a row behind on it a skip. A reload asks the device, which kept how it got
  // there — see `reached`.
  const [ran, setRan] = createSignal(
    screen() === "hints" && reached() === "run",
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
    (chosen()[row.dependency] ?? ticksItself(row));

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

  /// Open the by-hand screen over every row this machine is missing, ticked or
  /// not, with no run: somebody installing by hand wants the commands for all
  /// of it.
  const byHand = (): void => {
    setRefused(null);
    setRan(false);
    setHinted(missing(props.reading));
    keepReached("choice");
    show("hints");
  };

  /// The rows the by-hand screen draws, as the reading has them now.
  const hinting = (): DependencyView[] =>
    props.reading.dependencies.filter((row) =>
      hinted().includes(row.dependency),
    );

  /// And how many of them this machine has since been found to have.
  const detected = (): number =>
    hinting().filter((row) => !absent(row.state)).length;

  /// Whether the step's objective is met by the machine as it is: a sandbox,
  /// `git` and one harness there. Which is what releases the by-hand screen's
  /// press, however that screen was reached — the rows it lists are every
  /// harness and `gh` too, and none of those is needed.
  const objective = (): boolean => {
    const there = (dependency: Dependency): boolean =>
      rowFor(dependency)?.state.state === "Present";

    return there("Sandbox") && there("Git") && HARNESS.some(there);
  };

  /// The rows the install screen lists: the ones the press settled as ticked,
  /// which a run that is going still has to get through or has landed.
  const underway = (): DependencyView[] =>
    props.reading.dependencies.filter(
      (row) => chosen()[row.dependency] === true,
    );

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
  // is what makes the install screen move and what keeps the by-hand screen
  // watching after the objective is met.
  createEffect(() => props.poll(cadence()));
  onCleanup(() => props.poll(null));

  // And what the paper is called, which is the step's title on the ticks.
  createEffect(() =>
    props.name(
      screen() === "installing"
        ? INSTALLING
        : screen() === "hints"
          ? BY_HAND
          : null,
    ),
  );
  onCleanup(() => props.name(null));

  /// Which of the two it is, or nothing where the frame's own rule is right.
  const cadence = (): number | null => {
    if (running(props.reading.run)) {
      return RUNNING;
    }

    return screen() === "hints" && detected() < hinted().length ? PROBE : null;
  };

  // The install screen moves on by itself once the run is over: to the next
  // step where every ticked row landed, and to the by-hand screen where one did
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
    setRan(true);
    keepReached("run");
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
            byHand={byHand}
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
              machine={props.reading.machine}
              platform={props.reading.platform}
              rows={underway()}
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
            ready={objective()}
            skipping={ran() && detected() < hinted().length}
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
/// else is the checkbox screen, unless the device was left past it and the
/// by-hand screen it was on still has something to say: a row the run could
/// not install, or — where it was opened by hand — any row still missing.
function opening(reading: OnboardingView): Screen {
  if (running(reading.run)) {
    return "installing";
  }

  return openScreen() !== "choosing" && hints(reading).length > 0
    ? "hints"
    : "choosing";
}

/// The rows a by-hand screen a reload lands on is about, by how this device
/// reached it: every row still missing where that was by hand, and the rows
/// the run failed where it was a run.
function hints(reading: OnboardingView): Dependency[] {
  return reached() === "choice" ? missing(reading) : failing(reading);
}

/// Every row this machine has not got, ticked or not: what the by-hand screen
/// lists when it is opened by hand.
function missing(reading: OnboardingView): Dependency[] {
  return reading.dependencies
    .filter((row) => absent(row.state))
    .map((row) => row.dependency);
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
/// is the only kind the by-hand screen ever has anything to say about.
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

/// Whether a row is one this machine has not got — which is what carries a
/// checkbox, and what the objective is not met by. A row part of the way there
/// is one of these too; a row that cannot be had here is not, and has no box.
export function absent(state: DependencyState): boolean {
  return state.state === "Absent" || state.state === "Owed";
}

/// The checkbox screen: every row in its group, a tick to put on the ones that
/// are missing, and the press that installs them.
function Choosing(props: {
  reading: OnboardingView;
  ticked: (row: DependencyView) => boolean;
  tick: (row: DependencyView, on: boolean) => void;
  ready: boolean;
  pressing: boolean;
  press: () => void;
  byHand: () => void;
  refused: string | null;
}): JSX.Element {
  /// How many rows the press would install, which is what it is called by.
  const count = (): number =>
    props.reading.dependencies.filter(props.ticked).length;

  return (
    <>
      <p class={styles.standing}>
        Tick what {props.reading.machine} is missing and Verkstead installs it.
      </p>

      <For each={GROUPS}>
        {(group) => {
          const rows = (): DependencyView[] =>
            props.reading.dependencies.filter((row) =>
              group.holds(row.dependency),
            );

          return (
            <Show when={rows().length > 0}>
              <section class={styles.group} data-group={group.label}>
                <h3 class={styles.label}>
                  {group.label}
                  <Show when={group.gloss}>
                    {(gloss) => <span class={styles.gloss}>{gloss()}</span>}
                  </Show>
                </h3>

                <ul class={group.tiled ? styles.tiles : styles.card}>
                  <For each={rows()}>
                    {(row) => (
                      <Choice
                        row={row}
                        platform={props.reading.platform}
                        tiled={group.tiled}
                        ticked={props.ticked(row)}
                        tick={(on) => props.tick(row, on)}
                      />
                    )}
                  </For>
                </ul>
              </section>
            </Show>
          );
        }}
      </For>

      <Show when={props.refused}>
        {(why) => (
          <div class={styles.onwards}>
            <ErrorLine class={styles.failure}>{why()}</ErrorLine>
          </div>
        )}
      </Show>

      <Foot>
        <Show when={!props.ready}>
          <span class={styles.held}>{HELD}</span>
        </Show>

        <button
          type="button"
          class={styles.next}
          disabled={!props.ready || props.pressing}
          onClick={() => props.press()}
        >
          {count() === 0 ? "Next" : `Install ${count()}`}
          <Icon of={faArrowRight} class={styles.arrow} />
        </button>

        {/* The quiet way past the run: the commands for everything missing,
            for somebody who would rather type them. */}
        <button
          type="button"
          class={styles.byHand}
          onClick={() => props.byHand()}
        >
          Install by hand instead
        </button>
      </Foot>
    </>
  );
}

/// One row of the checkbox screen: a line in a group's card, or a tile under
/// *An agent*.
///
/// A row that is missing is a label round the whole of it with the box inside,
/// so a press anywhere on it is a tick — see `rowPress`. A row that is there is
/// a tick and an *Installed* pill and nothing to press, and a row that cannot be
/// had here says why in place of the box.
function Choice(props: {
  row: DependencyView;
  platform: Platform;
  tiled: boolean;
  ticked: boolean;
  tick: (on: boolean) => void;
}): JSX.Element {
  const state = (): DependencyState => props.row.state;

  /// Whether the row is one to tick: the machine has not got it.
  const choosing = (): boolean => absent(state());

  /// Why a row cannot be had here, where it cannot.
  const barred = (): string | null => {
    const said = state();

    return said.state === "Refused" ? BARRED[said.barred] : null;
  };

  const command = (): string | undefined =>
    called(props.row.dependency, props.platform);

  const box = (): JSX.Element => (
    <input
      type="checkbox"
      class={styles.box}
      checked={props.ticked}
      onChange={(ev) => props.tick(ev.currentTarget.checked)}
    />
  );

  /// What stands where the box would on a row that is not being chosen: the
  /// tick on a row that is there, and nothing on one that cannot be had.
  const standing = (): JSX.Element => (
    <Show when={state().state === "Present"}>
      <span class={styles.mark}>
        <Mark standing="met" />
      </span>
    </Show>
  );

  /// The pill on the end of a line, which says in a word what the box and the
  /// tick say in a shape.
  const pill = (): JSX.Element => (
    <Switch>
      <Match when={state().state === "Present"}>
        <span class={styles.installed}>Installed</span>
      </Match>
      <Match when={choosing()}>
        <span class={styles.missing}>Missing</span>
      </Match>
    </Switch>
  );

  const line = (): JSX.Element => (
    <>
      <Show when={choosing()} fallback={standing()}>
        {box()}
      </Show>
      <span class={styles.text}>
        <span class={styles.name}>
          {NAMES[props.row.dependency]}
          <Show when={command()}>
            {(command) => <code class={styles.called}>{command()}</code>}
          </Show>
        </span>
        <span class={styles.why}>{WHY[props.row.dependency]}</span>
        <Show when={barred()}>
          {(why) => (
            <span class={styles.barred} data-barred>
              {why()}
            </span>
          )}
        </Show>
      </span>
      {pill()}
    </>
  );

  /// A harness as a tile: its mark, and in the corner the box or the tick.
  const tile = (): JSX.Element => (
    <>
      <span class={styles.top}>
        <HarnessMark
          of={HARNESSES[props.row.dependency] ?? null}
          class={styles.harness}
        />
        <Show
          when={choosing()}
          fallback={
            <span class={styles.corner}>
              {standing()}
              {pill()}
            </span>
          }
        >
          {box()}
        </Show>
      </span>
      <span class={styles.name}>{NAMES[props.row.dependency]}</span>
      <Show when={command()}>
        {(command) => <code class={styles.called}>{command()}</code>}
      </Show>
    </>
  );

  const body = (): JSX.Element => (props.tiled ? tile() : line());

  return (
    <li
      class={props.tiled ? styles.tile : styles.line}
      classList={{ [styles.chosen!]: choosing() && props.ticked }}
      data-dependency={props.row.dependency}
      data-state={state().state}
    >
      <Show when={choosing()} fallback={<div class={styles.inner}>{body()}</div>}>
        {/* The name labels the box by being inside it, and the row answers the
            press rather than the box a label forwards one to — see
            `rowPress`. */}
        <label
          class={styles.inner}
          onClick={rowPress(() => props.tick(!props.ticked))}
        >
          {body()}
        </label>
      </Show>
    </li>
  );
}

/// The install screen: how far the run has got, what it is doing, which rows it
/// is getting through, and the press that stops it.
export function Installing(props: {
  /// The machine the run is on, as the standing line names it.
  machine: string;

  /// And what it is, which is what names the sandbox row's package.
  platform: Platform;

  /// The rows the press ticked, as the reading has them now.
  rows: DependencyView[];

  run: RunView;
  stopping: boolean;
  cancel: () => void;
}): JSX.Element {
  return (
    <>
      <p class={styles.standing}>
        Verkstead is installing what you ticked on {props.machine}.
      </p>

      <div
        class={styles.run}
        data-done={props.run.done}
        data-total={props.run.total}
      >
        <span class={styles.count} data-count>
          {props.run.done} of {props.run.total} installed
        </span>
        <progress
          class={styles.bar}
          value={props.run.done}
          max={props.run.total}
        />
      </div>

      {/* What a machine raises and how often is the machine's own — one dialog
          for the whole batch where a package manager takes a list of names, one
          per install where it does not, and none at all on a Mac — so the
          status line says what is happening rather than how many dialogs to
          expect. While it is waiting on one, it is the one thing on this
          screen with something to do, and the doing is somewhere else. */}
      <Show
        when={props.run.phase === "Asking"}
        fallback={
          <p class={styles.status} data-status>
            {props.run.status}
          </p>
        }
      >
        <div class={styles.callout} data-asking>
          <span class={styles.bang} aria-hidden="true">
            !
          </span>
          <div>
            <p class={styles.status} data-status>
              {props.run.status}
            </p>
            <p class={styles.answer}>{ANSWER}</p>
          </div>
        </div>
      </Show>

      {/* The rows, with the one the run is on saying so. A row still waiting
          says nothing: the bar is what says how far the run has got, and a
          word on every row behind it would be the same thing seven times. */}
      <ul class={styles.card}>
        <For each={props.rows}>
          {(row) => (
            <li
              class={styles.line}
              data-dependency={row.dependency}
              data-state={row.state.state}
            >
              <div class={styles.inner}>
                <Mark standing={going(row)} />
                <Named row={row} platform={props.platform} />
                <Show when={going(row) === "going"}>
                  <span class={styles.going}>Installing</span>
                </Show>
              </div>
            </li>
          )}
        </For>
      </ul>

      {/* A press cannot stop a package manager that is already unpacking — a
          half-installed machine is worse than a fully installed one — so Cancel
          skips what has not started and says so until the unit under way is
          over. And there is nothing to press onwards with: the screen moves on
          by itself when the run is over. */}
      <Foot>
        <button
          type="button"
          class={styles.cancel}
          disabled={props.run.cancelling || props.stopping}
          onClick={() => props.cancel()}
        >
          {props.run.cancelling ? "Cancelling…" : "Cancel"}
        </button>
      </Foot>
    </>
  );
}

/// How a row on the install screen stands: there, being installed, or waiting
/// its turn.
function going(row: DependencyView): Standing {
  if (row.state.state === "Present") {
    return "met";
  }

  return row.install.install === "Installing" ? "going" : "waiting";
}

/// A row's harness mark where it is a harness, its name, and the command it is
/// where the name is not one already — as the install screen's rows draw it.
function Named(props: {
  row: DependencyView;
  platform: Platform;
}): JSX.Element {
  return (
    <span class={styles.name}>
      <HarnessMark
        of={HARNESSES[props.row.dependency] ?? null}
        class={styles.harness}
      />
      {NAMES[props.row.dependency]}
      <Show when={called(props.row.dependency, props.platform)}>
        {(command) => <code class={styles.called}>{command()}</code>}
      </Show>
    </span>
  );
}

/// The by-hand screen: the rows still missing, what to run for each of them,
/// and the press that is held until a session could start.
export function Hints(props: {
  reading: OnboardingView;
  rows: DependencyView[];
  detected: number;

  /// Whether the step's objective is met, which is what releases the press.
  ready: boolean;

  /// And whether pressing it leaves behind a row a run failed, which is what
  /// it is then called.
  skipping: boolean;

  back: () => void;
  onwards: () => void;
}): JSX.Element {
  // Which OS the commands are for. The detected one to begin with, and the
  // human's after they have chosen: a machine whose `ID_LIKE` sent them to the
  // wrong one is exactly who this control is for, and the reading re-arriving
  // every ten seconds must not put them back.
  const [showing, setShowing] = createSignal<Distro>(props.reading.distro);

  const guide = () => GUIDES[showing()];

  /// What one row is told to run for the machine chosen.
  ///
  /// Asked of `instructions.ts` rather than read off the guide, because one row
  /// is about this server rather than about the choice: the Windows sandbox row
  /// makes an account named after a Data Directory, and the line has to name the
  /// one this machine keeps.
  const instruction = (row: DependencyView): Instruction =>
    instructionFor(
      showing(),
      row.dependency,
      props.reading.data_directory,
      row.state,
    );

  return (
    <>
      <p class={styles.standing}>
        Run these on {props.reading.machine}. Each one ticks itself off here as
        it lands.
      </p>

      <div class={styles.over}>
        <span class={styles.missingCount} data-missing>
          {props.rows.length - props.detected} still missing
        </span>

        <label class={styles.choosing}>
          Commands for
          <select
            class={styles.select}
            value={showing()}
            onChange={(ev) => setShowing(ev.currentTarget.value as Distro)}
          >
            <For each={DISTROS}>
              {(distro) => (
                <option value={distro} data-distro={distro}>
                  {GUIDES[distro].title}
                </option>
              )}
            </For>
          </select>
        </label>
      </div>

      {/* What one choice is, where two of them are the same operating system
          with different commands under them: the Intel Mac's says so in a
          sentence, drawn where the other Mac draws Homebrew's own line. */}
      <Show when={guide().about}>
        {(about) => <Note class={styles.about}>{about()}</Note>}
      </Show>

      {/* What this machine's own commands want first, where they want the
          same thing: the Mac's Homebrew, drawn once above the rows rather than
          under each of the four that are a `brew install`. */}
      <Show when={guide().before}>
        {(first) => (
          <div class={styles.before} data-before>
            <p class={styles.firstly}>{FIRST}</p>
            <Instructed of={first()} />
          </div>
        )}
      </Show>

      {/* By position rather than by row: the list is fixed when the screen
          opens, and a row re-read under somebody who has opened what it failed
          with keeps it open. */}
      <ul class={styles.blocks}>
        <Index each={props.rows}>
          {(row) => <Row row={row()} instruction={instruction(row())} />}
        </Index>
      </ul>

      <Show when={props.reading.path.length > 0}>
        <div class={styles.looks} data-path>
          <span class={styles.pathLabel}>PATH</span>
          <div class={styles.looking}>
            <ol class={styles.path}>
              <For each={props.reading.path}>
                {(entry) => (
                  <li>
                    <code>{entry}</code>
                  </li>
                )}
              </For>
            </ol>
            <p class={styles.restart}>{RESTART}</p>
          </div>
        </div>
      </Show>

      {/* Back rather than a way of unticking a row here: what a row nobody
          wants after all is, is a tick to take off, and the screen with the
          ticks on it is the one before this. */}
      <Foot>
        <button type="button" class={styles.back} onClick={() => props.back()}>
          Back
        </button>

        <span class={styles.held} data-detected>
          {props.detected} of {props.rows.length} found
        </span>

        <button
          type="button"
          class={styles.next}
          disabled={!props.ready}
          onClick={() => props.onwards()}
        >
          {props.skipping ? "Skip" : "Next"}
          <Icon of={faArrowRight} class={styles.arrow} />
        </button>
      </Foot>
    </>
  );
}

/// One block of the by-hand screen: what the row is, whether this machine has
/// it, what the run made of it, what the probe found, and what to run by hand.
function Row(props: {
  row: DependencyView;

  /// What to run by hand.
  instruction: Instruction;
}): JSX.Element {
  // Whether what the run said is open. Closed to begin with: the command under
  // it is what the screen is for, and the failure is there for whoever wants
  // to know why.
  const [saying, setSaying] = createSignal(false);

  const state = (): DependencyState => props.row.state;

  /// What a failed run said, which is only ever the Linux sandbox row's: a
  /// `bwrap` that is installed and will not make a namespace says why in its
  /// own words, and no sentence written here would be as useful.
  const trouble = (): string | null => {
    const said = state();

    return said.state === "Absent" ? said.trouble : null;
  };

  /// And what the install run made of it, where it failed: a dismissed password
  /// dialog, a package manager's own refusal, or a row no archive here carries.
  /// Nothing once the row is there, whatever the run said before it landed.
  const failed = (): string | null => {
    const made = props.row.install;

    return made.install === "Failed" && absent(state()) ? made.why : null;
  };

  /// And why a row cannot be had here, where it cannot.
  const barred = (): string | null => {
    const said = state();

    return said.state === "Refused" ? BARRED[said.barred] : null;
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

  return (
    <li
      class={styles.block}
      data-dependency={props.row.dependency}
      data-state={state().state}
    >
      <div class={styles.head}>
        <Mark standing={standing(state())} />
        <span class={styles.name}>
          <HarnessMark
            of={HARNESSES[props.row.dependency] ?? null}
            class={styles.harness}
          />
          {NAMES[props.row.dependency]}
        </span>
        <span class={styles.why}>{WHY[props.row.dependency]}</span>

        <Show when={failed()}>
          <button
            type="button"
            class={styles.failedPill}
            aria-expanded={saying()}
            onClick={() => setSaying(!saying())}
          >
            Failed
            <Icon
              of={saying() ? faCaretUp : faCaretDown}
              class={styles.caret}
            />
          </button>
        </Show>
      </div>

      <Show when={saying() ? failed() : null}>
        {(why) => (
          <div class={styles.said}>
            <p class={styles.saidLabel}>What the machine said</p>
            <pre class={styles.failed} data-failed>
              {why()}
            </pre>
          </div>
        )}
      </Show>

      <Show when={trouble()}>
        {(said) => <pre class={styles.trouble}>{said()}</pre>}
      </Show>

      <Show when={barred()}>
        {(why) => (
          <p class={styles.barred} data-barred>
            {why()}
          </p>
        )}
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
/// Four sentences for the four ways that happens, because they are four
/// different things to do about it — a directory to put on the `PATH` Verkstead
/// is started with, an install to move somewhere a session can reach, a link
/// left behind by an install that has gone, and the Claude Code desktop app
/// standing where the CLI would. A row with none of them saw the name nowhere
/// at all, and what it says is the instruction under it.
///
/// The desktop app is the one of the four that is really there and really runs,
/// so its sentence says which of the two programs the file is and leaves the
/// install to the row's own instruction directly beneath it. **Naming a command
/// here would name the wrong one**: the only platform this row appears on is
/// Windows, where the row leads with Anthropic's own installer and keeps npm as
/// the alternative under it, so a sentence with `npm install` in it would put
/// the row's second choice above the row's first. The server's half of the same
/// answer does name a command — see `sessions::desktop_app` — because a Notice
/// has no row under it to read one off.
function Seen(props: { at: SeenSomewhere }): JSX.Element {
  return (
    <Note class={styles.seen}>
      <span data-seen={props.at.seen}>
        <Show when={props.at.seen === "Beyond"}>
          Found at <code>{props.at.at}</code>, which isn't on the{" "}
          <code>PATH</code> a session gets, so a session can't open it.
        </Show>

        <Show when={props.at.seen === "Leading" ? props.at : null}>
          {(leading) => (
            <>
              <code>{leading().at}</code> is a link to{" "}
              <code>{leading().target}</code>, which a session can't reach.
            </>
          )}
        </Show>

        <Show when={props.at.seen === "Dangling"}>
          <code>{props.at.at}</code> is a link with nothing at the end of it.
        </Show>

        <Show when={props.at.seen === "Desktop"}>
          <code>{props.at.at}</code> is the Claude Code desktop app, not the
          command-line tool. The CLI is installed separately, as below.
        </Show>
      </span>
    </Note>
  );
}

/// What to run, where to get it, and what neither says for itself.
///
/// Once more under itself where the instruction keeps an alternative, which goes
/// one level deep and no further: the second install is a command with a note
/// exactly as the first is, so it is drawn by the same function rather than by a
/// smaller copy of it. It stands behind a link naming where it comes from,
/// because the one that leads is the one to run, and the two are the same
/// program.
function Instructed(props: { of: Instruction }): JSX.Element {
  const [other, setOther] = createSignal(false);

  return (
    <div class={styles.instruction}>
      <Show when={props.of.command}>
        {(command) => (
          <div class={styles.command}>
            <span class={styles.prompt} aria-hidden="true">
              $
            </span>
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

      <Show when={props.of.note}>
        {(note) => <p class={styles.note}>{note()}</p>}
      </Show>

      {/* And the other way to get the same program, under the one that leads:
          Claude Code's row is the vendor's installer with this machine's own
          package kept beneath it. */}
      <Show when={props.of.alternative}>
        {(alternative) => (
          <>
            <button
              type="button"
              class={styles.or}
              aria-expanded={other()}
              onClick={() => setOther(!other())}
            >
              Or from {alternative().from ?? "elsewhere"}
              <Icon
                of={other() ? faChevronDown : faChevronRight}
                class={styles.caret}
              />
            </button>
            <Show when={other()}>
              <div class={styles.alternative} data-alternative>
                <Instructed of={alternative()} />
              </div>
            </Show>
          </>
        )}
      </Show>
    </div>
  );
}

/// How a row's state stands, as the mark beside its name.
function standing(state: DependencyState): Standing {
  return state.state === "Present" ? "met" : "waiting";
}
