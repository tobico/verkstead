//! How a conflicted pull request is resolved, on the settings page: the base
//! merged into the branch, or the branch rebased onto the base and force-pushed.
//!
//! A wrapping Conversation's pull requests are watched for conflicts, and one
//! GitHub cannot merge gets a session of Verkstead's own sent at it. What that
//! session is *told to do* is this setting: the two are different acts on a
//! branch, and only one of them is safe to make unattended without saying so
//! first.
//!
//! **Merge is what nobody choosing anything gets**, and the pane says why in the
//! same breath as it offers the other: a rebase rewrites the branch and has to
//! be force-pushed, which moves every commit a reviewer has already read and
//! breaks whatever was stacked on it. That is the kind of thing found weeks
//! later by a stage that will not push, so it is said here, beside the choice,
//! rather than left to be discovered.
//!
//! One setting for every Repo, and any Repo may say otherwise — that override is
//! a fact about the Repo and is set from its own pane, in `repos/RepoList.tsx`.
//! This is what a Repo that says nothing falls back to.
//!
//! Two halves in two panes, which is what this page is: a card in the middle
//! saying how conflicts are resolved, and the picker that changes it in the
//! details pane it opens, at `/settings/conflicts`. Both read the one settings
//! query the sections above them read, and the save goes through the one
//! settings endpoint, which writes both files — so the author, the token and the
//! rest ride along as they stand.

import { useMutation, useQueryClient } from "@tanstack/solid-query";
import { Match, Show, Switch as Choose, type JSX } from "solid-js";

import { CardButton } from "../CardButton";
import { PaneSticky } from "../Panes";
import { loadSettings, saveSettings } from "../api/client";
import type { ConflictResolution, SettingsSaved, SettingsView } from "../api/types";
import { useReading } from "../freshness";
import { Empty, ErrorLine, Note } from "../notices";
import { Picker } from "../picking";
import { PaneHead } from "../workbench/PaneHead";
import styles from "./Conflicts.module.css";
import { heldPaths } from "./held";

/// What each strategy is called where a human reads it, and what it does said
/// in one line.
///
/// Written once and read by both panes here and by the Repo's own pane, because
/// all three are offering the same two answers: three spellings of *rebase*
/// would be three chances to describe it differently, and the description is the
/// part that matters.
export const RESOLUTION: Record<ConflictResolution, string> = {
  Merge: "Merge the base branch in",
  Rebase: "Rebase onto the base branch",
};

/// And what each of them does to the pull request, for the line under the
/// picker: what a human is choosing between rather than what it is called.
export const RESOLVES: Record<ConflictResolution, string> = {
  Merge:
    "The base branch is merged into the work branch and the merge is pushed. Every commit stays where it is, so nothing anybody has read moves.",
  Rebase:
    "The work branch is rebased onto the base branch and force-pushed. Every commit is rewritten, so what reviewers have already read moves and anything stacked on the branch breaks.",
};

/// What is said wherever a rebase is the answer in force. The cost of the
/// choice, said where the choice is made.
export function forcePushed(): JSX.Element {
  return (
    <p class={styles.warning}>
      A rebase is force-pushed. It rewrites what reviewers have already seen, and
      it breaks any stage stacked on this branch.
    </p>
  );
}

/// The settings as they stand, read once for the two panes that draw them — the
/// same read, by the same key, that every other section of this page makes.
function useSettings() {
  return useReading(() => ({
    queryKey: ["settings"],
    queryFn: loadSettings,
    freshness: { reconcile: "id" },
  }));
}

/// How conflicts are resolved, as the card that opens the section.
export function ConflictsCard(props: {
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
            class={styles.conflictsCard}
            open={props.open}
            press={props.press}
          >
            <h2>Conflict resolution</h2>

            {/* Which of the two is in force, which is what somebody scanning
                the page came to check — and the warning with it where it is the
                one that rewrites branches, because whoever needs to read that
                is precisely whoever is not editing. */}
            <p class={styles.standing}>
              A pull request that will not merge is resolved by{" "}
              <span class={styles.strategy}>
                {told().conflict_resolution === "Rebase"
                  ? "rebasing the branch onto its base"
                  : "merging the base branch in"}
              </span>
              , unless the repo says otherwise.
            </p>

            <Show when={told().conflict_resolution === "Rebase"}>
              {forcePushed()}
            </Show>
          </CardButton>
        )}
      </Match>
    </Choose>
  );
}

/// And the picker that changes it, which is the details pane the card opens.
///
/// There is no Save and no Cancel: a picker is its own press, the way the build
/// cache's switch is — a choice that needed confirming afterwards is a choice
/// the human has to make twice.
export function ConflictsPane(props: {
  /// The way back to the settings, which is the pane this one was entered from.
  back: () => void;
}): JSX.Element {
  const queries = useQueryClient();
  const settings = useSettings();

  const told = (): SettingsView | undefined => settings.data;

  const save = useMutation(() => ({
    mutationFn: (conflict_resolution: ConflictResolution) => {
      const settings = told();

      return saveSettings({
        // The rest of both files as they stand: the endpoint writes them whole,
        // and this section has no business with any of it.
        git_author: settings?.git_author ?? { name: "", email: "" },
        github_token: "Keep",
        rust_build_cache: {
          enabled: settings?.rust_build_cache.enabled ?? true,
          size: settings?.rust_build_cache.size_configured
            ? (settings?.rust_build_cache.size ?? "")
            : "",
        },
        conflict_resolution,
        // And the paths as the read left them: a list this section left out
        // would be a list it emptied — see [`heldPaths`].
        ...heldPaths(settings),
      });
    },
    // The save's answer *is* a fresh read of both files, so a second read would
    // learn nothing and could only disagree with what is on screen.
    onSuccess: (saved: SettingsSaved) =>
      queries.setQueryData(["settings"], saved.settings),
  }));

  return (
    <>
      <PaneSticky>
        <PaneHead
          back={{ to: "Settings", go: props.back }}
          title="Conflict resolution"
        />
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
          {(set) => (
            <div class={styles.conflicts}>
              <Note>
                A branch its base has moved under conflicts without anybody
                touching it, and nothing lands until the conflict is resolved.
                Verkstead sends a session at one and this is what that session is
                told to do. A repo can be given an answer of its own, and then
                this is what every other repo does.
              </Note>

              <div class={styles.choosing}>
                <label for="conflict-resolution">
                  How a conflicted pull request is resolved
                </label>
                <Picker
                  id="conflict-resolution"
                  options={["Merge", "Rebase"] satisfies ConflictResolution[]}
                  value={(resolution) => resolution}
                  label={(resolution) => RESOLUTION[resolution]}
                  chosen={set().conflict_resolution}
                  disabled={save.isPending}
                  pick={(picked) => save.mutate(picked as ConflictResolution)}
                />
              </div>

              {/* What the choice in force does, in a line — and the warning
                  under it where that is a rebase. */}
              <p class={styles.resolves}>
                {RESOLVES[set().conflict_resolution]}
              </p>

              <Show when={set().conflict_resolution === "Rebase"}>
                {forcePushed()}
              </Show>

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
