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
//! **Which step is open is this device's**, kept in browser storage and never
//! sent anywhere — see `steps.ts`. A step that stands met can be opened again
//! by pressing its heading, which is the whole of the navigation the frame has:
//! moving *forward* is each step's own Next, and a step nobody has met yet
//! is not one to skip into.

import { For, Show, createSignal, type JSX } from "solid-js";

import { loadOnboarding } from "../api/client";
import { useReading } from "../freshness";
import { Empty, ErrorLine } from "../notices";
import { Accounts } from "./Accounts";
import { Dependencies } from "./Dependencies";
import { Git } from "./Git";
import { Mark } from "./Mark";
import {
  PROBE,
  STEPS,
  TITLES,
  after,
  keepStep,
  met,
  openStep,
  type Step,
} from "./steps";
import styles from "./SetupPage.module.css";

/// The wizard, whole.
export function SetupPage(): JSX.Element {
  const [open, setOpen] = createSignal<Step>(openStep());

  // What the open step has asked for, where it has asked for anything: the
  // dependencies step reads faster while an install of its own is going, and
  // goes on reading while its hint screen waits for a row to land. Nothing from
  // a step that has nothing to say, which is the frame's own rule standing.
  const [asked, setAsked] = createSignal<number | null>(null);

  /// Open a step, and remember on this device that it is the one open.
  const show = (step: Step): void => {
    setOpen(step);
    keepStep(step);
  };

  /// What a step's own Next does: open the one after it.
  ///
  /// Pressed once the step has done whatever it was for — the accounts step
  /// saves what is ticked before it presses this — so what arrives here is a
  /// step that is over rather than one being skipped.
  ///
  /// The last step's Next is the wizard *finishing* rather than a step
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

  return (
    <section class={styles.setup}>
      <h1>Set Verkstead up</h1>
      <p class={styles.standing}>
        There is a little to settle before any work can be started here. Nothing
        else is reachable until it is done.
      </p>

      <Show
        when={onboarding.data}
        fallback={
          <Show
            when={onboarding.isError}
            fallback={<Empty>Reading this machine…</Empty>}
          >
            <ErrorLine>This machine could not be read.</ErrorLine>
          </Show>
        }
      >
        {(view) => (
          <ol class={styles.steps}>
            <For each={STEPS}>
              {(step) => (
                <li
                  class={styles.step}
                  classList={{ [styles.open!]: open() === step }}
                  data-step={step}
                  data-met={met(view().steps, step) ? "yes" : "no"}
                  aria-current={open() === step ? "step" : undefined}
                >
                  <StepHead
                    step={step}
                    met={met(view().steps, step)}
                    open={open() === step}
                    show={show}
                  />
                  {/* And under the open one, what that step is about. */}
                  <Show when={open() === step}>
                    <div class={styles.body}>
                      <Show when={step === "dependencies"}>
                        <Dependencies
                          reading={view()}
                          onwards={() => onwards(step)}
                          poll={setAsked}
                        />
                      </Show>
                      <Show when={step === "accounts"}>
                        <Accounts
                          reading={view()}
                          onwards={() => onwards(step)}
                        />
                      </Show>
                      {/* And the last of the three, whose Next is the
                          wizard finishing rather than a step opening — which is
                          why it takes no `onwards`: what it goes on to is the
                          app itself. */}
                      <Show when={step === "git"}>
                        <Git />
                      </Show>
                    </div>
                  </Show>
                </li>
              )}
            </For>
          </ol>
        )}
      </Show>
    </section>
  );
}

/// One step's heading: what it is called, whether it is met, and — where it is —
/// the press that opens it again.
///
/// A button only for a step that stands met, because that is the only one there
/// is anything to go back to: an unmet step reached by pressing past the one in
/// front of it would be the skip ADR-0016 refused, drawn as navigation.
function StepHead(props: {
  step: Step;
  met: boolean;
  open: boolean;
  show: (step: Step) => void;
}): JSX.Element {
  /// The mark and the name, which is the heading either way round.
  const named = (): JSX.Element => (
    <>
      <Mark standing={props.met ? "met" : "waiting"} />
      {TITLES[props.step]}
    </>
  );

  return (
    <h2 class={styles.head}>
      <Show when={props.met && !props.open} fallback={named()}>
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

