//! How much Verkstead runs at once, on the settings page: how many stages of one
//! roadmap may be under way together, and how many Conversations take a place
//! across the whole server.
//!
//! Two numbers and one card, rather than a field tucked inside a section about
//! something else — the question both answer is *how much of this machine does
//! Verkstead help itself to*, and there is more than one answer to it. They limit
//! different things and both are in force: the roadmap's is about how much of one
//! effort is open at once, and the server's is there for the machine, a stage
//! being a heavy build and a test run on hardware everything else shares.
//!
//! **Three and four where nobody has said**, which is a declared roadmap shaped
//! like a fan getting on with its fan while no one roadmap takes the whole
//! server. Set the roadmap's to one and a declared roadmap runs its stages one at
//! a time in whatever order its declarations allow; set the server's to one and
//! Verkstead starts nothing by itself while anything at all is running — which is
//! what somebody reaches for on a machine that is doing too much.
//!
//! **A place is taken by every stage that is under way, whatever it is doing**,
//! and a place on the server by every Conversation with a session running or a
//! driver registered — a grilling, a Review, a Tinker, another roadmap's stage
//! alike. A stage waiting on an answer and a stage waiting to join the chain are
//! both holding one, so a roadmap set to one starts its next stage when the one
//! before it settles and not before. That is the human's own choice rather than
//! an oversight, and the pane says it in a line.
//!
//! **And the pane says how many of the server's places are taken as it was
//! drawn.** A server whose places are all held by stages waiting on answers
//! starts nothing more until one is answered, and a number nobody could see would
//! leave that looking like a stall. A reading rather than a setting: it is off the
//! same two registers a start is weighed against, as of the moment the page asked,
//! and it can stand above the limit — a press goes ahead over it and is counted
//! from then on.
//!
//! Two halves in two panes, like every section beside it: a card in the middle
//! pane saying how much runs at once, and the fields that change it in the
//! details pane it opens, at `/settings/at-once`. Both read the one settings
//! query the rest of the page reads, and the save goes through the one settings
//! endpoint — so the author, the token and the rest ride along as they stand.
//!
//! One press per field, the way a duration on the Cleanup pane is saved: the
//! numbers are typed, so nothing is committed while somebody is halfway through
//! writing one. Either press sends both, the page having one request to write the
//! whole file with.
//!
//! **And these are the one pair of fields here the page itself refuses.** The
//! ignore rules can be turned down as well, but by the server, over a pattern it
//! could not compile. Everywhere else what cannot be read is the default asked for
//! back — an empty box, a word where a number goes — and that holds here too, for
//! the empty box. What does not is a number below one: what it asks for is a
//! roadmap, or a server, that never starts anything, and a page that quietly
//! turned it into three would be a page that saved something else. So the press is
//! refused and the line says what a number here can be.

import { useMutation, useQueryClient } from "@tanstack/solid-query";
import { Match, Show, Switch as Choose, createSignal, type JSX } from "solid-js";

