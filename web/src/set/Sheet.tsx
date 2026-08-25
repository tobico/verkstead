//! One Question Set laid out to be answered — its Preface, the Diff that is the
//! evidence for it, and then every Question and Sub-question in the order the
//! agent asked them.
//!
//! A Set that has settled gets the same sheet read rather than filled in: its own
//! material above, and under it what was decided — the Option chosen beside the
//! one the agent recommended, whatever was written, and the questions that went
//! back open. A Set is answered once, so there is nothing here to press. Which of
//! the two is drawn is decided from the Set as the server loads it, so an
//! answered Set never flashes a form.
//!
//! A Set that was archived reads like an answered one — permanently, and with
//! nothing to press — except that there is no Response to show, because there
//! never was one.
//!
//! Its own module rather than the page's, because a Set is reached two ways: as
//! a page of its own, and as the details pane of the Timeline Event it landed on
//! — see `workbench/Asked`. The rendering is the same either way, and a second
//! copy of it would be a second reading of one decision.

import type { JSX } from "solid-js";
import { For, Show, createMemo, createSignal, onCleanup, onMount } from "solid-js";

import app from "../App.module.css";
import type {
  Answer,
  AskView,
  Direction,
  Liveness,
  OptionView,
  ProposalView,
  QuestionView,
  Response,
  SetView,
} from "../api/types";
import { setWrapping, wrapping } from "../device";
import { DIRECTION, DIRECTION_LABEL, DIRECTIONS } from "../directions";
import { Answering } from "./Answering";
import { AskText } from "./AskText";
import { Contents, PageHeader, navigation } from "./Contents";
import { Diff } from "./Diff";
import { Postscript } from "./Postscript";
import styles from "./Sheet.module.css";
import { Standing } from "./Standing";
import { drawDiagrams } from "./diagrams";
import { anchor, outline, spied } from "./outline";
import { Head, starred } from "./table";
import { settledAge, utcStamp } from "./when";


