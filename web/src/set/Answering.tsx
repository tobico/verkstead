//! The questions as a sheet to fill in, with the submit that ends the agent's
//! wait.
//!
//! The sheet is one store holding one entry per question, in the order the Set
//! asked them — which is the order a Response has to account for them in. A
//! question that went missing here would make the Response incomplete, and the
//! server refuses those by name rather than letting one through.
//!
//! Everything the human puts in is written to `localStorage` as they put it in,
//! and read back the next time the page is opened. The draft is per device and
//! never sent anywhere: a phone and a laptop keep their own of the same Set.
//!
//! **The files are the exception, and they are not in the page at all.** Every
//! Question and Sub-question carries a paperclip, and a file chosen or dropped
//! on one goes up the moment it is chosen — onto the Set, under that Question's
//! label — so what the row of pills draws is read back off the Set like the
//! Questions themselves. Which is what makes them survive a reload the draft
//! cannot help with, and what lets a × take one off the record rather than out
//! of a list waiting to be sent. The attaching itself is the one piece the
//! composers share — see [`Attaching`](../Attaching.tsx) — and what is here is
//! the one thing that differs: where a chosen file goes.
//!
//! **And the control is the composer's plus-menu**, at every Question and at
//! the comment box alike: *Attach file…* first — at the comment box a file is
//! on the Set as a whole, answering nothing — then the declared MCP servers the Conversation does not have yet, and *Add
//! a server…*. A server picked there is recorded on the Set the moment it is
//! picked, under the Question it was picked at or under none at the comment
//! box, and drawn as a chip beside that Answer's files — read back off the Set
//! like them, so it survives a reload and shows on another device. Nothing
//! reaches the Conversation until the Set is answered (ADR-0028).
//!
//! **And past the servers, the repos and the devices**: each registered repo
//! the work could have alongside it, through the steer's own fields on a card,
//! each read-only companion to open up through the same card, and each device
//! of the cluster the work could be permitted to move to, picked at once.
//! Recorded and drawn exactly as a server is.

import { useMutation, useQueryClient } from "@tanstack/solid-query";
import type { JSX } from "solid-js";
import {
  For,
  Show,
  createEffect,
  createMemo,
  createSignal,
  createUniqueId,
} from "solid-js";
import { createStore } from "solid-js/store";

import app from "../App.module.css";
import { attaching, type Chip, type Shown } from "../Attaching";
import { Modal } from "../Modal";
import {
  addToSet,
  attachToAnswer,
  attachToSet,
  removeAnswerAttachment,
  removeFromSet,
  submitResponse,
} from "../api/client";
import type {
  AddableView,
  AdditionMade,
  AdditionRemoved,
  AdditionView,
  AnswerAttached,
  AnswerAttachmentRemoved,
  AskView,
  AttachmentView,
  ClusterDevice,
  DeviceIdentity,
  Direction,
  Ending,
  OptionView,
  Pick as Picked,
  ProposalView,
  QuestionView,
  RepoEntry,
  Response as Decided,
  SteerCompanionRefusal,
  Submitted,
} from "../api/types";
import {
  DIRECTION,
  DIRECTION_LABEL,
  DIRECTION_NOTE,
  DIRECTIONS,
} from "../directions";
import { ErrorLine, Note } from "../notices";
import { keyOf, useDevice } from "../reaching";
import { addingServer } from "../workbench/AddServer";
import { otherAuthor } from "../workbench/Timeline";
import { addingRepo } from "./AddingRepo";
import { ServerRows } from "../workbench/Composer";
import { rowPress } from "../rows";
import styles from "./Answering.module.css";
import { AskText } from "./AskText";
import { Postscript } from "./Postscript";
import { Placing, Wanting, ticking } from "./Wanting";
// The question vocabulary, defined once for the form and the record alike —
// every class below that a settled Set also draws comes from here.
import page from "./Sheet.module.css";
import { anchor } from "./outline";
import { Head, starred } from "./table";
import {
  clearDraft,
  clicked,
  drafted,
  keepDraft,
  storedDraft,
  unanswered,
} from "./filling";
import type { Filled } from "./filling";

/// One question as the sheet holds on to it: the name it answers to, and
/// whether it offered Options — which is what decides whether leaving it open is
/// worth warning about.
type Asked = { label: string; multipleChoice: boolean };

/// The live fields of one question, as the row drawing it reaches them: what is
/// in them, and the three gestures that change it.
type Fields = {
  selected: () => number | null;
  free_text: () => string;
  /// A click, which selects the Option — or clears the question, when it landed
  /// on the Option already selected.
  pick: (n: number) => void;
  /// An arrow key, which only ever moves the selection.
  move: (n: number) => void;
  say: (said: string) => void;
};

