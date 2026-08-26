//! The Question Set as it arrives from an agent: what YAML parses into, and
//! which shapes the question grammar refuses.

use verkstead_schema::{Decided, Direction, QuestionSet, Response};

/// A Set exercising every part of the wire format at once.
const FULL_SET: &str = r#"
title: Storage layout for the pending list
preface: |
  We need to settle how Sets are stored before the UI lands.

  The candidates differ mainly in how much SQL the pending list needs.
project: verkstead
branch: api-core-and-cli
diff: |
  diff --git a/notes.md b/notes.md
  +a line
questions:
  - label: Q1
    text: How should a Question Set be stored?
    options:
      - n: 1
        text: One JSON body column
        recommended: true
      - n: 2
        text: Fully normalised tables
  - label: Q2
    text: What identifies a Set on the wire?
    subquestions:
      - letter: a
        text: Integer rowid or UUID?
        options:
          - n: 1
            text: Integer rowid
            recommended: true
          - n: 2
            text: UUID
      - letter: b
        text: Should the id appear in URLs?
postscript: |
  Whichever way this goes, the pending list is the part I'd want a second
  opinion on.

  Anything else about the store is worth saying here.
"#;

#[test]
fn a_full_set_parses_into_its_parts() {
    let set = QuestionSet::from_yaml(FULL_SET).expect("the example Set should parse");

    assert_eq!(set.title, "Storage layout for the pending list");
    assert_eq!(set.project.as_deref(), Some("verkstead"));
    assert_eq!(set.branch.as_deref(), Some("api-core-and-cli"));
    assert!(set.diff.as_deref().unwrap().contains("+a line"));
    assert_eq!(
        set.preface.as_deref(),
        Some(
            "We need to settle how Sets are stored before the UI lands.\n\n\
             The candidates differ mainly in how much SQL the pending list needs.\n"
        )
    );
    assert_eq!(
        set.postscript.as_deref(),
        Some(
            "Whichever way this goes, the pending list is the part I'd want a second\n\
             opinion on.\n\n\
             Anything else about the store is worth saying here.\n"
        )
    );

    let [q1, q2] = &set.questions[..] else {
        panic!("expected two Questions, got {}", set.questions.len());
    };

    assert_eq!(q1.label, "Q1");
    assert_eq!(q1.text, "How should a Question Set be stored?");
    assert_eq!(q1.options.len(), 2);
    assert_eq!(q1.options[0].n, 1);
    assert_eq!(q1.options[0].text, "One JSON body column");
    assert!(q1.options[0].recommended);
    assert!(!q1.options[1].recommended);
    assert!(q1.subquestions.is_empty());

    assert_eq!(q2.label, "Q2");
    assert!(q2.options.is_empty());
    assert_eq!(q2.subquestions.len(), 2);
    assert_eq!(q2.subquestions[0].letter, "a");
    assert_eq!(q2.subquestions[0].options.len(), 2);
    assert!(q2.subquestions[0].options[0].recommended);
    assert!(q2.subquestions[1].options.is_empty());

    set.validate().expect("the example Set should be valid");
}

#[test]
fn a_minimal_set_needs_only_a_title_and_questions() {
    let set = QuestionSet::from_yaml(
        "
title: Just the one
questions:
  - label: Q1
    text: Ship it?
",
    )
    .expect("a Set without a preface should parse");

    assert_eq!(set.preface, None);
    assert_eq!(set.postscript, None);
    assert_eq!(set.project, None);
    assert_eq!(set.branch, None);
    assert_eq!(set.diff, None);
    set.validate().expect("a bare Set should be valid");
}

#[test]
fn an_unknown_field_is_a_parse_error() {
    let error = QuestionSet::from_yaml(
        "
title: Typo
questions:
  - label: Q1
    text: Ship it?
    options:
      - n: 1
        text: Yes
        recomended: true
",
    )
    .expect_err("a misspelled field should not be silently ignored");

    assert!(
        error.to_string().contains("recomended"),
        "the error should name the unknown field, got: {error}"
    );
}