/// One Set, top to bottom: how it stands and its own material — what the agent
/// asked about and the evidence for it — and then the record of what became of
/// it.
///
/// The material above is the same however it stands: a settled Set is read for
/// what was decided *and* for what the decision was about.
///
/// `lead` is whatever the sheet is reached through — the way back to the list it
/// is on, or the pane header of the Timeline Event it belongs to. Nothing is
/// drawn where there is none.
///
/// `contents` is where the table of contents is being drawn, and what decides
/// which width it picks its shape from: the window on a page, and the pane's own
/// width in a details pane — where the 60rem cap leaves a margin of its own for
/// the sidebar to stand in. `"none"` is a sheet drawn without one at all.
///
/// The floating header belongs to the page alone. It names where the reader is
/// across the top of the column, and a pane already has a header of its own
/// there.
export function Sheet(props: {
  set: SetView;
  lead?: JSX.Element;
  contents?: "page" | "pane" | "none";
}): JSX.Element {
  // The renderer, named by a Set that has a Diagram on it to draw and by no
  // other: mermaid is megabytes, so a Set without one loads none of them. What a
  // page that does ask for it gets is the source blocks the markdown renderer
  // already wrote, drawn over — and if the bundle never arrives, the blocks
  // themselves, which is a readable page and not a broken one.
  //
  // Once, on mount, rather than from an effect over the Set: whether this page
  // carries a renderer is a fact about the Set it is drawing, and drawing the
  // same one again when the Set is fetched afresh would find every block already
  // replaced.
  onMount(() => {
    if (!props.set.diagrams) {
      return;
    }

    onCleanup(drawDiagrams());
  });

  // How this device wants Diffs drawn. It lives up here because two controls
  // drive it — the switch beside the Diff heading and the one in the floating
  // header — and they have to be two views of one setting rather than two
  // settings that happen to agree.
  //
  // Read from the device as the page is built rather than from an effect
  // afterwards: there is no server-rendered first paint to disagree with any
  // more, so a reader who asked for wrapping gets it without the Diff arriving
  // unwrapped and reflowing into it.
  const [wrapped, setWrapped] = createSignal(wrapping());

  /// Turn wrapping on or off for this device, and for every Diff it opens after
  /// this one.
  const flip = (on: boolean) => {
    setWrapped(on);
    setWrapping(on);
  };

  // Both are descriptions of the whole Set, and both come from the one outline,
  // so the sidebar, the floating header and the scroll-spy cannot end up
  // describing different pages.
  const sections = createMemo(() => outline(props.set));
  const watched = createMemo(() => spied(sections()));

  const nav = navigation();

  const standing = () => props.set.standing;

  /// Whether anyone is still on the other end — and `null` once the Set has
  /// settled, which is what decides whether this page is a sheet to fill in or a
  /// record to read.
  const waiting = (): Liveness | null => {
    const how = standing();
    return "Waiting" in how ? how.Waiting : null;
  };

  /// Whether this Set has settled, one way or the other.
  const decided = () => waiting() === null;

  /// The Response behind the record, and `null` when there is none: a Set closed
  /// unanswered is a record with nothing decided in it.
  const response = (): Response | null => {
    const how = standing();
    return "Answered" in how ? how.Answered.response : null;
  };

  // The clock the settling is aged against, ticking once a minute: an age is a
  // distance from now, and a page left open should not go on saying "2m ago"
  // an hour later. A minute because that is the roughest unit the wording ever
  // moves by.
  const [now, setNow] = createSignal(Date.now());
  const ticking = setInterval(() => setNow(Date.now()), 60_000);
  onCleanup(() => clearInterval(ticking));

  /// When the Set settled, said in words, and what to call the settling. On the
  /// provenance line rather than down with the Answers: on a settled Set, when
  /// it was settled is part of knowing what one is reading — and for one that
  /// was closed unanswered it is most of what there is to know. The exact
  /// minute rides behind the words, as the tooltip.
  const when = () => {
    const how = standing();

    if ("Answered" in how) {
      return {
        // A class of its own for each: the two sit in the same place and are
        // styled together, but nothing about an archived Set was answered.
        mark: styles.answeredAt!,
        said: `Answered ${settledAge(how.Answered.submitted_at, now())}`,
        stamp: utcStamp(how.Answered.submitted_at),
      };
    }

    if ("ArchivedUnanswered" in how) {
      return {
        mark: styles.archivedAt!,
        said: `Archived unanswered ${settledAge(how.ArchivedUnanswered, now())}`,
        stamp: utcStamp(how.ArchivedUnanswered),
      };
    }

    return null;
  };

  /// Where the table of contents is drawn — the page by default, because the
  /// page is what a Set is usually read as.
  const where = () => props.contents ?? "page";

  /// Whether it is drawn at all.
  const listed = () => where() !== "none";

  return (
    <>
      {props.lead}
      <h1>{props.set.title}</h1>
      {/* After the title and before the rest: the page says what it is, then
          what is in it. It is taken out of the flow and put in the margin by
          the stylesheet, so where it sits here is a reading order rather than
          a position. */}
      <Show when={listed()}>
        <Contents
          sections={sections()}
          watched={watched()}
          nav={nav}
          paned={where() === "pane"}
        />
        {/* Under the nav in reading order and pinned to the top edge by the
            stylesheet, which is also what keeps it off a narrow viewport: there
            the nav's own bar is already doing this job. And off a pane, which
            has the pane header across its top already. */}
        <Show when={where() === "page"}>
          <PageHeader
            watched={watched()}
            nav={nav}
            wrapped={wrapped()}
            flip={flip}
          />
        </Show>
      </Show>
      {/* One line about the Set rather than from it: where it came from at the
          near end, and how it stands at the far end — the date it settled, or
          the badge and the menu for a Set still waiting. Never empty, because
          every Set stands somewhere, even one sent from outside a repo with no
          provenance to name. A `div` because the far end holds controls, which
          are more than a paragraph may contain. */}
      <div class={styles.meta}>
        <Show when={props.set.project}>
          {(project) => <span class={styles.project}>{project()}</span>}
        </Show>
        <Show when={props.set.branch}>
          {(branch) => <span class={styles.branch}>{branch()}</span>}
        </Show>
        <Show when={when()}>
          {(when) => (
            <span class={when().mark} title={when().stamp}>
              {when().said}
            </span>
          )}
        </Show>
        {/* Whether anyone is still on the other end, and the one thing to do
            about it if nobody is. Up here because both are about the ask rather
            than about answering it — and because archiving is decided with the
            badge and the Questions in view, not from a list row. */}
        <Show when={waiting()}>
          {(liveness) => <Standing id={props.set.id} liveness={liveness()} />}
        </Show>
      </div>
      {/* Named and anchored like the Questions below it: the heading is what a
          jump from the table of contents lands on, and the id is what it jumps
          to.

          The body is marked as rendered markdown, so the agent's headings,
          tables and code get the same rules there as they get inside a Question
          — the section around it is all that is the Preface's own. */}
      <Show when={props.set.preface_html}>
        {(html) => (
          <section class={styles.preface} id="preface">
            <h2 class={app.sectionHeading}>Preface</h2>
            <div class={`${styles.prefaceBody} markdown`} innerHTML={html()} />
          </section>
        )}
      </Show>
      {/* Between the Preface and the Questions: the Preface says what the agent
          is asking about, and the Diff is the evidence for it. */}
      <Show when={props.set.diff}>
        {(diff) => <Diff diff={diff()} wrapped={wrapped()} flip={flip} />}
      </Show>
      <Show
        when={decided()}
        fallback={
          <Answering
            id={props.set.id}
            questions={props.set.questions}
            postscript={props.set.postscript_html}
            proposal={props.set.proposal}
          />
        }
      >
        <Questions
          questions={props.set.questions}
          response={response()}
          postscript={props.set.postscript_html}
          proposal={props.set.proposal}
        />
      </Show>
    </>
  );
}

