//! The Response as it comes back from the human: what YAML parses into, and
//! which Responses fail to resolve the Set they claim to answer.

use verkstead_schema::{QuestionSet, Response};

/// A Set of one Question with Options, one Heading over two Sub-questions, and
/// a Sub-question without Options of its own.
///
/// What there is to answer is `Q1`, `Q2a` and `Q2b`. `Q2` is not among them:
/// Sub-questions and no Options of its own make it a Heading, which asks
/// nothing and takes no entry.
const SET: &str = r#"
title: How should the wait end?
questions:
  - label: Q1
    text: How long should the hold window be?
    options:
      - n: 1
        text: Thirty seconds
        recommended: true
      - n: 2
        text: Five minutes
  - label: Q2
    text: What comes back when nothing has been answered yet?
    subquestions:
      - letter: a
        text: Which status?
        options:
          - n: 1
            text: 204 No Content
          - n: 2
            text: 200 with a pending document
      - letter: b
        text: Anything else worth saying in the reply?
"#;

fn set() -> QuestionSet {
    QuestionSet::from_yaml(SET).expect("the example Set should parse")
}

#[test]
fn a_full_response_parses_into_its_parts() {
    let response = Response::from_yaml(
        "
answers:
  - label: Q1
    selected: 1
  - label: Q2a
    selected: 1
    free_text: Empty body, no ambiguity.
  - label: Q2b
    free_text: |
      Say nothing.

      The status is the message.
comment: Happy with all of this.
",
    )
    .expect("the example Response should parse");

    let [q1, q2a, q2b] = &response.answers[..] else {
        panic!(
            "expected an entry per answerable question, got {}",
            response.answers.len()
        );
    };

    assert_eq!(q1.label, "Q1");
    assert_eq!(q1.selected, Some(1));
    assert_eq!(q1.free_text, None);
    assert!(!q1.unanswered);
    assert!(q1.is_answer());

    assert_eq!(q2a.selected, Some(1));
    assert_eq!(q2a.free_text.as_deref(), Some("Empty body, no ambiguity."));

    assert_eq!(q2b.selected, None);
    assert_eq!(
        q2b.free_text.as_deref(),
        Some("Say nothing.\n\nThe status is the message.\n")
    );
    assert!(q2b.is_answer());

    assert_eq!(response.comment.as_deref(), Some("Happy with all of this."));

    response
        .validate(&set())
        .expect("this Response resolves it");
}

#[test]
fn a_response_leaving_every_question_unanswered_is_valid() {
    let response = Response::from_yaml(
        "
answers:
  - label: Q1
    unanswered: true
  - label: Q2a
    unanswered: true
  - label: Q2b
    unanswered: true
comment: None of these are the real question. Why hold the connection at all?
",
    )
    .unwrap();

    assert!(
        response.answers.iter().all(|answer| !answer.is_answer()),
        "a counter-question carries no Answers"
    );
    response
        .validate(&set())
        .expect("zero Answers plus a comment is the counter-question case");
}

#[test]
fn an_unknown_field_is_a_parse_error() {
    let error = Response::from_yaml(
        "
answers:
  - label: Q1
    selectd: 1
",
    )
    .expect_err("a misspelled field should not be silently ignored");

    assert!(
        error.to_string().contains("selectd"),
        "the error should name the unknown field, got: {error}"
    );
}

#[test]
fn a_missing_question_is_rejected_naming_it() {
    let error = Response::from_yaml(
        "
answers:
  - label: Q1
    selected: 1
  - label: Q2a
    selected: 1
",
    )
    .unwrap()
    .validate(&set())
    .expect_err("every question appears, one way or the other");

    assert!(
        error.names("Q2b"),
        "the error should name the missing Q2b, got: {error}"
    );
}

#[test]
fn a_heading_takes_no_entry_of_its_own() {
    let error = Response::from_yaml(
        "
answers:
  - label: Q1
    selected: 1
  - label: Q2
    free_text: Answered the heading and stopped there.
  - label: Q2a
    selected: 1
  - label: Q2b
    unanswered: true
",
    )
    .unwrap()
    .validate(&set())
    .expect_err("Q2 heads its Sub-questions and asks nothing");

    assert!(error.names("Q2"), "got: {error}");
    assert!(
        error.to_string().contains("heads its Sub-questions"),
        "the refusal should say why there is nothing to answer, got: {error}"
    );
}