#[test]
fn a_third_level_of_nesting_is_rejected_naming_the_sub_question() {
    let set = QuestionSet::from_yaml(
        "
title: Too deep
questions:
  - label: Q7
    text: Which storage layout?
    subquestions:
      - letter: a
        text: Rowid or UUID?
        subquestions:
          - letter: i
            text: And how wide?
",
    )
    .expect("three levels should parse, so validation can name the offender");

    let error = set
        .validate()
        .expect_err("Sub-questions are leaves; a third level is invalid");

    assert!(
        error.names("Q7a"),
        "the error should name Q7a, got: {error}"
    );
}

#[test]
fn two_recommended_options_are_rejected_naming_the_question() {
    let set = QuestionSet::from_yaml(
        "
title: Two stars
questions:
  - label: Q3
    text: Which one?
    options:
      - n: 1
        text: This
        recommended: true
      - n: 2
        text: That
        recommended: true
",
    )
    .unwrap();

    let error = set
        .validate()
        .expect_err("at most one Option may be the Recommendation");

    assert!(error.names("Q3"), "the error should name Q3, got: {error}");
}

#[test]
fn two_recommended_options_on_a_sub_question_are_rejected_naming_it() {
    let set = QuestionSet::from_yaml(
        "
title: Two stars, nested
questions:
  - label: Q4
    text: Which one?
    subquestions:
      - letter: b
        text: Really, which one?
        options:
          - n: 1
            text: This
            recommended: true
          - n: 2
            text: That
            recommended: true
",
    )
    .unwrap();

    let error = set.validate().expect_err("one Recommendation per question");

    assert!(
        error.names("Q4b"),
        "the error should name Q4b, got: {error}"
    );
}

#[test]
fn a_missing_title_is_rejected() {
    let error = QuestionSet::from_yaml(
        "
questions:
  - label: Q1
    text: Ship it?
",
    )
    .expect_err("title is required");

    assert!(
        error.to_string().contains("title"),
        "the error should mention the missing title, got: {error}"
    );
}

#[test]
fn an_empty_title_is_rejected() {
    let set = QuestionSet::from_yaml(
        "
title: '   '
questions:
  - label: Q1
    text: Ship it?
",
    )
    .unwrap();

    let error = set.validate().expect_err("a blank title is no title");

    assert!(
        error.to_string().contains("title"),
        "the error should mention the title, got: {error}"
    );
}

#[test]
fn duplicate_labels_are_rejected() {
    let set = QuestionSet::from_yaml(
        "
title: Same name twice
questions:
  - label: Q5
    text: First
  - label: Q5
    text: Second
",
    )
    .unwrap();

    let error = set
        .validate()
        .expect_err("a Response answers by label, so labels must be distinct");

    assert!(error.names("Q5"), "the error should name Q5, got: {error}");
}

#[test]
fn duplicate_option_numbers_are_rejected() {
    let set = QuestionSet::from_yaml(
        "
title: Ambiguous selection
questions:
  - label: Q6
    text: Which one?
    options:
      - n: 1
        text: This
      - n: 1
        text: That
",
    )
    .unwrap();

    let error = set
        .validate()
        .expect_err("an Answer selects by number, so numbers must be distinct");

    assert!(error.names("Q6"), "the error should name Q6, got: {error}");
}

#[test]
fn a_question_needs_a_label_and_text() {
    let set = QuestionSet::from_yaml(
        "
title: Nameless
questions:
  - label: ''
    text: ''
",
    )
    .unwrap();

    let error = set
        .validate()
        .expect_err("a Question needs a label and text");

    assert!(
        error.to_string().contains("label"),
        "the error should mention the label, got: {error}"
    );
}