/// The sheet a waiting Set is answered on.
export function Answering(props: {
  id: number;
  questions: QuestionView[];
  postscript: string | null;

  /// The wrap-up proposal this Set carries, on the one Set that carries one.
  /// What puts the direction chooser on the sheet; `null` leaves it off.
  proposal: ProposalView | null;

  /// Which ending the Nothing-else option in the closing section would bring
  /// about, which is also what puts the option there at all. `null` on every Set
  /// asked from a state that has no such ending, which leaves the option off it.
  ending: Ending | null;

  /// The files already put on this Set's Answers, oldest first, each naming the
  /// Question it was put under.
  ///
  /// The record rather than anything this page is holding: a pill is drawn from
  /// this, so a reload draws the same row, and the × that takes one off is
  /// answered by the Set being read again.
  attachments: AttachmentView[];

  /// What has been picked on this Set to add to the work, each under the
  /// Question it was picked at or under none — the record, like the files.
  additions: AdditionView[];

  /// The MCP servers the Conversation has already, which the menus leave out.
  servers: string[];

  /// And the rest of what the menus offer: Repos to add alongside, read-only
  /// companions to open up, and devices the work may be transferred to.
  addable: AddableView;

  /// What the page around the sheet does once an answer is sent, where it does
  /// anything: the workbench puts its Timeline back into advancing, so the
  /// next landing carries the selection on from the Set just answered. The
  /// standalone sheet has no selection to carry, and passes none.
  answered?: () => void;
}): JSX.Element {
  // Read once rather than through a memo: the fields are the Set's own shape,
  // and rebuilding them under a human who is typing into them would throw away
  // what they had written.
  const asked: Asked[] = flatten(props.questions);
  const labels = asked.map((ask) => ask.label);

  // The draft goes in before the first paint rather than from an effect
  // afterwards: there is no server-rendered page to disagree with any more, so a
  // half-finished sheet arrives finished-as-far-as-it-went instead of blank and
  // then filling itself in.
  const kept = storedDraft(props.id, labels);
  const [sheet, setSheet] = createStore({
    filled: kept?.filled ?? asked.map(blank),
    comment: kept?.comment ?? "",
    direction: kept?.direction ?? null,
    nothing_else: kept?.nothing_else ?? false,
  });

  // Whether this Set is done with: it has an answer, or it can never take one.
  // Nothing more is drafted after that — keeping a draft would only resurface
  // it, stale and unsendable, on some later visit.
  const [settled, setSettled] = createSignal(false);

  // Every change from here on, written out as it happens. Reading each field is
  // what subscribes this to them, so it runs again on every tap and every
  // keystroke — and it reads them before it looks at whether the Set is spent,
  // so a sheet that goes on being typed into after a refusal stays subscribed.
  createEffect(() => {
    const draft = {
      filled: sheet.filled.map((field) => ({ ...field })),
      comment: sheet.comment,
      direction: sheet.direction,
      nothing_else: sheet.nothing_else,
    };

    if (settled()) {
      return;
    }

    keepDraft(props.id, draft);
  });

  // What becomes of a file put on one of these Answers, held once for the
  // sheet: the pills each Question draws, the requests a choice and a × make,
  // and what came back refused from either.
  const files = putting({
    set: () => props.id,
    attached: () => props.attachments,
  });

  // And what is picked at the same menus to add to the work, held once for the
  // sheet the same way: the chips each Answer and the comment box draw, the
  // presses that put one on and take it off, and what came back refused.
  const additions = picking({
    set: () => props.id,
    picked: () => props.additions,
    mirroring: () => props.addable.branch,
    ticking: () =>
      props.proposal === null
        ? []
        : ticking(props.proposal.wants, props.addable.cluster).map(
            (member) => member.device.device,
          ),
  });

  // The cards *Add a server…* and a repo's row open, one each for the whole
  // sheet and drawn beside it — the menu's rows go with the press that opened
  // it. Which menu one was opened from is held here, so what it confirms is
  // picked there.
  let openedAt: string | null = null;
  const declaring = addingServer((name) =>
    additions.pick(openedAt, { Server: { name } }, name),
  );
  const joining = addingRepo(
    (pick) => additions.pick(openedAt, pick, "That repo"),
    () => props.addable.branch,
  );

  /// The rows an Attach menu holds under *Attach file…*, at `label` or at the
  /// comment box where `null`: the servers the Conversation does not have and
  /// this Set has not had picked, and *Add a server…*; then the repos and the
  /// devices, past what this Set has had picked already.
  const offering = (label: string | null) => (shut: () => void) => (
    <>
      <ServerRows
        attached={[...props.servers, ...additions.servers()]}
        attach={(name) => {
          shut();
          additions.pick(label, { Server: { name } }, name);
        }}
        add={() => {
          shut();
          openedAt = label;
          declaring.open();
        }}
      />
      <AddableRows
        addable={props.addable}
        repos={additions.repos()}
        devices={additions.devices()}
        join={(repo, opening) => {
          shut();
          openedAt = label;
          joining.open(repo, opening);
        }}
        permit={(device, name) => {
          shut();
          additions.pick(label, { Device: { device } }, name);
        }}
      />
    </>
  );

  // The comment box's own piece: the files put on the Set as a whole, which
  // answer no Question, and the chips of what was picked at its menu.
  const comment = attaching({
    shown: () => files.shown(null),
    add: (chosen) => files.send(null, chosen),
    onto: "the question set",
    chips: () => additions.chips(null),
    offering: offering(null),
  });

  /// What the sheet says right now, as a Response.
  ///
  /// The labels a file was put on go with it: a Question with one on it and
  /// nothing typed or picked is answered rather than open, and the record's own
  /// rows are what say which those are.
  const response = (): Decided =>
    drafted(
      sheet.filled,
      sheet.comment,
      sheet.direction,
      sheet.nothing_else,
      files.answered(),
    );

  const queries = useQueryClient();
  const device = useDevice();

  const submit = useMutation(() => ({
    mutationFn: (sending: Decided) =>
      submitResponse(device(), props.id, sending),
    onSuccess: (outcome: Submitted) => {
      if (typeof outcome !== "string") {
        // Rejected by the grammar, or for a server picked on it that has gone
        // from the settings since. The first is a bug here and the second the
        // human's to put right with the chip's × — either way their draft
        // stands, because it is the only copy of what they wrote.
        return;
      }

      // Either it was taken or this Set can take nothing: both ways the sheet
      // is spent.
      setSettled(true);
      clearDraft(props.id);
      props.answered?.();

      // And the sheet stays where it is, read back as the record of what was
      // decided rather than left behind for a list — there is no list. Which is
      // why everything is invalidated rather than nothing: this page is not
      // going anywhere, and the Set it is drawing has just changed. The
      // Conversation's Timeline goes with it, because the row this Set is on
      // has changed too, and in the workbench that row is on screen beside it.
      void queries.invalidateQueries();
    },
  }));

  // The names of the multiple-choice questions being left open, which puts the
  // warning between the human and the send. It is never an empty list: with no
  // offered choice left open there is nothing to warn about, and the Response
  // goes straight out.
  const [confirming, setConfirming] = createSignal<string[] | null>(null);

  const start = () => {
    const sending = response();
    const choices = asked
      .filter((ask) => ask.multipleChoice)
      .map((ask) => ask.label);
    const open = unanswered(sending, choices);

    if (open.length === 0) {
      submit.mutate(sending);
    } else {
      setConfirming(open);
    }
  };

  // Drafted again rather than kept from the warning: nothing can have changed
  // while the dialog was up, and this way there is no stale copy to send.
  const sendAnyway = () => {
    setConfirming(null);
    submit.mutate(response());
  };

  /// Where a question's fields sit in the sheet. The labels are the Set's own
  /// and distinct across it, so this is exact.
  const at = (label: string) => labels.indexOf(label);

  const fields = (label: string) => ({
    selected: () => sheet.filled[at(label)]!.selected,
    free_text: () => sheet.filled[at(label)]!.free_text,
    pick: (n: number) =>
      setSheet("filled", at(label), "selected", (held) => clicked(held, n)),
    // An arrow key moves a radio selection and fires a change without ever
    // firing a click, so it has to land somewhere too — on the Option moved to,
    // which is never a clearing.
    move: (n: number) => setSheet("filled", at(label), "selected", n),
    say: (said: string) => setSheet("filled", at(label), "free_text", said),
  });

  const failed = createMemo(() =>
    refused(submit.data, submit.error as Error | null),
  );

  return (
    <>
      <h2 class={app.sectionHeading} id="questions">
        Questions
      </h2>
      <ol class={page.questions}>
        <For each={props.questions}>
          {(question, index) => (
            <li class={page.question} id={anchor(question.ask.name, index() + 1)}>
              {/* A Heading is its text and nothing else — no Options, no field,
                  nothing to leave open. What it heads is directly under it. */}
              <Show
                when={!question.heading}
                fallback={
                  <div class={`${page.ask} ${page.heading}`}>
                    <AskText
                      name={question.ask.name}
                      html={question.ask.text_html}
                    />
                  </div>
                }
              >
                <Asking
                  ask={question.ask}
                  fields={fields(question.ask.name)}
                  files={files}
                  additions={additions}
                  offering={offering(question.ask.name)}
                />
              </Show>
              {/* Sub-questions get no anchor of their own: one scrolls into
                  view with its parent. */}
              <Show when={question.subquestions.length > 0}>
                <ol class={page.subquestions}>
                  <For each={question.subquestions}>
                    {(subquestion) => (
                      <li class={page.subquestion}>
                        <Asking
                          ask={subquestion}
                          fields={fields(subquestion.name)}
                          files={files}
                          additions={additions}
                          offering={offering(subquestion.name)}
                        />
                      </li>
                    )}
                  </For>
                </ol>
              </Show>
            </li>
          )}
        </For>
      </ol>
      {/* The chooser, on the one Set that carries a proposal. Under the
          Questions because it is the closing decision: what is still uncertain
          is asked above, and this is what to do now that it is not. */}
      <Show when={props.proposal}>
        {(proposal) => (
          <Choosing
            proposal={proposal()}
            cluster={props.addable.cluster}
            ticked={(device) => additions.devices().includes(device)}
            tick={(device, ticked) =>
              ticked
                ? additions.pick(
                    null,
                    { Device: { device: device.device } },
                    device.name,
                    CHOOSER,
                  )
                : additions.untick(device.device, CHOOSER)
            }
            refused={() => additions.refused(CHOOSER)}
            picked={() => sheet.direction}
            pick={(direction) =>
              setSheet("direction", (held) =>
                held === direction ? null : direction,
              )
            }
            move={(direction) => setSheet("direction", direction)}
          />
        )}
      </Show>
      {/* The agent's closing word, wrapped around the box it is inviting
          something into: what it suggests taking up is read on the way to
          writing, rather than asked as a Question of its own. A Set that closed
          without one still draws the card, so the box is in the same place
          either way. */}
      <Postscript html={props.postscript}>
        <section
          class={page.setComment}
          classList={{ [styles.taking!]: comment.over() }}
          {...comment.dropping}
        >
          <div class={styles.writing}>
            <div
              class={`${app.grow} ${styles.field}`}
              data-value={sheet.comment}
            >
              <textarea
                id="set-comment"
                name="set-comment"
                rows="1"
                placeholder="Other comments"
                aria-label="Other comments"
                value={sheet.comment}
                onInput={(event) =>
                  setSheet("comment", event.currentTarget.value)
                }
              />
            </div>
            <comment.Clip class={styles.attach} />
          </div>
          <comment.Pills class={styles.attachments} />
          <For each={files.refusals(null)}>
            {(said) => (
              <ErrorLine class={styles.attachFailure}>{said}</ErrorLine>
            )}
          </For>
          <Show when={files.refusedRemoval(null)}>
            {(said) => (
              <ErrorLine class={styles.attachFailure}>{said()}</ErrorLine>
            )}
          </Show>
          <Show when={additions.refused(null)}>
            {(said) => (
              <ErrorLine class={styles.attachFailure}>{said()}</ErrorLine>
            )}
          </Show>
        </section>
        {/* And, on the Sets of a state that has an ending to mark, the way to
            say there is nothing else — under the box rather than beside it,
            because it is what the human reaches for once they have finished
            writing. */}
        <Show when={props.ending}>
          {(ending) => (
            <NothingElse
              ending={ending()}
              ticked={() => sheet.nothing_else}
              tick={(ticked) => setSheet("nothing_else", ticked)}
            />
          )}
        </Show>
      </Postscript>
      {declaring.modal()}
      {joining.modal()}
      <section class={styles.submit}>
        <button type="button" onClick={start} disabled={submit.isPending}>
          {submit.isPending ? "Sending…" : "Submit"}
        </button>
        <Show when={failed()}>
          {(said) => <ErrorLine class={styles.failure}>{said()}</ErrorLine>}
        </Show>
      </section>
      {/* The warning that stands between the human and a submit skipping
          offered choices: every multiple-choice question left open, by name, and
          the choice to go back. A skipped free-text question passes without a
          word — nothing was offered, so nothing was overlooked. It warns and
          never blocks: leaving the whole Set open with only a comment is a
          counter-question, not a mistake, and it comes through here like any
          other. */}
      <Modal
        class={page.confirm!}
        open={confirming() !== null}
        close={() => setConfirming(null)}
        labelledBy="confirm-title"
      >
        <p id="confirm-title" class={page.confirmTitle}>
          Send with these unanswered?
        </p>
        <ul class={page.unanswered}>
          <For each={confirming() ?? []}>{(name) => <li>{name}</li>}</For>
        </ul>
        <Note class={page.caveat}>
          The agent will be told these are still open.
        </Note>
        <div class={page.confirmActions}>
          <button
            type="button"
            class={`${page.secondary} secondary`}
            onClick={() => setConfirming(null)}
          >
            Cancel
          </button>
          <button type="button" onClick={sendAnyway}>
            Send anyway
          </button>
        </div>
      </Modal>
    </>
  );
}