/// The questions of a settled Set, and what became of them.
///
/// The heading is drawn unconditionally, unlike the Preface's: the Questions are
/// the one section every Set has. Its id sits on the heading rather than on the
/// list, so a jump lands on the name of the thing rather than just above its
/// first row. A Set still waiting is asked on the sheet instead — see
/// [`Answering`] — which draws the same heading and the same anchors.
function Questions(props: {
  questions: QuestionView[];
  response: Response | null;
  postscript: string | null;
  proposal: ProposalView | null;
}): JSX.Element {
  /// A Set that settled with no Response behind it, which is the one standing
  /// that was never answered by anybody.
  const orphaned = () => props.response === null;

  /// What to say at the head of a Response that resolved nothing.
  const nothing = () => {
    const response = props.response;
    return response === null ? null : nothingAnswered(response);
  };

  /// Shown only when there is one, exactly as the submit only ever sends one that
  /// has something in it.
  const comment = () => {
    const said = props.response?.comment?.trim();
    return said === undefined || said === "" ? null : said;
  };

  return (
    <>
      {/* Above the heading: what a Response resolved — or did not — is said at
          the head of the page, about the Set as a whole, not under the
          Questions. */}
      <Show when={orphaned()}>
        <p class={styles.counterQuestion}>
          This Set was archived unanswered: nobody answered these questions, and
          no Response was ever sent. The agent was told the Set had been
          archived.
        </p>
      </Show>
      <Show when={nothing()}>
        {(said) => <p class={styles.counterQuestion}>{said()}</p>}
      </Show>
      <h2 class={app.sectionHeading} id="questions">
        Questions
      </h2>
      <ol class={`${styles.questions} ${styles.decided}`}>
        <For each={props.questions}>
          {(question, index) => (
            <Question
              question={question}
              position={index() + 1}
              response={props.response}
            />
          )}
        </For>
      </ol>
      {/* What became of the chooser, on the one Set that carried a proposal:
          the same three, with the recommendation still marked and the pick
          beside it. What was turned down is half of the decision here as it is
          on an Option. */}
      <Show when={props.proposal}>
        {(proposal) => (
          <Chosen proposal={proposal()} picked={props.response?.direction ?? null} />
        )}
      </Show>
      {/* Drawn whether or not anything came back in the box: the Postscript is
          what the agent closed with, so it belongs above the comment on a Set
          that has one and above where the comment would have been on a Set that
          does not — a Set closed unanswered has no Response and so never had
          one. The card is skipped only where there is neither, since an empty
          one on the record would be a box drawn around nothing at all — where
          the sheet always has its field to hold. */}
      <Show when={props.postscript || comment()}>
        <Postscript html={props.postscript}>
          <Show when={comment()}>
            {(comment) => (
              <section class={`${styles.setComment} ${styles.decided}`}>
                <h2>On the Set as a whole</h2>
                <p class={styles.comment}>{comment()}</p>
              </section>
            )}
          </Show>
        </Postscript>
      </Show>
    </>
  );
}