#[test]
fn every_violation_is_reported_at_once() {
    let set = QuestionSet::from_yaml(
        "
title: ''
questions:
  - label: Q1
    text: Which one?
    options:
      - n: 1
        text: This
        recommended: true
      - n: 2
        text: That
        recommended: true
  - label: Q2
    text: And which one?
    subquestions:
      - letter: a
        text: Nested too far
        subquestions:
          - letter: i
            text: Way too far
",
    )
    .unwrap();

    let error = set.validate().unwrap_err();

    assert_eq!(
        error.violations.len(),
        3,
        "an agent should learn about every problem in one round trip, got: {error}"
    );
    assert!(error.names("Q1") && error.names("Q2a"));
}

#[test]
fn a_multi_line_preface_and_postscript_round_trip_through_yaml() {
    let set = QuestionSet::from_yaml(FULL_SET).unwrap();
    let yaml = set.to_yaml().expect("a Set should serialise");
    let reparsed = QuestionSet::from_yaml(&yaml).expect("our own YAML should parse");

    assert_eq!(reparsed.preface, set.preface);
    assert_eq!(reparsed.postscript, set.postscript);
    assert_eq!(reparsed.questions.len(), set.questions.len());
    assert!(
        yaml.contains("preface: |"),
        "the markdown Preface should ride in a block scalar, got:\n{yaml}"
    );
    assert!(
        yaml.contains("postscript: |"),
        "the markdown Postscript should ride in one too, got:\n{yaml}"
    );
}

#[test]
fn a_set_may_close_with_a_postscript_and_nothing_else_new() {
    let set = QuestionSet::from_yaml(
        "
title: Just the one
questions:
  - label: Q1
    text: Ship it?
postscript: Anything else about the release goes in the comment.
",
    )
    .expect("a Set with a Postscript should parse");

    assert_eq!(
        set.postscript.as_deref(),
        Some("Anything else about the release goes in the comment.")
    );
    set.validate()
        .expect("a Postscript is prose: there is nothing in it to refuse");
}

#[test]
fn a_question_may_declare_the_axes_its_options_are_compared_along() {
    let set = QuestionSet::from_yaml(
        "
title: Where the counter lives
questions:
  - label: Q1
    text: Where should the request counter live?
    columns:
      - Latency
      - '`ops` cost'
    options:
      - n: 1
        text: In-process
        cells:
          - Sub-`ms`
          - None
      - n: 2
        text: In Redis
        recommended: true
        cells:
          - A hop
          - A box to run
    subquestions:
      - letter: a
        text: And the eviction policy?
        columns:
          - Memory
        options:
          - n: 1
            text: LRU
            cells:
              - Bounded
",
    )
    .expect("a Set declaring an Answer Table should parse");

    let q1 = &set.questions[0];
    assert_eq!(q1.columns, ["Latency", "`ops` cost"]);
    assert_eq!(q1.options[0].cells, ["Sub-`ms`", "None"]);
    assert_eq!(q1.options[1].cells, ["A hop", "A box to run"]);

    let q1a = &q1.subquestions[0];
    assert_eq!(q1a.columns, ["Memory"]);
    assert_eq!(q1a.options[0].cells, ["Bounded"]);

    set.validate()
        .expect("a well-formed Answer Table is a legal Set");
}

#[test]
fn a_question_without_columns_declares_no_table() {
    let set = QuestionSet::from_yaml(FULL_SET).unwrap();

    assert!(
        set.questions[0].columns.is_empty(),
        "the presence of `columns` is what makes an Answer Table"
    );
    assert!(set.questions[0].options[0].cells.is_empty());

    set.validate()
        .expect("a Set that declares no table is untouched by the table's rules");
}

