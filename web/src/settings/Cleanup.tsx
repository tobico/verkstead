//! What becomes of a Conversation the human has archived, on the settings page:
//! the trim that takes its bulk some days on, and the delete that takes the
//! whole of it later.
//!
//! The one section here about the record rather than about a machine.
//! Everything else Verkstead does keeps what it was given; this is where it is
//! told to let go, and both clocks start at the archiving — the moment the human
//! said they were finished looking.
//!
//! **Two rows that fall back the two different ways**, which is the shape of the
//! whole section. The trim is **on** at three days: what it takes is the full
//! agent output, the Transcripts and the session names, which is everything a
//! Share never carried, so a human should not be keeping gigabytes of it for
//! never having found this page. The delete is **off** at thirty: it is the one
//! thing in Verkstead that forgets, and forgetting is not something to start
//! doing to somebody who has never said it should.
//!
//! Nothing here refuses anything, a delete sooner than the trim included: the
//! two clocks run from the archiving independently, so the Conversation is
//! simply deleted before it was ever trimmed. The pane says so where the two
//! numbers read that way, because it is a surprising thing to have typed rather
//! than a mistake to correct.
//!
//! Two halves in two panes, which is what the settings page is: a card in the
//! middle pane saying what happens and when, and the controls that change it in
//! the details pane it opens, at `/settings/cleanup`. Both read the one settings
//! query every other section reads, and the save goes through the one settings
//! endpoint, which writes both files — so the author, the token and the rest
//! ride along as they stand.
//!
//! Two ways to save, because there are two kinds of control, and the build
//! cache's own for the same reasons: a switch saves itself the moment it is
//! flipped, and a duration is typed, so it saves on a press of its own — nothing
//! is committed while somebody is halfway through writing `30`.

import { useMutation, useQueryClient } from "@tanstack/solid-query";
import { Match, Show, Switch as Choose, createSignal, type JSX } from "solid-js";

import { CardButton } from "../CardButton";
import { PaneSticky } from "../Panes";
import { Switch } from "../Switch";
import { loadSettings, saveSettings } from "../api/client";
import type {
  CleanupEdit,
  CleanupStepView,
  CleanupView,
  SettingsSaved,
  SettingsView,
} from "../api/types";
import { useReading } from "../freshness";
import { Empty, ErrorLine, Note } from "../notices";
import { PaneHead } from "../workbench/PaneHead";
import { heldPaths } from "./held";
import styles from "./Cleanup.module.css";

/// The settings as they stand, read once for the two panes that draw them — the
/// same read, by the same key, that every other section of this page makes.
function useSettings() {
  return useReading(() => ({
    queryKey: ["settings"],
    queryFn: loadSettings,
    freshness: { reconcile: "id" },
  }));
}

/// How one of the two steps stands, in a line: who it is about, what happens to
/// them, and how long after the archiving.
///
/// The days are drawn as the number they are whether or not anybody typed them,
/// because what the card answers is *when*, and the default is as much an answer
/// to that as a choice would be.
function standing(step: CleanupStepView, who: string, does: string): JSX.Element {
  return (
    <p class={styles.standing}>
      <Show when={step.enabled} fallback={<>{who} are never {does}.</>}>
        {who} are {does} <span class={styles.days}>{step.days} days</span> after
        archiving.
      </Show>
    </p>
  );
}

/// Whether the two durations read as a delete that arrives before the trim ever
/// would.
///
/// Only where both are on: with the delete off there is nothing to arrive
/// first, and with the trim off nothing was ever going to happen at the other
/// number.
function deletedFirst(cleanup: CleanupView): boolean {
  return (
    cleanup.trim.enabled &&
    cleanup.delete.enabled &&
    cleanup.delete.days <= cleanup.trim.days
  );
}

/// What is said where they do. Not a refusal and not an error: the two clocks
/// run from the archiving independently, so this is what those numbers mean
/// rather than what is wrong with them.
function deletedFirstNote(): JSX.Element {
  return (
    <p class={styles.ordering}>
      The delete comes first at these durations, so an archived conversation is
      deleted before it is ever trimmed.
    </p>
  );
}

/// What becomes of an archived Conversation, as the card that opens the
/// section.
export function CleanupCard(props: {
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
      <Match when={settings.data?.cleanup}>
        {(cleanup) => (
          <CardButton
            as="article"
            class={styles.cleanupCard}
            open={props.open}
            press={props.press}
          >
            <h2>Cleanup</h2>

            {standing(cleanup().trim, "Archived conversations", "trimmed")}
            {standing(cleanup().delete, "They", "deleted for good")}

            <Show when={deletedFirst(cleanup())}>{deletedFirstNote()}</Show>
          </CardButton>
        )}
      </Match>
    </Choose>
  );
}

