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
//! — and this is both the reading of it and the judging that stands on the
//! reading. See [ADR-0021](../../../../docs/adr/0021-parallel-stages.md), which
//! settled the grammar and why it is on the line.
//!
//! **The reading reads and judges nothing.** Whether the labels name stages of
//! this roadmap, whether the platform is one of the three words there are,
//! whether a roadmap declaring on some lines and not others is a roadmap at all:
//! all of that is [`judge`], which answers for a whole file at once and refuses
//! in words a human can act on rather than seeing nothing there. So a word that
//! is not a platform still *reads* as a platform naming that word, and an
//! `after` naming nothing still reads as a declaration.
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

use crate::checklist;
use crate::platform::Platform;

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
///
/// All three dashes, because a line typed by hand is typed with whichever one
/// came to hand — `checklist::split` trims the same three off the end of a title
/// for that reason. An ASCII hyphen left out of this would join nothing, and a
/// hand-written `— after 01 - on windows` would stand on `01` and on a stage
/// called `-`, refusing the roadmap over a label nobody wrote. Nothing a
/// declaration says is spelled with one: the labels are digits and the platforms
/// are three words.
const JOINERS: [char; 5] = [',', '-', '—', '–', ';'];

/// The words of one piece of the tail.
fn words(piece: &str) -> impl Iterator<Item = &str> {
    piece
        .split(|c: char| c.is_whitespace() || JOINERS.contains(&c))
        .filter(|word| !word.is_empty())
}

/// The judgement over a whole `ROADMAP.md`: what it declares, or why it is
/// refused.
///
/// One judgement with three callers, of which this stage builds the first. The
/// roadmap's own session is refused at `verkstead done` — see
/// [`crate::runner::lacking`], which reads a landed roadmap and asks this about
/// it. The other two are the scheduler's: a running roadmap that can start
/// nothing says why on the Timeline, and *Continue a roadmap* says so at the
/// press. They show the same sentences, so the sentences are written for a human
/// reading them anywhere.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Judgement {
    /// Not one line declares, which is every roadmap written before any of this.
    /// It runs strictly in order, exactly as before, and nothing refuses it —
    /// reading silence as *start everything at once* would start stages on top
    /// of the work they were written to follow.
    ///
    /// A roadmap whose index plans nothing is this too: there is no line to have
    /// declared, and nothing to refuse about a directory.
    Undeclared,

    /// Every line declares, and here is what they said: which stage stands on
    /// which, and the platform each of them wants.
    Declared(Vec<Declared>),

    /// The roadmap declares badly, with the fault in words a human can act on.
    ///
    /// Refused rather than repaired, and never run in order instead: falling
    /// back to running in order runs a roadmap in a way nobody wrote down.
    Refused(String),
}

/// One stage of a declaring roadmap, as the judgement hands it over: the labels
/// checked against the roadmap's own and the platform word matched to one of the
/// three there are.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Declared {
    /// Its label as the roadmap writes it — `03`.
    pub(crate) label: String,

    /// The stages it stands on, by their labels, and empty for a root: `no
    /// dependencies` and `after` a list of nothing are not the same line to
    /// write, but a root and a stage standing on nobody are the same thing to
    /// start. Every label here is a stage of this roadmap, that being the
    /// judging.
    pub(crate) stands_on: Vec<String>,

    /// The platform it wants, where it named one — the crate's own
    /// [`crate::platform::Platform`], which is what a device's OS will be read
    /// as when something finally places a stage by it. See [`PLATFORMS`].
    pub(crate) platform: Option<Platform>,
}

/// The three platforms a stage can ask for, by the word a roadmap writes.
///
/// Rust's own `target_os` names, which are what the repository's `#[cfg]`s and
/// its CI runners already say. Three words rather than free text, because the
/// thing that will eventually read this is a device's own OS and a typo there
/// would place a stage nowhere at all — cluster mode's free-text *Linux (WSL)*
/// for drawing an icon beside a hostname is a different thing, and neither of
/// them should be made to serve the other.
///
/// And [`crate::platform::Platform`] rather than three of our own, it being the
/// crate's name for an operating system already: what a declared platform is for
/// is being matched against the OS of a device, and
/// [`crate::platform::Platform::HERE`] is exactly that. A second enum of the
/// same three would be a conversion waiting to be written.
const PLATFORMS: [(&str, Platform); 3] = [
    ("linux", Platform::Linux),
    ("macos", Platform::MacOs),
    ("windows", Platform::Windows),
];

/// The platform `word` names, or `None` where it names none of the three.
fn matching(word: &str) -> Option<Platform> {
    PLATFORMS
        .iter()
        .find_map(|(named, platform)| is(word, named).then_some(*platform))
}