#[test]
fn a_row_short_of_the_declared_axes_is_rejected_naming_the_question() {
    let set = QuestionSet::from_yaml(
        "
title: Where the counter lives
questions:
  - label: Q1
    text: Where should the request counter live?
    columns: [Latency, Cost]
    options:
      - n: 1
        text: In-process
        cells: [Sub-ms, None]
      - n: 2
        text: In Redis
        cells: [A hop]
",
    )
    .unwrap();

    let error = set
        .validate()
        .expect_err("a row that does not fill every column is a broken table");

    assert!(error.names("Q1"), "the error should name Q1, got: {error}");
}

#[test]
fn cells_without_columns_are_rejected_naming_the_question() {
    let set = QuestionSet::from_yaml(
        "
title: Cells with nowhere to go
questions:
  - label: Q2
    text: Where should the request counter live?
    options:
      - n: 1
        text: In-process
        cells: [Sub-ms]
",
    )
    .unwrap();

    let error = set
        .validate()
        .expect_err("cells fill columns; without any declared there is no table to fill");

    assert!(error.names("Q2"), "the error should name Q2, got: {error}");
}

#[test]
fn a_table_only_some_options_have_rows_for_is_rejected() {
    let set = QuestionSet::from_yaml(
        "
title: Half a table
questions:
  - label: Q3
    text: Where should the request counter live?
    columns: [Latency]
    options:
      - n: 1
        text: In-process
        cells: [Sub-ms]
      - n: 2
        text: In Redis
",
    )
    .unwrap();

    let error = set
        .validate()
        .expect_err("declaring columns commits every Option to a row");

    assert!(error.names("Q3"), "the error should name Q3, got: {error}");
}

#[test]
fn a_malformed_table_on_a_sub_question_is_rejected_naming_it() {
    let set = QuestionSet::from_yaml(
        "
title: Nested table
questions:
  - label: Q4
    text: Where should the request counter live?
    subquestions:
      - letter: a
        text: And the eviction policy?
        columns: [Memory, Cost]
        options:
          - n: 1
            text: LRU
            cells: [Bounded]
",
    )
    .unwrap();

    let error = set
        .validate()
        .expect_err("a Sub-question declares its own table and answers for it");

    assert!(
        error.names("Q4a"),
        "the error should name Q4a, got: {error}"
    );
}

#[test]
fn an_empty_columns_list_declares_no_table_at_all() {
    let set = QuestionSet::from_yaml(
        "
title: No axes named
questions:
  - label: Q5
    text: Where should the request counter live?
    columns: []
    options:
      - n: 1
        text: In-process
",
    )
    .unwrap();

    set.validate()
        .expect("an empty `columns` is indistinguishable from none, and reads as none");

    let with_cells = QuestionSet::from_yaml(
        "
title: No axes named
questions:
  - label: Q5
    text: Where should the request counter live?
    columns: []
    options:
      - n: 1
        text: In-process
        cells: [Sub-ms]
",
    )
    .unwrap();

    let error = with_cells
        .validate()
        .expect_err("reading as none means cells beside it have no columns to fill");

    assert!(error.names("Q5"), "the error should name Q5, got: {error}");
}

#[test]
fn an_answer_table_round_trips_through_yaml() {
    let set = QuestionSet::from_yaml(
        "
title: Where the counter lives
questions:
  - label: Q1
    text: Where should the request counter live?
    columns: [Latency]
    options:
      - n: 1
        text: In-process
        cells: [Sub-ms]
",
    )
    .unwrap();
    let yaml = set.to_yaml().expect("a Set should serialise");
    let reparsed = QuestionSet::from_yaml(&yaml).expect("our own YAML should parse");

    assert_eq!(reparsed, set);
}

/// A Set proposing wrap-up: the closing move a grilling ends by, and the one
/// shape that carries a `proposal` block.
const PROPOSING: &str = r#"
title: Ready to build the rate limiter
preface: |
  I think we have this.
questions:
  - label: Q14
    text: Ready to build it this way?
    options:
      - n: 1
        text: Yes, go ahead
        recommended: true
      - n: 2
        text: Not yet — more to work through
proposal:
  direction: task-list
  rationale: |
    Six changes across the limiter, the config and the migration, each
    independently testable.