/// And the controls that change it, which is the details pane the card opens.
///
/// There is no Save over the whole of it and no Cancel: each switch is its own
/// press and each duration has one of its own, and a details pane is left by
/// opening something else or by the way back a narrow window draws.
export function CleanupPane(props: {
  /// The way back to the settings, which is the pane this one was entered from.
  back: () => void;
}): JSX.Element {
  const queries = useQueryClient();
  const settings = useSettings();

  // What has been typed into each field, or `null` while nothing has — the
  // field follows the server until somebody touches it, as the build cache's
  // size does.
  const [trimTyped, setTrimTyped] = createSignal<string | null>(null);
  const [deleteTyped, setDeleteTyped] = createSignal<string | null>(null);

  const told = (): SettingsView | undefined => settings.data;
  const cleanup = () => told()?.cleanup;

  /// What one field holds: what was typed, else the days somebody configured,
  /// else nothing at all — because a duration nobody chose is drawn as the
  /// placeholder underneath rather than as text in the box.
  const held = (step: CleanupStepView | undefined, typed: string | null) =>
    typed ?? (step?.days_configured ? String(step.days) : "");

  const trimDays = () => held(cleanup()?.trim, trimTyped());
  const deleteDays = () => held(cleanup()?.delete, deleteTyped());

  /// The two rows as they stand on the page, which is what every save sends:
  /// one request writes the whole of `config.yaml`, so the row this press is
  /// not about rides along as it is.
  const rows = (): CleanupEdit => ({
    trim: { enabled: cleanup()?.trim.enabled ?? true, days: trimDays() },
    delete: { enabled: cleanup()?.delete.enabled ?? false, days: deleteDays() },
  });

  const save = useMutation(() => ({
    mutationFn: (edit: CleanupEdit) => {
      const author = told()?.git_author ?? { name: "", email: "" };

      return saveSettings({
        git_author: author,
        // Untouched. This form has no business with the credentials, and a
        // blank token field read as *clear this* is exactly what `Keep` is
        // here to stop.
        github_token: "Keep",
        // And the build cache as it stands, which is the section above this
        // one: a save says what the file holds afterwards, so a value left out
        // would be a value unset.
        rust_build_cache: {
          enabled: told()?.rust_build_cache.enabled ?? true,
          size: told()?.rust_build_cache.size_configured
            ? (told()?.rust_build_cache.size ?? "")
            : "",
        },
        cleanup: edit,
        // And how a conflict is resolved, and whether Done shares the record to
        // the pull request, for that reason again.
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
      });
    },
    onSuccess: (saved: SettingsSaved) => {
      // What was typed goes, because the answer is now what the fields follow.
      setTrimTyped(null);
      setDeleteTyped(null);

      // The save's answer *is* a fresh read of both files, so a second read
      // would learn nothing and could only disagree with what is on screen.
      queries.setQueryData(["settings"], saved.settings);
    },
  }));

  /// A switch flipped, which saves itself: the row it is on takes the new
  /// answer and the other rides along as it stands.
  const flip = (which: keyof CleanupEdit, enabled: boolean) => {
    const asked = rows();
    save.mutate({ ...asked, [which]: { ...asked[which], enabled } });
  };

  /// And a duration pressed, which sends both rows as the page holds them —
  /// the field it was pressed from included.
  const commit = (ev: SubmitEvent) => {
    ev.preventDefault();
    save.mutate(rows());
  };

  return (
    <>
      <PaneSticky>
        <PaneHead back={{ to: "Settings", go: props.back }} title="Cleanup" />
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
        <Match when={cleanup()}>
          {(set) => (
            <div class={styles.cleanup}>
              <Note>
                Both clocks start when a conversation is archived, and
                unarchiving one stops them. Neither touches anything outside
                Verkstead: the git branch stays, and a share that has been
                published stays published.
              </Note>

              <section class={styles.step}>
                <Switch
                  label="Trim archived conversations"
                  on={set().trim.enabled}
                  disabled={save.isPending}
                  flip={(enabled) => flip("trim", enabled)}
                />

                <Note>
                  The full agent output, the transcripts and the session records
                  go — everything a share never carried. Every card on the
                  timeline stays, so the record still reads whole.
                </Note>

                <Show when={set().trim.enabled}>
                  <form class={styles.timing} onSubmit={commit}>
                    <label for="cleanup-trim-days">
                      Days after archiving before trimming
                    </label>
                    <div class={styles.field}>
                      <input
                        id="cleanup-trim-days"
                        type="text"
                        inputmode="numeric"
                        autocapitalize="off"
                        autocorrect="off"
                        spellcheck={false}
                        // The default, so an empty box reads as the duration
                        // nobody has chosen rather than as no duration at all.
                        placeholder={String(set().trim.days)}
                        value={trimDays()}
                        onInput={(ev) => setTrimTyped(ev.currentTarget.value)}
                      />
                      <button type="submit" disabled={save.isPending}>
                        Save
                      </button>
                    </div>
                  </form>
                </Show>
              </section>

              <section class={styles.step}>
                <Switch
                  label="Delete archived conversations for good"
                  on={set().delete.enabled}
                  disabled={save.isPending}
                  flip={(enabled) => flip("delete", enabled)}
                />

                <Note>
                  The whole conversation goes: every card, every session and the
                  timeline itself, off the sidebar even under Show archived. It
                  cannot be undone.
                </Note>

                <Show when={set().delete.enabled}>
                  <form class={styles.timing} onSubmit={commit}>
                    <label for="cleanup-delete-days">
                      Days after archiving before deleting
                    </label>
                    <div class={styles.field}>
                      <input
                        id="cleanup-delete-days"
                        type="text"
                        inputmode="numeric"
                        autocapitalize="off"
                        autocorrect="off"
                        spellcheck={false}
                        placeholder={String(set().delete.days)}
                        value={deleteDays()}
                        onInput={(ev) => setDeleteTyped(ev.currentTarget.value)}
                      />
                      <button type="submit" disabled={save.isPending}>
                        Save
                      </button>
                    </div>
                  </form>
                </Show>
              </section>

              <Show when={deletedFirst(set())}>{deletedFirstNote()}</Show>

              <Show when={save.isError}>
                <ErrorLine class={styles.failure}>
                  The settings could not be saved: {save.error?.message}
                </ErrorLine>
              </Show>
            </div>
          )}
        </Match>
      </Choose>
    </>
  );
}
