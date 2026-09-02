//! The compose page: the composer, before there is anything for it to be about.
//!
//! The same box, the same row of options along its bottom edge and the same
//! press underneath — and none of it saved anywhere, because there is no
//! Conversation yet to save it into. What it holds is held on the device (see
//! [`composing`](./composing.ts)), so a reload, a closed tab or a phone put down
//! mid-sentence loses nothing, and the first thing that reaches the server is a
//! button being pressed.
//!
//! **Two presses rather than one**, because creating and starting are two acts
//! here where they are one on a Conversation that already exists: *Start work*
//! makes the Conversation, puts every touched field on it and kicks the work
//! off; *Save as draft* stops after the fields. Both land in the Conversation
//! they made, which is where the work is read from that moment on.
//!
//! No Timeline beside it, for the reason a Conversation whose record is the one
//! Event has none: there is nothing yet to read. So the page is the sidebar and
//! this, and the frame widens exactly as it does there — see `Workbench.tsx`,
//! where the same two panes are handed to the same frame.
//!
//! The one thing it reads that a saved composer has no need of is the repo's own
//! memory of what it was last grilled with. A draft has that applied to it when
//! it is created; this page has no draft yet, so it asks for the same answer and
//! shows it — see `showing`, which is careful to show it rather than hold it.
//!
//! What it does *not* do is decide anything the composer decides. Every control
//! here is the composer's own component drawn over the compose state instead of
//! over a Conversation — see `Setup.tsx`, where they live — so the two pages
//! cannot come to ask different questions or word them differently.

import { useNavigate } from "@solidjs/router";
import { useMutation, useQueryClient } from "@tanstack/solid-query";
import { For, Show, createEffect, createSignal, type JSX } from "solid-js";

import app from "../App.module.css";
import { PaneSticky, Panes } from "../Panes";
import { Switch as Toggle } from "../Switch";
import { listRepos, loadRepoPairings } from "../api/client";
import type { RepoEntry } from "../api/types";
import { useReading } from "../freshness";
import { ErrorLine, Note } from "../notices";
import * as pairing from "../pairing";
import { Conversations } from "./Conversations";
import styles from "./Composer.module.css";
import { PaneHead } from "./PaneHead";
import {
  BasePicker,
  BranchField,
  CompanionChoice,
  ForRepo,
  ProfileChoices,
  RULE,
  RepoChoice,
  RepoOptions,
  RolePicker,
} from "./Setup";
import setup from "./Setup.module.css";
import { AUTOMATIC } from "./naming";
import {
  blank,
  clear,
  create,
  keep,
  leaveRefusals,
  stored,
  type Alongside,
  type Composed,
} from "./composing";
import { pathOf } from "./openings";

/// The page: the conversations down the left and the composer beside them.
///
/// Two panes rather than three, and the frame is handed no middle one at all —
/// there is no record to read, so there is no level between the list and this.
/// Which makes the narrow walk two steps as well: the page opens on the
/// composer, and the way off it is the way out of the page.
export function ComposePage(): JSX.Element {
  const navigate = useNavigate();

  return (
    <Panes
      pane="details"
      middleLabel="Timeline"
      conversations={
        <Conversations selected="" open={(id) => navigate(pathOf(id))} />
      }
      details={
        <Compose
          back={{ to: "Conversations", go: () => navigate("/") }}
        />
      }
    />
  );
}