#[test]
fn a_response_that_passes_over_a_heading_entirely_is_complete() {
    Response::from_yaml(
        "
answers:
  - label: Q1
    selected: 1
  - label: Q2a
    selected: 1
  - label: Q2b
    unanswered: true
",
    )
    .unwrap()
    .validate(&set())
    .expect("a Heading is not missing from a Response that never had to carry it");
}

#[test]
fn a_missing_sub_question_is_rejected_even_where_its_heading_is_passed_over() {
    let error = Response::from_yaml(
        "
answers:
  - label: Q1
    selected: 1
",
    )
    .unwrap()
    .validate(&set())
    .expect_err("Sub-questions are questions in their own right");

    assert!(error.names("Q2a") && error.names("Q2b"), "got: {error}");
}

#[test]
fn an_empty_response_names_every_question_it_missed() {
    let error = Response::from_yaml("comment: Not answering any of this.")
        .unwrap()
        .validate(&set())
        .expect_err("explicitness means every question appears");

    // Three, not four: the Heading is not among the questions a Response has to
    // account for, so there is nothing missing where it stands.
    assert_eq!(error.violations.len(), 3, "got: {error}");
    assert!(error.names("Q1") && error.names("Q2a") && error.names("Q2b"));
    assert!(
        !error.names("Q2"),
        "a Heading is never missing, got: {error}"
    );
}

#[test]
fn an_answer_naming_no_question_in_the_set_is_rejected() {
    let error = complete_but(
        "
  - label: Q9
    selected: 1
",
    )
    .expect_err("an Answer to nothing is a mistake, not a spare part");

    assert!(error.names("Q9"), "got: {error}");
}

#[test]
fn answering_the_same_question_twice_is_rejected() {
    let error = complete_but(
        "
  - label: Q1
    selected: 2
",
    )
    .expect_err("one entry per question");

    assert!(error.names("Q1"), "got: {error}");
}

#[test]
fn a_question_that_is_neither_answered_nor_marked_unanswered_is_rejected() {
    let error = Response::from_yaml(
        "
answers:
  - label: Q1
  - label: Q2a
    selected: 1
  - label: Q2b
    unanswered: true
",
    )
    .unwrap()
    .validate(&set())
    .expect_err("leaving a question open has to be said out loud");

    assert!(error.names("Q1"), "got: {error}");
}

#[test]
fn blank_free_text_is_not_an_answer() {
    let error = Response::from_yaml(
        "
answers:
  - label: Q1
    free_text: '   '
  - label: Q2a
    selected: 1
  - label: Q2b
    unanswered: true
",
    )
    .unwrap()
    .validate(&set())
    .expect_err("whitespace is not an Answer");

    assert!(error.names("Q1"), "got: {error}");
}

#[test]
fn an_answer_that_is_also_marked_unanswered_is_rejected() {
    let error = Response::from_yaml(
        "
answers:
  - label: Q1
    selected: 1
    unanswered: true
  - label: Q2a
    selected: 1
  - label: Q2b
    unanswered: true
",
    )
    .unwrap()
    .validate(&set())
    .expect_err("a question is answered or left open, not both");

    assert!(error.names("Q1"), "got: {error}");
}

#[test]
fn selecting_an_option_the_question_does_not_offer_is_rejected() {
    let error = Response::from_yaml(
        "
answers:
  - label: Q1
    selected: 7
  - label: Q2a
    selected: 1
  - label: Q2b
    unanswered: true
",
    )
    .unwrap()
    .validate(&set())
    .expect_err("Q1 offers Options 1 and 2");

    assert!(error.names("Q1"), "got: {error}");
    assert!(
        error.to_string().contains("1, 2"),
        "the error should say what is on offer, got: {error}"
    );
}

#[test]
fn selecting_an_option_on_a_question_with_none_is_rejected() {
    let error = Response::from_yaml(
        "
answers:
  - label: Q1
    selected: 1
  - label: Q2a
    selected: 1
  - label: Q2b
    selected: 1
",
    )
    .unwrap()
    .validate(&set())
    .expect_err("Q2b is free text only");

    assert!(error.names("Q2b"), "got: {error}");
}

#[test]
fn free_text_alone_answers_a_question_that_offers_options() {
    Response::from_yaml(
        "
answers:
  - label: Q1
    free_text: Neither; make it configurable.
  - label: Q2a
    selected: 2
    free_text: But say why in the body.
  - label: Q2b
    unanswered: true
",
    )
    .unwrap()
    .validate(&set())
    .expect("the human may always answer in their own words");
}