/// The direction chooser, injected onto any Set whose agent closed with a
/// proposal: how the work gets built, decided in the one place the human is
/// already looking.
///
/// Drawn as the page's own sections are: a heading naming what is decided, and
/// a card under it holding the whole of the deciding. Which makes this a
/// "Direction" section carrying one question-like card, rather than the bare
/// run of controls it briefly was — the label keeps the place a Question's
/// number keeps, so the two line up down the page.
///
/// All three every time, whichever one was recommended — the recommendation is
/// marked and never preselected, exactly as an Option's ★ is, so nothing is
/// picked until the human picks it. The rationale sits beside the three, because
/// what they are deciding against is the agent's argument rather than a bare
/// word.
///
/// And the chooser says what picking does. It is the one control on the page
/// whose meaning is not obvious from what it is labelled: picking accepts the
/// proposal, and every other way of answering sends it back. That used to be
/// left to the agent to write into the Preface, which made the mechanics
/// something each grilling explained in its own words — or forgot to.
///
/// A second click on the picked direction clears it, the way a second click on
/// a selected Option does: un-picking is the only gesture a radio group has no
/// button for, and changing your mind about accepting is exactly the case that
/// wants one.
function Choosing(props: {
  proposal: ProposalView;
  /// The cluster the work could be placed on, or `null` in none — see
  /// [`Placing`].
  cluster: ClusterDevice[] | null;
  /// Whether a device is picked on the set, wherever it was picked.
  ticked: (device: string) => boolean;
  /// A tick, or the tick taken off.
  tick: (device: DeviceIdentity, ticked: boolean) => void;
  /// What came back refused from either.
  refused: () => string | null;
  picked: () => Direction | null;
  /// A click, which picks the direction — or clears the choice, when it landed
  /// on the direction already picked.
  pick: (direction: Direction) => void;
  /// An arrow key, which only ever moves the choice.
  move: (direction: Direction) => void;
}): JSX.Element {
  return (
    <section class={page.directionPick} id="direction">
      {/* Headed like the Preface and the Postscript, and holding one card like
          the Questions do: the section names what is being decided and the card
          is the deciding, so this reads as one more question under a heading of
          its own rather than as furniture of a different kind. */}
      <h2 class={app.sectionHeading}>Direction</h2>
      <div class={page.directionCard}>
        {/* Asked as a Question is asked: the label floated in the accent with
            the agent's argument running beside it, and the three to pick from
            under it where a Question's Options are. */}
        <div class={page.ask}>
          <AskText
            name={DIRECTION_LABEL}
            html={props.proposal.rationale_html}
          />
        </div>
        <ul class={page.directions}>
          <For each={DIRECTIONS}>
            {(offered) => {
              const recommended = () => props.proposal.direction === offered;

              return (
                <li
                  class={page.direction}
                  classList={{ [page.recommended!]: recommended() }}
                >
                  <label onClick={rowPress(() => props.pick(offered))}>
                    <input
                      type="radio"
                      id={`direction-${offered}`}
                      name="direction"
                      value={offered}
                      checked={props.picked() === offered}
                      // Both gestures, for the reason an Option answers
                      // both: an arrow key fires a change and never a click,
                      // and a click on the radio itself when it is already
                      // picked fires a click and never a change. A press
                      // anywhere else in the row is the row's own — see
                      // [`rowPress`].
                      onChange={() => props.move(offered)}
                      onClick={() => props.pick(offered)}
                    />
                    <span class={page.directionName}>{DIRECTION[offered]}</span>
                    <Show when={recommended()}>
                      <span class={page.star} title="The agent's recommendation">
                        ★
                      </span>
                    </Show>
                  </label>
                  <Note class={page.means}>{DIRECTION_NOTE[offered]}</Note>
                </li>
              );
            }}
          </For>
        </ul>
        {/* What the work wants: ticks where there is a cluster to place it
            on, and the line of prose where there is none. */}
        <Show
          when={props.proposal.wants.length > 0 && props.cluster}
          fallback={<Wanting wants={props.proposal.wants} />}
        >
          {(cluster) => (
            <Placing
              wants={props.proposal.wants}
              cluster={cluster()}
              ticked={props.ticked}
              tick={props.tick}
              refused={props.refused}
            />
          )}
        </Show>
        <p class={page.semantics}>
          Picking a direction accepts the proposal and the agent gets on with
          it. Anything else, your own answer included, sends it back for
          another round.
        </p>
      </div>
    </section>
  );
}

