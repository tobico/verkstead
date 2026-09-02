//! The Brief, opened: the whole of what the work started from, and under it what
//! the Conversation was set up with.
//!
//! A pane of its own rather than the plain [`Document`] the handoff and the
//! instruction share, because it carries the one thing nothing else on the page
//! carries. The composer goes when the round stops drafting — the branch, the base,
//! the pairings and the companions all settled at that moment — so from then on
//! a read-only companion leaves no trace anywhere: a read-write one surfaces
//! later through its commits and its pull request, and a read-only one never
//! does. The worktree directories and the Pairings are as unfindable, and
//! they belong in the same place.
//!
//! Read where the frozen Brief is read, because that is where the human goes to
//! ask what this piece of work *is* — and a Conversation with no companions gets
//! the summary all the same.
//!
//! Read-only throughout. The composer is still the only place any of this is
//! changed, so there is not a control on this pane: what it says of a
//! Conversation past drafting is what nobody can change any more.
//!
//! Everything it draws is carried by the Conversation, but for one list: the
//! saved Agent Profiles, which say whether the account behind a Pairing is the
//! only one on its backend and so whether its name is said after the model. That
//! read is inside [`Machine`], which is the half of the summary a share does not
//! draw — a share makes no request to anything, and this is what keeps it that
//! way.
//!
//! [`Document`]: ./Document.tsx

import { For, Show, type JSX } from "solid-js";

import { listProfiles } from "../api/client";
import type {
  BriefEvent,
  CompanionMode,
  CompanionView,
  ConversationView,
  PairingView,
  PickedView,
  ProfileEntry,
  Worktree,
} from "../api/types";
import { useReading } from "../freshness";
import { HarnessMark } from "../HarnessMark";
import { PaneSticky } from "../Panes";
import { Empty } from "../notices";
import * as pairing from "../pairing";
import styles from "./Brief.module.css";
import { chosen } from "./naming";
import { PaneHead } from "./PaneHead";
import { ABBREVIATED } from "./Timeline";

/// How far into a companion the work may reach, in the words its switch is
/// labelled with on the composer — the same fact, said after it was settled.
const ACCESS: Record<CompanionMode, string> = {
  ReadOnly: "Read-only",
  ReadWrite: "Read-write",
};

/// A whole commit hash and nothing else: forty hex characters, which is what git
/// writes and what the base becomes once grilling has resolved it.
///
/// What is stored before that is the branch the human picked, and a branch name
/// is not a thing to shorten — `release-1.4` cut to seven characters is a branch
/// that does not exist. So the test decides whether there is anything to
/// abbreviate, rather than the length.
const COMMIT = /^[0-9a-f]{40}$/;

/// What stands where a branch has no name to report yet: the Conversation's
/// own, and a companion mirroring the Conversation's own.
///
/// Read on the one Conversation whose Brief is frozen while it is still
/// drafting — an adopting one, whose stage brief comes down frozen from the
/// start — and nowhere else: every other Conversation with a summary to draw
/// has been named or has a branch by the time there is a pane to open.
export const UNNAMED = "Chosen when the work starts.";

export function Brief(props: {
  conversation: ConversationView;
  brief: BriefEvent;

  /// The way out of the pane, named as well as pressed — the composer's
  /// arrangement, and for the composer's reason: this pane is the other thing a
  /// Conversation whose record is the one Event can open, and such a record has
  /// no Timeline drawn to go back to. See `Composer.tsx`.
  back: { to: string; go: () => void };

  /// Whether this is a record read out of a file — a share — rather than the
  /// Conversation on the machine it was worked on.
  ///
  /// What it takes off is the half of the Configuration that is about the
  /// machine rather than about the work: where each checkout sits on somebody's
  /// disk, and which account and model each kind of session ran under. A share
  /// is emailed about and attached to pull requests, and none of that is
  /// anything its reader is owed.
  ///
  /// The record it is drawn from does not carry them either — see `shared` in
  /// `crates/render/src/sharing.rs`, which is the half that keeps them out of
  /// the file rather than merely off the page. This is why what is left of them
  /// there is never read.
  readOnly?: boolean;
}): JSX.Element {
  return (
    <>
      <PaneSticky>
        <PaneHead back={props.back} title="Brief" />
      </PaneSticky>

      <Show
        when={props.brief.html !== ""}
        fallback={<Empty>Nothing was written.</Empty>}
      >
        <div class={`${styles.brief} markdown`} innerHTML={props.brief.html} />
      </Show>

      <Configuration
        conversation={props.conversation}
        readOnly={props.readOnly}
      />
    </>
  );
}

