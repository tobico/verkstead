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
//! — a brief, and every role the Process uses answered. Short of that it draws
//! inert and does nothing at all when it is pressed, exactly as the composer's
//! own start does: a press that created the Conversation and then reported the
//! grilling refused would be doing the opposite of what it promised.
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
//! **The files are the one thing on it that is not held on the device.** The
//! paperclip at the near edge of the row the presses are at the far edge of
//! picks them — as does a drop anywhere on the box, which is the same piece
//! doing the same thing (see `src/Attaching.tsx`) — they are drawn as the same
//! pills a draft draws them as, and they stay in the page until a press — a `File` being a handle the browser gave
//! this page rather than text, so a reload keeps what was typed and loses what
//! was picked. The press uploads them through the route a draft's own paperclip
//! uses, as one more field of the replay. See `src/holding.ts`.
//!
//! The one thing it reads that a saved composer has no need of is the repo's own
//! memory of what it was last grilled with — or, for a repo nothing has grilled,
//! what the server prefills in its place. A draft has that applied to it when
//! it is created; this page has no draft yet, so it asks for the same answer and
//! shows it — see `showing`, which is careful to show it rather than hold it.
//!
//! **And it is where work that is already somewhere else is taken up from**,
//! that being the other way into the pipeline rather than another page for it:
//! the *Other actions* menu under the box holds one nested level per way of
//! doing it, and picking a row loads what it names into what this device is
//! holding. *Continue a roadmap* is the first level — the roadmaps nothing is
//! driving, each named with its Repo and the stage a press would start. The box
//! locks to a card naming the roadmap and that stage, the repo and the base are
//! the roadmap's own, and the pairings and the repos alongside stay the human's
//! to settle — which is the whole of what adopting asks for. Clearing it gives
//! the box back whatever was typed in it.
//!
//! *Wrap up a pull request* is the second: every pull request open across the
//! registered Repos, read off GitHub when this page opens and again on each
//! reopen, and held nowhere — GitHub owns that list, and a copy here would be
//! one this page had to work out when to stop believing. Any author, forks left
//! out, and one a Conversation already holds listed all the same with its row
//! leading to that Conversation rather than loading anything. It is a `gh` per
//! Repo, so the level opens on a line saying it is reading rather than on an
//! empty card.
//!
//! **A free row of it loads differently from a roadmap row, and that is the
//! point of it.** A roadmap locks a card over the box, an adopted stage's brief
//! being the repository's own; a pull request brings words of its own — the one
//! thing taken up that does — so the box stays a box and is *prefilled* with its
//! title as a heading and its description under it, editable, and what is left
//! there is the Brief. What was already typed is stowed and given back on clear,
//! the way a held file is. The repo is the pull request's and reads settled, the
//! branch and the base are not drawn at all — its branch is the head branch and
//! its base is that branch's own head at take-up — and the grilling picker goes
//! with them, the Process reading as a Review and a Review having no round for
//! a grilling to open.
//!
//! The menu is drawn whenever the box is empty and nothing is loaded, and a
//! level with nothing under it is greyed rather than hidden: what there is to do
//! here should not change shape with a list the human cannot see. *Adopt* stays
//! the word in the code, the endpoints and the store; *continue* and *wrap up*
//! are what the human reads.
//!
//! What the two presses do with a roadmap loaded is what they always do, under
//! the other name: *Start work* creates the adopting Conversation and adopts the
//! stage, and *Save as draft* creates it and leaves the stage to be adopted on
//! its own page. With a pull request loaded they hold that shape once more:
//! *Start work* creates the Conversation and takes the pull request up, which
//! puts it on the head branch and starts the wrap-up, and *Save as draft*
//! creates it and leaves the take-up on that draft's own page.
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
import { attaching } from "../Attaching";
import { Icon } from "../Icon";
import { Menu, Nested } from "../Menu";
import { PaneSticky, Panes } from "../Panes";
import shell from "../Panes.module.css";
import { Switch as Toggle } from "../Switch";
import {
  listAbandonedRoadmaps,
  listOpenPullRequests,
  listRepos,
  loadRepoPairings,
} from "../api/client";
import type { OpenPullRequest, Process, RepoEntry } from "../api/types";
import { useReading } from "../freshness";
import { holding } from "../holding";
import { ErrorLine, Note } from "../notices";
import * as pairing from "../pairing";
import { ShowArchived } from "./Archived";
import { Conversations } from "./Conversations";
import styles from "./Composer.module.css";
import { PaneHead } from "./PaneHead";
import { Wordmark } from "./Wordmark";
import {
  AgentOptions,
  BasePicker,
  BranchField,
  CompanionChoice,
  ForRepo,
  ProcessPicker,
  ProfileChoices,
  RULE,
  RepoChoice,
  RepoOptions,
  RepoSelect,
  RolePicker,
  TARGET,
} from "./Setup";
import setup from "./Setup.module.css";
import { HeldPullRequest } from "./TakeUp";
import { AUTOMATIC } from "./naming";
import {
  blank,
  clear,
  create,
  keep,
  leaveRefusals,
  on,
  stored,
  written,
  type Adopting,
  type AdoptingPullRequest,
  type Alongside,
  type Composed,
} from "./composing";
import { pathOf } from "./openings";
import {
  away,
  label,
  needed,
  ROLES,
  targeted,
  uses,
  type Role,
} from "./processes";
import { namesPullRequest } from "./targets";
import { useZero } from "./zero";

