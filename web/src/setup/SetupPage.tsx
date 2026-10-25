//! The wizard: the only page there is while onboarding mode is on, and no page
//! at all while it is off.
//!
//! What it draws is a frame — the three steps in the order a machine is set up
//! in, which of them stands met, and which one is open — over the one endpoint
//! that says how this Verkstead stands (`GET /api/ui/onboarding`). What each
//! step *contains* is its own component — and the last of them reads a little
//! more of its own, the fields it fills being nothing the frame is about.
//!
//! **The read runs on an interval while the open step is unmet**, and stops the
//! moment it is met. Somebody is standing at a terminal waiting for an install
//! to land, and the thing they are waiting for is a row on this page ticking;
//! ten seconds is short enough that the tick follows the install rather than
//! the human's patience. Once the step is met there is nothing left to watch
//! for, so the interval goes — and coming back to a phone that was put down is
//! covered by the app's own refocus re-read (`App.tsx`), which is why there is
//! no second mechanism for it here.
//!
//! **Unless the open step asks for something else.** The dependencies step runs
//! installs of its own, and how often the machine is worth reading while one is
//! going — and while its hint screen waits for a row to land after the step is
//! met — is a fact about that step rather than about the frame. So the step says
//! what it wants and the frame's own rule stands for every moment it says
//! nothing.
//!
//! **And inside the app it is dragged by its top.** The window has no title bar
//! (ADR-0020) and what moves it everywhere else is a pane head; this page has
//! none, so it draws the bare bar that stands where one would — see
//! [`DragBar`](../DragBar.tsx). Nothing at all in a browser, which is where a
//! phone on the tailnet walks this same wizard.
//!
//! **Which step is open is this device's**, kept in browser storage and never
//! sent anywhere — see `steps.ts`. A step that stands met can be opened again
//! by pressing its heading, or by Back from the step after it, which is the
//! whole of the navigation the frame has: moving *forward* is each step's own
//! press, and a step nobody has met yet is not one to skip into.
//!
//! **Drawn as a rail beside a paper once the window is wide** — the same 60rem
//! the workbench breaks at (`BESIDE` in `widths.ts`), so the two change shape
//! at one width. The rail is the brand, the three steps and what the page is
//! reading; the paper is the open step, under its number and title, with a foot
//! pinned to the bottom of it that the frame owns and the step presses in —
//! see [`Foot`](./Foot.tsx).
//!
//! **Narrower than that, the rail folds into a header** over the paper: the
//! brand, a strip of three dots naming the open step, and a Steps button that
//! opens the rail's own list in the app's one [`Modal`](../Modal.tsx), at the
//! top of the window where the strip it stands for is. The list there is the
//! rail's, with an Open on the end of a met step rather than its heading being
//! the press: a heading on a card a thumb is reaching into is no target. And
//! the foot stays stuck to the bottom of the window, its press the whole width
//! of it.

import { For, Show, createSignal, type JSX } from "solid-js";
import { faChevronDown } from "@fortawesome/free-solid-svg-icons";

import { DragBar } from "../DragBar";
import { Icon } from "../Icon";
import { Modal } from "../Modal";
import { listProfiles, loadOnboarding } from "../api/client";
import type { OnboardingView } from "../api/types";
import { useReading } from "../freshness";
import { band } from "../head";
import { Empty, ErrorLine } from "../notices";
import { bridge } from "../settings/bridge";
import { Accounts } from "./Accounts";
import { Dependencies, running } from "./Dependencies";
import { FootContext } from "./Foot";
import { Git } from "./Git";
import { Mark } from "./Mark";
import {
  PROBE,
  RUNNING,
  STEPS,
  TITLES,
  after,
  before,
  keepStep,
  met,
  openStep,
  type Step,
} from "./steps";
import { harnesses, present } from "./summaries";
import styles from "./SetupPage.module.css";

