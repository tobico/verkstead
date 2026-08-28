//! The numbered checkbox list both of a repository's plan files are written as,
//! and the one place that knows how to read a line of one.
//!
//! `.tasks/TODO.md` and `docs/roadmaps/<name>/ROADMAP.md` write the same line —
//! `- [ ] 01: <title> — [link](NN-<slug>.md)` — because the skills that produce
//! them are forks of each other. A ticked box is what says the thing is done in
//! both, the document it names staying where it is either way. What the mark
//! means beyond that stays each reader's own business; what does not differ is
//! the line, so it is read once here rather than twice over there.

/// How an entry can be written: to do, done, and done by whoever holds the
/// shift key.
///
/// A line has to carry one of them to be an entry at all, because that is what
/// a checkbox list is.
const MARKS: [(&str, bool); 3] = [("- [ ] ", false), ("- [x] ", true), ("- [X] ", true)];

/// One entry of a numbered checkbox list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Entry<'a> {
    /// The number it answers to, for matching a file against.
    pub(crate) number: u32,

    /// That number as the list writes it — `01`. Zero-padding is the list's
    /// own, and a Timeline that renumbered a plan would be showing its own list
    /// rather than the repository's.
    pub(crate) label: &'a str,

    /// What it is called, without the link to the file it details.
    pub(crate) title: &'a str,

    /// What the link points at — `04-wrap-up.md` — or empty where the entry
    /// carries no link.
    ///
    /// The target rather than the whole link, because that is the only part
    /// anything does anything with: a roadmap's entry points at the brief the
    /// stage is started from, which is a file to go and read.
    pub(crate) link: &'a str,

    /// Whatever the list wrote after the link, as it wrote it — `*(in progress:
    /// `wrap-up`)*`.
    ///
    /// The roadmap keeping its own score rather than anything about what the
    /// stage is called, which is why it is not part of the title. Empty on the
    /// ordinary entry, which is every entry of a backlog and most of a roadmap's.
    pub(crate) after: &'a str,

    /// Whether the box is ticked. What that says is the reader's to decide —
    /// see the module docs.
    pub(crate) checked: bool,
}

/// The entry `line` is, or `None` where it is not one.
///
/// Anything else in the file — the description above the list, a heading, a
/// checkbox that is not numbered — is not an entry.
pub(crate) fn entry(line: &str) -> Option<Entry<'_>> {
    let line = line.trim();

    let (rest, checked) = MARKS
        .iter()
        .find_map(|(mark, checked)| Some((line.strip_prefix(mark)?, *checked)))?;

    let (label, title) = rest.split_once(':')?;
    let label = label.trim();

    if label.is_empty() || !label.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }

    let (title, link, after) = split(title);

    Some(Entry {
        number: label.parse().ok()?,
        label,
        title,
        link,
        after,
        checked,
    })
}

/// The heading a plan file leads with, which is what the thing it plans is
/// called.
///
/// Empty where there is none. The Timeline says what the Event is either way —
/// this is the name of the work, not the name of the list.
pub(crate) fn heading(list: &str) -> String {
    list.lines()
        .find_map(|line| line.trim().strip_prefix("# "))
        .unwrap_or_default()
        .trim()
        .to_owned()
}

