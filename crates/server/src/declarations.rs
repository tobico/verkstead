//! What a roadmap's stage line declares: the stages it stands on, and the
//! platform it wants.
//!
//! `ROADMAP.md` writes both after the link to the brief, which is the place
//! `checklist::Entry::after` already hands back whole —
//!
//! ```text
//! - [ ] 04: The scheduler — [brief](04-the-scheduler.md) — after 01, 03
//! - [ ] 01: Dependencies — [brief](01-dependencies.md) — no dependencies
//! - [ ] 05: Mac packaging — [brief](05-packaging.md) — after 04 — on macos
//! ```
//!
//! — and this is the reading of it. See
//! [ADR-0021](../../../../docs/adr/0021-parallel-stages.md), which settled the
//! grammar and why it is on the line.
//!
//! **Only the reading.** Whether the labels name stages of this roadmap, whether
//! the platform is one of the three words there are, whether a roadmap declaring
//! on some lines and not others is a roadmap at all: all of that is the judging,
//! and it refuses in words a human can act on rather than seeing nothing there.
//! So a word that is not a platform still reads as a platform naming that word,
//! and an `after` naming nothing still reads as a declaration.
//!
//! **The tail has a tenant already**, the in-flight annotation `*(in progress:
//! `branch`)*`, and the two share one line in either order. The annotation is
//! matched by the branch in backticks — see `stages::ours` — and a declaration
//! holds none, which is what keeps the two apart from that side; from this side
//! what keeps them apart is that nothing inside an annotation is read at all.
//!
//! Nothing here is a backlog's. `.tasks/TODO.md` has no dependencies — the order
//! is the dependency — and the one line reader stays the one line reader: a task
//! list's tail is whatever it always was.

/// What a stage's line says it stands on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum StandsOn<'a> {
    /// `no dependencies`: a root. The human's own wording, chosen over `after
    /// nothing`.
    Nothing,

    /// `after 01, 03`: other stages of the same roadmap, by their labels **as
    /// the roadmap writes them** — zero-padding and all, because that is what
    /// the roadmap's own lines answer to and forgiving `1` for `01` here would
    /// be this reading deciding something the judging says out loud.
    ///
    /// Empty where the line said `after` and named nobody, which is a
    /// declaration standing on a list of nothing rather than a root: the
    /// wording for a root is the other one, and the difference is the judging's
    /// to refuse.
    Stages(Vec<&'a str>),
}

/// The tail of a stage line, read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Reading<'a> {
    /// What the stage stands on, or `None` where the line declares nothing —
    /// which is every line of every roadmap written before any of this, and is
    /// a roadmap run in order.
    ///
    /// A platform on its own does not declare: a line whose whole tail is `on
    /// windows` is undeclared with a platform, because a forgotten declaration
    /// hiding behind a platform is exactly what the all-or-nothing rule is for.
    pub(crate) stands_on: Option<StandsOn<'a>>,

    /// The platform the line names, as it named it — `windows` of `on windows` —
    /// or `None` where it named none.
    ///
    /// There are three words it can be, `linux`, `macos` and `windows`, and this
    /// is not where they are checked: see the module docs. Read, recorded and
    /// shown, and acted on by nothing — placing a stage on a device that matches
    /// is a follow-up once cluster mode has landed, and cluster mode's own
    /// free-text *Linux (WSL)* for drawing an icon beside a hostname is a
    /// different thing that neither of them should be made to serve.
    pub(crate) platform: Option<&'a str>,
}

/// The words that start something, and so end the list an `after` was
/// collecting: `after 01, 03 — on windows` names two stages and a platform
/// rather than three stages.
const STARTS: [&str; 3] = ["after", "no", "on"];

/// What `after` — the raw tail of a stage line — declares.
///
/// Reads the words rather than a shape, because what joins them is the
/// roadmap's own punctuation: a dash before the declaration, commas inside an
/// `after`, and whichever of those a human types when they add a platform by
/// hand.
pub(crate) fn read(after: &str) -> Reading<'_> {
    let mut stands_on = None;
    let mut platform = None;

    let mut words = outside(after).into_iter().flat_map(words).peekable();

    while let Some(word) = words.next() {
        if is(word, "after") {
            let mut stages = Vec::new();

            while let Some(next) = words.peek() {
                if STARTS.iter().any(|start| is(next, start)) {
                    break;
                }

                stages.push(*next);
                words.next();
            }

            stands_on = Some(StandsOn::Stages(stages));
        } else if is(word, "no") && words.peek().is_some_and(|next| is(next, "dependencies")) {
            words.next();
            stands_on = Some(StandsOn::Nothing);
        } else if is(word, "on") {
            platform = words.next();
        }
    }

    Reading {
        stands_on,
        platform,
    }
}

/// Whether `word` is `keyword`, whatever case it was typed in.
///
/// The roadmaps and the skills write these in lower case, and a line a human
/// started with a capital is still the line they meant.
fn is(word: &str, keyword: &str) -> bool {
    word.eq_ignore_ascii_case(keyword)
}