/// The direction chooser as the record of what was decided: the three that were
/// offered, the one the agent recommended, and the one the human picked.
///
/// The two marks are the two an Option's record carries, and are as deliberately
/// different to read: the ★ is what was suggested and the outline is what was
/// decided. A Set that went back with nothing picked says so in a line — that is
/// the proposal refused, which is a decision as much as accepting it was, and a
/// record showing three unmarked directions would read as a page whose answer
/// failed to arrive.
function Chosen(props: {
  proposal: ProposalView;
  picked: Direction | null;
}): JSX.Element {
  return (
    <section class={`${styles.directionPick} ${styles.decided}`} id="direction">
      {/* Headed and carded exactly as the chooser is — see `Choosing` — because
          the record is that same section read after the fact, and a heading the
          answering page had and this one dropped would be two pages. */}
      <h2 class={app.sectionHeading}>Direction</h2>
      <div class={styles.directionCard}>
        {/* Asked as a Question is asked, and read back the same way: the label
            floated in the accent with the agent's argument running beside it,
            keeping the place a Question's number keeps. */}
        <div class={styles.ask}>
          <AskText
            name={DIRECTION_LABEL}
            html={props.proposal.rationale_html}
          />
        </div>
        <ul class={styles.directions}>
          <For each={DIRECTIONS}>
            {(offered) => (
              <li
                class={styles.direction}
                classList={{
                  [styles.recommended!]: props.proposal.direction === offered,
                  [styles.chosen!]: props.picked === offered,
                }}
              >
                <span class={styles.directionName}>{DIRECTION[offered]}</span>
                <Show when={props.proposal.direction === offered}>
                  <span class={styles.star} title="the agent's Recommendation">
                    ★
                  </span>
                </Show>
                <Show when={props.picked === offered}>
                  <span class={styles.chose}>chosen</span>
                </Show>
              </li>
            )}
          </For>
        </ul>
        <Show when={props.picked === null}>
          <p class={styles.semantics}>
            No direction was picked, so the proposal went back to the agent.
          </p>
        </Show>
      </div>
    </section>
  );
}

/// One Question, with its Sub-questions nested one level under it.
function Question(props: {
  question: QuestionView;
  position: number;
  response: Response | null;
}): JSX.Element {
  return (
    <li
      class={styles.question}
      id={anchor(props.question.ask.name, props.position)}
    >
      {/* A Heading asked nothing, so there is nothing that became of it: it is
          read as the words over its Sub-questions and never as a question left
          open. A Set answered before Headings existed may still carry an entry
          naming one; it is passed over here rather than drawn against a
          Question that was never put. */}
      <Show
        when={!props.question.heading}
        fallback={
          <div class={`${styles.ask} ${styles.heading}`}>
            <AskText
              name={props.question.ask.name}
              html={props.question.ask.text_html}
            />
          </div>
        }
      >
        <Ask ask={props.question.ask} response={props.response} />
      </Show>
      {/* Sub-questions get no anchor of their own: one scrolls into view with
          its parent. */}
      <Show when={props.question.subquestions.length > 0}>
        <ol class={styles.subquestions}>
          <For each={props.question.subquestions}>
            {(subquestion) => (
              <li class={styles.subquestion}>
                <Ask ask={subquestion} response={props.response} />
              </li>
            )}
          </For>
        </ol>
      </Show>
    </li>
  );
}