"#;

#[test]
fn a_proposal_parses_into_the_direction_and_the_reasoning() {
    let set = QuestionSet::from_yaml(PROPOSING).expect("the proposing Set should parse");

    set.validate()
        .expect("a proposal with reasoning and a question to answer is legal");

    let proposal = set.proposal.expect("this Set proposes wrapping up");

    assert_eq!(proposal.direction, Direction::TaskList);
    assert!(
        proposal.rationale.starts_with("Six changes across"),
        "the rationale is markdown the human reads, got: {:?}",
        proposal.rationale
    );
}

#[test]
fn an_ordinary_set_carries_no_proposal_at_all() {
    let set = QuestionSet::from_yaml(FULL_SET).expect("the example Set should parse");

    assert_eq!(
        set.proposal, None,
        "a grilling Set must never be mistaken for the one that ends the grilling",
    );
}

#[test]
fn a_proposal_round_trips_through_yaml() {
    let set = QuestionSet::from_yaml(PROPOSING).unwrap();
    let yaml = set.to_yaml().expect("a Set should serialise");
    let reparsed = QuestionSet::from_yaml(&yaml).expect("our own YAML should parse");

    assert_eq!(reparsed, set);
}

#[test]
fn a_direction_the_wire_format_does_not_know_is_refused_as_it_parses() {
    let unknown = PROPOSING.replace("direction: task-list", "direction: telepathy");

    QuestionSet::from_yaml(&unknown)
        .expect_err("there are three directions, and the format knows all of them");
}

#[test]
fn a_proposal_needs_reasoning_the_human_can_read() {
    let bare = PROPOSING
        .split("  rationale:")
        .next()
        .expect("the fixture has a rationale to cut off")
        .to_owned()
        + "  rationale: \"   \"\n";

    let error = QuestionSet::from_yaml(&bare)
        .unwrap()
        .validate()
        .expect_err("the chooser shows the reasoning, so there has to be some");

    assert!(
        error.to_string().contains("rationale"),
        "the refusal should say what is missing, got: {error}"
    );
}

/// A proposal Set needs nothing answerable of its own.
///
/// The chooser is what there is to decide on one, and the viewer injects it —
/// so a closing Set that asks nothing beside it is a Set with one decision on
/// it rather than none at all.
#[test]
fn a_proposal_needs_no_question_beside_it() {
    QuestionSet::from_yaml(
        "
title: Ready to build it
questions: []
proposal:
  direction: inline
  rationale: One session is enough.
",
    )
    .unwrap()
    .validate()
    .expect("the chooser is what there is to answer on a closing Set");
}

/// A Set carrying the wrap-up review's findings: a Question per finding, and the
/// block that says which Answer to each means *fix it*.
const REVIEWING: &str = r#"
title: Review of the rate limiter branch
preface: |
  Two things worth a decision.
questions:
  - label: Q1
    text: |
      The window counter is never reset between windows.
    options:
      - n: 1
        text: Fix it
        recommended: true
      - n: 2
        text: Leave it
  - label: Q2
    text: |
      Two clocks now, and the tests pin both.
    options:
      - n: 1
        text: Fix it
      - n: 2
        text: Leave it
        recommended: true
review:
  findings:
    - fix: Q1.1
      what: |
        `window.rs` — `Window::count` is never reset as the window rolls, so a
        client that exceeds the limit is refused for ever.
    - fix: Q2.1
      what: |
        `limits.rs` and `window.rs` each hold their own notion of now. Collapse
        them onto one clock.
"#;

#[test]
fn a_review_parses_into_a_finding_per_question() {
    let set = QuestionSet::from_yaml(REVIEWING).expect("the review Set should parse");

    set.validate()
        .expect("findings that each name an Option the Set offers are legal");

    let review = set.review.expect("this Set carries a review");

    assert_eq!(review.findings.len(), 2);
    assert_eq!(review.findings[0].fixing(), Some(("Q1", 1)));
    assert!(
        review.findings[0].what.contains("Window::count"),
        "the finding carries what the fix session is told, got: {:?}",
        review.findings[0].what,
    );
}