/// The wizard, whole.
export function SetupPage(): JSX.Element {
  const [open, setOpen] = createSignal<Step>(openStep());

  // Whether the phone's Steps sheet is up. Never up from 60rem, where there is
  // no button to have opened it.
  const [listing, setListing] = createSignal(false);

  // What the open step has asked for, where it has asked for anything: the
  // dependencies step reads faster while an install of its own is going, and
  // goes on reading while its hint screen waits for a row to land. Nothing from
  // a step that has nothing to say, which is the frame's own rule standing.
  const [asked, setAsked] = createSignal<number | null>(null);

  // And what the open step has called the paper, where it has: the
  // dependencies step's run and its by-hand screen are titled for what they
  // are rather than for the step. The step's own title otherwise.
  const [retitled, setRetitled] = createSignal<string | null>(null);

  /// Open a step, and remember on this device that it is the one open.
  const show = (step: Step): void => {
    setOpen(step);
    keepStep(step);
  };

  /// What a step's own press does once it is over: open the one after it.
  ///
  /// Pressed once the step has done whatever it was for — the accounts step
  /// saves what is ticked before it presses this — so what arrives here is a
  /// step that is over rather than one being skipped.
  ///
  /// The last step's press is the wizard *finishing* rather than a step
  /// opening — the mode going off and the app landing on `/compose` — so it
  /// never arrives here: see [`Git`](./Git.tsx), which does that itself.
  const onwards = (step: Step): void => {
    const next = after(step);

    if (next !== null) {
      show(next);
    }
  };

  const onboarding = useReading(() => {
    // Read here rather than inside the interval below, so that a step asking
    // for a different cadence is a change to the query's own options: the
    // interval itself is settled by the query core when it next looks.
    const wanted = asked();

    return {
      queryKey: ["onboarding"],
      queryFn: loadOnboarding,
      // What the open step asked for, and otherwise: re-read while that step is
      // unmet, and stop once it is met. Read off the query's own last answer
      // rather than off a signal beside it, because the thing being asked about
      // is exactly what the last read said.
      refetchInterval: (query) =>
        wanted ?? (met(query.state.data?.steps, open()) ? false : PROBE),
      // Merged rather than replaced, keyed by what a dependency row carries: the
      // page is re-read under somebody who is reading it, and a rebuild would
      // take the open step's own controls down every ten seconds. The accounts
      // beside those rows carry no such key and are matched by position, which is
      // what a list of at most four in a fixed order can be matched by.
      freshness: { reconcile: "dependency" },
    };
  });

  // The Profiles saved, for the line the rail draws under a met second step —
  // the settings' own read, which the accounts step already invalidates when it
  // saves. Only once that step is met: before then there is no line to draw,
  // and nothing to ask the server for.
  const profiles = useReading(() => ({
    queryKey: ["profiles"],
    queryFn: () => listProfiles(null),
    enabled: met(onboarding.data?.steps, "accounts"),
    freshness: { reconcile: "id" },
  }));

  /// What the rail says under a step that stands met, where it says anything.
  const summary = (view: OnboardingView, step: Step): string | null => {
    if (!met(view.steps, step)) {
      return null;
    }

    switch (step) {
      case "dependencies":
        return present(view.dependencies);
      case "accounts":
        return profiles.data === undefined ? null : harnesses(profiles.data);
      // Met is the wizard over, and nobody is reading the rail by then.
      case "git":
        return null;
    }
  };

  // The right-hand end of the paper's foot, made before anything is drawn into
  // it so that the step's press has somewhere to go the moment it is drawn.
  const foot = document.createElement("div");
  foot.className = styles.press!;

  return (
    <>
      <DragBar />
      {/* How tall the drag bar over it stands, which is nothing in a browser:
          the rail is the window's height less that, and sticks under it. */}
      <section
        class={styles.setup}
        data-wizard
        style={{ "--band": `${bridge() === null ? 0 : band()}px` }}
      >
        <div class={styles.rail}>
          {/* The brand rather than a title: this is Verkstead before it is
              anything, and what it is doing is the label under it. */}
          <h1 class={styles.brand}>
            <img src="/icons/icon-192.png" alt="" />
            Verkstead
          </h1>
          {/* The phone's way to the list the rail holds, which the header
              has no room for. */}
          <Show when={onboarding.data}>
            <button
              type="button"
              class={styles.listing}
              aria-haspopup="dialog"
              onClick={() => setListing(true)}
            >
              Steps
              <Icon of={faChevronDown} class={styles.caret!} />
            </button>
          </Show>
          <p class={styles.label}>Setup</p>

          <Show when={onboarding.data}>
            {(view) => (
              <>
                <Strip open={open()} />

                <div class={styles.listed}>
                  <StepList
                    view={view()}
                    open={open()}
                    summary={(step) => summary(view(), step)}
                    show={show}
                  />

                  <p class={styles.reading}>
                    <span class={styles.dot} aria-hidden="true" />
                    {reading(view())}
                  </p>
                </div>
              </>
            )}
          </Show>
        </div>

        {/* The rail's list on a phone, over a veil. Nothing of it is on the
            page until Steps is pressed, which is why the rail's own rows stay
            the only ones a narrow page's frame has. */}
        <Modal
          class={styles.sheet!}
          open={listing() && onboarding.data !== undefined}
          close={() => setListing(false)}
          name="Steps"
        >
          <div class={styles.sheetHead}>
            {/* The brand as the header draws it, and not a second heading:
                the page's one `h1` is still the header's, under this veil. */}
            <p class={styles.brand}>
              <img src="/icons/icon-192.png" alt="" />
              Verkstead
            </p>
            <button
              type="button"
              class={styles.close}
              onClick={() => setListing(false)}
            >
              Close
            </button>
          </div>
          <p class={styles.label}>Setup</p>

          <StepList
            view={onboarding.data!}
            open={open()}
            summary={(step) => summary(onboarding.data!, step)}
            show={(step) => {
              show(step);
              setListing(false);
            }}
            sheet
          />

          <p class={styles.reading}>
            <span class={styles.dot} aria-hidden="true" />
            {reading(onboarding.data!)}
          </p>
        </Modal>

        <div class={styles.paper}>
          <Show
            when={onboarding.data}
            fallback={
              <Show
                when={onboarding.isError}
                fallback={<Empty>Reading this machine…</Empty>}
              >
                <ErrorLine>This machine couldn't be read.</ErrorLine>
              </Show>
            }
          >
            {(view) => (
              <FootContext.Provider value={foot}>
                <p class={styles.eyebrow}>
                  Step {STEPS.indexOf(open()) + 1} of {STEPS.length}
                </p>
                <h2 class={styles.title}>{retitled() ?? TITLES[open()]}</h2>

                <div class={styles.body}>
                  <Show when={open() === "dependencies"}>
                    <Dependencies
                      reading={view()}
                      onwards={() => onwards("dependencies")}
                      poll={setAsked}
                      name={setRetitled}
                    />
                  </Show>
                  <Show when={open() === "accounts"}>
                    <Accounts
                      reading={view()}
                      onwards={() => onwards("accounts")}
                    />
                  </Show>
                  {/* And the last of the three, whose press is the wizard
                      finishing rather than a step opening — which is why it
                      takes no `onwards`: what it goes on to is the app itself. */}
                  <Show when={open() === "git"}>
                    <Git />
                  </Show>
                </div>

                <div class={styles.foot}>
                  {/* Back to the step before, which is always met: a step is
                      only ever open once the ones before it are. */}
                  <Show when={before(open())}>
                    {(back) => (
                      <button
                        type="button"
                        class={styles.back}
                        onClick={() => show(back())}
                      >
                        Back
                      </button>
                    )}
                  </Show>
                  {foot}
                </div>
              </FootContext.Provider>
            )}
          </Show>
        </div>
      </section>
    </>
  );
}