/// What the Conversation was configured with, under the Brief it was configured
/// for.
function Configuration(props: {
  conversation: ConversationView;
  /// Drawn out of a share, which is a shorter list — see [`Brief`].
  readOnly?: boolean;
}): JSX.Element {
  return (
    <section class={styles.configuration} aria-label="Configuration">
      <h2>Configuration</h2>

      <dl class={styles.facts}>
        <Fact term="Repo">{props.conversation.repo.name}</Fact>
        {/* The name where there is one, and the rule that will pick one where
            there is not — which is what a Conversation adopting a roadmap
            reads, its Brief being frozen from the start and its branch being
            the stage's own slug once it is adopted. */}
        <Fact term="Branch">
          <Show
            when={chosen(props.conversation)}
            fallback={<span class={styles.rule}>{UNNAMED}</span>}
          >
            {(branch) => <span class={styles.ref}>{branch()}</span>}
          </Show>
        </Fact>
        {/* The commit rather than the branch that was picked: the pick is a
            name while the Conversation drafts and is replaced by whatever it
            resolved to when grilling started, which is the honest thing to
            report about work already under way. */}
        <Fact term="Base">
          <Show
            when={props.conversation.base_commit}
            fallback={
              <span class={styles.rule}>
                The default branch ({props.conversation.repo.default_branch}),
                resolved when the work starts.
              </span>
            }
          >
            {(base) => (
              <span class={styles.ref}>
                {COMMIT.test(base()) ? base().slice(0, ABBREVIATED) : base()}
              </span>
            )}
          </Show>
        </Fact>
        {/* And where it was worked, and by whom — the half of this section that
            is about the machine rather than about the work, so the half a share
            does not draw. The record it would be drawn from is empty there
            anyway; this is what keeps the pane from reporting that emptiness as
            *not checked out* and *no grilling*, which would be a share telling
            the reader something untrue instead of nothing at all. */}
        <Show when={!props.readOnly}>
          <Machine conversation={props.conversation} />
        </Show>
      </dl>

      {/* And the repos this work was let into beside its own, where there are
          any. No heading over an empty list: a Conversation with no companions
          is most of them, and a section saying so would read as something
          having gone missing. */}
      <Show when={props.conversation.companions.length > 0}>
        <h3>Companion repos</h3>
        <ul class={styles.companions}>
          <For each={props.conversation.companions}>
            {(companion) => (
              <li class={styles.companion}>
                <p class={styles.companionName}>{companion.repo.name}</p>
                <dl class={styles.facts}>
                  <Fact term="Access">{ACCESS[companion.mode]}</Fact>
                  <Holding
                    companion={companion}
                    conversation={props.conversation}
                  />
                  {/* And its checkout, off a share for the reason the
                      Conversation's own is. */}
                  <Show when={!props.readOnly}>
                    <Fact term="Worktree">
                      <Where worktree={companion.worktree} />
                    </Fact>
                  </Show>
                </dl>
              </li>
            )}
          </For>
        </ul>
      </Show>
    </section>
  );
}

/// The half of the Configuration that is about the machine rather than about the
/// work: which checkout it was worked in, and which account and model each kind
/// of session runs under.
///
/// Its own component because of the one read this pane makes. A Pairing carries
/// the Profile whole, so the backend and the model are in hand — but whether the
/// Profile's *name* is worth saying is a fact about the other Profiles, and that
/// is the saved list. So the list is read here, inside the half a share does not
/// draw: a share makes no request to anything, and a query mounted above the
/// `Show` would be one.
///
/// The same query the pickers make, so the cache is what a second caller pays —
/// and the reading says the Profile's name while it is still in flight, saying
/// it being the answer that can never misattribute a run.
function Machine(props: { conversation: ConversationView }): JSX.Element {
  const profiles = useReading(() => ({
    queryKey: ["profiles"],
    queryFn: listProfiles,
    freshness: { reconcile: "id" },
  }));

  return (
    <>
      <Fact term="Worktree">
        <Where worktree={props.conversation.worktree} />
      </Fact>
      <Fact term="Grilling">
        <Picked
          picked={props.conversation.grilling_pairing}
          saved={profiles.data}
          away="No grilling."
        />
      </Fact>
      <Fact term="Implementation">
        <Paired
          pairing={props.conversation.implementation_pairing}
          saved={profiles.data}
        />
      </Fact>
      <Fact term="Review">
        <Picked
          picked={props.conversation.review_pairing}
          saved={profiles.data}
          away="No review."
        />
      </Fact>
    </>
  );
}