#[test]
fn an_ordinary_set_carries_no_review_at_all() {
    let set = QuestionSet::from_yaml(FULL_SET).expect("the example Set should parse");

    assert_eq!(
        set.review, None,
        "any Set could otherwise be mistaken for the wrap-up review's",
    );
}

#[test]
fn a_review_round_trips_through_yaml() {
    let set = QuestionSet::from_yaml(REVIEWING).unwrap();
    let yaml = set.to_yaml().expect("a Set should serialise");
    let reparsed = QuestionSet::from_yaml(&yaml).expect("our own YAML should parse");

    assert_eq!(reparsed, set);
}

/// The same rule the proposal's acceptance has, one finding at a time: a finding
/// nobody can accept could never become work, and nothing would ever say so.
///
/// Plus the two only a review has — a finding that says nothing to the session
/// that would fix it, and a block that found nothing at all, which is a review
/// that should have asked nothing rather than asked emptily.
#[test]
fn a_finding_nobody_can_act_on_is_refused() {
    for (how, set) in [
        (
            "an Option the question does not offer",
            "
title: Review
questions:
  - label: Q1
    text: The counter is never reset.
    options:
      - n: 1
        text: Fix it
review:
  findings:
    - fix: Q1.4
      what: Reset it as the window rolls.
",
        ),
        (
            "a question the Set does not ask",
            "
title: Review
questions:
  - label: Q1
    text: The counter is never reset.
    options:
      - n: 1
        text: Fix it
review:
  findings:
    - fix: Q9.1
      what: Reset it as the window rolls.
",
        ),
        (
            "something that is not the notation",
            "
title: Review
questions:
  - label: Q1
    text: The counter is never reset.
    options:
      - n: 1
        text: Fix it
review:
  findings:
    - fix: please fix it
      what: Reset it as the window rolls.
",
        ),
        (
            "a finding with nothing to tell the session that would fix it",
            "
title: Review
questions:
  - label: Q1
    text: The counter is never reset.
    options:
      - n: 1
        text: Fix it
review:
  findings:
    - fix: Q1.1
      what: \"   \"
",
        ),
        (
            "two findings on one Option, which is one Answer meaning two things",
            "
title: Review
questions:
  - label: Q1
    text: The counter is never reset.
    options:
      - n: 1
        text: Fix it
review:
  findings:
    - fix: Q1.1
      what: Reset it as the window rolls.
    - fix: Q1.1
      what: And collapse the two clocks.
",
        ),
        (
            "a review that found nothing, which raises no Set at all",
            "
title: Review
questions:
  - label: Q1
    text: The counter is never reset.
    options:
      - n: 1
        text: Fix it
review:
  findings: []
",
        ),
    ] {
        assert!(
            QuestionSet::from_yaml(set).unwrap().validate().is_err(),
            "{how} should be refused, and was not",
        );
    }
}