/// The composer itself, over the compose state.
function Compose(props: {
  /// The way out of the pane, named as well as pressed — the conversations,
  /// there being no Timeline on this page for it to be anything else.
  back: { to: string; go: () => void };
}): JSX.Element {
  const navigate = useNavigate();
  const queries = useQueryClient();

  // What is being composed, off this device to begin with: a page opened on a
  // draft somebody left is that draft rather than a blank one.
  const [state, setState] = createSignal<Composed>(stored());

  // And written back whenever it moves. Every keystroke, which is what makes
  // this survive a reload — there is nowhere else it is being kept, and a page
  // that wrote itself out on a pause would lose whatever was typed in the
  // pause.
  createEffect(() => keep(state()));

  /// One field settled, the rest left where it was.
  const change = (part: Partial<Composed>) =>
    setState((was) => ({ ...was, ...part }));

  // The registered Repos, for the name in the trigger and for the rows the
  // companions are drawn as: the compose state holds ids, and everything drawn
  // off one needs the repository it names. Read under the key every other
  // reader of this list uses, so it is the one read.
  const repos = useReading(() => ({
    queryKey: ["repos"],
    queryFn: listRepos,
    freshness: { reconcile: "id" },
  }));

  /// Which Repo the work would be in, where the list has it — nothing until one
  /// is picked, and nothing while the list is still on its way.
  const repo = (): RepoEntry | null =>
    (repos.data ?? []).find((entry) => entry.id === state().repo) ?? null;

  // And what that repo was last grilled with, which is what a draft created on
  // it would arrive showing. Read the moment one is picked and again whenever it
  // changes, the key carrying the repo: the memory is the repo's, so another
  // repo is another answer — and there is nothing to ask until one is picked.
  //
  // Asked of the server rather than worked out here, for the reason the server
  // holds the memory at all: the workbench is answered from a phone as readily
  // as from a desk, and every judgement about whether a remembered pairing still
  // runs is made where the profiles and the watched paths are.
  const remembered = useReading(() => ({
    queryKey: ["repos", state().repo, "pairings"],
    queryFn: () => loadRepoPairings(state().repo!),
    enabled: state().repo !== null,

    // Merged by the id each profile carries, for the pickers below: what a
    // re-read that changed nothing must not do is rebuild a dropdown the human
    // has open.
    freshness: { reconcile: "id" },
  }));

  /// What one role's picker shows: what the human picked, and the repo's own
  /// memory until they pick anything.
  ///
  /// The prefill is *shown* rather than held — it never lands in the compose
  /// state, which is what keeps a picker nobody touched untouched at create
  /// time: the server applies its own prefill to the Conversation it makes, and
  /// a page that had copied the same answer into its state would be sending it
  /// back as though somebody had chosen it. Which also means switching repos
  /// simply reads another memory, and a role the human did touch stands through
  /// it.
  const showing = (role: "grilling" | "implementation" | "review"): string => {
    const picked = state()[role];
    if (picked !== null) {
      return picked;
    }

    const prefill = remembered.data;
    if (prefill === undefined) {
      return "";
    }

    return role === "implementation"
      ? pairing.chosen(prefill.implementation)
      : pairing.settled(prefill[role]);
  };

  /// Whether there is anything to create at all, which is the repo and nothing
  /// else: everything else on this page may stay empty, as it always may while
  /// a conversation drafts.
  const ready = () => state().repo !== null;

  const [gone, setGone] = createSignal(false);

  const make = useMutation(() => ({
    mutationFn: (work: boolean) => create(state(), work),
    onSuccess: (outcome) => {
      if (outcome === "NoSuchRepo") {
        // Picked out of a list this page read a moment ago: the Repo was there
        // and is not now, and nothing was created. Reading it again is both the
        // correction and the explanation, and what was composed stays where it
        // is.
        setGone(true);
        void queries.invalidateQueries({ queryKey: ["repos"] });
        return;
      }

      // The Conversation exists, so this device has nothing left to hold: it
      // would only ever offer to make the same one again. What the replay could
      // not do goes with the navigation instead, to be said on the draft it is
      // about.
      setGone(false);
      clear();
      setState(blank());
      leaveRefusals(outcome.conversation, outcome.refused);

      void queries.invalidateQueries({ queryKey: ["conversations"] });
      navigate(pathOf(outcome.conversation));
    },
  }));

  /// Moving what is being composed onto another Repo, which takes the same two
  /// things with it that a switch on a saved draft takes: the base goes back to
  /// the new repository's rule, and a companion that has just become the
  /// conversation's own Repo goes away — nothing is ever a companion of itself.
  const moveTo = (repoId: number) =>
    setState((was) => ({
      ...was,
      repo: repoId,
      base: null,
      companions: was.companions.filter((row) => row.repo_id !== repoId),
    }));

  /// One repo put alongside the work: read-only, off its own default branch,
  /// with no branch of its own — the least there is to say about one, which is
  /// what an add records on a saved draft too.
  ///
  /// Nothing is refused here and nothing needs to be: the two repos the server
  /// would refuse are the conversation's own and one already added, and neither
  /// can be picked twice into a list this page can simply read.
  const alongside = (repoId: number) =>
    setState((was) =>
      was.repo === repoId || was.companions.some((row) => row.repo_id === repoId)
        ? was
        : {
            ...was,
            companions: [
              ...was.companions,
              { repo_id: repoId, mode: "ReadOnly", base: RULE, branch: "" },
            ],
          },
    );

  /// One of them settled, and one of them taken away.
  const settle = (settled: Alongside) =>
    setState((was) => ({
      ...was,
      companions: was.companions.map((row) =>
        row.repo_id === settled.repo_id ? settled : row,
      ),
    }));

  const forget = (repoId: number) =>
    setState((was) => ({
      ...was,
      companions: was.companions.filter((row) => row.repo_id !== repoId),
    }));

  return (
    <>
      <PaneSticky>
        <PaneHead back={props.back} title="New conversation" />
      </PaneSticky>

      <div class={styles.composer}>
        <div class={styles.box}>
          {/* A copy of what has been typed gives the field its height — see
              `.grow` in `App.module.css`. */}
          <div class={app.grow} data-value={state().brief}>
            <textarea
              rows="1"
              aria-label="Brief"
              placeholder="What is this piece of work?"
              value={state().brief}
              onInput={(ev) => change({ brief: ev.currentTarget.value })}
            />
          </div>

          <section class={setup.options} aria-label="Setup">
            {/* The repository first, because everything under it is a fact
                about the one this picks — which is why the panel holds nothing
                else until one is picked. */}
            <RepoOptions
              name={repo()?.name ?? "Select"}
              alongside={state().companions.length}
            >
              {() => (
                <>
                  <RepoChoice
                    chosen={state().repo === null ? "" : String(state().repo)}
                    disabled={make.isPending}
                    pick={(repoId) => moveTo(repoId)}
                  >
                    {/* Said before the move rather than after, exactly as the
                        composer says it: the base is the one thing here that
                        picking another repo resets. */}
                    <Show when={ready()}>
                      <Note class={setup.aside}>
                        Moving this onto another repo puts its base back on that
                        repo's default branch. Its branch name, its pairings and
                        the repos it works alongside are kept.
                      </Note>
                    </Show>
                  </RepoChoice>

                  <Show when={repo()}>
                    {(on) => (
                      <>
                        <BranchField
                          id="branch"
                          label="Branch"
                          class={setup.branchName!}
                          placeholder={AUTOMATIC}
                          value={state().branch}
                          set={(branch) => change({ branch })}
                        />

                        <BasePicker
                          id="base-branch"
                          label="Base branch"
                          repo={on()}
                          chosen={state().base ?? RULE}
                          pick={(branch) => change({ base: branch })}
                        />

                        {/* The invitation goes back to being the invitation the
                            moment something is picked out of it: an add is done
                            rather than held, and the row it makes is under the
                            control. */}
                        <CompanionChoice
                          chosen=""
                          add={(repoId) => alongside(repoId)}
                        />

                        <Show when={state().companions.length}>
                          <ul
                            class={setup.companions}
                            aria-label="Companion repos"
                          >
                            <For each={state().companions}>
                              {(row) => (
                                <Beside
                                  alongside={row}
                                  repo={
                                    (repos.data ?? []).find(
                                      (entry) => entry.id === row.repo_id,
                                    ) ?? null
                                  }
                                  mirrors={state().branch}
                                  settle={settle}
                                  forget={() => forget(row.repo_id)}
                                />
                              )}
                            </For>
                          </ul>
                        </Show>
                      </>
                    )}
                  </Show>
                </>
              )}
            </RepoOptions>

            {/* And the three accounts, one trigger each — the same three
                questions, asked before there is a record for an answer to be
                about. Each of them stands on what the repo was last grilled
                with until it is touched, which is what a created draft would
                have arrived showing — and a picker left on it sends nothing
                when this is created, so the server's own prefill stands. */}
            <ProfileChoices>
              {(saved) => (
                <>
                  <RolePicker
                    saved={saved()}
                    role="grilling"
                    label="Grilling"
                    away="No grilling"
                    chosen={showing("grilling")}
                    pick={(picked) => change({ grilling: picked })}
                  />
                  <RolePicker
                    saved={saved()}
                    role="implementation"
                    label="Implementation"
                    chosen={showing("implementation")}
                    pick={(picked) => change({ implementation: picked })}
                  />
                  <RolePicker
                    saved={saved()}
                    role="review"
                    label="Review"
                    away="No review"
                    chosen={showing("review")}
                    pick={(picked) => change({ review: picked })}
                  />
                </>
              )}
            </ProfileChoices>
          </section>
        </div>

        {/* And the two presses, under the box and against its far edge: what
            becomes of what is in the box, which is the whole of why they are
            the only controls outside it. The quieter one first, because Start
            is what the page is arranged for. */}
        <div class={styles.startGrilling}>
          <div class={styles.presses}>
            <button
              type="button"
              class={styles.draft}
              disabled={!ready() || make.isPending}
              onClick={() => make.mutate(false)}
            >
              Save as draft
            </button>
            <button
              type="button"
              class={styles.start}
              disabled={!ready() || make.isPending}
              onClick={() => make.mutate(true)}
            >
              {make.isPending ? "Starting…" : "Start work"}
            </button>
          </div>

          <Show
            when={ready()}
            fallback={
              // Truly disabled rather than explaining itself on a press, unlike
              // the composer's own start: there is one thing missing and no
              // reading of the record could work out what — so it is said here,
              // where it can be read without pressing anything.
              <Note>Pick a repo to create this conversation in.</Note>
            }
          >
            <Note>
              Starting creates the conversation, its branch and its worktree, and
              freezes the brief. Saving it as a draft creates it and leaves it to
              be started.
            </Note>
          </Show>

          <Show when={gone()}>
            <ErrorLine class={styles.failure}>
              That repo is not registered any more, so nothing was created.
            </ErrorLine>
          </Show>
          <Show when={make.isError}>
            <ErrorLine class={styles.failure}>
              The conversation could not be created: {make.error?.message}
            </ErrorLine>
          </Show>
        </div>
      </div>
    </>
  );
}