/// The Nothing-else option, drawn in the closing section of a follow-up's Sets
/// and nowhere else: the human saying that there is nothing more they want from
/// this follow-up.
///
/// Inside the Postscript's card with the comment box, because the two are one
/// closing move — what they have to say about the round, and whether there is to
/// be another one. It is not a Question and it is not the agent's: the agent
/// writes an ordinary Postscript, is handed an ordinary Response, and never
/// learns whether this was ticked. Ending the follow-up is Verkstead's own.
///
/// A checkbox rather than an Option, because there is nothing here to choose
/// between: it is on or it is off, and a second click takes it off again — the
/// clearing an Option needs a rule of its own for.
///
/// The row is pressed the way every other row on the sheet is — see
/// [`rowPress`] — so the tick is the row's own doing wherever the press landed,
/// and the box's `change` answers only a press on the box.
///
/// And it says what ticking does, for the reason the direction chooser does:
/// what a control means has to be on the control, rather than left to whatever
/// the agent happened to write above it.
function NothingElse(props: {
  ending: Ending;
  ticked: () => boolean;
  tick: (ticked: boolean) => void;
}): JSX.Element {
  return (
    <section class={page.ending}>
      <label onClick={rowPress(() => props.tick(!props.ticked()))}>
        <input
          type="checkbox"
          id="nothing-else"
          name="nothing-else"
          checked={props.ticked()}
          onChange={(event) => props.tick(event.currentTarget.checked)}
        />
        <span class={page.endingName}>Nothing else</span>
      </label>
      <p class={page.semantics}>{ENDING[props.ending]}</p>
    </section>
  );
}

/// What ticking it will do, said by which ending it would bring about.
///
/// The box and the mark are the same either way — see `store::nothing_else` —
/// and what differs is where the Conversation goes afterwards: a follow-up ends
/// in the wrap-up, and an investigation ends where it was asked from with nothing
/// committed. A line promising a wrap-up under an investigation's round would be
/// offering a review of work nobody did.
const ENDING: Record<Ending, string> = {
  FollowUp:
    "Tick this when you want nothing more from this follow-up. Everything " +
    "you've written still goes back, the agent finishes the round, and the " +
    "conversation wraps up.",
  Investigation:
    "Tick this when you want nothing more found out. Everything you've written " +
    "still goes back, the agent finishes the round, and nothing it wrote is " +
    "committed.",
};

/// A Question or a Sub-question — both are asked the same way: the name it
/// answers to, its text, its Options as a radio group, then a free-text field
/// with a paperclip in it and whatever has been put on it under that.
///
/// The name is what a Response answers by (`Q7`, `Q7a`), so it names the fields
/// too — and the paperclip and the row of pills, there being one of each per
/// question and nothing else to tell them apart by.
///
/// **The whole of the question is what a file is dropped onto**, its text, its
/// Options and its field alike, the way the composer's whole box is: what the
/// human is dropping onto is the thing they are answering. It highlights while
/// a drag carrying files is over it. A Question that heads Sub-questions keeps
/// its own region and each of them keeps theirs, because a file names the
/// Answer it was put on and a card holding four of them could not say which.
function Asking(props: {
  ask: AskView;
  fields: Fields;
  files: Putting;
  additions: Picking;
  /// The rows its Attach menu holds under *Attach file…*.
  offering: (shut: () => void) => JSX.Element;
}): JSX.Element {
  const group = () => `${props.ask.name}-option`;
  const field = () => `${props.ask.name}-free-text`;
  const options = () => props.ask.options;

  // The attaching piece, one per question: the pills under the field, the
  // paperclip inside it, and the drop the question itself takes. What becomes
  // of a chosen file is the sheet's — see [`putting`] — and everything the
  // human touches is the composer's.
  const clip = attaching({
    shown: () => props.files.shown(props.ask.name),
    add: (chosen) => props.files.send(props.ask.name, chosen),
    onto: props.ask.name,
    chips: () => props.additions.chips(props.ask.name),
    offering: props.offering,
  });

  // With no Options the free text *is* the answer; with them it is whatever the
  // human has to say, which may stand instead of an Option or beside one. Hence
  // the neutral word: "Or in your own words" read as a choice between the two,
  // which was never what it meant.
  //
  // Named for the question it belongs to, because five fields prompted alike is
  // five fields nothing tells apart — which a screen reader has no way around at
  // all, and which is worth a reminder of where you are even when you can see
  // the whole page.
  const prompt = () =>
    `${props.ask.name} · ${
      options().length === 0 ? "Your answer" : "Your thoughts"
    }`;

  return (
    <div
      class={page.ask}
      classList={{ [styles.taking!]: clip.over() }}
      {...clip.dropping}
    >
      <AskText name={props.ask.name} html={props.ask.text_html} />
      {/* The Options, as a table where the agent declared the axes to compare
          them along and as the list they have always been where it did not. The
          declaration is the whole of what decides it: nothing is read off the
          Options themselves, so the two cannot be confused for one another. */}
      <Show when={options().length > 0}>
        <Show
          when={props.ask.columns.length > 0}
          fallback={
            <ul class={page.options}>
              <For each={options()}>
                {(option) => (
                  <Offered
                    option={option}
                    group={group()}
                    fields={props.fields}
                  />
                )}
              </For>
            </ul>
          }
        >
          <Tabulated ask={props.ask} group={group()} fields={props.fields} />
        </Show>
      </Show>
      {/* The prompt is the placeholder rather than a label above the field: one
          line of small print per question, times five questions, was more of the
          page spent saying what a text box is for than reading the Questions. It
          is the `aria-label` as well, in the same words, because a placeholder is
          not a label — it is a hint the browser is free to leave unspoken, and a
          field with nothing else naming it would reach a screen reader unnamed.

          The wrapper carries the text a second time, where the stylesheet uses
          it to give the field its height — see `.grow` in `App.module.css`. It
          is the field's own value, so a restored draft arrives at the right
          height rather than one line tall with the rest of it hidden. */}
      <div class={styles.writing}>
        <div
          class={`${app.grow} ${styles.field}`}
          data-value={props.fields.free_text()}
        >
          <textarea
            id={field()}
            name={field()}
            rows="1"
            placeholder={prompt()}
            aria-label={prompt()}
            value={props.fields.free_text()}
            onInput={(event) => props.fields.say(event.currentTarget.value)}
          />
        </div>
        {/* Inside the field's own padding at its far bottom corner, which on a
            field nothing has been typed into yet is the middle of it: an icon
            and nothing else, in the quiet ink everything that is not the
            answering is drawn in. */}
        <clip.Clip class={styles.attach} />
      </div>
      {/* And what has been put on this one, under the field it was put beside:
          the record's own pills, and a file still on its way up drawn dimmed at
          the end of them. */}
      <clip.Pills class={styles.attachments} />
      {/* What could not be attached, one line per file: a choice is several
          files and each of them was refused for its own reason. */}
      <For each={props.files.refusals(props.ask.name)}>
        {(said) => <ErrorLine class={styles.attachFailure}>{said}</ErrorLine>}
      </For>
      {/* And what could not be taken off again — one line for the row rather
          than one inside a pill, a pill being a name on a line with nowhere in
          it to say a sentence. */}
      <Show when={props.files.refusedRemoval(props.ask.name)}>
        {(said) => <ErrorLine class={styles.attachFailure}>{said()}</ErrorLine>}
      </Show>
      {/* And what could not be added or taken off at its menu. */}
      <Show when={props.additions.refused(props.ask.name)}>
        {(said) => <ErrorLine class={styles.attachFailure}>{said()}</ErrorLine>}
      </Show>
    </div>
  );
}