/// The line at the foot of the rail: which machine the page is reading, and
/// how often — the frame's two cadences, the faster while a run of
/// Verkstead's own is going.
function reading(view: OnboardingView): string {
  return running(view.run)
    ? `Reading ${view.machine} every ${RUNNING / 1000} s while it installs`
    : `Reading ${view.machine} every ${PROBE / 1000} s`;
}

/// The three steps as the rail lists them, and as the phone's sheet lists them
/// again: a mark, a name, and what meeting the step came to under it.
///
/// The one difference is where the way back into a met step is. In the rail it
/// is the heading itself; on the sheet it is an Open on the end of the row,
/// because what a thumb on a phone reaches for is a word that says it will
/// open something. Neither draws one on an unmet step — see [`StepHead`].
function StepList(props: {
  view: OnboardingView;
  open: Step;
  summary: (step: Step) => string | null;
  show: (step: Step) => void;
  sheet?: boolean;
}): JSX.Element {
  return (
    <ol class={styles.steps}>
      <For each={STEPS}>
        {(step) => {
          const isMet = (): boolean => met(props.view.steps, step);
          const isOpen = (): boolean => props.open === step;

          return (
            <li
              class={styles.step}
              classList={{ [styles.open!]: isOpen() }}
              data-step={step}
              data-met={isMet() ? "yes" : "no"}
              aria-current={isOpen() ? "step" : undefined}
            >
              <div class={styles.row}>
                <StepHead
                  step={step}
                  met={isMet()}
                  open={isOpen()}
                  show={props.show}
                  pressable={!props.sheet}
                />
                <Show when={props.sheet && isMet() && !isOpen()}>
                  <button
                    type="button"
                    class={styles.reach}
                    onClick={() => props.show(step)}
                  >
                    Open
                  </button>
                </Show>
              </div>
              <Show when={props.summary(step)}>
                {(line) => <p class={styles.summary}>{line()}</p>}
              </Show>
            </li>
          );
        }}
      </For>
    </ol>
  );
}