/// One repo the work would run alongside, drawn as the composer's own rows are
/// drawn: the name and the × on a line, what the work will do with it under
/// them, and the branch it is done on under that where there is one.
///
/// Assembled here out of the same controls the saved row is assembled out of —
/// see `Companion` in `Setup.tsx`, and the steer modal's own rows, which is the
/// third place these three questions are asked. What is different is only where
/// the answers go.
function Beside(props: {
  /// The repository, where the list of them has it — nothing while the read is
  /// still on its way, which is the one state a row cannot be drawn in: the
  /// base dropdown is over that repository's branches.
  repo: RepoEntry | null;
  alongside: Alongside;
  /// What the conversation's own branch is called, for the branch field's
  /// prefill: mirroring is drawn as the name it follows.
  mirrors: string;
  settle: (alongside: Alongside) => void;
  forget: () => void;
}): JSX.Element {
  return (
    <Show when={props.repo}>
      {(repo) => (
        <li class={setup.companion}>
          <div class={setup.companionLine}>
            <span class={setup.companionName}>{repo().name}</span>
            {/* A mark rather than a word, because the row is one line and the
                name beside it is what says which repository is being taken
                away. The screen reader gets the sentence. */}
            <button
              type="button"
              class={setup.forget}
              aria-label={`Remove ${repo().name}`}
              onClick={() => props.forget()}
            >
              ×
            </button>
          </div>

          <div class={setup.companionConfig}>
            <BasePicker
              id={`companion-${repo().id}-base`}
              label={<>Base<ForRepo repo={repo().name} /></>}
              repo={repo()}
              chosen={props.alongside.base}
              pick={(branch) =>
                props.settle({ ...props.alongside, base: branch ?? RULE })
              }
            />

            <div class={setup.companionMode}>
              <Toggle
                label={<>Read-write<ForRepo repo={repo().name} /></>}
                on={props.alongside.mode === "ReadWrite"}
                flip={(on) =>
                  props.settle({
                    ...props.alongside,
                    mode: on ? "ReadWrite" : "ReadOnly",
                    // A branch name left behind on a row flipped back to
                    // read-only would be a name for a branch nobody will cut: a
                    // read-only checkout is detached and holds none.
                    branch: on ? props.alongside.branch : "",
                  })
                }
              />
            </div>
          </div>

          {/* And, where it may be written to, what its branch is called. Drawn
              prefilled with the conversation's own name rather than empty, so
              what the human reads is what they will get — and cleared, it is
              mirroring again. */}
          <Show when={props.alongside.mode === "ReadWrite"}>
            <BranchField
              id={`companion-${repo().id}-branch`}
              label={<>Branch<ForRepo repo={repo().name} /></>}
              class={setup.companionBranch!}
              value={props.alongside.branch || props.mirrors}
              set={(branch) => props.settle({ ...props.alongside, branch })}
            />
          </Show>
        </li>
      )}
    </Show>
  );
}