/// One thing the Conversation was set up with: what it is called, and what it
/// says.
///
/// A `<dl>` row rather than a line of prose, because a pane read to answer "what
/// is this work against?" is scanned down the terms rather than read — and
/// because the wrapping `<div>` is what lets the pair lay out as a row on a
/// window with room for one and stack where there is not.
function Fact(props: { term: string; children: JSX.Element }): JSX.Element {
  return (
    <div class={styles.fact}>
      <dt>{props.term}</dt>
      <dd>{props.children}</dd>
    </div>
  );
}

/// What a companion's checkout is holding, which is a different fact in each of
/// the two modes.
///
/// A read-write companion is on a branch cut for it — the Conversation's own
/// name where it was left mirroring, which is what mirroring resolves to. A
/// read-only one has no branch at all: it is detached at the commit its base
/// came to when the checkout was made, which is the same honesty the base row
/// above is written for — the branch that was picked is a name, and a name that
/// has moved since says the checkout is somewhere it is not.
///
/// The name is the fallback and not the answer: it is what a checkout made
/// before Verkstead kept the commit has left to say.
function Holding(props: {
  conversation: ConversationView;
  companion: CompanionView;
}): JSX.Element {
  return (
    <Show
      when={props.companion.mode === "ReadWrite"}
      fallback={
        <Fact term="Detached at">
          <span class={styles.ref}>
            <Show
              when={props.companion.base_commit}
              fallback={
                props.companion.base_ref ?? props.companion.repo.default_branch
              }
            >
              {(base) =>
                COMMIT.test(base()) ? base().slice(0, ABBREVIATED) : base()
              }
            </Show>
          </span>
        </Fact>
      }
    >
      {/* Mirroring — no name of its own in the record — is the conversation's
          own branch, so where that has no name yet neither has this: the same
          words stand under both. */}
      <Fact term="Branch">
        <Show
          when={props.companion.branch || chosen(props.conversation)}
          fallback={<span class={styles.rule}>{UNNAMED}</span>}
        >
          {(branch) => <span class={styles.ref}>{branch()}</span>}
        </Show>
      </Fact>
    </Show>
  );
}

/// Where a checkout is, which is the one fact about a Conversation the page has
/// never said anywhere.
///
/// Nothing to say before grilling made it and nothing to say once closing took
/// it away, which are the same fact about it; and a directory deleted by hand is
/// said rather than left for whatever next tries to work in it to fall over on.
function Where(props: { worktree: Worktree | null }): JSX.Element {
  return (
    <Show
      when={props.worktree}
      fallback={<span class={styles.rule}>Not checked out.</span>}
    >
      {(worktree) => (
        <>
          <span class={styles.path}>{worktree().path}</span>
          <Show when={worktree().missing}>
            <span class={styles.gone}>gone from disk</span>
          </Show>
        </>
      )}
    </Show>
  );
}

/// One of the Pairings, said the way every picker of one says it: the backend's
/// mark, the backend, the model that account runs on, and the account's name
/// where that is what tells two of them apart.
///
/// A Profile chosen before models were paired beside them is half a choice, and
/// the pane says the half there is rather than inventing the other — there is no
/// default model anywhere. That reading is the shared one's too, which is why
/// nothing is decided here.
function Paired(props: {
  pairing: PairingView | null;
  /// The Profiles as they stand, which is what says whether the account's own
  /// name is worth saying — `undefined` until the list has been read.
  saved: ProfileEntry[] | undefined;
}): JSX.Element {
  return (
    <Show
      when={props.pairing}
      fallback={<span class={styles.rule}>Not chosen.</span>}
    >
      {(paired) => (
        <>
          {/* The backend's own mark in front of its name, as every other site
              that says who runs a session draws it. A Profile always carries a
              backend, so a paired row always has one to draw — the missing
              half here is the model rather than the harness. The space after it
              is the row's own `column-gap`, this `dd` being a flex row. */}
          <HarnessMark of={paired().profile.account.agent_type} />
          {pairing.label(paired(), props.saved)}
        </>
      )}
    </Show>
  );
}

/// And a role that could be picked away as well as paired, which is the grilling
/// and the review.
///
/// Said as the choice it was rather than as an absence: *no grilling* and *no
/// review* are what the human picked, and a pane that read either as "not
/// chosen" would show a settled conversation as an unsettled one.
function Picked(props: {
  picked: PickedView;
  saved: ProfileEntry[] | undefined;
  away: string;
}): JSX.Element {
  return (
    <Show
      when={props.picked !== "Skipped"}
      fallback={<span class={styles.rule}>{props.away}</span>}
    >
      <Paired pairing={pairing.under(props.picked)} saved={props.saved} />
    </Show>
  );
}