/// A Question or a Sub-question — both are read the same way: the name it answers
/// to, its text as the server rendered it, every Option it offered, and, once
/// there is a Response, what became of it.
///
/// Every Option is kept, not just the chosen one: what was turned down is half of
/// what a decision was.
///
/// Given the whole Response rather than this question's entry, because the absence
/// of a Response is itself something to draw: with none at all the Set was
/// archived unanswered, and there was nobody to tell that these questions were
/// still open.
function Ask(props: {
  ask: AskView;
  response: Response | null;
}): JSX.Element {
  const answer = () => {
    const response = props.response;
    return response === null ? undefined : answerTo(response, props.ask.name);
  };

  const selected = () => answer()?.selected ?? null;

  const said = () => {
    const words = answer()?.free_text?.trim();
    return words === undefined || words === "" ? null : words;
  };

  // No Option and no words is the Unanswered marker, whether or not the flag is
  // set: either way nothing was answered here. Only a Response can leave a
  // question open, though — an archived Set says so once, at the head of the
  // page, rather than claiming the agent was told anything.
  const open = () =>
    props.response !== null && selected() === null && said() === null;

  // The form's own wording, minus the name of the Question it prefixes there: a
  // field in a column of five needs telling apart from the other four, and this
  // sits inside the one Question it belongs to with nothing to be confused with.
  const prompt = () =>
    props.ask.options.length === 0 ? "Your answer" : "Your thoughts";

  return (
    <div class={`${styles.ask} ${styles.decided}`}>
      <AskText name={props.ask.name} html={props.ask.text_html} />
      {/* The Options as the sheet showed them: the table where the agent
          declared the axes to compare them along, and the list they have always
          been where it did not. The declaration is the whole of what decides it
          here as there — a record that drew a different shape from the sheet it
          was filled in on would be a second reading of one decision. */}
      <Show when={props.ask.options.length > 0}>
        <Show
          when={props.ask.columns.length > 0}
          fallback={
            <ul class={styles.options}>
              <For each={props.ask.options}>
                {(option) => <Offered option={option} selected={selected()} />}
              </For>
            </ul>
          }
        >
          <Tabulated ask={props.ask} selected={selected()} />
        </Show>
      </Show>
      <Show when={said()}>
        {(said) => (
          <p class={styles.answerText}>
            <span class={styles.prompt}>{prompt()}</span>
            {said()}
          </p>
        )}
      </Show>
      <Show when={open()}>
        <p class={styles.unanswered}>
          Unanswered — the agent was told this one is still open.
        </p>
      </Show>
    </div>
  );
}

/// One Option as it was offered: numbered and worded as the agent wrote it,
/// marked if they recommended it, and marked apart from that if this is the one
/// the human chose.
///
/// The two marks are deliberately different things to read: the ★ is what was
/// suggested, and the outline is what was decided, which on any given question may
/// well not be the same Option.
///
/// "chosen" is still written, and the stylesheet takes it out of the layout rather
/// than out of the page — the outline says which one to a reader looking at it and
/// nothing at all to one who is not, and an archive that cannot say what was
/// decided is not much of an archive. See `.ask.decided .chose`.
function Offered(props: {
  option: OptionView;
  selected: number | null;
}): JSX.Element {
  const chosen = () => props.selected === props.option.n;

  // The text is filled in wholesale, and it is inline markup all the way down —
  // the rendering flattened anything blockier on the way here, because an Option
  // is one line beside its number. It is marked as rendered markdown all the
  // same: what did survive, a code span above all, is drawn as it is everywhere
  // else.
  return (
    <li class={marks(props.option, chosen())}>
      <span class={styles.n}>{props.option.n}</span>
      <span
        class={`${styles.optionText} markdown`}
        innerHTML={props.option.text_html}
      />
      <Show when={props.option.recommended}>
        <span class={styles.star} title="the agent's Recommendation">
          ★
        </span>
      </Show>
      <Show when={chosen()}>
        <span class={styles.chose}>chosen</span>
      </Show>
    </li>
  );
}

