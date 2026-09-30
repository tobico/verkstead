//! How much Verkstead runs at once, on the settings page: how many stages of one
//! roadmap may be under way together.
//!
//! One number so far, and a card of its own for it rather than a field tucked
//! inside a section about something else — the question is *how much of this
//! machine does Verkstead help itself to*, and there is more than one answer to
//! it: the limit across the whole server is the next one, and it reads beside
//! this one rather than anywhere else.
//!
//! **Three where nobody has said**, which is what a declared roadmap shaped like
//! a fan gets on with. Set to one, a declared roadmap runs its stages one at a
//! time in whatever order its declarations allow — which is the one setting here
//! somebody might reach for on a machine that is doing too much.
//!
//! **A place is taken by every stage that is under way, whatever it is doing.** A
//! stage waiting on an answer and a stage waiting to join the chain are both
//! holding one, so a roadmap set to one starts its next stage when the one before
//! it settles and not before. That is the human's own choice rather than an
//! oversight, and the pane says it in a line.
//!
//! Two halves in two panes, like every section beside it: a card in the middle
//! pane saying how much runs at once, and the field that changes it in the
//! details pane it opens, at `/settings/at-once`. Both read the one settings
//! query the rest of the page reads, and the save goes through the one settings
//! endpoint — so the author, the token and the rest ride along as they stand.
//!
//! One press saves, the way a duration on the Cleanup pane does: the number is
//! typed, so nothing is committed while somebody is halfway through writing it.
//!
//! **And this is the one field here the page itself refuses.** The ignore rules
//! can be turned down as well, but by the server, over a pattern it could not
//! compile. Everywhere else what cannot be read is the default asked for back —
//! an empty box, a word where a number goes — and that holds here too, for the
//! empty box. What does not is a number below one: what it asks for is a roadmap
//! that never starts anything, and a page that quietly turned it into three would
//! be a page that saved something else. So the press is refused and the line says
//! what a number here can be.

import { useMutation, useQueryClient } from "@tanstack/solid-query";
import { Match, Show, Switch as Choose, createSignal, type JSX } from "solid-js";

import { CardButton } from "../CardButton";
import { PaneSticky } from "../Panes";
import { loadSettings, saveSettings } from "../api/client";
import type { AtOnceView, SettingsSaved, SettingsView } from "../api/types";
import { useReading } from "../freshness";
import { Empty, ErrorLine, Note } from "../notices";
import { PaneHead } from "../workbench/PaneHead";
import { heldCleanup, heldInstructions, heldPaths } from "./held";
import styles from "./AtOnce.module.css";

/// What the section is called, in the one place both panes read it from.
const TITLE = "How much runs at once";

/// The settings as they stand, read once for the two panes that draw them — the
/// same read, by the same key, that every other section of this page makes.
function useSettings() {
  return useReading(() => ({
    queryKey: ["settings"],
    queryFn: loadSettings,
    freshness: { reconcile: "id" },
  }));
}


/// How much Verkstead runs at once, as the card that opens the section.
export function AtOnceCard(props: {
  /// Whether the pane beside this is the one that is open.
  open: boolean;
  /// What pressing it does, which is opening that pane.
  press: () => void;
}): JSX.Element {
  const settings = useSettings();

  return (
    <Choose>
      <Match when={settings.isPending}>
        <Empty>Loading…</Empty>
      </Match>
      <Match when={settings.isError}>
        <ErrorLine>
          Could not read the settings: {settings.error?.message}
        </ErrorLine>
      </Match>
      <Match when={settings.data?.at_once}>
        {(atOnce) => (
          <CardButton
            as="article"
            class={styles.atOnceCard}
            open={props.open}
            press={props.press}
          >
            <h2>{TITLE}</h2>

            {/* The number drawn whether or not anybody chose it, the way the
                Cleanup's durations are: what the card answers is *how many*, and
                the default is as much an answer to that as a choice would be.
                In the words that agree with it — one is the number somebody is
                most likely to have set, and *up to 1 stages* would say nobody
                read the line back. */}
            <p class={styles.standing}>
              <Show
                when={atOnce().roadmap_stages === 1}
                fallback={
                  <>
                    Up to{" "}
                    <span class={styles.places}>
                      {atOnce().roadmap_stages} stages
                    </span>{" "}
                    of one roadmap at a time.
                  </>
                }
              >
                <span class={styles.places}>One stage</span> of one roadmap at a
                time.
              </Show>
            </p>
          </CardButton>
        )}
      </Match>
    </Choose>
  );
}

/// Whether what has been typed is a number Verkstead would run a roadmap at, and
/// what is wrong with it where it is not.
///
/// `null` for a field there is nothing to say about, which includes the empty one:
/// clearing it is how the default is asked for back, and there is nothing to
/// refuse in that.
function refused(typed: string): string | null {
  const said = typed.trim();

  if (said === "") {
    return null;
  }

  return /^\d+$/.test(said) && Number(said) >= 1
    ? null
    : "A whole number of stages, one or more — or nothing at all for the default.";
}