/// The page: the conversations down the left and the composer beside them.
///
/// Two panes rather than three, and the frame is handed no middle one at all —
/// there is no record to read, so there is no level between the list and this.
/// Which makes the narrow walk two steps as well: the page opens on the
/// composer, and the way off it is the way out of the page.
///
/// **And one pane where there is nothing to list.** The zero state is a fact
/// about the sidebar's list rather than about onboarding — see `zero.ts` — and
/// where it holds there is no list to draw, so the pane is not handed to the
/// frame at all and this page is the whole window: entered at the wordmark
/// rather than at a way back to a pane that is not there, with the switch that
/// could bring the archived ones back pinned to its corner. It is the page a
/// fresh Verkstead lands on, and the page the last Conversation being archived
/// leaves behind.
///
/// Nothing at all until the list has answered, for the reason the app's own gate
/// draws nothing until the machine has (see `Gate` in `src/App.tsx`): the two
/// shapes are not a detail of this page but the whole of it, and a first sight
/// of Verkstead that was somebody else's empty sidebar for half a second before
/// the page it is meant to be is worse than the half-second. Reached from the
/// sidebar the answer is already in hand — it is that pane's own query — so this
/// waits on nothing at all in the case it happens in most.
export function ComposePage(): JSX.Element {
  const navigate = useNavigate();
  const zero = useZero();

  return (
    <Show when={!zero().pending}>
      <Panes
        pane="details"
        middleLabel="Timeline"
        conversations={
          zero().holds ? undefined : (
            <Conversations selected="" open={(id) => navigate(pathOf(id))} />
          )
        }
        details={
          <Compose
            zero={zero().holds}
            archived={zero().archived}
            back={{ to: "Conversations", go: () => navigate("/") }}
          />
        }
      />
    </Show>
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
  /// there being no Timeline on this page for it to be anything else. Not drawn
  /// in the zero state: the pane it names is not on the page.
  back: { to: string; go: () => void };

  /// Whether there is nothing to list, which is what makes this page the whole
  /// of Verkstead rather than one pane of it — see `zero.ts` and [`ComposePage`].
  zero: boolean;

  /// And whether anything is archived, which is what the switch in the corner
  /// would have to bring back. A switch with nothing behind it is not drawn.
  archived: boolean;
}): JSX.Element {
  const navigate = useNavigate();
  const queries = useQueryClient();

  // What is being composed, off this device to begin with: a page opened on a
  // draft somebody left is that draft rather than a blank one.
  const [state, setState] = createSignal<Composed>(stored());

  // The files picked here, which are not part of what is written back: a
  // `File` cannot be stored and read again, so they live for as long as this
  // page does and the press is what sends them.
  const files = holding();

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
  // the pipeline: the rows behind Continue a roadmap under the box. Read under
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

  // And the pull requests open in those same Repos, which is the other level of
  // that menu: work already somewhere else, waiting to be wrapped up.
  //
  // Read when this page opens and again whenever it is opened, and held nowhere
  // — GitHub owns the list, and a copy in the browser would be one this page had
  // to work out when to stop believing. Its own key rather than the roadmaps'
  // for what it costs: this is a `gh` per registered Repo, and a Nudge that
  // re-read it would be a call out to GitHub every time anything anywhere moved.
  const open = useReading(() => ({
    queryKey: ["open-pull-requests"],
    queryFn: listOpenPullRequests,

    // Keyed by `repo_id` for the reason above it: the list is Repos, and a
    // re-read landing while the level is open must not rebuild the row the human
    // had tabbed to.
    freshness: { reconcile: "repo_id" },
  }));

  /// The roadmap this page is loaded with, where it is loaded with one.
  const adopting = (): Adopting | null => state().adopting;

  /// And the pull request, where it is loaded with one of those instead. Never
  /// both: the menu that loads either is drawn only while nothing is loaded.
  const pull = (): AdoptingPullRequest | null => state().pull;

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

  /// And every open pull request there is, flat and in the shape a row draws —
  /// each still knowing which Repo it is in, because a number is a fact about
  /// one repository and two of them may each have a `#41`.
  const pulls = (): Pull[] =>
    (open.data ?? []).flatMap((held) =>
      held.pull_requests.map((pull) => ({
        ...pull,
        repo_id: held.repo_id,
        repo: held.repo,
      })),
    );

  /// Which Repo the work would be in, where the list has it — the roadmap's own
  /// while one is loaded, nothing until one is picked, and nothing while the
  /// list is still on its way.
  const repo = (): RepoEntry | null =>
    (repos.data ?? []).find((entry) => entry.id === on(state())) ?? null;

  // And what that repo was last grilled with — or, where nothing has grilled it,
  // what the server prefills instead — which is what a draft created on it
  // would arrive showing. Read the moment one is picked and again whenever it
  // changes, the key carrying the repo: the memory is the repo's, so another
  // repo is another answer — and there is nothing to ask until one is picked.
  //
  // Asked of the server rather than worked out here, for the reason the server
  // holds the memory at all: the workbench is answered from a phone as readily
  // as from a desk, and every judgement about whether a remembered pairing still
  // runs is made where the profiles and the repositories are.
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
  const showing = (role: Role): string => {
    const picked = state()[role];
    if (picked !== null) {
      return picked;
    }

    const prefill = remembered.data;
    if (prefill === undefined) {
      return "";
    }

    // The review is the one role whose memory can say *no session at all*; the
    // other two are an account or nothing.
    return role === "review"
      ? pairing.settled(prefill.review)
      : pairing.chosen(prefill[role]);
  };

  /// What kind of work this page is composing: what the picker was moved to,
  /// Develop until it is moved at all — and Review over a loaded pull request,
  /// which settles it the way it settles the repo. What a take-up makes *is* a
  /// Review, rather than anything a dropdown here chose.
  ///
  /// The one reading the row is drawn off, the way [`on`] is the one reading of
  /// the repo: which pickers stand in it, what the press waits on, and what an
  /// inert press says it is waiting on all come off this and the table it
  /// indexes.
  const process = (): Process =>
    pull() !== null ? "Review" : state().process ?? "Develop";

  /// Whether there is anything to create at all, which is the repo and nothing
  /// else: everything else on this page may stay empty, as it always may while
  /// a conversation drafts. A roadmap carries its own repo, so a page loaded
  /// with one is ready by holding it.
  const ready = () => on(state()) !== null;

  /// And whether *Start work* would actually start anything, which is more:
  /// creating a Conversation and kicking the work off are one press here, and
  /// the second half has the conditions it has always had — a brief to grill
  /// from, and every role the Process uses answered.
  ///
  /// The same questions `ready_to_grill` asks, less the one this side cannot
  /// see: whether the account behind a chosen pairing is still where it was
  /// left. So this decides how the press *behaves* rather than what is true, and
  /// the server checks every one of them again — see [`create`], which carries
  /// whatever it is refused to the draft it made.
  ///
  /// A roadmap answers the brief for itself: the stage's own arrives with the
  /// adoption, which is why there is nothing to write in the box while one is
  /// loaded. Nothing chosen for a role is the empty string on every picker —
  /// the row that runs no session is a choice like any other, and it lets the
  /// work start.
  ///
  /// A pull request answers the grilling for itself, by reading as a Review:
  /// Review does not use that role, so there is no picker for it and nothing
  /// waiting on one. Its Brief is a question like any other's — it arrives
  /// prefilled with the pull request's own words, but the box is a box and what
  /// is left in it is what the wrap-up reads.
  /// And whether what is in the Target field names a pull request, which is
  /// what takes the base picker off the panel: a pull request brings GitHub's
  /// own base along, and a branch's base is what its pull request will be
  /// opened against.
  const onAPullRequest = () =>
    targeted(process()) && namesPullRequest(state().target);

  const startable = () =>
    ready() &&
    (adopting() !== null || state().brief.trim() !== "") &&
    (pull() !== null ||
      !targeted(process()) ||
      state().target.trim() !== "") &&
    ROLES[process()].uses.every((role) => showing(role) !== "");

  /// What starting is waiting on, in the words the composer's own start says
  /// them in — the brief left out where a roadmap answers for it, and the roles
  /// named as the Process has them.
  ///
  /// A `title` on the press rather than a line under it. It is about a button
  /// rather than about the page, and a sentence standing under the box whether
  /// or not anybody wanted it is the page explaining itself unasked — see the
  /// composer's own start, where the same words moved for the same reason.
  const waiting = () =>
    `Starting needs ${needed(process(), {
      brief: adopting() === null,
      // A page holding a pull request off the retired menu is pointed
      // already: the row that loaded it is the target, and there is no field
      // for it to wait on.
      target: pull() === null,
    })}.`;

  const [gone, setGone] = createSignal(false);

  const make = useMutation(() => ({
    mutationFn: (work: boolean) => create(state(), work, files),
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

  // And the one piece the attaching UI is drawn through, here and on the
  // composer of a draft alike: the pills inside the box, the paperclip in the
  // row of presses, and the drop the box itself takes. What is different here
  // is only what becomes of a chosen file — it is held in the page rather than
  // sent — which is the one thing the piece is handed. See `Attaching.tsx`.
  const attach = attaching({
    shown: () =>
      files.held().map((one) => ({
        name: one.file.name,
        remove: () => files.drop(one.key),
        // The one thing a × here can be refused for: the press has been made
        // and the files are on their way up.
        removing: make.isPending,
      })),
    add: files.add,
    // Not offered while a roadmap is loaded: the box is locked to a card then,
    // and there is nothing being written for a file to be handed over with.
    offered: () => adopting() === null,
  });

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

  /// A pull request loaded into what is being composed, which creates nothing
  /// either: the row that was pressed is written into the compose state, and the
  /// press under the box is still the first thing that reaches the server.
  ///
  /// What is different from a roadmap is what it does to the box. A roadmap
  /// locks a card over it, so what was typed simply stays underneath; a pull
  /// request *fills* it — the title as a heading, the description under it —
  /// which is the whole reason it is worth loading one at all. So what was in
  /// the box is stowed on the record that displaced it, and clearing gives it
  /// back.
  const take = (held: Pull) =>
    setState((was) => ({
      ...was,
      brief: brief(held),
      pull: {
        repo_id: held.repo_id,
        repo: held.repo,
        number: held.number,
        title: held.title,
        url: held.url,
        head: held.head,
        base: held.base,
        stowed: was.brief,
      },
      // Nothing is ever a companion of itself, which is what a switch onto
      // another Repo does with one too.
      companions: was.companions.filter((row) => row.repo_id !== held.repo_id),
    }));

  /// And put down again: the box goes back to whatever was in it, and the page
  /// to composing work of its own.
  ///
  /// What was written *over* the prefill goes with it. The box is the pull
  /// request's from the moment one is loaded — that is what a prefill is — so
  /// giving back the stowed text is giving the box back rather than merging two
  /// drafts of it.
  const putDown = () =>
    setState((was) => ({
      ...was,
      brief: was.pull?.stowed ?? was.brief,
      pull: null,
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
      {/* The head this page is entered at. Ordinarily the pane's own — what it
          is and the way back to the list beside it — and the wordmark where
          there is no list to go back to: this page is then the whole of
          Verkstead, and what stands over it is the thing rather than the name of
          one pane of it. Drawn by the same component the sidebar draws it with,
          gear and all, so the way out to the settings is where it always was.
          See `Wordmark.tsx`. */}
      <PaneSticky>
        <Show
          when={props.zero}
          fallback={<PaneHead back={props.back} title="New conversation" />}
        >
          <Wordmark />
        </Show>
      </PaneSticky>

      <div class={`${styles.composer} ${shell.paneComposer}`}>
        {/* The box, and the whole of it a drop target — the same box the
            composer beside this one draws, taking a drop the same way. */}
        <div
          class={styles.box}
          classList={{ [styles.over!]: attach.over() }}
          {...attach.dropping}
        >
          {/* And the pull request that has been loaded, where one has: a band
              across the top of the box naming it, over the field rather than in
              place of it. A pull request brings words of its own, so the box is
              still a box — prefilled with its title and description, and what is
              left there is the Brief. See `TakeUp.tsx`. */}
          <Show when={pull()}>
            {(held) => (
              <HeldPullRequest
                repo={held().repo}
                number={held().number}
                title={held().title}
                url={held().url}
                head={held().head}
                base={held().base}
                clear={() => putDown()}
              />
            )}
          </Show>

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
                  onInput={(ev) =>
                    // The box, and the Target filled out of it while it is
                    // empty: the same rule the server keeps when a Brief is
                    // saved, kept here because nothing is saved yet.
                    setState((was) => written(was, ev.currentTarget.value))
                  }
                />
              </div>
            }
          >
            {(held) => <Loaded roadmap={held()} clear={() => unload()} />}
          </Show>

          {/* And the files that will go up with it, drawn exactly as a draft's
              are — the same row through the same piece, because a file picked
              before there is a Conversation and a file on one are the same
              thing to look at. The × drops the held file rather than making a
              request: there is nothing on the server yet to take anything
              off.

              The row goes with the box while a roadmap is loaded, because the
              box is locked to a card then and the files were picked for what it
              is standing over. Held rather than dropped, the way everything
              else a roadmap covers is held: clearing it gives them back, and
              the press sends none of them meanwhile — see `composing.ts`. */}
          <Show when={adopting() === null}>
            <attach.Pills class={styles.attachments} />
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
              {/* Named by the repo the id stands for, and by the invitation
                  where there is no name to be had: the id is what this page
                  keeps on the device, and the name for it comes off a read
                  that has not landed yet — or off a repo that has been
                  deregistered since, which is the picker inside the panel's to
                  put right. A blank line under the label would say neither. */}
              <RepoOptions
                name={
                  repo()?.name ?? adopting()?.repo ?? pull()?.repo ?? "Select"
                }
                alongside={state().companions.length}
              >
                {() => (
                  <>
                    <RepoChoice
                      chosen={on(state()) === null ? "" : String(on(state()))}
                      // Settled while anything is loaded, the way it is settled
                      // once a branch has been cut: the stage is in the
                      // repository the roadmap is written in, and `#41` is a
                      // number in the repository the pull request was opened in
                      // — so moving the work off would be moving it away from
                      // what it is taking up.
                      disabled={
                        make.isPending ||
                        adopting() !== null ||
                        pull() !== null
                      }
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

                      {/* And what the pull request has settled, the same way:
                          the branch is its head branch and the base is the
                          commit that branch is at when it is taken up, so the
                          two fields that would have asked are not drawn and
                          this is where they are answered. */}
                      <Show when={pull()}>
                        {(held) => (
                          <Note class={setup.aside}>
                            The work carries on <code>{held().head}</code>, from
                            wherever it stands when it is taken up. Clear the
                            pull request to compose work of your own.
                          </Note>
                        )}
                      </Show>
                    </RepoChoice>

                    <Show when={repo()}>
                      {(chosen) => (
                        <>
                          {/* Neither is asked of a page adopting a roadmap: a
                              stage is worked on its own slug, and the base went
                              out with the row that loaded it. Nor of one holding
                              a pull request, whose branch is the head branch
                              GitHub names and whose base is that branch's own
                              head at take-up. What a control cannot do it does
                              not draw. */}
                          <Show when={adopting() === null && pull() === null}>
                            <BranchField
                              id="branch"
                              label="Branch"
                              class={setup.branchName!}
                              placeholder={AUTOMATIC}
                              value={state().branch}
                              set={(branch) => change({ branch })}
                            />

                            {/* And what the work is pointed at, for the
                                Processes that are pointed at work already
                                somewhere else — `processes.ts`'s list, so a
                                Process that gains a target gains the field
                                here without a line changing. Held on the
                                device like everything else in this panel, and
                                filled from the box while it is empty. */}
                            <Show when={targeted(process())}>
                              <BranchField
                                id="target"
                                label="Target"
                                class={setup.target!}
                                placeholder={TARGET}
                                value={state().target}
                                set={(target) => change({ target })}
                              />
                            </Show>

                            {/* The base, unless what is in that field is a
                                pull request: GitHub's base is the fact then,
                                and the take-up records it. A branch keeps the
                                picker, its pull request being opened against
                                what is picked here. */}
                            <Show when={!onAPullRequest()}>
                              <BasePicker
                                id="base-branch"
                                label="Base branch"
                                repo={chosen()}
                                chosen={state().base ?? RULE}
                                pick={(branch) => change({ base: branch })}
                              />
                            </Show>
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

            {/* Then what kind of work it is, which is the same question asked
                before there is a record for an answer to be about — and the one
                field in this row that is *not* remembered per repo: the
                pairings are the same answer most of the time, and a Process is
                the one thing about a Conversation likeliest to differ from the
                last. So it stands on Develop for every repo, and a picker left
                on it sends nothing when this is created.

                Settled over a loaded pull request, the way the Repo above it
                is: what a take-up makes is a Review, which is what the
                Conversation *is* rather than anything picked here. */}
            <ProcessPicker
              chosen={process()}
              disabled={make.isPending || pull() !== null}
              pick={(picked) => change({ process: picked })}
            />

            {/* And who runs it, which is one control: a picker per role the
                Process uses, in the shape the Process asks for — a panel behind
                one trigger where there are several, and the one picker itself
                where there is one. Both of those are [`ROLES`]'s to say and not
                this page's. The composer's own option drawn over what this
                device is holding rather than over a record, so the two pages
                cannot come to ask this question in two shapes.

                Each picker stands on what the repo was last grilled with (or
                its prefill, where nothing has grilled it) until it is touched,
                which is what a created draft would have arrived showing — and a
                picker left on it sends nothing when this is created, so the
                server's own prefill stands. */}
            <ProfileChoices>
              {(saved) => (
                <AgentOptions
                  process={process()}
                  saved={saved()}
                  // Composed off what the pickers are showing rather than off
                  // what the device is holding, so the trigger and the panel
                  // read as one thing: a page whose repo memory has not landed
                  // reads *Not chosen* until it does, and switching repos simply
                  // reads another memory.
                  picked={{
                    grilling: showing("grilling"),
                    implementation: showing("implementation"),
                    review: showing("review"),
                  }}
                >
                  {() => (
                    <>
                      {/* Not drawn over a loaded pull request, and by the table
                          rather than by a test for one: what a take-up makes
                          reads as a Review, and a Review has no round for a
                          grilling to open. */}
                      <Show when={uses(process(), "grilling")}>
                        <RolePicker
                          saved={saved()}
                          role="grilling"
                          label={label(process(), "grilling")}
                          chosen={showing("grilling")}
                          pick={(picked) => change({ grilling: picked })}
                        />
                      </Show>
                      <Show when={uses(process(), "implementation")}>
                        <RolePicker
                          saved={saved()}
                          role="implementation"
                          label={label(process(), "implementation")}
                          chosen={showing("implementation")}
                          pick={(picked) => change({ implementation: picked })}
                        />
                      </Show>
                      <Show when={uses(process(), "review")}>
                        <RolePicker
                          saved={saved()}
                          role="review"
                          label={label(process(), "review")}
                          away={away(process(), "review")}
                          chosen={showing("review")}
                          pick={(picked) => change({ review: picked })}
                        />
                      </Show>
                    </>
                  )}
                </AgentOptions>
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
            {/* What stands at the near edge of the row the two presses are at
                the far edge of, and so is plainly not one of them: the
                paperclip, and the way work gets into the pipeline without being
                written. Grouped, because one margin has to push both of them
                over — see `.near`. */}
            <div class={styles.near}>
              {/* The paperclip, left of Save as draft — and not offered at all
                  while a roadmap is loaded: the box is locked to a card then,
                  and there is nothing being written for a file to be handed
                  over with. */}
              <Show when={adopting() === null}>
                <attach.Clip />
              </Show>

              {/* And the work that is already somewhere else, taken up as it
                  stands. Drawn while the box is empty and nothing is loaded —
                  what a row loads stands in place of what would have been
                  written there, and a menu offering to replace a half-written
                  brief would be offering to lose it. Nothing to take up is a
                  level greyed rather than a menu gone: what there is to do here
                  is not a list the human can see, so it should not come and go
                  with one. */}
              <Show
                when={
                  state().brief.trim() === "" &&
                  adopting() === null &&
                  pull() === null
                }
              >
                <OtherActions
                  roadmaps={roadmaps()}
                  load={load}
                  pulls={pulls()}
                  take={take}
                  reading={open.isPending}
                  go={(id) => navigate(pathOf(id))}
                />
              </Show>
            </div>

            <button
              type="button"
              class={`${styles.draft} secondary`}
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

      {/* And the corner of the page, where there is no sidebar to keep it in:
          the switch that says whether what has been put away is drawn. It is
          what brings the sidebar back — a list with the archived ones in it is
          not an empty list — which is why it is on this page at all, and why it
          is not drawn where nothing is archived: a switch that could bring
          nothing back is a control with no state to be in.

          Last in the pane, which is what stands it against the bottom edge: the
          pane is a column, the composer's own margins take the room that is left
          over, and this wears the frame's `paneFoot` exactly as it does at the
          foot of the sidebar. */}
      <Show when={props.zero && props.archived}>
        <ShowArchived />
      </Show>
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

/// Everything that puts something in the box other than typing it, one nested
/// level per way of doing it.
///
/// An action with a chevron rather than a label over a value: the options along
/// the box's edge say what the work *is*, and this says what to put in the box —
/// which is why it stands under the box with the presses rather than in the row
/// inside it.
///
/// One menu rather than a dropdown apiece. Each of these lists something the
/// human cannot see from here, so a control that came and went with its list
/// would be a row of chrome that changed shape between one visit and the next —
/// and two of them would be two. So the menu is drawn whenever there is a box to
/// put something in, and a level with nothing under it is greyed: *nothing to
/// continue* is an answer, where a missing dropdown is not.
///
/// Pressing a row creates nothing. It writes what the row names into what this
/// device is holding, and the menu goes with the load: there is one thing in the
/// box at a time, and the way to another is to clear the one that is in it.
function OtherActions(props: {
  roadmaps: Adopting[];
  load: (roadmap: Adopting) => void;

  /// The pull requests open across the registered Repos, flat.
  pulls: Pull[];

  /// And what a free row of it does: the pull request loaded into the box.
  take: (pull: Pull) => void;

  /// Whether that reading is still on its way, which is the one thing this
  /// level has that the roadmaps' does not: it is a call out to GitHub per Repo,
  /// so it is often still going when the menu is opened. Greying it then would
  /// say *there is nothing to wrap up* about a list nobody has read yet, so the
  /// level opens and says it is looking.
  reading: boolean;

  /// Go to a Conversation, which is what a row for a pull request one already
  /// holds does instead of loading it.
  go: (id: number) => void;
}): JSX.Element {
  // The menu's own way to shut, so a press that has done its work takes the
  // card back and hands the focus to the trigger it came from.
  let shut = (): void => {};

  /// What a row of any level does: the menu taken back, and what it named
  /// loaded into the box behind it.
  const pick = (held: Adopting) => {
    shut();
    props.load(held);
  };

  /// And what a pull request another Conversation is already holding does: the
  /// menu taken back, and that Conversation opened. There is one Conversation
  /// per piece of work, so what a row like this offers is the one that has it
  /// rather than a second one over the same branch.
  const goTo = (id: number) => {
    shut();
    props.go(id);
  };

  /// And what a free one does: the same as a roadmap row, over the other kind of
  /// thing to take up.
  const takeUp = (pull: Pull) => {
    shut();
    props.take(pull);
  };

  return (
    <Menu
      class={styles.actions!}
      name="Other actions"
      closer={(close) => (shut = close)}
      trigger={
        <>
          Other actions
          {/* Which way the menu will go, and no part of what the button
              says. */}
          <Icon of={faChevronDown} />
        </>
      }
    >
      {() => (
        <>
          <Nested
            label="Continue a roadmap"
            disabled={props.roadmaps.length === 0}
          >
            {() => <RoadmapRows roadmaps={props.roadmaps} load={pick} />}
          </Nested>

          {/* Greyed on an answer rather than on the absence of one: while the
              reading is still out there is no list to say anything about, and a
              level that greyed until GitHub answered would be a control that
              ungreyed under the hand reaching past it. */}
          <Nested
            label="Wrap up a pull request"
            disabled={!props.reading && props.pulls.length === 0}
          >
            {() => (
              <PullRows
                pulls={props.pulls}
                reading={props.reading}
                take={takeUp}
                go={goTo}
              />
            )}
          </Nested>
        </>
      )}
    </Menu>
  );
}

/// The roadmaps nothing is driving, as the rows of the level that lists them.
///
/// Each row is worded the way the sidebar's menu worded it, this being where
/// those rows moved to: the roadmap, the Repo it is in — the list is flat, and
/// two repositories may each hold an `mvp` — the stage that would be started,
/// and where the roadmap was found when that is somewhere other than the
/// default branch.
function RoadmapRows(props: {
  roadmaps: Adopting[];
  load: (roadmap: Adopting) => void;
}): JSX.Element {
  return (
    <For each={props.roadmaps}>
      {(held) => (
        <button
          type="button"
          role="menuitem"
          class={styles.roadmapRow}
          onClick={() => props.load(held)}
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
  );
}

/// One open pull request as this page holds one: what the server said about it,
/// with the Repo it was read in carried down onto the row.
///
/// The Repo is flattened on for the roadmap rows' reason — the list is flat, and
/// `#41` in one repository is a different pull request from `#41` in the next,
/// so a row that did not say which repository would be a row naming nothing.
type Pull = OpenPullRequest & { repo_id: number; repo: string };

/// What loading a pull request puts in the box: its title as a heading, and its
/// description under it.
///
/// A Brief the human then edits rather than a card locked over the box. A pull
/// request is the one thing taken up that brings words of its own, and the one
/// the human is likeliest to have something to add to — so what it fills the box
/// with is a starting point, and what is left there is what the wrap-up reads.
///
/// The heading is the title even where the description already opens with one:
/// two headings is something to delete, where a Brief that opened on a paragraph
/// nobody could see the subject of is something to go and find.
function brief(pull: Pull): string {
  return pull.body.trim() === ""
    ? `# ${pull.title}\n`
    : `# ${pull.title}\n\n${pull.body.trim()}\n`;
}

/// The open pull requests, as the rows of the level that lists them.
///
/// Each row says what a human picks a pull request by: which repository, its
/// number and title, the branch the work is on and the branch it goes into, and
/// whose it is. Any author, because whose it is says nothing about whether it is
/// worth wrapping up — it is on the row because a list of a repository's open
/// pull requests is a list of several people's work.
///
/// **A row for a pull request a Conversation already holds leads there instead.**
/// There is one Conversation per piece of work, so what such a row offers is the
/// one that has it rather than a second one over the same branch — and it says
/// so, because a row that quietly navigated somewhere else would be a row that
/// did not do what the level said it would.
///
/// And while the reading is still out, one row saying so. This level is a `gh`
/// per registered Repo, each a call to GitHub, so a card that came down empty
/// would read as *there is nothing here* for as long as GitHub took to answer.
///
/// A free row loads it into the box, which creates nothing: the row is written
/// into what this device is holding, the box is prefilled with the pull
/// request's own title and description, and the press under the box is still the
/// first thing that reaches the server.
function PullRows(props: {
  pulls: Pull[];
  reading: boolean;
  take: (pull: Pull) => void;
  go: (id: number) => void;
}): JSX.Element {
  return (
    <Show
      when={!props.reading}
      fallback={
        // Not a `menuitem`: there is nothing here to press, and a row a
        // keyboard could land on would be one that answered nothing.
        <p class={styles.reading}>Reading GitHub…</p>
      }
    >
      <For each={props.pulls}>
        {(pull) => (
          <button
            type="button"
            role="menuitem"
            class={styles.pullRow}
            onClick={() => {
              if (pull.conversation_id === null) {
                props.take(pull);
              } else {
                props.go(pull.conversation_id);
              }
            }}
          >
            <span class={styles.what}>
              <code>
                {pull.repo} #{pull.number}
              </code>
              <span class={styles.pullTitle}>{pull.title}</span>
            </span>
            <span class={styles.next}>
              <code>{pull.head}</code> into <code>{pull.base}</code>
            </span>
            <span class={styles.found}>
              <Show when={pull.author}>by {pull.author}</Show>
              <Show when={pull.conversation_id !== null}>
                {" "}
                — already in a conversation
              </Show>
            </span>
          </button>
        )}
      </For>
    </Show>
  );
}

/// One repo the work would run alongside, drawn as the composer's own rows are
/// drawn: the name and the × on a line, what the work will do with it under
/// them, and the branch it is done on under that where there is one.
///
/// Assembled here out of the same controls the saved row is assembled out of —
/// see `Companion` in `Setup.tsx`, and the steer form's own rows, which is the
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