import { CardButton } from "../CardButton";
import { PaneSticky } from "../Panes";
import { loadSettings, saveSettings } from "../api/client";
import type {
  AtOnceEdit,
  AtOnceView,
  SettingsSaved,
  SettingsView,
} from "../api/types";
import { useReading } from "../freshness";
import { Empty, ErrorLine, Note } from "../notices";
import { PaneHead } from "../workbench/PaneHead";
import {
  heldCleanup,
  heldInstructions,
  heldLanguages,
  heldPaths,
} from "./held";
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

            {/* Both numbers drawn whether or not anybody chose them, the way the
                Cleanup's durations are: what the card answers is *how many*, and
                a default is as much an answer to that as a choice would be.
                In the words that agree with each — one is the number somebody is
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

            <p class={styles.standing}>
              <Show
                when={atOnce().conversations === 1}
                fallback={
                  <>
                    Up to{" "}
                    <span class={styles.places}>
                      {atOnce().conversations} conversations
                    </span>{" "}
                    across the whole server.
                  </>
                }
              >
                <span class={styles.places}>One conversation</span> across the
                whole server.
              </Show>
            </p>
          </CardButton>
        )}
      </Match>
    </Choose>
  );
}

/// Whether what has been typed is a number Verkstead would run on, and what is
/// wrong with it where it is not.
///
/// `null` for a field there is nothing to say about, which includes the empty one:
/// clearing it is how the default is asked for back, and there is nothing to
/// refuse in that.
///
/// `what` is what the number counts, because the line says it back: a floor
/// nobody can read is one somebody argues with.
function refused(typed: string, what: string): string | null {
  const said = typed.trim();

  if (said === "") {
    return null;
  }

  return /^\d+$/.test(said) && Number(said) >= 1
    ? null
    : `A whole number of ${what}, one or more — or nothing at all for the default.`;
}

/// And the fields that change it, which is the details pane the card opens.
///
/// Two fields and a press each. There is no Cancel: a details pane is left by
/// opening something else or by the way back a narrow window draws, and what was
/// typed and never saved goes with it.
export function AtOncePane(props: {
  /// The way back to the settings, which is the pane this one was entered from.
  back: () => void;
}): JSX.Element {
  const queries = useQueryClient();
  const settings = useSettings();

  // What has been typed into each field, or `null` while nothing has — a field
  // follows the server until somebody touches it, as the build cache's size does.
  const [stagesTyped, setStagesTyped] = createSignal<string | null>(null);
  const [serverTyped, setServerTyped] = createSignal<string | null>(null);

  const told = (): SettingsView | undefined => settings.data;
  const atOnce = (): AtOnceView | undefined => told()?.at_once;

  /// What one field holds: what was typed, else the number somebody configured,
  /// else nothing at all — because a limit nobody chose is drawn as the
  /// placeholder underneath rather than as text in the box.
  const held = (
    limit: number | undefined,
    configured: boolean | undefined,
    typed: string | null,
  ) => typed ?? (configured ? String(limit) : "");

  const stages = () =>
    held(
      atOnce()?.roadmap_stages,
      atOnce()?.roadmap_stages_configured,
      stagesTyped(),
    );
  const conversations = () =>
    held(
      atOnce()?.conversations,
      atOnce()?.conversations_configured,
      serverTyped(),
    );

  /// What is wrong with what is in each box, where anything is: the one refusal
  /// on this page that is the page's own rather than the server's.
  const stagesTrouble = () => refused(stages(), "stages");
  const serverTrouble = () => refused(conversations(), "conversations");

  /// Both fields as the page holds them, typed numbers and all, which is what
  /// either press sends: one request writes the whole of `config.yaml`, so the
  /// field this press is not about rides along as it stands on screen.
  const both = (): AtOnceEdit => ({
    roadmap_stages: stages().trim(),
    conversations: conversations().trim(),
  });

  const save = useMutation(() => ({
    mutationFn: (at_once: AtOnceEdit) =>
      saveSettings({
        // The rest of both files as they stand: the endpoint writes them whole,
        // so a section left out would be a section emptied.
        git_author: told()?.git_author ?? { name: "", email: "" },
        // Untouched. This form has no business with the credentials, and a
        // blank token field read as *clear this* is exactly what `Keep` is
        // here to stop.
        github_token: "Keep",
        // And the languages as they stand — see [`heldLanguages`].
        ...heldLanguages(told()),
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
        // is the default asked for back, and the press never sends a number
        // nothing could be run at — see [`refused`].
        at_once,
      }),
    onSuccess: (saved: SettingsSaved) => {
      // What was typed goes, because the answer is now what the fields follow.
      setStagesTyped(null);
      setServerTyped(null);

      // The save's answer *is* a fresh read of both files, so a second read
      // would learn nothing and could only disagree with what is on screen.
      queries.setQueryData(["settings"], saved.settings);
    },
  }));

  /// A press on either field, which sends both as the page holds them — and is
  /// refused while either of them holds a number nothing could be run at, this
  /// one request being what writes them both.
  const commit = (ev: SubmitEvent) => {
    ev.preventDefault();

    if (stagesTrouble() !== null || serverTrouble() !== null) {
      return;
    }

    save.mutate(both());
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
              {/* The one thing about these numbers a human cannot work out from
                  them: which work is counted. A stage nobody is waiting on and a
                  stage waiting on an answer take a place alike, so a roadmap — or
                  a whole server — can be holding all of them without anything
                  running. */}
              <Note>
                Every stage that is under way holds a place, including one
                waiting on an answer from you, and so does every conversation
                with a session running or a run being driven. A change takes
                effect at the next start and stops nothing already running.
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
                  value={stages()}
                  disabled={save.isPending}
                  onInput={(ev) => setStagesTyped(ev.currentTarget.value)}
                />
                <button type="submit" disabled={save.isPending}>
                  Save
                </button>
              </div>

              <Show when={stagesTrouble()}>
                {(why) => (
                  <ErrorLine class={styles.failure}>{why()}</ErrorLine>
                )}
              </Show>

              <label for="at-once-conversations">
                Conversations across the whole server
              </label>
              <div class={styles.field}>
                <input
                  id="at-once-conversations"
                  type="text"
                  inputmode="numeric"
                  autocapitalize="off"
                  autocorrect="off"
                  spellcheck={false}
                  placeholder={String(set().conversations)}
                  value={conversations()}
                  disabled={save.isPending}
                  onInput={(ev) => setServerTyped(ev.currentTarget.value)}
                />
                <button type="submit" disabled={save.isPending}>
                  Save
                </button>
              </div>

              <Show when={serverTrouble()}>
                {(why) => (
                  <ErrorLine class={styles.failure}>{why()}</ErrorLine>
                )}
              </Show>

              {/* What the limit above is measured against, beside it: a server
                  holding every place starts nothing more until one comes free,
                  and this is what says so rather than leaving it to look like a
                  stall. As of the read this pane was drawn from — it can stand
                  above the limit, a press going ahead over it. */}
              <p class={styles.taken}>
                <span class={styles.places}>
                  {set().places_taken} of {set().conversations}
                </span>{" "}
                taken when this was read.
              </p>

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