/// And the field that changes it, which is the details pane the card opens.
///
/// One field and one press. There is no Cancel: a details pane is left by opening
/// something else or by the way back a narrow window draws, and what was typed
/// and never saved goes with it.
export function AtOncePane(props: {
  /// The way back to the settings, which is the pane this one was entered from.
  back: () => void;
}): JSX.Element {
  const queries = useQueryClient();
  const settings = useSettings();

  // What has been typed, or `null` while nothing has — the field follows the
  // server until somebody touches it, as the build cache's size does.
  const [typed, setTyped] = createSignal<string | null>(null);

  const told = (): SettingsView | undefined => settings.data;
  const atOnce = (): AtOnceView | undefined => told()?.at_once;

  /// What the field holds: what was typed, else the number somebody configured,
  /// else nothing at all — because a limit nobody chose is drawn as the
  /// placeholder underneath rather than as text in the box.
  const held = () =>
    typed() ??
    (atOnce()?.roadmap_stages_configured
      ? String(atOnce()?.roadmap_stages)
      : "");

  /// What is wrong with what is in the box, where anything is: the one refusal on
  /// this page that is the page's own rather than the server's.
  const trouble = () => refused(held());

  const save = useMutation(() => ({
    mutationFn: (roadmap_stages: string) =>
      saveSettings({
        // The rest of both files as they stand: the endpoint writes them whole,
        // so a section left out would be a section emptied.
        git_author: told()?.git_author ?? { name: "", email: "" },
        // Untouched. This form has no business with the credentials, and a
        // blank token field read as *clear this* is exactly what `Keep` is
        // here to stop.
        github_token: "Keep",
        rust_build_cache: {
          enabled: told()?.rust_build_cache.enabled ?? true,
          size: told()?.rust_build_cache.size_configured
            ? (told()?.rust_build_cache.size ?? "")
            : "",
        },
        // And what becomes of an archived Conversation — see [`heldCleanup`].
        cleanup: heldCleanup(told()),
        conflict_resolution: told()?.conflict_resolution ?? "Merge",
        share_on_done: told()?.share_on_done ?? false,
        // And the paths as they stand — see [`heldPaths`].
        ...heldPaths(told()),
        // And the text every session is given, likewise — see
        // [`heldInstructions`].
        ...heldInstructions(told()),
        // And the ignore rules left exactly where they are. Alone among the
        // settings they travel as an action rather than a value: this form has
        // nothing to say about them, and one that spoke for them could have its
        // own save refused over a pattern it never showed anybody — see
        // [`IgnoredCommentsEdit`].
        ignored_comments: "Keep",
        // And the declared MCP servers, for the reason beside it: they are the
        // other setting that travels as an action, and a section that spoke for
        // them could have its own save refused over a name somebody hand-edited
        // into the file weeks ago — see [`McpServersEdit`].
        mcp_servers: "Keep",
        // And the one thing this form is about, as it was typed: an empty field
        // is the default asked for back, and the press never sends a number the
        // roadmap could not run — see [`refused`].
        at_once: { roadmap_stages },
      }),
    onSuccess: (saved: SettingsSaved) => {
      // What was typed goes, because the answer is now what the field follows.
      setTyped(null);

      // The save's answer *is* a fresh read of both files, so a second read
      // would learn nothing and could only disagree with what is on screen.
      queries.setQueryData(["settings"], saved.settings);
    },
  }));

  const commit = (ev: SubmitEvent) => {
    ev.preventDefault();

    if (trouble() !== null) {
      return;
    }

    save.mutate(held().trim());
  };

  return (
    <>
      <PaneSticky>
        <PaneHead back={{ to: "Settings", go: props.back }} title={TITLE} />
      </PaneSticky>

      <Choose>
        <Match when={settings.isPending}>
          <Empty>Loading…</Empty>
        </Match>
        <Match when={settings.isError}>
          <ErrorLine>
            Could not read the settings: {settings.error?.message}
          </ErrorLine>
        </Match>
        <Match when={atOnce()}>
          {(set) => (
            <form class={styles.atOnce} onSubmit={commit}>
              {/* The one thing about this number a human cannot work out from
                  it: which stages are counted. A stage nobody is waiting on and
                  a stage waiting on an answer take a place alike, so a roadmap
                  can be holding all of them without anything running. */}
              <Note>
                Every stage that is under way holds a place, including one
                waiting on an answer from you. A change takes effect at the next
                start and stops nothing already running.
              </Note>

              <label for="at-once-roadmap-stages">
                Stages of one roadmap at once
              </label>
              <div class={styles.field}>
                <input
                  id="at-once-roadmap-stages"
                  type="text"
                  inputmode="numeric"
                  autocapitalize="off"
                  autocorrect="off"
                  spellcheck={false}
                  // The default, so an empty box reads as the number nobody has
                  // chosen rather than as no number at all.
                  placeholder={String(set().roadmap_stages)}
                  value={held()}
                  disabled={save.isPending}
                  onInput={(ev) => setTyped(ev.currentTarget.value)}
                />
                <button type="submit" disabled={save.isPending}>
                  Save
                </button>
              </div>

              <Show when={trouble()}>
                {(why) => (
                  <ErrorLine class={styles.failure}>{why()}</ErrorLine>
                )}
              </Show>

              <Show when={save.isError}>
                <ErrorLine class={styles.failure}>
                  The settings could not be saved: {save.error?.message}
                </ErrorLine>
              </Show>
            </form>
          )}
        </Match>
      </Choose>
    </>
  );
}