/// What one Option is marked as: an Option, the one that was chosen, the one
/// that was recommended — any two of which may be the same Option and often are
/// not. Shared by the list entry and the table row, because the marks are the
/// Option's rather than the shape's.
function marks(option: OptionView, chosen: boolean): string {
  return [
    styles.option,
    chosen && styles.chosen,
    option.recommended && styles.recommended,
  ]
    .filter(Boolean)
    .join(" ");
}

/// The Options as the Answer Table the question declared, read rather than
/// filled in: the same columns the sheet drew — see [`Head`] — with the radios
/// gone and what was decided marked on the row that took it.
///
/// Every row is kept, chosen or not, exactly as every list entry is.
function Tabulated(props: {
  ask: AskView;
  selected: number | null;
}): JSX.Element {
  const marked = () => starred(props.ask.options);

  return (
    <table class={styles.answerTable}>
      <Head columns={props.ask.columns} starred={marked()} />
      <tbody>
        <For each={props.ask.options}>
          {(option) => (
            <Chose
              option={option}
              selected={props.selected}
              starred={marked()}
            />
          )}
        </For>
      </tbody>
    </table>
  );
}

/// One Option as a row of the record's Answer Table: numbered and worded as the
/// agent wrote it, compared along their axes, marked if they recommended it, and
/// marked apart from that if this is the row the human chose.
///
/// The two marks are the list's two marks, and are as deliberately different to
/// read here: the ★ is what was suggested and the row's treatment is what was
/// decided. "chosen" is written beside the number for the same reason it is
/// written in the list — an outline says nothing at all to a reader not looking
/// at one, and the stylesheet takes the word out of the layout rather than out
/// of the page. See `.ask.decided .chose`.
function Chose(props: {
  option: OptionView;
  selected: number | null;
  starred: boolean;
}): JSX.Element {
  const chosen = () => props.selected === props.option.n;

  return (
    <tr class={marks(props.option, chosen())}>
      <td class={styles.pick}>
        <span class={styles.n}>{props.option.n}</span>
        <Show when={chosen()}>
          <span class={styles.chose}>chosen</span>
        </Show>
      </td>
      <td
        class={`${styles.optionText} markdown`}
        innerHTML={props.option.text_html}
      />
      <For each={props.option.cells}>
        {(cell) => <td class="markdown" innerHTML={cell} />}
      </For>
      <Show when={props.starred}>
        <td class={styles.starCell}>
          <Show when={props.option.recommended}>
            <span class={styles.star} title="the agent's Recommendation">
              ★
            </span>
          </Show>
        </td>
      </Show>
    </tr>
  );
}

/// The Response's entry for this question, if it has one.
///
/// A stored Response was validated against its Set, so every question has exactly
/// one entry. The lookup is still fallible because the page draws the Set rather
/// than the Response: a question with nothing to show reads as Unanswered, which
/// is true of one, rather than as a gap in the page.
function answerTo(response: Response, name: string): Answer | undefined {
  return response.answers.find((answer) => answer.label.trim() === name);
}

/// Whether an entry carries an Answer at all — an Option was selected or something
/// was written. One that carries neither is the Unanswered marker.
function isAnswer(answer: Answer): boolean {
  return (
    (answer.selected ?? null) !== null ||
    (answer.free_text ?? "").trim() !== ""
  );
}

/// What to say at the head of a Response that resolved nothing — and `null` when
/// it resolved something, which is the ordinary case.
///
/// Answering a Set by leaving every question open is allowed: with the set-level
/// comment it is a counter-question, the human's "not these questions", and it is
/// as much a Response as any other. It has to read as one rather than as a page
/// whose Answers failed to arrive, which is what a column of Unanswered with no
/// word about why would look like.
///
/// Which is the whole of what this says, so a Set answered in silence gets
/// nothing: with no comment there is no counter-question to explain, and the
/// line was only the column of Unanswered read back at whoever was reading it.
function nothingAnswered(response: Response): string | null {
  if (response.answers.some(isAnswer)) {
    return null;
  }

  const commented = (response.comment ?? "").trim() !== "";

  return commented
    ? "Nothing here was answered. The comment below is the whole Response — a " +
        "counter-question — and every question went back to the agent still open."
    : null;
}