/// One Option on offer: a radio labelled by its number and text.
///
/// The Recommendation is marked and never selected — nothing is selected on
/// load, so an unread Recommendation cannot be submitted by accident. Clicking
/// the selected Option clears it, which puts the question back to unanswered and
/// so back into the warning before submit.
///
/// The row carries the press itself — see [`rowPress`], which is why.
function Offered(props: {
  option: OptionView;
  group: string;
  fields: Fields;
}): JSX.Element {
  const n = () => props.option.n;

  // The label wraps the radio: the whole row becomes the tap target, and the two
  // are associated without a `for` to keep in step with the id.
  //
  // The text is filled in wholesale, and it is inline markup all the way down —
  // anything blockier inside the label would end the row it is the tap target
  // for, so the rendering flattened it on the way here. It is marked as rendered
  // markdown all the same: what did survive, a code span above all, is drawn as
  // it is everywhere else.
  return (
    <li class={marks(props.option)}>
      <label onClick={rowPress(() => props.fields.pick(n()))}>
        <input
          type="radio"
          id={`${props.group}-${n()}`}
          name={props.group}
          value={n()}
          checked={props.fields.selected() === n()}
          // Both, because they answer different gestures. An arrow key moves the
          // selection and fires a change without ever firing a click; a click on
          // the radio itself — by pointer or by the space bar — is the other way
          // round when it lands on the Option already selected, the browser
          // firing no change because as far as it is concerned nothing changed.
          // Space is a click here too, which is what gives the keyboard the
          // clearing.
          //
          // The click runs before the change, so it still sees what the question
          // held before this gesture — which is the whole of how a second click
          // on the same Option is told from a first.
          //
          // A press that landed anywhere else in the row never reaches this one:
          // the row's own handler has it, and cancels what the label would have
          // forwarded here.
          onChange={() => props.fields.move(n())}
          onClick={() => props.fields.pick(n())}
        />
        <span class={page.n}>{n()}</span>
        <span
          class={`${page.optionText} markdown`}
          innerHTML={props.option.text_html}
        />
        <Show when={props.option.recommended}>
          <span class={page.star} title="The agent's recommendation">
            ★
          </span>
        </Show>
      </label>
    </li>
  );
}

/// The Options as the Answer Table the question declared: one row per Option,
/// compared across the axes the agent named.
///
/// The table *is* the choice rather than an illustration of one drawn above it,
/// so the row is what the human picks. The columns fixed around the agent's are
/// the record's too — see [`Head`].
function Tabulated(props: {
  ask: AskView;
  group: string;
  fields: Fields;
}): JSX.Element {
  const marked = () => starred(props.ask.options);

  return (
    <table class={page.answerTable}>
      <Head columns={props.ask.columns} starred={marked()} />
      <tbody>
        <For each={props.ask.options}>
          {(option) => (
            <Row
              option={option}
              group={props.group}
              fields={props.fields}
              starred={marked()}
            />
          )}
        </For>
      </tbody>
    </table>
  );
}

/// One Option as a row of the Answer Table: the same radio the list offers, in
/// the same group, with the row itself as the tap target.
///
/// A row cannot be wrapped in a label the way a list entry is, so the click sits
/// on the row and the radio is named by the cell holding its text — which is the
/// accessible name a wrapping label would have given it. The gestures are the
/// list's down to the handler they go through: a press anywhere in the row
/// selects or clears, a press on the radio is the radio's own, and an arrow key
/// only moves. See [`rowPress`], which is what makes the two shapes one
/// arrangement — here there is no label and so no forwarding to cancel, and
/// what it is here for is the focus and the link it stands out of.
function Row(props: {
  option: OptionView;
  group: string;
  fields: Fields;
  starred: boolean;
}): JSX.Element {
  const n = () => props.option.n;
  const naming = () => `${props.group}-${n()}-text`;

  return (
    <tr
      class={marks(props.option)}
      onClick={rowPress(() => props.fields.pick(n()))}
    >
      <td class={page.pick}>
        <input
          type="radio"
          id={`${props.group}-${n()}`}
          name={props.group}
          value={n()}
          checked={props.fields.selected() === n()}
          aria-labelledby={naming()}
          // Both, exactly as the list's radio answers both — see `Offered`,
          // which is the same two lines for the same two gestures. A press that
          // landed anywhere else in the row is the row's own and never reaches
          // here.
          onChange={() => props.fields.move(n())}
          onClick={() => props.fields.pick(n())}
        />
        <span class={page.n}>{n()}</span>
      </td>
      <td
        id={naming()}
        class={`${page.optionText} markdown`}
        innerHTML={props.option.text_html}
      />
      <For each={props.option.cells}>
        {(cell) => <td class="markdown" innerHTML={cell} />}
      </For>
      <Show when={props.starred}>
        <td class={page.starCell}>
          <Show when={props.option.recommended}>
            <span class={page.star} title="The agent's recommendation">
              ★
            </span>
          </Show>
        </td>
      </Show>
    </tr>
  );
}