#[test]
fn every_violation_is_reported_at_once() {
    let error = Response::from_yaml(
        "
answers:
  - label: Q1
    selected: 7
  - label: Q2
    selected: 1
    unanswered: true
  - label: Q9
    selected: 1
",
    )
    .unwrap()
    .validate(&set())
    .unwrap_err();

    // Q1 selects an Option that is not offered, Q2 is a Heading and takes no
    // entry, Q9 is not in the Set, and Q2a and Q2b are missing entirely.
    assert_eq!(
        error.violations.len(),
        5,
        "the human should learn about every problem in one round trip, got: {error}"
    );
}

#[test]
fn a_multi_line_comment_round_trips_through_yaml() {
    let response = Response::from_yaml(
        "
answers:
  - label: Q1
    selected: 1
  - label: Q2a
    selected: 1
  - label: Q2b
    unanswered: true
comment: |
  Two paragraphs, the second with `code` and a \"quote\".

      an indented line
",
    )
    .unwrap();

    let yaml = response.to_yaml().expect("a Response should serialise");
    let reparsed = Response::from_yaml(&yaml).expect("our own YAML should parse");

    assert_eq!(reparsed, response);
    assert!(
        yaml.contains("comment: |"),
        "a multi-line comment should ride in a block scalar, got:\n{yaml}"
    );
}

/// [`SET`] closed with a wrap-up proposal on it, which is what puts the
/// direction chooser on the page and so what makes a pick something to answer
/// with.
fn proposing() -> QuestionSet {
    QuestionSet::from_yaml(&format!(
        "{SET}proposal:\n  direction: task-list\n  rationale: Five loosely coupled changes.\n"
    ))
    .expect("the proposing Set should parse")
}

/// The pick, as it comes back on the Response.
///
/// A field of its own rather than an entry in `answers`, because it answers no
/// Question: the chooser is the viewer's, and the three directions it offers are
/// the same three every time.
#[test]
fn a_response_carries_the_direction_the_human_picked() {
    let picked = Response::from_yaml(
        "
answers:
  - label: Q1
    unanswered: true
  - label: Q2a
    unanswered: true
  - label: Q2b
    unanswered: true
direction: roadmap
",
    )
    .unwrap();

    assert_eq!(picked.direction, Some(verkstead_schema::Direction::Roadmap));
    picked
        .validate(&proposing())
        .expect("a pick on a Set that carries a proposal is what accepting is");
}

/// And a Response without one, which is how the human sends a proposal back.
#[test]
fn a_response_that_picks_nothing_resolves_a_proposal_set_all_the_same() {
    let sent_back = Response::from_yaml(
        "
answers:
  - label: Q1
    selected: 1
  - label: Q2a
    unanswered: true
  - label: Q2b
    free_text: Not until the migration is settled.
",
    )
    .unwrap();

    assert_eq!(sent_back.direction, None);
    sent_back
        .validate(&proposing())
        .expect("nothing about a Response obliges it to pick");
}

#[test]
fn a_pick_round_trips_through_yaml() {
    let response = Response::from_yaml("answers: []\ndirection: task-list\n").unwrap();
    let reparsed = Response::from_yaml(&response.to_yaml().unwrap()).unwrap();

    assert_eq!(reparsed, response);
}

/// A pick on a Set with no chooser on it is a direction nobody offered.
///
/// Refused rather than dropped on the way past, so whoever sent it hears about
/// it: nothing would act on one, and a Response quietly losing half of what it
/// said is worse than one turned away.
#[test]
fn a_pick_on_a_set_that_carries_no_proposal_is_refused() {
    let error = Response::from_yaml(
        "
answers:
  - label: Q1
    selected: 1
  - label: Q2a
    selected: 1
  - label: Q2b
    unanswered: true
direction: inline
",
    )
    .unwrap()
    .validate(&set())
    .expect_err("the chooser is drawn on the closing Set alone");

    assert!(
        error.to_string().contains("proposal"),
        "the refusal should say why there was nothing to pick, got: {error}"
    );
}

/// The complete Response to [`SET`], with `extra` appended to its answers.
fn complete_but(extra: &str) -> Result<(), verkstead_schema::ValidationError> {
    let response = Response::from_yaml(&format!(
        "
answers:
  - label: Q1
    selected: 1
  - label: Q2a
    selected: 1
  - label: Q2b
    unanswered: true
{extra}"
    ))
    .unwrap();

    response.validate(&set())
}
