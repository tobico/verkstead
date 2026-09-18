//! The one text every session is given, on the settings page: what a human
//! would have put in their own global `CLAUDE.md`, said here instead.
//!
//! A session runs in a Built Root of Verkstead's own making, which holds none
//! of the account's files — so the global instructions a human has spent a year
//! refining are not there, and a session starts knowing only what its Repo
//! carries. This is what takes their place: one text, written into each root as
//! the file that harness reads, and given to any harness that has no such file
//! as a section of its prompt.
//!
//! **One text for every Agent Profile**, rather than one per Profile. It is a
//! thing Verkstead is *told* — like the git author and the sandbox binds, and
//! unlike anything a Conversation is settled against — so it sits in
//! `config.yaml` and is read at the moment a session needs it: a change here
//! reaches the next session, and a running one keeps what it started with.
//!
//! The Repo's own instructions are untouched. A `CLAUDE.md` or an `AGENTS.md`
//! in the Worktree is the repository's, and is read exactly as it always was;
//! this text sits above it the way the human's global file used to.
//!
//! Two halves in two panes, like every section beside it: a card in the middle
//! pane saying whether there is a text and what it reaches, and the box that
//! rewrites it in the details pane it opens, at `/settings/instructions`. Both
//! read the one settings query the rest of the page reads, and the save goes
//! through the one settings endpoint — so the author, the token and the rest
//! ride along as they stand.
//!
//! **Nothing here can be refused.** It is a paragraph of somebody's prose for
//! an agent to read: there is no grammar to get wrong, nothing to compile and
//! nobody to ask about it. An empty box is a setting nobody has made rather
//! than an error, which is what a fresh installation draws.
//!
//! One press saves, the way a duration on the Cleanup pane does and unlike a
//! checkbox anywhere: the text is typed, so nothing is committed while somebody
//! is halfway through a sentence. And what is saved is what was typed, verbatim
//! — the blank line between two paragraphs and the indent of a list are the
//! writing rather than slips in it, and a harness is handed these words.

import { useMutation, useQueryClient } from "@tanstack/solid-query";
import { Match, Show, Switch as Choose, createSignal, type JSX } from "solid-js";

import app from "../App.module.css";
import { CardButton } from "../CardButton";
import { PaneSticky } from "../Panes";
import { loadSettings, saveSettings } from "../api/client";
import type { SettingsSaved, SettingsView } from "../api/types";
import { useReading } from "../freshness";
import { Empty, ErrorLine, Note } from "../notices";
import { PaneHead } from "../workbench/PaneHead";
import { heldCleanup, heldPaths } from "./held";
import styles from "./Instructions.module.css";

/// What the section is called, in the one place both panes read it from.
const TITLE = "Instructions";

/// The settings as they stand, read once for the two panes that draw them — the
/// same read, by the same key, that every other section of this page makes.
function useSettings() {
  return useReading(() => ({
    queryKey: ["settings"],
    queryFn: loadSettings,
    freshness: { reconcile: "id" },
  }));
}

/// The text every session is given, as the card that opens the section.
///
/// What is on the card is the one thing somebody scanning the page is after:
/// whether there is a text at all, and what it reaches either way. The words
/// themselves are behind the press — a card is read past rather than read, and
/// a paragraph of prose on one would be the whole page.
export function InstructionsCard(props: {
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
      <Match when={settings.data}>
        {(told) => (
          <CardButton
            as="article"
            class={styles.instructionsCard}
            open={props.open}
            press={props.press}
          >
            <h2>{TITLE}</h2>

            <p class={styles.standing}>
              <Show
                when={told().instructions !== ""}
                fallback={
                  <>
                    Nothing is configured, so a session is told only what its
                    repository carries.
                  </>
                }
              >
                Given to every session, whatever agent runs it, above whatever
                instructions its repository carries.
              </Show>
            </p>
          </CardButton>
        )}
      </Match>
    </Choose>
  );
}

/// And the box that rewrites it, which is the details pane the card opens.
///
/// One field and one press. There is no Cancel: a details pane is left by
/// opening something else or by the way back a narrow window draws, and what
/// was typed and never saved goes with it.
export function InstructionsPane(props: {
  /// The way back to the settings, which is the pane this one was entered from.
  back: () => void;
}): JSX.Element {
  const queries = useQueryClient();
  const settings = useSettings();

  // What has been typed, or `null` while nothing has — the field follows the
  // server until somebody touches it, the way every other field on this page
  // does. Which is what lets a save made on a phone show up in a tab left open
  // here, and what stops one overwriting a paragraph being written.
  const [typed, setTyped] = createSignal<string | null>(null);

  const told = (): SettingsView | undefined => settings.data;

  /// What the box holds: what was typed, else the text the server last gave.
  ///
  /// No placeholder underneath it. Every other empty field on this page is
  /// showing a default somebody would otherwise have to guess at; the default
  /// here is *nothing*, and a suggestion drawn in the box would be Verkstead
  /// putting words in the human's mouth.
  const text = () => typed() ?? told()?.instructions ?? "";

  const save = useMutation(() => ({
    mutationFn: (instructions: string) =>
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
        // And the ignore rules left exactly where they are. Alone among the
        // settings they travel as an action rather than a value: this form has
        // nothing to say about them, and one that spoke for them could have its
        // own save refused over a pattern it never showed anybody — see
        // [`IgnoredCommentsEdit`].
        ignored_comments: "Keep",
        // And the one thing this form is about, exactly as it was typed: what
        // is sent is what the file holds afterwards, so an emptied box is the
        // text taken away.
        instructions,
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
    save.mutate(text());
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
        <Match when={told()}>
          <form class={styles.instructions} onSubmit={commit}>
            {/* What the section is for, in the one line a pane on this page
                says its own in. Both halves of it: where the text goes, and
                what it does not displace. */}
            <Note>
              Given to every session as the agent's own global instructions,
              whatever agent its profile runs. A repository's own CLAUDE.md or
              AGENTS.md is read as it always was, under this.
            </Note>

            {/* A copy of what has been typed gives the field its height — see
                `.grow` in `App.module.css`. */}
            <div class={`${app.grow} ${styles.field}`} data-value={text()}>
              <textarea
                id="instructions"
                rows="1"
                aria-label={TITLE}
                value={text()}
                disabled={save.isPending}
                onInput={(ev) => setTyped(ev.currentTarget.value)}
              />
            </div>

            <div class={styles.saving}>
              <button type="submit" disabled={save.isPending}>
                Save
              </button>
            </div>

            <Show when={save.isError}>
              <ErrorLine class={styles.failure}>
                The settings could not be saved: {save.error?.message}
              </ErrorLine>
            </Show>
          </form>
        </Match>
      </Choose>
    </>
  );
}