/// What one Option on offer is marked as: an Option, and the one the agent
/// recommended. Shared by the list entry and the table row, because the mark is
/// the Option's rather than the shape's.
///
/// The record's `marks` in `Sheet.tsx` carries a third — what was chosen — which
/// only a settled Set has.
function marks(option: OptionView): string {
  return [page.option, option.recommended && page.recommended]
    .filter(Boolean)
    .join(" ");
}

/// What could not be put on an Answer, one sentence per way of being refused.
///
/// The Brief's own list said from the other page — see `ATTACH_REFUSAL` in
/// `workbench/Composer.tsx` — with the freeze in the other place: what fixes an
/// Answer's files is the Set settling, and each way that can have happened is
/// something different to know.
export const ANSWER_ATTACH_REFUSAL: Record<
  Exclude<AnswerAttached, { Attached: unknown }>,
  string
> = {
  NoSuchSet: "This question set is no longer here.",
  Answered: "This question set has been answered, so its files are settled.",
  Locked: "This question set was locked unanswered, so its files are settled.",
  Closed: "This conversation is closed.",
  NoSuchLabel: "This question set doesn't ask that question.",
  TooLarge: "That file is larger than 32 MB.",
  NotAName:
    "That name can't be a file here. Use no folders, and nothing starting with a dot.",
};

/// And taking one off again, which is refused by the same four.
export const ANSWER_ATTACHMENT_REMOVAL_REFUSAL: Record<
  AnswerAttachmentRemoved,
  string
> = {
  Removed: "",
  NoSuchSet: "This question set is no longer here.",
  Answered: "This question set has been answered, so its files are settled.",
  Locked: "This question set was locked unanswered, so its files are settled.",
  Closed: "This conversation is closed.",
};

/// One file this device is still sending, and the Answer it is going onto.
///
/// Its own key rather than its name, because two files chosen together may
/// share one — and what the key is for is taking the right one out of the row
/// when the right request lands.
/// Each keyed by the label of the Question it is under, or `null` at the
/// comment box, where a file is on the Set as a whole.
type Landing = { key: number; label: string | null; name: string };

/// Something said about one Answer's files, or the comment box's.
type Said = { label: string | null; said: string };

/// What the sheet does about the files on its Answers, held once for the page:
/// the row of pills each question hands the attaching piece, the requests a
/// choice and a × make, and what came back refused from either of them.
type Putting = {
  /// The row under one question, or under the comment box where `null`: what
  /// the record holds for it, and what this device is still sending after
  /// that.
  shown: (label: string | null) => Array<Shown>;

  /// Every question the record says a file was put on, which is what makes an
  /// entry carrying nothing else an Answer rather than a question left open.
  answered: () => string[];

  /// What could not be attached under one question, one line per file.
  refusals: (label: string | null) => string[];

  /// And what could not be taken off again, one line for its row.
  refusedRemoval: (label: string | null) => string | null;

  /// Send every file chosen under one question, each on a request of its own.
  send: (label: string | null, files: Array<File>) => void;
};

/// The whole of that, made once by the sheet.
///
/// The composer's own arrangement said again for a page holding a row per
/// question — see `sendingOn` in `workbench/Composer.tsx`, which this is the
/// answer sheet's half of, and which says why none of it is a mutation. What is
/// different here is only what a file is addressed by: the Set and the label of
/// the Question it answers, rather than the Conversation.
function putting(what: {
  set: () => number;

  /// The files the Set says are on its Answers, which is where every pill that
  /// is not still in flight comes from.
  attached: () => AttachmentView[];
}): Putting {
  const queries = useQueryClient();
  const device = useDevice();

  const [landing, setLanding] = createSignal<Array<Landing>>([]);
  const [refusals, setRefusals] = createSignal<Array<Said>>([]);
  const [refusedRemoval, setRefusedRemoval] = createSignal<Array<Said>>([]);

  // Which of the record's files have a removal in flight, by id: the one thing
  // a × can be truly disabled for is a press already made.
  const [removing, setRemoving] = createSignal<Array<number>>([]);

  let keys = 0;

  /// The record's own files under one label, in the order they were attached —
  /// the comment box's being the ones under none.
  const on = (label: string | null) =>
    what.attached().filter((attachment) => attachment.label === label);

  const send = (label: string | null, files: Array<File>) => {
    // A fresh choice clears what the last one under this question had to say:
    // the lines are about files the human has moved on from.
    setRefusals((said) => said.filter((one) => one.label !== label));

    for (const file of files) {
      const key = (keys += 1);
      setLanding((held) => [...held, { key, label, name: file.name }]);

      void (
        label === null
          ? attachToSet(device(), what.set(), file)
          : attachToAnswer(device(), what.set(), label, file)
      )
        .then(async (outcome: AnswerAttached) => {
          if (typeof outcome === "string") {
            setRefusals((said) => [
              ...said,
              { label, said: `${file.name}: ${ANSWER_ATTACH_REFUSAL[outcome]}` },
            ]);
            return;
          }

          // What came back is the record it made, and the pill is drawn from
          // the Set — so the read is both how the pill arrives and how it
          // arrives under the name the server renamed it to.
          //
          // Waited on, because the dimmed pill comes off the row in the
          // `finally` below and the pill that replaces it is the one this read
          // brings back: firing the read and carrying straight on would be the
          // file blinking out of the row and back into it.
          await queries.invalidateQueries({
            queryKey: keyOf(device(), "set"),
          });
        })
        .catch((error: unknown) => {
          setRefusals((said) => [
            ...said,
            {
              label,
              said: `${file.name} couldn't be attached. ${
                error instanceof Error ? error.message : String(error)
              }`,
            },
          ]);
        })
        .finally(() =>
          setLanding((held) => held.filter((one) => one.key !== key)),
        );
    }
  };

  /// And taking one off the record, which is the × on a pill.
  const forget = (attachment: AttachmentView) => {
    const label = attachment.label;
    setRemoving((was) => [...was, attachment.id]);

    void removeAnswerAttachment(device(), what.set(), attachment.id)
      .then((outcome: AnswerAttachmentRemoved) => {
        setRefusedRemoval((said) => [
          ...said.filter((one) => one.label !== label),
          ...(outcome === "Removed"
            ? []
            : [{ label, said: ANSWER_ATTACHMENT_REMOVAL_REFUSAL[outcome] }]),
        ]);

        // Either way: what came back is about a Set this page read a moment
        // ago, so reading it again is both the correction and — where the pill
        // is simply gone — the whole of what there was to do.
        void queries.invalidateQueries({ queryKey: keyOf(device(), "set") });
      })
      .catch((error: unknown) =>
        setRefusedRemoval((said) => [
          ...said.filter((one) => one.label !== label),
          {
            label,
            said: `${attachment.name} couldn't be removed. ${
              error instanceof Error ? error.message : String(error)
            }`,
          },
        ]),
      )
      .finally(() =>
        setRemoving((was) => was.filter((id) => id !== attachment.id)),
      );
  };

  const shown = (label: string | null): Array<Shown> => [
    ...on(label).map((attachment) => ({
      name: attachment.name,
      remove: () => forget(attachment),
      removing: removing().includes(attachment.id),
    })),
    ...landing()
      .filter((one) => one.label === label)
      .map((one) => ({ name: one.name, landing: true })),
  ];

  /// The record's word on which questions have a file, which is what the submit
  /// counts as an Answer. A file still on its way up is not among them: what
  /// the server will count is its own rows, and a pill that has not landed is
  /// not one of those yet.
  const answered = () =>
    what
      .attached()
      .map((attachment) => attachment.label)
      .filter((label): label is string => label !== null);

  const said = (held: Array<Said>, label: string | null) =>
    held.filter((one) => one.label === label).map((one) => one.said);

  return {
    shown,
    answered,
    refusals: (label) => said(refusals(), label),
    refusedRemoval: (label) => said(refusedRemoval(), label)[0] ?? null,
    send,
  };
}