/// The phone's header strip: the three steps as dots, the open one filled,
/// and the open one's name beside them.
///
/// Hidden from a screen reader, which has the paper's own *Step N of 3* and
/// title to read this from: the strip is a picture of where the open step
/// stands, and read aloud it would be the same two facts a second time.
function Strip(props: { open: Step }): JSX.Element {
  return (
    <div class={styles.strip} aria-hidden="true">
      <ol class={styles.dots}>
        <For each={STEPS}>
          {(step, index) => (
            <li
              class={styles.pip}
              classList={{ [styles.here!]: props.open === step }}
            >
              {index() + 1}
            </li>
          )}
        </For>
      </ol>
      <span class={styles.where}>{TITLES[props.open]}</span>
    </div>
  );
}

/// One step's heading in the rail: what it is called, whether it is met, and —
/// where it is — the press that opens it again.
///
/// A button only for a step that stands met and is not open, because that is
/// the only one there is anything to go back to: an unmet step reached by
/// pressing past the one in front of it would be the skip ADR-0016 refused,
/// drawn as navigation. And never on the sheet, which draws its own Open.
function StepHead(props: {
  step: Step;
  met: boolean;
  open: boolean;
  show: (step: Step) => void;
  pressable: boolean;
}): JSX.Element {
  /// The mark and the name, which is the heading either way round. An unmet
  /// step's mark is its number, drawn by the stylesheet so the heading reads
  /// as the name alone.
  const named = (): JSX.Element => (
    <>
      <Show
        when={props.met}
        fallback={
          <span
            class={styles.number}
            data-number={STEPS.indexOf(props.step) + 1}
            aria-hidden="true"
          />
        }
      >
        <span class={styles.ticked}>
          <Mark standing="met" />
        </span>
      </Show>
      <span class={styles.name}>{TITLES[props.step]}</span>
    </>
  );

  return (
    <h2 class={styles.head}>
      <Show
        when={props.pressable && props.met && !props.open}
        fallback={named()}
      >
        <button
          type="button"
          class={styles.reopen}
          onClick={() => props.show(props.step)}
        >
          {named()}
        </button>
      </Show>
    </h2>
  );
}