/// What turns a finding into work, read straight off the Response — and what the
/// session that fixes it is told the human said.
#[test]
fn only_the_named_option_accepts_a_finding() {
    let set = QuestionSet::from_yaml(REVIEWING).unwrap();
    let review = set.review.as_ref().expect("this Set carries a review");
    let finding = &review.findings[0];

    let answered = |yaml: &str| finding.accepted(&Response::from_yaml(yaml).unwrap());

    assert!(
        answered("answers:\n  - label: Q1\n    selected: 1\n"),
        "picking the Option the finding names is what dispatches a fix",
    );
    assert!(
        answered("answers:\n  - label: Q1\n    selected: 1\n    free_text: Keep the signature.\n"),
        "words beside a picked Option are a qualification, not a refusal",
    );

    assert!(
        !answered("answers:\n  - label: Q1\n    selected: 2\n"),
        "leaving it is the human declining the finding",
    );
    assert!(
        !answered("answers:\n  - label: Q1\n    free_text: Not worth it yet.\n"),
        "an answer in their own words wins over the Options, so it is not the named one",
    );
    assert!(
        !answered("answers:\n  - label: Q1\n    unanswered: true\n"),
        "a question left open never dispatches anything",
    );

    let qualified = Response::from_yaml(
        "answers:\n  - label: Q1\n    selected: 1\n    free_text: Keep the signature.\n",
    )
    .unwrap();

    assert_eq!(
        finding.said(&qualified),
        "Keep the signature.",
        "and what they wrote goes with the finding to whoever fixes it",
    );
    assert_eq!(
        finding.said(&Response::from_yaml("answers:\n  - label: Q1\n    selected: 1\n").unwrap()),
        "",
        "agreeing without a word said is the ordinary way of agreeing",
    );
}

/// A review that judged one finding too big to fix in the sitting it was found
/// in: the same block, with a second Option named beside the first.
const SPLITTING: &str = r#"
title: Review of the rate limiter branch
questions:
  - label: Q1
    text: |
      The window counter is never reset between windows, and unpicking it
      touches every caller.
    options:
      - n: 1
        text: Fix it here
      - n: 2
        text: Leave it
      - n: 3
        text: Split it out as a task
        recommended: true
  - label: Q2
    text: |
      Two clocks now, and the tests pin both.
    options:
      - n: 1
        text: Fix it here
        recommended: true
      - n: 2
        text: Leave it
review:
  findings:
    - fix: Q1.1
      split: Q1.3
      what: |
        `window.rs` — `Window::count` is never reset as the window rolls, so a
        client that exceeds the limit is refused for ever.
    - fix: Q2.1
      what: |
        `limits.rs` and `window.rs` each hold their own notion of now. Collapse
        them onto one clock.
"#;

#[test]
fn a_finding_can_offer_a_split_option_beside_its_fix() {
    let set = QuestionSet::from_yaml(SPLITTING).expect("the review Set should parse");

    set.validate()
        .expect("a split naming an Option the Set offers, distinct from the fix, is legal");

    let review = set.review.as_ref().expect("this Set carries a review");

    assert_eq!(review.findings[0].splitting(), Some(("Q1", 3)));
    assert_eq!(
        review.findings[1].splitting(),
        None,
        "a finding without a split is exactly the finding it always was",
    );

    let yaml = set.to_yaml().expect("a Set should serialise");
    assert_eq!(
        QuestionSet::from_yaml(&yaml).expect("our own YAML should parse"),
        set,
        "the split survives the round trip",
    );
}

#[test]
fn an_ordinary_finding_offers_no_split_at_all() {
    let set = QuestionSet::from_yaml(REVIEWING).expect("the review Set should parse");
    let review = set.review.expect("this Set carries a review");

    assert!(
        review
            .findings
            .iter()
            .all(|finding| finding.split.is_none()),
        "a review that judged nothing too big says nothing about splitting",
    );
}