/// What could not be picked on a Set or taken off it, one sentence per way of
/// being refused — the files' own, with the two only a server meets.
export const ADDITION_REFUSAL: Record<
  Exclude<AdditionMade | AdditionRemoved, { Added: unknown } | "Removed">,
  string
> = {
  NoSuchSet: "This question set is no longer here.",
  Answered: "This question set has been answered, so what it adds is settled.",
  Locked: "This question set was locked unanswered, so what it adds is settled.",
  Closed: "This conversation is closed.",
  NoSuchLabel: "This question set doesn't ask that question.",
  NoSuchServer: "That server is no longer declared in the settings.",
  AlreadyOn: "This conversation has that already.",
  NoSuchRepo: "That repo is no longer registered.",
  NotACompanion: "That repo isn't alongside this conversation, so there's nothing to open up.",
  NotAMember: "That device isn't linked to this one any more.",
  WorkIsHere: "The work is on that device already.",
};

/// What the sheet does about what is picked at its menus, held once for the
/// page: the chips under each Answer and the comment box, the two presses, and
/// one line apiece of what came back refused.
type Picking = {
  /// The chips under one question, or under the comment box where `null`.
  chips: (label: string | null) => Array<Chip>;

  /// Every server picked on the Set wherever it was picked, which the menus
  /// leave out: a Set adds a server once.
  servers: () => string[];

  /// And every repo, added or opened up, and every device, for the same reason.
  repos: () => number[];
  devices: () => string[];

  /// Pick something at one question's menu, or at the comment box's, with
  /// what a failure to reach the server calls it — and, where it is not the
  /// label's own, the place a refusal of it is said.
  pick: (
    label: string | null,
    pick: Picked,
    called: string,
    at?: Place,
  ) => void;

  /// Take a device picked on the set off it again, wherever it was picked,
  /// saying a refusal at `at`.
  untick: (device: string, at: Place) => void;

  /// And what could not be done there.
  refused: (at: Place) => string | null;
};

/// Where a refusal is said: under one question, at the comment box where
/// `null`, or under the direction chooser's ticks — which pick under no label,
/// as the comment box does, and say what came back under themselves.
export const CHOOSER: unique symbol = Symbol("chooser");
type Place = string | null | typeof CHOOSER;

/// The whole of that, made once by the sheet.
///
/// [`putting`]'s arrangement for a reference rather than bytes: each press is
/// its own request with its own answer, and the chips are drawn off the Set,
/// so reading it again is how one arrives and how one goes.
function picking(what: {
  set: () => number;

  /// What the Set says has been picked on it.
  picked: () => AdditionView[];

  /// What a read-write branch picked with no name comes to.
  mirroring: () => string;

  /// The devices the direction chooser draws a tick for, whose additions the
  /// comment box draws no chip for: one device, shown once.
  ticking: () => string[];
}): Picking {
  const queries = useQueryClient();
  const device = useDevice();

  const [refusals, setRefusals] = createSignal<
    Array<{ at: Place; said: string }>
  >([]);

  // Which picks have a press in flight, by what was picked for a pick and by
  // the row's id for a ×: the one thing either can be truly disabled for.
  const [working, setWorking] = createSignal<Array<string | number>>([]);

  const say = (at: Place, said: string | null) =>
    setRefusals((held) => [
      ...held.filter((one) => one.at !== at),
      ...(said === null ? [] : [{ at, said } as { at: Place; said: string }]),
    ]);

  const settle = (
    at: Place,
    key: string | number,
    made: Promise<AdditionMade | AdditionRemoved>,
    failed: string,
  ) => {
    setWorking((was) => [...was, key]);

    void made
      .then(async (outcome) => {
        say(
          at,
          typeof outcome !== "string" || outcome === "Removed"
            ? null
            : ADDITION_REFUSAL[outcome],
        );

        await queries.invalidateQueries({ queryKey: keyOf(device(), "set") });
      })
      .catch((error: unknown) =>
        say(
          at,
          `${failed} ${error instanceof Error ? error.message : String(error)}`,
        ),
      )
      .finally(() => setWorking((was) => was.filter((one) => one !== key)));
  };

  const pick = (
    label: string | null,
    picked: Picked,
    called: string,
    at: Place = label,
  ) =>
    settle(
      at,
      JSON.stringify(picked),
      addToSet(device(), what.set(), { label, pick: picked }),
      `${called} couldn't be added.`,
    );

  const forget = (addition: AdditionView, at: Place = addition.label) =>
    settle(
      at,
      addition.id,
      removeFromSet(device(), what.set(), addition.id),
      "That couldn't be removed.",
    );

  const untick = (ticked: string, at: Place) => {
    const addition = what
      .picked()
      .find(
        (addition) =>
          addition.state === "Pending" &&
          "Device" in addition.added &&
          addition.added.Device.device === ticked,
      );

    if (addition !== undefined) forget(addition, at);
  };

  /// Whether the direction chooser draws this addition as a tick, which only a
  /// device picked under no label can be.
  const ticked = (addition: AdditionView) =>
    addition.label === null &&
    "Device" in addition.added &&
    what.ticking().includes(addition.added.Device.device);

  const chips = (label: string | null): Array<Chip> =>
    what
      .picked()
      .filter((addition) => addition.label === label && !ticked(addition))
      .map((addition) =>
        // Struck as it crossed on a transfer: drawn as the record has it, with
        // nothing to take off.
        addition.state === "Struck"
          ? { ...chipped(addition, what.mirroring()), struck: true }
          : {
              ...chipped(addition, what.mirroring()),
              remove: () => forget(addition),
              removing: working().includes(addition.id),
            },
      );

  const servers = () =>
    what
      .picked()
      .flatMap((addition) =>
        "Server" in addition.added ? [addition.added.Server.name] : [],
      );

  const repos = () =>
    what.picked().flatMap((addition) => {
      if ("Companion" in addition.added) return [addition.added.Companion.repo_id];
      if ("OpenedUp" in addition.added) return [addition.added.OpenedUp.repo_id];
      return [];
    });

  const devices = () =>
    what
      .picked()
      .flatMap((addition) =>
        "Device" in addition.added ? [addition.added.Device.device] : [],
      );

  return {
    chips,
    servers,
    repos,
    devices,
    pick,
    untick,
    refused: (at) => refusals().find((one) => one.at === at)?.said ?? null,
  };
}

