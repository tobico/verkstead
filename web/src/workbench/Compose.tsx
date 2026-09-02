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
//! Which is why they are not pressable on the same things. A repo is the whole
//! of what creating needs, so *Save as draft* waits on that alone; *Start work*
//! carries a grilling start as well, and waits on what one has always waited on
//! — a brief, and the three roles answered. Short of that it draws inert and
//! does nothing at all when it is pressed, exactly as the composer's own start
//! does: a press that created the Conversation and then reported the grilling
//! refused would be doing the opposite of what it promised.
//!
//! **Nothing under the presses says any of that.** What each of them would do
//! is what its own word says, and why one of them cannot be pressed is a `title`
//! on the button rather than a paragraph under the box: the page is a box, a row
//! and two presses, and every sentence added to it was read once and then read
//! past forever.
//!
//! Which is why neither of them is ever truly `disabled` for a thing that is
//! missing — only for a press already in flight. A disabled button is one no
//! browser will hover and no keyboard will reach, so the `title` explaining it
//! would go the same way as the press it never takes, and the page would be back
//! to refusing without saying why. Both draw inert, both answer a press with
//! nothing, and `aria-disabled` is what says so to whoever is not looking at
//! them.
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
//! **And it is where a roadmap is adopted from**, that being the other way work
//! gets into the pipeline rather than another page for it: the *Adopt a roadmap*
//! dropdown under the box lists the roadmaps nothing is driving, and picking one
//! loads it into what this device is holding. The box locks to a card naming the
//! roadmap and the stage that would be started, the repo and the base are the
//! roadmap's own, and the pairings and the repos alongside stay the human's to
//! settle — which is the whole of what adopting asks for. Clearing it gives the
//! box back whatever was typed in it.
//!
//! What the two presses do with a roadmap loaded is what they always do, under
//! the other name: *Start work* creates the adopting Conversation and adopts the
//! stage, and *Save as draft* creates it and leaves the stage to be adopted on
//! its own page.
//!
//! What it does *not* do is decide anything the composer decides. Every control
//! here is the composer's own component drawn over the compose state instead of
//! over a Conversation — see `Setup.tsx`, where they live — so the two pages
//! cannot come to ask different questions or word them differently.

import { faChevronDown } from "@fortawesome/free-solid-svg-icons";
import { useNavigate } from "@solidjs/router";
import { useMutation, useQueryClient } from "@tanstack/solid-query";
import { For, Show, createEffect, createSignal, type JSX } from "solid-js";

import app from "../App.module.css";
import { Icon } from "../Icon";
import { Menu } from "../Menu";
import { PaneSticky, Panes } from "../Panes";
import shell from "../Panes.module.css";
import { Switch as Toggle } from "../Switch";
import {
  listAbandonedRoadmaps,
  listRepos,
  loadRepoPairings,
} from "../api/client";
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
  RepoSelect,
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
  on,
  stored,
  type Adopting,
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