/// The split is held to what the fix is held to — an Option the human can
/// actually pick — and to the two only a split has: it cannot be the finding's
/// own fix, and it cannot be an Option another finding already turns on.
#[test]
fn a_split_nobody_can_act_on_is_refused() {
    for (how, naming, set) in [
        (
            "a split that is not the notation",
            "Q1.1",
            "
title: Review
questions:
  - label: Q1
    text: The counter is never reset.
    options:
      - n: 1
        text: Fix it here
      - n: 3
        text: Split it out
review:
  findings:
    - fix: Q1.1
      split: split it out
      what: Reset it as the window rolls.
",
        ),
        (
            "a split on a question the Set does not ask",
            "Q1.1",
            "
title: Review
questions:
  - label: Q1
    text: The counter is never reset.
    options:
      - n: 1
        text: Fix it here
      - n: 3
        text: Split it out
review:
  findings:
    - fix: Q1.1
      split: Q9.3
      what: Reset it as the window rolls.
",
        ),
        (
            "a split on an Option the question does not offer",
            "Q1.1",
            "
title: Review
questions:
  - label: Q1
    text: The counter is never reset.
    options:
      - n: 1
        text: Fix it here
      - n: 3
        text: Split it out
review:
  findings:
    - fix: Q1.1
      split: Q1.4
      what: Reset it as the window rolls.
",
        ),
        (
            "a split that is the finding's own fix, which is one Answer meaning both",
            "Q1.1",
            "
title: Review
questions:
  - label: Q1
    text: The counter is never reset.
    options:
      - n: 1
        text: Fix it here
      - n: 3
        text: Split it out
review:
  findings:
    - fix: Q1.1
      split: Q1.1
      what: Reset it as the window rolls.
",
        ),
        (
            "a split another finding is fixed by, which is one Answer meaning two things",
            "two findings",
            "
title: Review
questions:
  - label: Q1
    text: The counter is never reset.
    options:
      - n: 1
        text: Fix it here
      - n: 3
        text: Split it out
review:
  findings:
    - fix: Q1.1
      split: Q1.3
      what: Reset it as the window rolls.
    - fix: Q1.3
      what: And collapse the two clocks.
",
        ),
    ] {
        let refused = QuestionSet::from_yaml(set)
            .unwrap()
            .validate()
            .expect_err(&format!("{how} should be refused, and was not"));

        assert!(
            refused.to_string().contains(naming),
            "{how} should be refused naming the finding at fault, got: {refused}",
        );
    }
}

/// The three outcomes a Response holds for one finding, told apart by which
/// named Option it picked and by nothing else.
#[test]
fn a_response_tells_fixing_here_from_splitting_out_from_declining() {
    let set = QuestionSet::from_yaml(SPLITTING).unwrap();
    let review = set.review.as_ref().expect("this Set carries a review");
    let finding = &review.findings[0];

    let decided = |yaml: &str| finding.decided(&Response::from_yaml(yaml).unwrap());

    assert_eq!(
        decided("answers:\n  - label: Q1\n    selected: 1\n"),
        Decided::Fix,
        "picking the Option the finding is fixed by is fixing it here",
    );
    assert_eq!(
        decided("answers:\n  - label: Q1\n    selected: 3\n"),
        Decided::Split,
        "picking the Option the finding is split out by is work for a backlog",
    );
    assert_eq!(
        decided("answers:\n  - label: Q1\n    selected: 2\n"),
        Decided::Declined,
        "any other Option is the human declining the finding",
    );
    assert_eq!(
        decided("answers:\n  - label: Q1\n    unanswered: true\n"),
        Decided::Declined,
        "a question left open never dispatches anything",
    );
    assert_eq!(
        decided("answers:\n  - label: Q1\n    free_text: Not worth it yet.\n"),
        Decided::Declined,
        "an answer in their own words wins over the Options, so it is neither",
    );

    let picked = |yaml: &str| Response::from_yaml(yaml).unwrap();

    assert!(
        !finding.accepted(&picked("answers:\n  - label: Q1\n    selected: 3\n")),
        "a split pick is not the finding accepted to fix here",
    );
    assert!(
        finding.accepted(&picked("answers:\n  - label: Q1\n    selected: 1\n")),
        "and the fix pick still is",
    );

    assert_eq!(
        finding.said(&picked(
            "answers:\n  - label: Q1\n    selected: 3\n    free_text: Its own task, then.\n"
        )),
        "Its own task, then.",
        "what they wrote beside a split goes with it to whoever works the task",
    );

    assert_eq!(
        review.findings[1].decided(&picked("answers:\n  - label: Q2\n    selected: 2\n")),
        Decided::Declined,
        "a finding that offers no split can never be split out",
    );
}