/// What one addition's chip says, whatever kind it is: the name, and for a repo
/// how far in the work reaches and on which branch — `mirroring` being what a
/// branch picked with no name comes to.
///
/// A name the record could not put to it any more — a repo unregistered, a
/// device unlinked since the pick — reads as gone, under the id it was picked
/// by, which is the whole of what is left to say about it.
export function chipped(
  addition: AdditionView,
  mirroring: string,
): Pick<Chip, "name" | "kind" | "detail" | "gone"> {
  const added = addition.added;

  if ("Server" in added) {
    return { name: added.Server.name, gone: !added.Server.declared };
  }

  if ("Companion" in added) {
    const { name, repo_id, mode, branch } = added.Companion;

    return {
      kind: "repo",
      name: name ?? `repo ${repo_id}`,
      detail:
        mode === "ReadOnly"
          ? "read-only"
          : `read-write on ${branch ?? mirroring}`,
      gone: name === null,
    };
  }

  if ("OpenedUp" in added) {
    const { name, repo_id, branch } = added.OpenedUp;

    return {
      kind: "repo",
      name: name ?? `repo ${repo_id}`,
      detail: `opened up on ${branch ?? mirroring}`,
      gone: name === null,
    };
  }

  // A repo picked on the device the work was moved from that none here
  // matches, which is why it is struck.
  if ("Unmatched" in added) {
    return {
      kind: "repo",
      name: added.Unmatched.name,
      detail: "not on this device",
    };
  }

  return {
    kind: "device",
    name: added.Device.name ?? added.Device.device.slice(0, 8),
    gone: added.Device.name === null,
  };
}

/// The rows of an Attach menu past the servers (ADR-0028): each registered
/// repo the work could have alongside it and each read-only companion it
/// could open up, under *Repos alongside*; then each device of the cluster the
/// work could be permitted to move to, under *May be transferred to*.
///
/// What the conversation has already is left out by the server, and what this
/// set has had picked is left out here: a set adds each once. A group with
/// nothing to offer is not drawn, which is every sheet with no cluster under
/// the second.
///
/// A repo's row opens a card, so it carries the ellipsis; a device's acts at
/// once. Only registered repos: registering is done at a desk, so the repo
/// dropdown's *Open repo…* and *Create repo…* are not here.
function AddableRows(props: {
  addable: AddableView;
  /// The repos and devices picked on the set already.
  repos: number[];
  devices: string[];
  join: (repo: RepoEntry, opening: boolean) => void;
  permit: (device: string, name: string) => void;
}): JSX.Element {
  const adding = () =>
    props.addable.repos.filter((repo) => !props.repos.includes(repo.id));
  const opening = () =>
    props.addable.read_only.filter(
      (companion) => !props.repos.includes(companion.repo.id),
    );
  const devices = () =>
    props.addable.devices.filter(
      (device) => !props.devices.includes(device.device),
    );

  const repos = createUniqueId();
  const moving = createUniqueId();

  return (
    <>
      <Show when={adding().length + opening().length > 0}>
        <div role="group" aria-labelledby={repos} class={styles.adding}>
          <div id={repos} role="presentation" class={styles.addingHead}>
            Repos alongside
          </div>
          <For each={adding()}>
            {(repo) => (
              <button
                type="button"
                role="menuitem"
                onClick={() => props.join(repo, false)}
              >
                {repo.name}…
              </button>
            )}
          </For>
          <For each={opening()}>
            {(companion) => (
              <button
                type="button"
                role="menuitem"
                onClick={() => props.join(companion.repo, true)}
              >
                Open up {companion.repo.name}…
              </button>
            )}
          </For>
        </div>
      </Show>

      <Show when={devices().length > 0}>
        <div role="group" aria-labelledby={moving} class={styles.adding}>
          <div id={moving} role="presentation" class={styles.addingHead}>
            May be transferred to
          </div>
          <For each={devices()}>
            {(device) => (
              <button
                type="button"
                role="menuitem"
                onClick={() => props.permit(device.device, device.name)}
              >
                {device.name}
              </button>
            )}
          </For>
        </div>
      </Show>
    </>
  );
}

/// Every question of the Set in the order it was asked, Sub-questions under the
/// Question that asked them — which is the order a Response accounts for them
/// in.
function flatten(questions: QuestionView[]): Asked[] {
  return questions.flatMap((question) =>
    [
      // A Heading asks nothing of its own, so the sheet holds no field for it
      // and the Response carries no entry: what there is to answer is the
      // Sub-questions under it. Left in, it was a blank field the human had
      // nothing to put in, coming back marked Unanswered — which tells the
      // agent a decision is still open when none was ever put.
      ...(question.heading ? [] : [question.ask]),
      ...question.subquestions,
    ].map((ask) => ({
      label: ask.name,
      multipleChoice: ask.options.length > 0,
    })),
  );
}

/// An untouched question.
function blank(ask: Asked): Filled {
  return { label: ask.label, selected: null, free_text: "" };
}

/// Why a repo picked on the set couldn't go in as it was picked, in the
/// Steer's terms but not its words: nothing was steered here.
const COMPANION_REFUSAL: Record<Extract<SteerCompanionRefusal, string>, string> = {
  OwnRepo: "It's this conversation's own repo.",
  AlreadyAdded: "It's already on this conversation.",
  NotACompanion: "It isn't on this conversation, so there's nothing to open up.",
  AlreadyReadWrite: "It's already read-write on this conversation.",
  FetchFailed: "Git couldn't fetch from its remote. The server log says why.",
  NoBaseCommit: "Its base has nothing to check out.",
  BranchExists: "The branch already exists there, and Verkstead didn't make it.",
  WorktreeRefused: "Git wouldn't make its worktree. The server log says why.",
};

/// Why the Response did not land, when it did not. A Response that was taken
/// says nothing here — the page is already on its way back to the list.
function refused(outcome: Submitted | undefined, error: Error | null): string | null {
  if (error !== null) {
    // The server's own wording where there is one, which is what the human has
    // to be shown — see `ApiError`.
    return `The response didn't get through. ${error.message}`;
  }

  if (outcome === undefined || outcome === "Accepted") {
    return null;
  }

  if (outcome === "AlreadyAnswered") {
    return "This question set was already answered. The first response stands, so yours wasn't stored.";
  }
  if (outcome === "NoSuchSet") {
    return "This question set is no longer here.";
  }
  if (outcome === "Locked") {
    return "This question set was locked unanswered, so your response wasn't stored.";
  }

  if ("ServerGone" in outcome) {
    return `The MCP server ${outcome.ServerGone} is no longer declared, so your response wasn't stored. Remove it, or declare it again in settings.`;
  }
  if ("RepoGone" in outcome) {
    return `The ${outcome.RepoGone} repo is no longer registered, so your response wasn't stored. Remove it, or register it again.`;
  }
  if ("Companion" in outcome) {
    const { repo, why } = outcome.Companion;
    const reason =
      typeof why === "object"
        ? otherAuthor({ companion: repo, ...why.OtherAuthor })
        : COMPANION_REFUSAL[why];
    return `The ${repo} repo can't be added, so your response wasn't stored. ${reason} Remove it to send your answers without it.`;
  }

  return `This response doesn't resolve the question set: ${outcome.Rejected.join("; ")}`;
}