/// Why neither press can be pressed while no repo is picked, said on both of
/// them.
///
/// The one thing a create cannot be done without, and the one control on this
/// page that has not been answered — so the button says it where the press is
/// refused rather than the page saying it above the row.
const NO_REPO = "No repo selected";

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

  // And the roadmaps nothing is driving, which is the other way work gets into
  // the pipeline: the rows behind the Adopt dropdown under the box. Read under
  // the key the rest of the app reads them under, and read again whenever the
  // page looks again — a roadmap somebody has picked up since simply stops
  // being on the list.
  const abandoned = useReading(() => ({
    queryKey: ["abandoned-roadmaps"],
    queryFn: listAbandonedRoadmaps,

    // Keyed by `repo_id`: this list is Repos rather than records of its own, and
    // a Nudge landing while the dropdown is open must not rebuild the row the
    // human had tabbed to.
    freshness: { reconcile: "repo_id" },
  }));

  /// The roadmap this page is loaded with, where it is loaded with one.
  const adopting = (): Adopting | null => state().adopting;

  /// Every roadmap there is to adopt, flat and in the shape the page holds one
  /// in — each still knowing which Repo it is in, which is what a line with two
  /// `mvp`s in it would otherwise be missing.
  const roadmaps = (): Adopting[] =>
    (abandoned.data ?? []).flatMap((held) =>
      held.roadmaps.map((roadmap) => ({
        repo_id: held.repo_id,
        repo: held.repo,
        roadmap: roadmap.name,
        title: roadmap.title,
        stage: roadmap.stage,
        stage_title: roadmap.stage_title,
        base: roadmap.base,
      })),
    );

  /// Which Repo the work would be in, where the list has it — the roadmap's own
  /// while one is loaded, nothing until one is picked, and nothing while the
  /// list is still on its way.
  const repo = (): RepoEntry | null =>
    (repos.data ?? []).find((entry) => entry.id === on(state())) ?? null;

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
    queryKey: ["repos", on(state()), "pairings"],
    queryFn: () => loadRepoPairings(on(state())!),
    enabled: on(state()) !== null,

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
  /// a conversation drafts. A roadmap carries its own repo, so a page loaded
  /// with one is ready by holding it.
  const ready = () => on(state()) !== null;

  /// And whether *Start work* would actually start anything, which is more:
  /// creating a Conversation and kicking the work off are one press here, and
  /// the second half has the conditions it has always had — a brief to grill
  /// from, and each of the three roles answered.
  ///
  /// The same questions `ready_to_grill` asks, less the one this side cannot
  /// see: whether the account behind a chosen pairing is still where it was
  /// left. So this decides how the press *behaves* rather than what is true, and
  /// the server checks every one of them again — see [`create`], which carries
  /// whatever it is refused to the draft it made.
  ///
  /// A roadmap answers the brief for itself: the stage's own arrives with the
  /// adoption, which is why there is nothing to write in the box while one is
  /// loaded. Nothing chosen for a role is the empty string on all three pickers
  /// — the row that runs no session is a choice like any other, and it lets the
  /// work start.
  const startable = () =>
    ready() &&
    (adopting() !== null || state().brief.trim() !== "") &&
    showing("grilling") !== "" &&
    showing("implementation") !== "" &&
    showing("review") !== "";

  /// What starting is waiting on, in the words the composer's own start says
  /// them in — the brief left out where a roadmap answers for it.
  ///
  /// A `title` on the press rather than a line under it. It is about a button
  /// rather than about the page, and a sentence standing under the box whether
  /// or not anybody wanted it is the page explaining itself unasked — see the
  /// composer's own start, where the same words moved for the same reason.
  const waiting = () =>
    adopting() === null
      ? "Starting needs a brief, and every role picked and working."
      : "Starting needs every role picked and working.";

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

  /// A roadmap loaded into what is being composed, which creates nothing: the
  /// row that was pressed is written into the compose state, and the press under
  /// the box is still the first thing that reaches the server.
  ///
  /// What is around it is left exactly where it was — the brief that was typed,
  /// the repo that was picked, the base under it — because clearing the roadmap
  /// has to give all of it back. The one thing that goes is a companion that has
  /// just become the work's own Repo: nothing is ever a companion of itself,
  /// which is what a switch onto another Repo does with one too.
  const load = (held: Adopting) =>
    setState((was) => ({
      ...was,
      adopting: held,
      companions: was.companions.filter((row) => row.repo_id !== held.repo_id),
    }));

  /// And unloaded, which puts the page back to composing work of its own.
  const unload = () => setState((was) => ({ ...was, adopting: null }));

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
      on(was) === repoId ||
      was.companions.some((row) => row.repo_id === repoId)
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

      <div class={`${styles.composer} ${shell.paneComposer}`}>
        <div class={styles.box}>
          {/* The field, or the roadmap that has been loaded in place of it: an
              adopted stage's brief is the repository's own and arrives with the
              adoption, so there is nothing here to write and the box says which
              stage instead. */}
          <Show
            when={adopting()}
            fallback={
              // A copy of what has been typed gives the field its height — see
              // `.grow` in `App.module.css`, and `.field` in the composer's own
              // module for the three lines it starts at.
              <div class={`${app.grow} ${styles.field}`} data-value={state().brief}>
                <textarea
                  rows="1"
                  aria-label="Brief"
                  placeholder="What is this piece of work?"
                  value={state().brief}
                  onInput={(ev) => change({ brief: ev.currentTarget.value })}
                />
              </div>
            }
          >
            {(held) => <Loaded roadmap={held()} clear={() => unload()} />}
          </Show>

          <section class={setup.options} aria-label="Setup">
            {/* The repository first, because everything under it is a fact
                about the one this picks — which is why the panel holds nothing
                else until one is picked, and why there is no panel at all until
                then: a dropdown while nothing is chosen, the arrangement of what
                was chosen after it. See `RepoSelect` in `Setup.tsx`. */}
            <Show
              when={on(state()) !== null}
              fallback={
                <RepoSelect
                  chosen=""
                  disabled={make.isPending}
                  pick={(repoId) => moveTo(repoId)}
                />
              }
            >
              <RepoOptions
                name={repo()?.name ?? adopting()?.repo ?? ""}
                alongside={state().companions.length}
              >
                {() => (
                  <>
                    <RepoChoice
                      chosen={on(state()) === null ? "" : String(on(state()))}
                      // Settled while a roadmap is loaded, the way it is
                      // settled once a branch has been cut: the stage is in
                      // the repository the roadmap is written in, and moving
                      // the work off it would be moving it away from what it
                      // is adopting.
                      disabled={make.isPending || adopting() !== null}
                      pick={(repoId) => moveTo(repoId)}
                    >
                      {/* And what the roadmap has settled, where one is loaded:
                          the two fields that would have asked are not drawn at
                          all, so this is where they are answered. */}
                      <Show when={adopting()}>
                        {(held) => (
                          <Note class={setup.aside}>
                            The stage is worked on its own branch, off{" "}
                            <Show
                              when={held().base}
                              fallback={<>this repo's default branch</>}
                            >
                              {(base) => <code>{base()}</code>}
                            </Show>
                            . Clear the roadmap to compose work of your own.
                          </Note>
                        )}
                      </Show>
                    </RepoChoice>

                    <Show when={repo()}>
                      {(chosen) => (
                        <>
                          {/* Neither is asked of a page adopting a roadmap: a
                              stage is worked on its own slug, and the base went
                              out with the row that loaded it. What a control
                              cannot do it does not draw. */}
                          <Show when={adopting() === null}>
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
                              repo={chosen()}
                              chosen={state().base ?? RULE}
                              pick={(branch) => change({ base: branch })}
                            />
                          </Show>

                          {/* The invitation goes back to being the
                              invitation the moment something is picked out of
                              it: an add is done rather than held, and the row
                              it makes is under the control. */}
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
            </Show>

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
            {/* The other way work gets into the pipeline, at the near edge of
                the row and so plainly not one of the two presses: a roadmap
                somebody staged before Verkstead was driving anything, taken up
                as it stands. Drawn only when there is one to take up and the box
                is empty — what it loads stands in place of what would have been
                written there, and a dropdown offering to replace a half-written
                brief would be offering to lose it. */}
            <Show
              when={
                roadmaps().length &&
                state().brief.trim() === "" &&
                adopting() === null
              }
            >
              <AdoptRoadmap roadmaps={roadmaps()} load={load} />
            </Show>

            <button
              type="button"
              class={styles.draft}
              classList={{ [styles.inert!]: !ready() }}
              // Truly `disabled` for a press already in flight and nothing
              // else. Having no repo to create in is the other thing entirely:
              // it draws inert, answers a press with nothing, and says why in a
              // `title` — which is the whole reason it is not disabled, a
              // button a browser will not hover being a button that cannot
              // explain itself. Exactly as the start beside it works.
              disabled={make.isPending}
              aria-disabled={!ready()}
              title={ready() ? undefined : NO_REPO}
              onClick={() => ready() && make.mutate(false)}
            >
              Save as draft
            </button>
            <button
              type="button"
              class={styles.start}
              classList={{ [styles.inert!]: !startable() }}
              // The same, with one more thing to wait on: creating is all the
              // press beside it does, and this one grills as well. So a repo is
              // not the whole of what it needs, and its `title` says whichever
              // of the two is missing.
              disabled={make.isPending}
              aria-disabled={!startable()}
              title={ready() ? (startable() ? undefined : waiting()) : NO_REPO}
              onClick={() => startable() && make.mutate(true)}
            >
              {make.isPending ? "Starting…" : "Start work"}
            </button>
          </div>

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

/// The roadmap this page is loaded with, standing where the brief would be
/// written.
///
/// A card rather than a field, because there is nothing here to write: the
/// stage's own brief is in the repository and becomes the Conversation's Brief
/// at the moment the stage is adopted, which is what an adopting draft's box
/// says on the composer beside this one. So what the box holds is the roadmap
/// named, the stage that would be started, and the way back out.
///
/// Everything on it was read off the row that loaded it. Nothing is asked of the
/// server to draw this: the stage brief is not on this device, and a page that
/// fetched it to show it would be reading a document nobody can edit here.
function Loaded(props: {
  roadmap: Adopting;
  /// Unload it, which puts the box back to the brief this device was holding.
  clear: () => void;
}): JSX.Element {
  return (
    <div class={styles.loaded}>
      <p class={styles.roadmapLine}>
        <code>{props.roadmap.roadmap}</code>
        <Show when={props.roadmap.title}>
          {(title) => <span class={styles.roadmapTitle}>{title()}</span>}
        </Show>
        {/* A mark rather than a word, as the companion rows' own is: the card
            beside it is what says which roadmap is being put down. The screen
            reader gets the sentence. */}
        <button
          type="button"
          class={styles.clear}
          aria-label={`Clear ${props.roadmap.roadmap}`}
          onClick={() => props.clear()}
        >
          ×
        </button>
      </p>

      <p class={styles.stage}>
        Stage {props.roadmap.stage}: {props.roadmap.stage_title}
      </p>

      <Note>
        The stage's own brief becomes this conversation's brief when the stage is
        adopted, and the work is done on the branch the stage is named for.
      </Note>
    </div>
  );
}

/// The roadmaps nothing is driving, and the dropdown that loads one.
///
/// An action with a chevron rather than a label over a value: the options along
/// the box's edge say what the work *is*, and this says what to put in the box —
/// which is why it stands under the box with the presses rather than in the row
/// inside it.
///
/// Pressing a row creates nothing. It writes the roadmap into what this device
/// is holding, and the dropdown goes with the load: there is one roadmap on a
/// page at a time, and the way to another is to clear the one in the box.
///
/// Each row is worded the way the sidebar's menu worded it, this being where
/// those rows moved to: the roadmap, the Repo it is in — the list is flat, and
/// two repositories may each hold an `mvp` — the stage that would be started,
/// and where the roadmap was found when that is somewhere other than the
/// default branch.
function AdoptRoadmap(props: {
  roadmaps: Adopting[];
  load: (roadmap: Adopting) => void;
}): JSX.Element {
  // The menu's own way to shut, so a press that has done its work takes the
  // card back and hands the focus to the trigger it came from.
  let shut = (): void => {};

  return (
    <Menu
      class={styles.adopt!}
      name="Adopt a roadmap"
      closer={(close) => (shut = close)}
      trigger={
        <>
          Adopt a roadmap
          {/* Which way the menu will go, and no part of what the button
              says. */}
          <Icon of={faChevronDown} />
        </>
      }
    >
      {() => (
        <For each={props.roadmaps}>
          {(held) => (
            <button
              type="button"
              role="menuitem"
              class={styles.roadmapRow}
              onClick={() => {
                shut();
                props.load(held);
              }}
            >
              <span class={styles.what}>
                <code>{held.roadmap}</code>
                <span class={styles.in}>in {held.repo}</span>
              </span>
              <span class={styles.next}>
                next is stage {held.stage}: {held.stage_title}
              </span>
              <Show when={held.base}>
                {(base) => (
                  <span class={styles.found}>
                    on <code>{base()}</code>
                  </span>
                )}
              </Show>
            </button>
          )}
        </For>
      )}
    </Menu>
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
