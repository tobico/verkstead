//! One Set and its Answers, written out as the markdown a prompt carries.
//!
//! Two things prime a session with an exchange the human has already had, and
//! they are the same writing job: a relaunched grilling, which is owed
//! everything already settled (see [`crate::grillings`]), and a Deferred Ask
//! whose Answers came back after the session that asked had finished (see
//! [`crate::deferrals`]). What differs between them is which Sets they gather
//! and what they say over the top; what a Set and its Response read as in a
//! prompt is one thing, and it is written here once.

use verkstead_schema::{QuestionOption, QuestionSet, Response};

/// What is written where the human left a question open, and where nothing came
/// back for one at all.
///
/// Said rather than left blank, because the two readings are opposite: a
/// question with nothing under it reads as a question nobody put, and this is a
/// question the human saw and chose not to settle. Whichever session reads it is
/// welcome to ask again, and this is what tells it that it may.
const LEFT_OPEN: &str = "_Left open._";

/// One Set and its Answers: what it was called, each question against what
/// became of it, and whatever the human said about the whole of it.
///
/// The agent's own markdown, kept as it was written. What this is going into is
/// a prompt rather than a table on a phone, so the question that was asked with
/// a code block in it is worth having with the code block still in it.
pub(crate) fn exchange(set: &QuestionSet, response: &Response) -> String {
    let mut said = format!("## {}\n\n", set.title.trim());

    for question in &set.questions {
        said.push_str(&format!(
            "**{}** {}\n\n",
            question.name(),
            question.text.trim()
        ));

        // A Heading asks nothing of its own — it heads its Sub-questions — so no
        // Answer ever comes back for one, and nothing is written under it.
        if !question.heading() {
            said.push_str(&format!(
                "{}\n\n",
                decided(response, question.name(), &question.options)
            ));
        }

        for subquestion in &question.subquestions {
            let name = subquestion.name(question);

            said.push_str(&format!(
                "**{name}** {}\n\n{}\n\n",
                subquestion.text.trim(),
                decided(response, &name, &subquestion.options)
            ));
        }
    }

    if let Some(comment) = response
        .comment
        .as_deref()
        .map(str::trim)
        .filter(|comment| !comment.is_empty())
    {
        said.push_str(&format!("**About the Set as a whole** {comment}\n\n"));
    }

    said
}

/// What became of one question: the Option that was chosen, whatever the human
/// wrote, or both.
///
/// The Timeline's own reading of an Answer — see `verkstead_render`'s — with the
/// markdown left in and the empty case spoken aloud. Both differences are the
/// reader: that one is a table a human skims, and this is a paragraph an agent
/// is being brought up to speed by.
fn decided(response: &Response, name: &str, options: &[QuestionOption]) -> String {
    let Some(answer) = response
        .answers
        .iter()
        .find(|answer| answer.label.trim() == name)
    else {
        return LEFT_OPEN.to_owned();
    };

    let chosen = answer
        .selected
        .and_then(|n| options.iter().find(|option| option.n == n))
        .map(|option| option.text.trim());

    let wrote = answer
        .free_text
        .as_deref()
        .map(str::trim)
        .filter(|wrote| !wrote.is_empty());

    // Both where the human picked an Option and said why, which is the ordinary
    // shape of an Answer that carries a qualification.
    match (chosen, wrote) {
        (Some(chosen), Some(wrote)) => format!("{chosen} — {wrote}"),
        (Some(only), None) | (None, Some(only)) => only.to_owned(),
        (None, None) => LEFT_OPEN.to_owned(),
    }
}
