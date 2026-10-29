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

use crate::answer_files::OnSet;

/// What is written where the human left a question open, and where nothing came
/// back for one at all.
///
/// Said rather than left blank, because the two readings are opposite: a
/// question with nothing under it reads as a question nobody put, and this is a
/// question the human saw and chose not to settle. Whichever session reads it is
/// welcome to ask again, and this is what tells it that it may.
const LEFT_OPEN: &str = "_Left open._";

/// And what is written where the file *is* the Answer.
///
/// **A file alone is an Answer** — see [`verkstead_schema::Answer::is_answer`],
/// which is the reading a submission is checked by and the one the record of a
/// settled Set is drawn from. So a question the human answered by handing
/// something over must not read as one they passed over: [`LEFT_OPEN`] invites
/// the session reading this to ask again, and there is nothing here to ask.
///
/// It points at the line under it rather than naming the files, because
/// [`attached`] names them on that line and one of them said twice is one a
/// reader has to check against itself.
const HANDED_OVER: &str = "_Answered with what was attached._";

/// One Set and its Answers: what it was called, each question against what
/// became of it, whatever the human attached to it, and whatever they said about
/// the whole of it.
///
/// The agent's own markdown, kept as it was written. What this is going into is
/// a prompt rather than a table on a phone, so the question that was asked with
/// a code block in it is worth having with the code block still in it.
///
/// `files` is what was put on this Set's Answers, which the session reading the
/// digest never saw handed over: an exchange it is brought up to speed on is one
/// it was not there for, so what came with each decision is named under it — see
/// [`attached`]. Read off the record beside the Response rather than out of it,
/// exactly as a waiting agent's own Response is filled in — see
/// [`crate::answer_files::onto`].
pub(crate) fn exchange(set: &QuestionSet, response: &Response, files: OnSet<'_>) -> String {
    let mut said = format!("## {}\n\n", set.title.trim());

    for question in &set.questions {
        said.push_str(&format!(
            "**{}** {}\n\n",
            question.name(),
            question.text.trim()
        ));

        // A Heading asks nothing of its own — it heads its Sub-questions — so no
        // Answer ever comes back for one, and no file is ever put on one either.
        if !question.heading() {
            // Read once and used twice: what became of the question depends on
            // whether anything was handed over with it, and the line under it
            // says what.
            let paths = files.under(question.name());

            said.push_str(&format!(
                "{}\n\n{}",
                decided(response, question.name(), &question.options, &paths),
                attached(&paths),
            ));
        }

        for subquestion in &question.subquestions {
            let name = subquestion.name(question);
            let paths = files.under(&name);

            said.push_str(&format!(
                "**{name}** {}\n\n{}\n\n{}",
                subquestion.text.trim(),
                decided(response, &name, &subquestion.options, &paths),
                attached(&paths),
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

    // And what was put at the comment box, which answers no Question: named
    // after whatever the human said about the whole, the way the box draws it.
    said.push_str(&attached(&files.on_the_set()));

    said
}

/// And what the human put on that Answer, named by the path the session reading
/// this opens each file at.
///
/// Under the decision rather than beside it, because that is what it is: a file
/// handed over with an Answer is part of the answer, and a session primed with
/// the exchange has to be able to open it. The path alone — how large it is is
/// in the prompt's own `# Attached files` listing, which names every one of them
/// again under the Set and the Question.
///
/// Nothing at all where the Answer carried none, which is nearly every one of
/// them: a line saying so would be a paragraph per question about files that
/// were never there.
fn attached(paths: &[String]) -> String {
    if paths.is_empty() {
        return String::new();
    }

    let named: Vec<String> = paths.iter().map(|path| format!("`{path}`")).collect();

    format!("_Attached:_ {}\n\n", named.join(", "))
}

/// What became of one question: the Option that was chosen, whatever the human
/// wrote, or both.
///
/// The Timeline's own reading of an Answer — see `verkstead_render`'s — with the
/// markdown left in and the empty case spoken aloud. Both differences are the
/// reader: that one is a table a human skims, and this is a paragraph an agent
/// is being brought up to speed by.
///
/// `paths` is what the human handed over with it, which is an Answer on its own
/// — see [`HANDED_OVER`]. It speaks only for the entry that carries nothing
/// else: an Option or words are what became of the question whether or not a
/// file came with them, and a question marked `unanswered` is one the human
/// left open on purpose, file or no file. Which is the reading the check a
/// submission goes through gives it, and the one the record of a settled Set is
/// drawn by.
///
/// An entry that is not there at all reads as one carrying nothing, so a file
/// still speaks for it: what a Set with an entry missing is is a Response no
/// submission could have made, and the file is the one thing about it that is
/// certainly true.
fn decided(
    response: &Response,
    name: &str,
    options: &[QuestionOption],
    paths: &[String],
) -> String {
    let handed_over = if paths.is_empty() {
        LEFT_OPEN
    } else {
        HANDED_OVER
    };

    let Some(answer) = response
        .answers
        .iter()
        .find(|answer| answer.label.trim() == name)
    else {
        return handed_over.to_owned();
    };

    if answer.unanswered {
        return LEFT_OPEN.to_owned();
    }

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
        (None, None) => handed_over.to_owned(),
    }
}