/// What `list` — a whole `ROADMAP.md` — declares, judged as one thing.
///
/// All or nothing, because a bare line in a declaring roadmap is a root and a
/// forgotten declaration at once and there is no telling which. So the first
/// question is how many lines declare: none is [`Judgement::Undeclared`], all of
/// them is the graph, and some of them is the first of the four faults.
///
/// The faults are looked for in the order a human would fix them: whether every
/// line declares, then whether the labels name stages that are there, then
/// whether the platforms are platforms, then whether what is left is a graph
/// anything could start. One fault at a time, named where it is — a roadmap with
/// two of them is refused over the first and refused again over the second, which
/// is a line to go and read either way.
///
/// **An undeclared roadmap is looked at no further**, platform words and all. A
/// tail written before any of this is prose, and a word that happens to follow an
/// `on` in it is not a platform somebody typed wrong: refusing over one would be
/// this changing what a roadmap written before it means, which is the one thing
/// every part of this promises not to do.
///
/// `roadmap` is the directory name, for saying which roadmap in the sentence:
/// the Timeline shows these beside a Conversation that may be a stage of one of
/// several.
pub(crate) fn judge(roadmap: &str, list: &str) -> Judgement {
    let lines: Vec<(checklist::Entry<'_>, Reading<'_>)> = list
        .lines()
        .filter_map(checklist::entry)
        .map(|entry| (entry, read(entry.after)))
        .collect();

    if lines.iter().all(|(_, read)| read.stands_on.is_none()) {
        return Judgement::Undeclared;
    }

    if let Some((entry, _)) = lines.iter().find(|(_, read)| read.stands_on.is_none()) {
        return Judgement::Refused(format!(
            "the {roadmap} roadmap declares what some of its stages stand on and not others: \
             stage {} declares nothing after its link. Every line of a declaring roadmap carries \
             a declaration — `no dependencies` where the stage stands on nothing — because a bare \
             line cannot be told from a forgotten one",
            entry.label,
        ));
    }

    let labels: Vec<&str> = lines.iter().map(|(entry, _)| entry.label).collect();

    for (entry, read) in &lines {
        let Some(StandsOn::Stages(stages)) = &read.stands_on else {
            continue;
        };

        if stages.is_empty() {
            return Judgement::Refused(format!(
                "stage {} of the {roadmap} roadmap says `after` and names no stage. A stage that \
                 stands on nothing says `no dependencies`",
                entry.label,
            ));
        }

        if let Some(named) = stages.iter().find(|named| !labels.contains(named)) {
            return Judgement::Refused(format!(
                "stage {} of the {roadmap} roadmap says `after {named}`, and no stage of it is \
                 labelled {named}. A stage may only name stages of its own roadmap, by their \
                 labels as the roadmap writes them — zero-padding and all",
                entry.label,
            ));
        }
    }

    for (entry, read) in &lines {
        if let Some(word) = read.platform
            && matching(word).is_none()
        {
            return Judgement::Refused(format!(
                "stage {} of the {roadmap} roadmap says `on {word}`, which is not a platform: it \
                 is one of {}",
                entry.label,
                PLATFORMS
                    .iter()
                    .map(|(word, _)| format!("`{word}`"))
                    .collect::<Vec<_>>()
                    .join(", "),
            ));
        }
    }

    if let Some(cycle) = cycle(&lines) {
        return Judgement::Refused(match cycle.as_slice() {
            [only] => format!(
                "stage {only} of the {roadmap} roadmap stands on itself, which is a cycle of one: \
                 it could never start"
            ),
            _ => format!(
                "the {roadmap} roadmap has a cycle in what its stages stand on — {} — so none of \
                 them could ever start",
                cycle
                    .iter()
                    .zip(cycle.iter().cycle().skip(1))
                    .take(cycle.len())
                    .map(|(stage, stands_on)| format!("{stage} stands on {stands_on}"))
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
        });
    }

    Judgement::Declared(
        lines
            .iter()
            .map(|(entry, read)| Declared {
                label: entry.label.to_owned(),
                // The root and a stage standing on nobody come over the same
                // way — see [`Declared::stands_on`].
                stands_on: match &read.stands_on {
                    Some(StandsOn::Stages(stages)) => {
                        stages.iter().map(|named| (*named).to_owned()).collect()
                    }
                    _ => Vec::new(),
                },
                platform: read.platform.and_then(matching),
            })
            .collect(),
    )
}

/// One cycle of what the roadmap's stages stand on, by the labels in it, or
/// `None` where there is none.
///
/// Depth-first, from every stage in the order the roadmap lists them, so a
/// roadmap with two cycles in it is always refused over the same one: a refusal
/// that named a different pair each time it was asked would read as two faults
/// rather than one.
///
/// The labels come back in the order they stand on each other, starting where the
/// walk re-entered the cycle, and a stage naming itself is a cycle of one. Every
/// label is a stage of this roadmap by the time this is asked, so what a walk
/// cannot find is nothing to say anything about.
fn cycle<'a>(lines: &[(checklist::Entry<'a>, Reading<'a>)]) -> Option<Vec<&'a str>> {
    let mut walked: Vec<&str> = Vec::new();
    let mut open: Vec<&str> = Vec::new();

    for (entry, _) in lines {
        if let Some(cycle) = walk(entry.label, lines, &mut walked, &mut open) {
            return Some(cycle);
        }
    }

    None
}

/// The walk itself: `open` is the stages this path stands on the way down, and
/// `walked` the ones already seen through to the end.
fn walk<'a>(
    label: &'a str,
    lines: &[(checklist::Entry<'a>, Reading<'a>)],
    walked: &mut Vec<&'a str>,
    open: &mut Vec<&'a str>,
) -> Option<Vec<&'a str>> {
    if let Some(at) = open.iter().position(|seen| *seen == label) {
        return Some(open[at..].to_vec());
    }

    if walked.contains(&label) {
        return None;
    }

    open.push(label);

    let stands_on = lines
        .iter()
        .find(|(entry, _)| entry.label == label)
        .and_then(|(_, read)| match &read.stands_on {
            Some(StandsOn::Stages(stages)) => Some(stages.clone()),
            _ => None,
        })
        .unwrap_or_default();

    for named in stands_on {
        if let Some(cycle) = walk(named, lines, walked, open) {
            return Some(cycle);
        }
    }

    open.pop();
    walked.push(label);

    None
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
            // The ASCII hyphen among them, which is what a hand typing a
            // platform onto the end of a line reaches for: without it the `-`
            // would be collected as a stage of its own, and the roadmap refused
            // over a label nobody wrote.
            "- after 01, 03 - on windows",
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

    /// A roadmap's lines, as `staging` writes them: the heading, the prose above
    /// the list and the entries, each with whatever tail it was given.
    fn roadmap(lines: &[&str]) -> String {
        let mut list = String::from("# Parallel stages\n\nWhat the roadmap is for.\n\n");

        for (number, line) in lines.iter().enumerate() {
            list.push_str(&format!(
                "- [ ] {0:02}: Stage {0} — [brief]({0:02}-stage.md){line}\n",
                number + 1,
            ));
        }

        list
    }

    /// Every roadmap written before any of this: not one line declares, so it
    /// runs in order exactly as it did and nothing refuses it.
    #[test]
    fn a_roadmap_declaring_on_no_line_is_undeclared() {
        assert_eq!(
            judge(
                "parallel-stages",
                &roadmap(&["", "", " *(in progress: `x`)*"])
            ),
            Judgement::Undeclared,
        );

        // And so is a directory whose index plans nothing: there is no line to
        // have declared, and nothing to refuse about a directory.
        assert_eq!(
            judge("parallel-stages", "# Nothing planned\n"),
            Judgement::Undeclared,
        );

        // And a tail written before any of this is prose, whatever words are in
        // it: the roadmap declares nothing, so nothing in it is judged and a
        // word after an `on` is not a platform somebody typed wrong.
        assert_eq!(
            judge("mvp", &roadmap(&[" — waits on the API landing"])),
            Judgement::Undeclared,
        );
    }

    /// A declaring roadmap hands over the graph: what each stage stands on, and
    /// the platform it wants where it named one.
    #[test]
    fn a_declaring_roadmap_says_which_stage_stands_on_which() {
        assert_eq!(
            judge(
                "parallel-stages",
                &roadmap(&[
                    " — no dependencies",
                    " — after 01 — on windows",
                    " *(in progress: `roadmaps/parallel-stages/03-stage`)* — after 01, 02",
                ]),
            ),
            Judgement::Declared(vec![
                Declared {
                    label: "01".to_owned(),
                    stands_on: Vec::new(),
                    platform: None,
                },
                Declared {
                    label: "02".to_owned(),
                    stands_on: vec!["01".to_owned()],
                    platform: Some(Platform::Windows),
                },
                Declared {
                    label: "03".to_owned(),
                    stands_on: vec!["01".to_owned(), "02".to_owned()],
                    platform: None,
                },
            ]),
        );
    }

    /// A bare line in a declaring roadmap is a root and a forgotten declaration
    /// at once, so the roadmap is refused and the line is named.
    #[test]
    fn a_roadmap_declaring_on_some_lines_is_refused_naming_a_bare_one() {
        let Judgement::Refused(why) = judge(
            "parallel-stages",
            &roadmap(&[" — no dependencies", "", " — after 01"]),
        ) else {
            panic!("a roadmap declaring on some lines and not others is refused");
        };

        assert!(why.contains("stage 02"), "{why}");
        assert!(why.contains("no dependencies"), "{why}");
    }

    /// Labels match as the roadmap writes them, so `after 1` against a stage
    /// labelled `01` names nobody rather than matching.
    #[test]
    fn an_after_naming_no_stage_of_the_roadmap_is_refused_naming_both() {
        let Judgement::Refused(why) = judge(
            "parallel-stages",
            &roadmap(&[" — no dependencies", " — after 1"]),
        ) else {
            panic!("an `after` naming no stage of the roadmap is refused");
        };

        assert!(why.contains("stage 02"), "{why}");
        assert!(why.contains("after 1"), "{why}");

        // And `after` with nobody after it is the same fault said its own way:
        // a stage that stands on nothing has a wording of its own.
        let Judgement::Refused(why) =
            judge("parallel-stages", &roadmap(&[" — after", " — after 01"]))
        else {
            panic!("an `after` naming nothing at all is refused");
        };

        assert!(why.contains("stage 01"), "{why}");
        assert!(why.contains("no dependencies"), "{why}");
    }

    /// There are three platforms, and a word that is not one of them is the
    /// human's to correct rather than Verkstead's to ignore.
    #[test]
    fn a_platform_that_is_not_one_is_refused_naming_the_line_and_the_word() {
        let Judgement::Refused(why) = judge(
            "parallel-stages",
            &roadmap(&[" — no dependencies", " — after 01 — on freebsd"]),
        ) else {
            panic!("a platform that is not one of the three is refused");
        };

        assert!(why.contains("stage 02"), "{why}");
        assert!(why.contains("on freebsd"), "{why}");
        assert!(why.contains("`linux`"), "{why}");

        // The three of them, in whatever case they were typed: the keyword and
        // the value are read the same way round as everywhere else.
        assert!(matches!(
            judge(
                "parallel-stages",
                &roadmap(&[
                    " — no dependencies — on Linux",
                    " — after 01 — on macos",
                    " — after 01 — on windows",
                ]),
            ),
            Judgement::Declared(_),
        ));
    }

    /// A cycle is refused naming the stages in it, in the order they stand on
    /// each other.
    #[test]
    fn a_cycle_is_refused_naming_the_stages_in_it() {
        let Judgement::Refused(why) = judge(
            "parallel-stages",
            &roadmap(&[" — no dependencies", " — after 03", " — after 02"]),
        ) else {
            panic!("a cycle is refused");
        };

        assert!(why.contains("02 stands on 03"), "{why}");
        assert!(why.contains("03 stands on 02"), "{why}");
        // The stage outside the cycle is not dragged into the sentence.
        assert!(!why.contains("01 stands on"), "{why}");
    }

    /// And a stage naming itself is a cycle of one, said as the one thing it is.
    #[test]
    fn a_stage_standing_on_itself_is_a_cycle_of_one() {
        let Judgement::Refused(why) = judge(
            "parallel-stages",
            &roadmap(&[" — no dependencies", " — after 02"]),
        ) else {
            panic!("a stage standing on itself is refused");
        };

        assert!(why.contains("stage 02"), "{why}");
        assert!(why.contains("stands on itself"), "{why}");
    }

    /// A roadmap whose stages stand on each other in a chain, with two of them
    /// on the same one, is a roadmap something can run — the walk that looks for
    /// a cycle meets a stage twice and that is not one.
    #[test]
    fn a_diamond_is_not_a_cycle() {
        assert!(matches!(
            judge(
                "parallel-stages",
                &roadmap(&[
                    " — no dependencies",
                    " — after 01",
                    " — after 01",
                    " — after 02, 03",
                ]),
            ),
            Judgement::Declared(_),
        ));
    }

    /// The refusal names the roadmap, because what shows these sentences shows
    /// them beside a Conversation that may be a stage of any of several.
    #[test]
    fn a_refusal_says_which_roadmap_it_is_about() {
        let Judgement::Refused(why) = judge("mvp", &roadmap(&[" — after 04"])) else {
            panic!("a roadmap naming a stage that is not there is refused");
        };

        assert!(why.contains("mvp roadmap"), "{why}");
    }
}