/// The three parts of what follows an entry's number: what it is called, what
/// it links to, and whatever the list wrote after that.
///
/// The link is how the list points at the file the entry details, and a title
/// reading `Commit Timeline Events — [details](03-commit-events.md)` would be a
/// line nobody wrote. The dash before it goes too, because it was punctuation
/// joining the two rather than part of either.
///
/// The title is cut at the link rather than having the link taken out of it,
/// because what follows one is not part of the title either: a roadmap entry
/// annotates the stage in flight after its link — `— [brief](04-wrap-up.md)
/// *(in progress: `wrap-up`)*` — and that is the roadmap keeping score rather
/// than what the stage is called. It comes back separately for whoever is
/// reading the score.
///
/// The link is found by its shape rather than by the word inside it: `TODO.md`
/// writes `[details]` and `ROADMAP.md` writes `[brief]`. A title carrying an
/// ordinary bracketed aside keeps it, there being no link in one.
fn split(rest: &str) -> (&str, &str, &str) {
    let rest = rest.trim();

    let Some(open) = rest.find("](").and_then(|link| rest[..link].rfind('[')) else {
        return (rest, "", "");
    };

    let title = rest[..open].trim().trim_end_matches(['—', '–', '-']).trim();

    // The target is between the `](` and the `)` that closes it. A link left
    // unclosed is not one, and what would have been the title is still the
    // title.
    let Some((target, after)) = rest[open..]
        .split_once("](")
        .and_then(|(_, target)| target.split_once(')'))
    else {
        return (title, "", "");
    };

    (title, target.trim(), after.trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_entry_is_a_number_a_title_and_a_box() {
        let entry = entry("- [ ] 03: The pinned task-list Event — [details](03-pinned.md)")
            .expect("that is an entry");

        assert_eq!(entry.number, 3);
        assert_eq!(entry.label, "03");
        assert_eq!(entry.title, "The pinned task-list Event");
        assert_eq!(entry.link, "03-pinned.md");
        assert_eq!(entry.after, "");
        assert!(!entry.checked);
    }

    /// Both spellings of a ticked box, because both are written by hand.
    #[test]
    fn a_ticked_box_is_ticked_whichever_way_it_was_typed() {
        for ticked in ["- [x] 01: A stage", "- [X] 01: A stage"] {
            assert!(entry(ticked).expect("that is an entry").checked);
        }
    }

    /// The two plan files link with different words, and neither is part of a
    /// title.
    #[test]
    fn the_link_goes_whichever_word_is_inside_it() {
        assert_eq!(
            entry("- [ ] 01: Workbench — [brief](01-workbench.md)")
                .unwrap()
                .title,
            "Workbench"
        );
        assert_eq!(
            entry("- [ ] 01: Workbench — [details](01-workbench.md)")
                .unwrap()
                .title,
            "Workbench"
        );
    }

    /// A roadmap keeps its own score, and the stage in flight is annotated
    /// after its link. That is the roadmap saying where the effort has got, not
    /// what the stage is called.
    #[test]
    fn what_a_roadmap_writes_after_the_link_is_not_the_title() {
        let entry = entry("- [ ] 04: Wrap-up — [brief](04-wrap-up.md) *(in progress: `wrap-up`)*")
            .expect("that is an entry");

        assert_eq!(entry.title, "Wrap-up");
        assert_eq!(entry.link, "04-wrap-up.md");
        assert_eq!(
            entry.after, "*(in progress: `wrap-up`)*",
            "the roadmap's own score, kept for whoever is reading it",
        );
    }

    #[test]
    fn a_title_carrying_no_link_is_taken_as_it_stands() {
        for (line, title) in [
            (
                "- [ ] 01: A task — with a dash in it",
                "A task — with a dash in it",
            ),
            ("- [ ] 01: A task (with an aside)", "A task (with an aside)"),
            // A link that never closes is not one, and cutting the title at it
            // would be reading half a link as a whole one.
            ("- [ ] 01: A task — [details](01-task.md", "A task"),
        ] {
            let entry = entry(line).expect("that is an entry");

            assert_eq!(entry.title, title);
            assert_eq!(entry.link, "", "{line:?} points at nothing");
        }
    }

    #[test]
    fn only_numbered_checkboxes_are_entries() {
        for refused in [
            "- [ ] not a task at all",
            "- a plain bullet",
            "# A heading",
            "- [ ] 01 no colon",
            "- [ ] : nothing before the colon",
            "",
        ] {
            assert_eq!(entry(refused), None, "{refused:?} should not be an entry");
        }
    }

    #[test]
    fn the_heading_is_what_the_plan_calls_itself() {
        assert_eq!(heading("# MVP roadmap\n\nSome prose.\n"), "MVP roadmap");
        assert_eq!(heading("No heading at all.\n"), "");
    }
}
