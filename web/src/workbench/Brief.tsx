//! The Brief, opened: the whole of what the work started from, and under it what
//! the Conversation was set up with.
//!
//! A pane of its own rather than the plain [`Document`] the handoff and the
//! instruction share, because it carries the one thing nothing else on the page
//! carries. The setup card goes when the Brief freezes — the branch, the base,
//! the pairings and the companions all settled at that moment — so from then on
//! a read-only companion leaves no trace anywhere: a read-write one surfaces
//! later through its commits and its pull request, and a read-only one never
//! does. The worktree directories and the two Pairings are as unfindable, and
//! they belong in the same place.
//!
//! Read where the frozen Brief is read, because that is where the human goes to
//! ask what this piece of work *is* — and a Conversation with no companions gets
//! the summary all the same.
//!
//! Read-only throughout. The setup card is still the only place any of this is
//! changed, so there is not a control on this pane: what it says of a
//! Conversation past drafting is what nobody can change any more.
//!
//! Everything it draws is carried by the Conversation, so nothing here fetches.
//!
//! [`Document`]: ./Document.tsx

import { For, Show, type JSX } from "solid-js";

import type {
  BriefEvent,
  CompanionMode,
  CompanionView,
  ConversationView,
  PairingView,
  PickedView,
  Worktree,
} from "../api/types";
import { Empty } from "../notices";
import * as pairing from "../pairing";
import styles from "./Brief.module.css";
import { PaneHead } from "./PaneHead";
import { ABBREVIATED } from "./Timeline";

/// How far into a companion the work may reach, in the words its switch is
/// labelled with on the setup card — the same fact, said after it was settled.
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

export function Brief(props: {
  conversation: ConversationView;
  brief: BriefEvent;
  back: () => void;
}): JSX.Element {
  return (
    <>
      <PaneHead back={{ to: "Timeline", go: props.back }} title="Brief" />

      <Show
        when={props.brief.html !== ""}
        fallback={<Empty>Nothing was written.</Empty>}
      >
        <div class={`${styles.brief} markdown`} innerHTML={props.brief.html} />
      </Show>

      <Configuration conversation={props.conversation} />
    </>
  );
}

/// What the Conversation was configured with, under the Brief it was configured
/// for.
function Configuration(props: { conversation: ConversationView }): JSX.Element {
  return (
    <section class={styles.configuration} aria-label="Configuration">
      <h2>Configuration</h2>

      <dl class={styles.facts}>
        <Fact term="Repo">{props.conversation.repo.name}</Fact>
        <Fact term="Branch">
          <span class={styles.ref}>{props.conversation.branch}</span>
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
        <Fact term="Worktree">
          <Where worktree={props.conversation.worktree} />
        </Fact>
        <Fact term="Grilling">
          <Paired pairing={props.conversation.grilling_pairing} />
        </Fact>
        <Fact term="Implementation">
          <Paired pairing={props.conversation.implementation_pairing} />
        </Fact>
        <Fact term="Review">
          <Reviewed picked={props.conversation.review_pairing} />
        </Fact>
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
                  <Fact term="Worktree">
                    <Where worktree={companion.worktree} />
                  </Fact>
                </dl>
              </li>
            )}
          </For>
        </ul>
      </Show>
    </section>
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
      <Fact term="Branch">
        <span class={styles.ref}>
          {props.companion.branch === ""
            ? props.conversation.branch
            : props.companion.branch}
        </span>
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

/// One of the Pairings, said the way every picker of one says it: the account,
/// and the model that account runs on.
///
/// A Profile chosen before models were paired beside them is half a choice, and
/// the pane says the half there is rather than inventing the other — there is no
/// default model anywhere.
function Paired(props: { pairing: PairingView | null }): JSX.Element {
  return (
    <Show
      when={props.pairing}
      fallback={<span class={styles.rule}>Not chosen.</span>}
    >
      {(paired) => (
        <>
          {paired().model
            ? pairing.label({ profile: paired().profile, model: paired().model! })
            : paired().profile.name}
        </>
      )}
    </Show>
  );
}

/// And a role that could be picked away as well as paired, which is the review.
///
/// Said as the choice it was rather than as an absence: *no review* is what the
/// human picked, and a pane that read it as "not chosen" would show a settled
/// conversation as an unsettled one.
function Reviewed(props: { picked: PickedView }): JSX.Element {
  return (
    <Show
      when={props.picked !== "Skipped"}
      fallback={<span class={styles.rule}>No review.</span>}
    >
      <Paired pairing={pairing.under(props.picked)} />
    </Show>
  );
}
