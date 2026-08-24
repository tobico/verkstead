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

import { useMutation, useQueryClient } from "@tanstack/solid-query";
import type { JSX } from "solid-js";
import { For, Show, createEffect, createMemo, createSignal } from "solid-js";
import { createStore } from "solid-js/store";

import { Modal } from "../Modal";
import { submitResponse } from "../api/client";
import type {
  AskView,
  Direction,
  OptionView,
  ProposalView,
  QuestionView,
  Response as Decided,
  Submitted,
} from "../api/types";
import {
  DIRECTION,
  DIRECTION_LABEL,
  DIRECTION_NOTE,
  DIRECTIONS,
} from "../directions";
import { AskText } from "./AskText";
import { Postscript } from "./Postscript";
import { anchor } from "./outline";
import { Head, starred } from "./table";
import {
  clearDraft,
  clicked,
  drafted,
  keepDraft,
  storedDraft,
  unanswered,
} from "./sheet";
import type { Filled } from "./sheet";

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
    };

    if (settled()) {
      return;
    }

    keepDraft(props.id, draft);
  });

  /// What the sheet says right now, as a Response.
  const response = (): Decided =>
    drafted(sheet.filled, sheet.comment, sheet.direction);

  const queries = useQueryClient();

  const submit = useMutation(() => ({
    mutationFn: (sending: Decided) => submitResponse(props.id, sending),
    onSuccess: (outcome: Submitted) => {
      if (typeof outcome !== "string") {
        // Rejected by the grammar. The page builds Responses that resolve the
        // Set, so that is a bug here rather than anything the human did — and
        // their draft stands, because it is the only copy of what they wrote.
        return;
      }

      // Either it was taken or this Set can take nothing: both ways the sheet
      // is spent.
      setSettled(true);
      clearDraft(props.id);

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
      <h2 class="section-heading" id="questions">
        Questions
      </h2>
      <ol class="questions">
        <For each={props.questions}>
          {(question, index) => (
            <li class="question" id={anchor(question.ask.name, index() + 1)}>
              {/* A Heading is its text and nothing else — no Options, no field,
                  nothing to leave open. What it heads is directly under it. */}
              <Show
                when={!question.heading}
                fallback={
                  <div class="ask heading">
                    <AskText
                      name={question.ask.name}
                      html={question.ask.text_html}
                    />
                  </div>
                }
              >
                <Asking ask={question.ask} fields={fields(question.ask.name)} />
              </Show>
              {/* Sub-questions get no anchor of their own: one scrolls into
                  view with its parent. */}
              <Show when={question.subquestions.length > 0}>
                <ol class="subquestions">
                  <For each={question.subquestions}>
                    {(subquestion) => (
                      <li class="subquestion">
                        <Asking
                          ask={subquestion}
                          fields={fields(subquestion.name)}
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
        <section class="set-comment">
          <div class="grow" data-value={sheet.comment}>
            <textarea
              id="set-comment"
              name="set-comment"
              rows="1"
              placeholder="Other comments"
              aria-label="Other comments"
              value={sheet.comment}
              onInput={(event) => setSheet("comment", event.currentTarget.value)}
            />
          </div>
        </section>
      </Postscript>
      <section class="submit">
        <button type="button" onClick={start} disabled={submit.isPending}>
          {submit.isPending ? "Sending…" : "Submit"}
        </button>
        <Show when={failed()}>{(said) => <p class="error">{said()}</p>}</Show>
      </section>
      {/* The warning that stands between the human and a submit skipping
          offered choices: every multiple-choice question left open, by name, and
          the choice to go back. A skipped free-text question passes without a
          word — nothing was offered, so nothing was overlooked. It warns and
          never blocks: leaving the whole Set open with only a comment is a
          counter-question, not a mistake, and it comes through here like any
          other. */}
      <Modal
        class="confirm"
        open={confirming() !== null}
        close={() => setConfirming(null)}
        labelledBy="confirm-title"
      >
        <p id="confirm-title">Going back unanswered:</p>
        <ul class="unanswered">
          <For each={confirming() ?? []}>{(name) => <li>{name}</li>}</For>
        </ul>
        <p class="note">The agent will be told these are still open.</p>
        <div class="confirm-actions">
          <button
            type="button"
            class="secondary"
            onClick={() => setConfirming(null)}
          >
            Keep answering
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
  picked: () => Direction | null;
  /// A click, which picks the direction — or clears the choice, when it landed
  /// on the direction already picked.
  pick: (direction: Direction) => void;
  /// An arrow key, which only ever moves the choice.
  move: (direction: Direction) => void;
}): JSX.Element {
  return (
    <section class="direction-pick" id="direction">
      {/* Headed like the Preface and the Postscript, and holding one card like
          the Questions do: the section names what is being decided and the card
          is the deciding, so this reads as one more question under a heading of
          its own rather than as furniture of a different kind. */}
      <h2 class="section-heading">Direction</h2>
      <div class="direction-card">
        {/* Asked as a Question is asked: the label floated in the accent with
            the agent's argument running beside it, and the three to pick from
            under it where a Question's Options are. */}
        <div class="ask">
          <AskText
            name={DIRECTION_LABEL}
            html={props.proposal.rationale_html}
          />
        </div>
        <ul class="directions">
          <For each={DIRECTIONS}>
            {(offered) => {
              const recommended = () => props.proposal.direction === offered;

              return (
                <li
                  class="direction"
                  classList={{ recommended: recommended() }}
                >
                  <label>
                    <input
                      type="radio"
                      id={`direction-${offered}`}
                      name="direction"
                      value={offered}
                      checked={props.picked() === offered}
                      // Both gestures, for the reason an Option answers
                      // both: an arrow key fires a change and never a click,
                      // and a click on what is already picked fires a click
                      // and never a change.
                      onChange={() => props.move(offered)}
                      onClick={() => props.pick(offered)}
                    />
                    <span class="direction-name">{DIRECTION[offered]}</span>
                    <Show when={recommended()}>
                      <span class="star" title="the agent's Recommendation">
                        ★
                      </span>
                    </Show>
                  </label>
                  <p class="note">{DIRECTION_NOTE[offered]}</p>
                </li>
              );
            }}
          </For>
        </ul>
        <p class="semantics">
          Picking a direction accepts the proposal and lets the agent get on
          with it. Anything else — an answer of your own, questions left open,
          nothing picked here — sends it back for another round.
        </p>
      </div>
    </section>
  );
}

/// A Question or a Sub-question — both are asked the same way: the name it
/// answers to, its text, its Options as a radio group, then a free-text field.
///
/// The name is what a Response answers by (`Q7`, `Q7a`), so it names the fields
/// too.
function Asking(props: { ask: AskView; fields: Fields }): JSX.Element {
  const group = () => `${props.ask.name}-option`;
  const field = () => `${props.ask.name}-free-text`;
  const options = () => props.ask.options;

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
    `${props.ask.name} — ${
      options().length === 0 ? "Your answer" : "Your thoughts"
    }`;

  return (
    <div class="ask">
      <AskText name={props.ask.name} html={props.ask.text_html} />
      {/* The Options, as a table where the agent declared the axes to compare
          them along and as the list they have always been where it did not. The
          declaration is the whole of what decides it: nothing is read off the
          Options themselves, so the two cannot be confused for one another. */}
      <Show when={options().length > 0}>
        <Show
          when={props.ask.columns.length > 0}
          fallback={
            <ul class="options">
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
          it to give the field its height — see `.grow`. It is the field's own
          value, so a restored draft arrives at the right height rather than one
          line tall with the rest of it hidden. */}
      <div class="grow" data-value={props.fields.free_text()}>
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
    </div>
  );
}

/// One Option on offer: a radio labelled by its number and text.
///
/// The Recommendation is marked and never selected — nothing is selected on
/// load, so an unread Recommendation cannot be submitted by accident. Clicking
/// the selected Option clears it, which puts the question back to unanswered and
/// so back into the warning before submit.
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
    <li class={props.option.recommended ? "option recommended" : "option"}>
      <label>
        <input
          type="radio"
          id={`${props.group}-${n()}`}
          name={props.group}
          value={n()}
          checked={props.fields.selected() === n()}
          // Both, because they answer different gestures. An arrow key moves the
          // selection and fires a change without ever firing a click; a click on
          // the Option already selected is the other way round — the browser
          // fires no change, because as far as it is concerned nothing changed.
          // Space is a click here too, which is what gives the keyboard the
          // clearing.
          //
          // The click runs before the change, so it still sees what the question
          // held before this gesture — which is the whole of how a second click
          // on the same Option is told from a first.
          onChange={() => props.fields.move(n())}
          onClick={() => props.fields.pick(n())}
        />
        <span class="n">{n()}</span>
        <span
          class="option-text markdown"
          innerHTML={props.option.text_html}
        />
        <Show when={props.option.recommended}>
          <span class="star" title="the agent's Recommendation">
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
    <table class="answer-table">
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
/// accessible name a wrapping label would have given it. A click on the radio
/// bubbles to the row like any other, so both reach the same handler exactly
/// once, and the gestures are the list's: a click selects or clears, an arrow
/// key only moves.
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
      class={props.option.recommended ? "option recommended" : "option"}
      onClick={() => props.fields.pick(n())}
    >
      <td class="pick">
        <input
          type="radio"
          id={`${props.group}-${n()}`}
          name={props.group}
          value={n()}
          checked={props.fields.selected() === n()}
          aria-labelledby={naming()}
          // The click is the row's, so only the arrow key's change is answered
          // here — see `Offered` for what the two gestures are between them.
          onChange={() => props.fields.move(n())}
        />
        <span class="n">{n()}</span>
      </td>
      <td
        id={naming()}
        class="option-text markdown"
        innerHTML={props.option.text_html}
      />
      <For each={props.option.cells}>
        {(cell) => <td class="markdown" innerHTML={cell} />}
      </For>
      <Show when={props.starred}>
        <td class="star-cell">
          <Show when={props.option.recommended}>
            <span class="star" title="the agent's Recommendation">
              ★
            </span>
          </Show>
        </td>
      </Show>
    </tr>
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

/// Why the Response did not land, when it did not. A Response that was taken
/// says nothing here — the page is already on its way back to the list.
function refused(outcome: Submitted | undefined, error: Error | null): string | null {
  if (error !== null) {
    // The server's own wording where there is one, which is what the human has
    // to be shown — see `ApiError`.
    return `The Response did not get through: ${error.message}`;
  }

  if (outcome === undefined || outcome === "Accepted") {
    return null;
  }

  if (outcome === "AlreadyAnswered") {
    return "This Set had already been answered. The first Response stands, so yours was not stored.";
  }
  if (outcome === "NoSuchSet") {
    return "This Set is no longer here.";
  }
  if (outcome === "Archived") {
    return "This Set was archived unanswered, which closed it for good, so your Response was not stored.";
  }

  return `This Response does not resolve the Set: ${outcome.Rejected.join("; ")}`;
}