/// The pieces of the tail that are outside an annotation.
///
/// The annotation is `*(in progress: `branch`)*` and nothing in it is read. The
/// words around the branch are prose a human may rewrite — `*(in progress:
/// `grilling`, after a rebase)*` — and what is inside an aside about whose the
/// stage is has nothing to say about what the stage stands on.
///
/// An annotation that never closes takes the rest of the line with it. What
/// follows an unclosed `*(` is inside something nobody finished writing, and a
/// declaration read out of that would be a guess.
fn outside(after: &str) -> Vec<&str> {
    let mut pieces = Vec::new();
    let mut rest = after;

    while let Some(open) = rest.find("*(") {
        pieces.push(&rest[..open]);

        let Some(close) = rest[open..].find(")*") else {
            return pieces;
        };

        rest = &rest[open + close + ")*".len()..];
    }

    pieces.push(rest);
    pieces
}

/// What a piece of the tail joins its words with, beside whitespace: the dash
/// before a declaration, and the commas inside an `after`.
const JOINERS: [char; 4] = [',', '—', '–', ';'];

/// The words of one piece of the tail.
fn words(piece: &str) -> impl Iterator<Item = &str> {
    piece
        .split(|c: char| c.is_whitespace() || JOINERS.contains(&c))
        .filter(|word| !word.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The three things a line can say, each on its own.
    #[test]
    fn a_line_declares_what_it_stands_on_and_where_it_wants_to_run() {
        assert_eq!(
            read("— after 01, 03"),
            Reading {
                stands_on: Some(StandsOn::Stages(vec!["01", "03"])),
                platform: None,
            },
            "the labels as the roadmap writes them, zero-padding and all",
        );

        assert_eq!(
            read("— no dependencies"),
            Reading {
                stands_on: Some(StandsOn::Nothing),
                platform: None,
            },
        );

        assert_eq!(
            read("— after 04 — on windows"),
            Reading {
                stands_on: Some(StandsOn::Stages(vec!["04"])),
                platform: Some("windows"),
            },
        );
    }

    /// Which is every line of every roadmap written before any of this, and is
    /// what makes such a roadmap one run in order.
    #[test]
    fn a_line_saying_none_of_it_declares_nothing() {
        assert_eq!(
            read(""),
            Reading {
                stands_on: None,
                platform: None,
            },
        );
    }

    /// A platform on its own is not a declaration. A bare line there could be a
    /// root or could be the agent forgetting, and a platform in front of the gap
    /// does not make it one or the other.
    #[test]
    fn a_platform_on_its_own_leaves_the_line_undeclared() {
        assert_eq!(
            read("— on windows"),
            Reading {
                stands_on: None,
                platform: Some("windows"),
            },
        );
    }

    /// The declaration and the annotation share one tail, and which of them was
    /// written first is nobody's business.
    #[test]
    fn a_declaration_and_an_annotation_both_read_either_way_round() {
        let both = Reading {
            stands_on: Some(StandsOn::Stages(vec!["02"])),
            platform: Some("macos"),
        };

        assert_eq!(
            read("— after 02 — on macos *(in progress: `roadmaps/mvp/03-implementation`)*"),
            both,
        );
        assert_eq!(
            read("*(in progress: `roadmaps/mvp/03-implementation`)* — after 02 — on macos"),
            both,
        );
    }

    /// Nothing inside an annotation is read: the words around the branch are
    /// prose a human may rewrite, and an aside about whose the stage is has
    /// nothing to say about what the stage stands on.
    #[test]
    fn nothing_inside_an_annotation_declares_anything() {
        assert_eq!(
            read("*(in progress: `grilling`, after a rebase — on a whim)*"),
            Reading {
                stands_on: None,
                platform: None,
            },
        );

        // And an annotation nobody closed takes the rest of the line with it,
        // rather than the reading stepping out of it on a guess.
        assert_eq!(
            read("*(in progress: `x` — after 02"),
            Reading {
                stands_on: None,
                platform: None,
            },
        );
    }

    /// What joins the words is the roadmap's own punctuation, and a human adding
    /// a platform by hand types whichever of it comes to mind.
    #[test]
    fn the_punctuation_between_them_is_whatever_was_typed() {
        for tail in [
            "— after 01, 03, on windows",
            "- after 01 03 on windows",
            "after 01,03 – on windows",
        ] {
            assert_eq!(
                read(tail),
                Reading {
                    stands_on: Some(StandsOn::Stages(vec!["01", "03"])),
                    platform: Some("windows"),
                },
                "{tail:?}",
            );
        }
    }

    /// And the words themselves are read whatever case they were typed in,
    /// while the platform comes back as it was written: the keyword is grammar
    /// and the platform is a value, which is the judging's to match and the
    /// pane's to show.
    #[test]
    fn a_capital_is_still_the_line_they_meant() {
        assert_eq!(
            read("  —  After 01, 03; On Windows  "),
            Reading {
                stands_on: Some(StandsOn::Stages(vec!["01", "03"])),
                platform: Some("Windows"),
            },
        );
    }

    /// An `after` that names nobody is still a declaration, and so is a platform
    /// that is not one of the three words. Both are a roadmap the judging
    /// refuses by name — seeing nothing there instead would be this reading
    /// deciding it quietly.
    #[test]
    fn a_badly_written_declaration_is_still_one() {
        assert_eq!(
            read("— after"),
            Reading {
                stands_on: Some(StandsOn::Stages(Vec::new())),
                platform: None,
            },
        );

        assert_eq!(
            read("— no dependencies — on freebsd").platform,
            Some("freebsd")
        );

        // `no` on its own is not the root's wording, there being nothing to say
        // it was about dependencies at all.
        assert_eq!(read("— no").stands_on, None);
    }
}
