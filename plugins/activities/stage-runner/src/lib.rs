//! **The stage runner**, a built-in Activity (ADR-0030, *Activity*): a roadmap
//! under `docs/roadmaps/<name>/` read, and every stage of it that is ready
//! started as a Conversation of its own.
//!
//! **Started whenever a stage could start, and ending once it has started what
//! it can** (Set 1839, Q1): a stage's wrap-up settling, a place on the server
//! coming free, a home device hearing a stage away settle. It is never a
//! long-lived Activity waiting on its stages, and the host says which
//! Conversation it reads from — the one that settled, or the foot of the chain —
//! and which roadmap that is a stage of, as the `roadmap` option.
//!
//! **It reaches the server through the host functions and nothing else.** The
//! record says which stages have settled, which are in flight and which joined
//! the chain; `ROADMAP.md`'s boxes and declarations say the rest, read off the
//! newest branch that holds them — the top of the chain, then the branch the
//! roadmap was planned on (the `planned-on` option), then the branch this
//! Conversation is on. A stage is ready when it is neither settled nor in
//! flight and everything its line says it stands on has settled — or, a roadmap
//! declaring nothing, the stage before it has.
//!
//! **The limits are the host's** (ADR-0021). Each ready stage is asked for in
//! the roadmap's order, lowest first, and the host answers whether there was a
//! place for it; one that waits is said after the ones that started. Each start
//! is its own act, so one stage's refusal does not stop the others. Where the
//! sentences go — the Timeline, the report a home device answers, or the log at
//! a look — is the host's too: what is said here is said through `notice`.
//!
//! **It ends in an Outcome, and never names a state.** `finished` once every
//! stage is ticked or settled, which is the roadmap complete, and `continued`
//! otherwise — something started, something waits, or nothing could.
//!
//! **Started again rather than remembered.** Nothing is kept for it between
//! starts: the record and the files in the Repo are the truth.

wit_bindgen::generate!({
    world: "activity",
    path: "../../../crates/server/wit",
});

use verkstead::plugin::host::{self, Link, NotStarted, Revision, Standing, Standings};

/// The option naming the roadmap the Conversation is a stage of.
const ROADMAP: &str = "roadmap";

/// The option naming the branch the roadmap was planned on, where the record
/// holds one.
const PLANNED_ON: &str = "planned-on";

/// Where roadmaps live in a Repo, and the index each one keeps.
const ROADMAPS: &str = "docs/roadmaps";
const INDEX: &str = "ROADMAP.md";

/// Where a Repo records how its work goes for review, and the block inside
/// its review process that says a stage stacks on the stage before it.
const GIT_WORKFLOW: &str = "docs/agents/git-workflow.md";
const REVIEW_PROCESS: &str = "## Review process";
const STACKING: &str = "### Stacking roadmap stages";

const FINISHED: &str = "finished";
const CONTINUED: &str = "continued";

struct Stages;

impl Guest for Stages {
    fn run(facts: Facts) -> String {
        let option = |key: &str| {
            facts
                .options
                .iter()
                .find(|(named, _)| named == key)
                .map(|(_, value)| value.as_str())
                .filter(|value| !value.is_empty())
        };

        let Some(roadmap) = option(ROADMAP) else {
            host::notice(
                "The stage runner was started without a roadmap to read, so no stage was started.",
            );
            return CONTINUED.to_owned();
        };

        let repo = &facts.worktree.repo;
        let branch = &facts.worktree.branch;

        // A record that will not be read is nothing to start anything from,
        // and the host has said why in its log.
        let Ok(record) = host::stage_standings(roadmap) else {
            return CONTINUED.to_owned();
        };

        // A chain that will not be read is no chain: the branch this
        // Conversation is on is what a stage is cut from then, which is the
        // safe answer.
        let chain = host::stage_chain(roadmap).unwrap_or_default();

        let base = cut_from(&chain, &record, branch);

        let (list, at) = match index(repo, roadmap, &chain, option(PLANNED_ON), branch) {
            Ok(read) => read,
            Err(why) => return unstartable(&why),
        };

        let entries: Vec<Entry> = list.lines().filter_map(entry).collect();

        if entries.is_empty() {
            return unstartable(&format!(
                "this conversation is a stage of the `{roadmap}` roadmap, and its `{INDEX}` on \
                 `{at}` has no stages in it"
            ));
        }

        let judged = judge(roadmap, &list);

        let ready = match readiness(&entries, &judged, &record, branch) {
            Ready::Stages(ready) => ready,
            Ready::Complete => {
                host::notice(&format!(
                    "Every stage of the `{roadmap}` roadmap is done, so there's no stage to \
                     start. The roadmap is complete."
                ));
                return FINISHED.to_owned();
            }
            Ready::InFlight => {
                host::notice(&format!(
                    "No stage of the `{roadmap}` roadmap can start yet. Every stage it has left \
                     is in flight or stands on one that is."
                ));
                return CONTINUED.to_owned();
            }
            Ready::Misdeclared(why) => return unstartable(&why),
        };

        let stacks = stacks(repo);

        // What did not start and is the human's to hear, said after the ones
        // that did: a Timeline reads in the order things happened, and a stage
        // waits for a place because the stages above it took theirs.
        let mut held = Vec::new();

        for entry in ready {
            if entry.link.is_empty() {
                held.push(format!(
                    "Stage {} of the `{roadmap}` roadmap was ready and couldn't be started. Its \
                     line names no brief to start it from.",
                    entry.label,
                ));
                continue;
            }

            let brief_path = format!("{ROADMAPS}/{roadmap}/{}", entry.link);

            let Some(brief) = text(repo, &Revision::Named(at.clone()), &brief_path)
                .ok()
                .flatten()
            else {
                held.push(format!(
                    "Stage {} of the `{roadmap}` roadmap was ready and couldn't be started. It \
                     names the brief `{brief_path}`, and there's nothing there to read on `{at}`.",
                    entry.label,
                ));
                continue;
            };

            let started = host::start_conversation(&host::Stage {
                roadmap: roadmap.to_owned(),
                label: entry.label.clone(),
                title: entry.title.clone(),
                brief_path,
                brief,
                base: base.clone(),
                platform: platform(&judged, &entry.label),
                stacks,
            });

            match started {
                Ok(_) => {}
                // What waits is said here, and a halt the host has said
                // already, where it says such things.
                Err(NotStarted::NoPlace(why)) => held.push(why),
                Err(NotStarted::Halted(_)) => {}
            }
        }

        for why in held {
            host::notice(&why);
        }

        CONTINUED.to_owned()
    }
}

export!(Stages);

/// Say that nothing of the roadmap could start, and why.
fn unstartable(why: &str) -> String {
    host::notice(&format!(
        "No stage of the roadmap could be started. {}",
        sentence(why)
    ));

    CONTINUED.to_owned()
}

/// `why` as a sentence of its own: its first letter up, and a full stop where
/// it ends without one.
fn sentence(why: &str) -> String {
    let why = why.trim();
    let mut letters = why.chars();

    let opened = match letters.next() {
        Some(first) => first.to_uppercase().collect::<String>() + letters.as_str(),
        None => String::new(),
    };

    match opened.ends_with(['.', '?', '!', '…']) {
        true => opened,
        false => format!("{opened}."),
    }
}

/// What the record says of the stage labelled `label`.
fn standing(record: &Standings, label: &str) -> Option<Standing> {
    record
        .stages
        .iter()
        .find(|stage| stage.label == label)
        .map(|stage| stage.standing)
}

/// Which branch the next stage is **cut from**: the highest stage of the chain
/// the record calls settled, and the branch this Conversation is on where the
/// chain holds none — the Conversation that wrote the roadmap, for its first
/// stage. A link still wrapping up may yet be pushed again, so it is passed
/// over.
fn cut_from(chain: &[Link], record: &Standings, branch: &str) -> String {
    chain
        .iter()
        .rev()
        .find(|link| {
            link.stage
                .as_deref()
                .is_some_and(|label| standing(record, label) == Some(Standing::Settled))
        })
        .map_or(branch, |link| link.branch.as_str())
        .to_owned()
}

/// The roadmap's index, read off the newest branch git still holds — the top
/// of its chain down, then the branch it was planned on, then this
/// Conversation's — and the branch it was read on. A branch that names no
/// commit gives way to the next; a branch that holds no index is said.
fn index(
    repo: &str,
    roadmap: &str,
    chain: &[Link],
    planned_on: Option<&str>,
    branch: &str,
) -> Result<(String, String), String> {
    let path = format!("{ROADMAPS}/{roadmap}/{INDEX}");

    for at in declaring(chain, planned_on, branch) {
        match text(repo, &Revision::Named(at.to_owned()), &path) {
            Ok(Some(list)) => return Ok((list, at.to_owned())),
            Ok(None) => {
                return Err(format!(
                    "this conversation is a stage of the `{roadmap}` roadmap, and there's no \
                     `{path}` on `{at}`, the branch its declarations are read off, to read what \
                     it has left"
                ));
            }
            Err(_) => continue,
        }
    }

    Err(format!(
        "this conversation is a stage of the `{roadmap}` roadmap, and no branch of this \
         repository holds its declarations to read"
    ))
}

/// The branches a roadmap's index is read off, newest first: the chain from
/// its top down, the branch the roadmap was planned on, and this
/// Conversation's.
fn declaring<'a>(chain: &'a [Link], planned_on: Option<&'a str>, branch: &'a str) -> Vec<&'a str> {
    chain
        .iter()
        .rev()
        .map(|link| link.branch.as_str())
        .chain(planned_on)
        .chain(std::iter::once(branch))
        .collect()
}

/// The file at `path` as text.
fn text(repo: &str, at: &Revision, path: &str) -> Result<Option<String>, String> {
    Ok(host::read_file(repo, at, path)?.map(|bytes| String::from_utf8_lossy(&bytes).into_owned()))
}

/// Whether the Repo records a way to stack a stage on the one before it: the
/// block under its review process, read as this Conversation has committed it.
fn stacks(repo: &str) -> bool {
    let Ok(Some(workflow)) = text(repo, &Revision::Head, GIT_WORKFLOW) else {
        return false;
    };

    stacking(&workflow)
}

/// Whether a `git-workflow.md`'s text records a stacking mechanism.
fn stacking(workflow: &str) -> bool {
    workflow
        .lines()
        .map(str::trim_end)
        .skip_while(|line| *line != REVIEW_PROCESS)
        .skip(1)
        .take_while(|line| !line.starts_with("## "))
        .any(|line| line == STACKING)
}

/// One line of `ROADMAP.md`'s checkbox list.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Entry {
    number: u32,
    label: String,
    title: String,
    link: String,
    after: String,
    checked: bool,
}

/// The entry `line` is — `- [ ] 03: <title> — [brief](03-<slug>.md)`, and
/// whatever the roadmap wrote after the link — or none where it is not one.
fn entry(line: &str) -> Option<Entry> {
    let line = line.trim();

    let (rest, checked) = [("- [ ] ", false), ("- [x] ", true), ("- [X] ", true)]
        .iter()
        .find_map(|(mark, checked)| Some((line.strip_prefix(mark)?, *checked)))?;

    let (label, rest) = rest.split_once(':')?;
    let label = label.trim();

    if label.is_empty() || !label.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }

    let (title, link, after) = split(rest);

    Some(Entry {
        number: label.parse().ok()?,
        label: label.to_owned(),
        title: title.to_owned(),
        link: link.to_owned(),
        after: after.to_owned(),
        checked,
    })
}

/// A line's title, its link's target, and what it wrote after the link.
fn split(rest: &str) -> (&str, &str, &str) {
    let rest = rest.trim();

    let Some(open) = rest.find("](").and_then(|link| rest[..link].rfind('[')) else {
        return (rest, "", "");
    };

    let title = rest[..open].trim().trim_end_matches(['—', '–', '-']).trim();

    let Some((target, after)) = rest[open..]
        .split_once("](")
        .and_then(|(_, target)| target.split_once(')'))
    else {
        return (title, "", "");
    };

    (title, target.trim(), after.trim())
}

/// Whether the line's annotation names `branch` — the stage this Conversation
/// is, written in progress over its own line.
fn ours(after: &str, branch: &str) -> bool {
    !branch.is_empty() && after.contains(&format!("`{branch}`"))
}

/// Whether a stage is done: what the record says of it, and its box where the
/// record holds no row, or only an abandoned one.
fn done(entry: &Entry, standing: Option<Standing>) -> bool {
    match standing {
        Some(Standing::Settled) => true,
        Some(Standing::InFlight) => false,
        Some(Standing::Abandoned) | None => entry.checked,
    }
}

/// What may start.
enum Ready<'a> {
    /// These, lowest-numbered first.
    Stages(Vec<&'a Entry>),
    /// Every stage is done.
    Complete,
    /// Nothing is ready and something is still to do.
    InFlight,
    /// The roadmap declares badly, and why.
    Misdeclared(String),
}

/// Which stages of the roadmap may start: neither done nor in flight, and
/// standing on nothing that has not settled. While the Conversation that wrote
/// the roadmap is in flight, a stage standing on nothing waits for it too.
fn readiness<'a>(
    entries: &'a [Entry],
    judged: &Judgement,
    record: &Standings,
    branch: &str,
) -> Ready<'a> {
    let stands_on: Vec<Vec<usize>> = match judged {
        Judgement::Refused(why) => return Ready::Misdeclared(why.clone()),

        // Each stage standing on the one before it: a roadmap run in order.
        Judgement::Undeclared => (0..entries.len())
            .map(|at| at.checked_sub(1).into_iter().collect())
            .collect(),

        Judgement::Declared(declared) => entries
            .iter()
            .map(|entry| {
                declared
                    .iter()
                    .find(|one| one.label == entry.label)
                    .map(|one| {
                        one.stands_on
                            .iter()
                            .filter_map(|label| entries.iter().position(|one| one.label == *label))
                            .collect()
                    })
                    .unwrap_or_default()
            })
            .collect(),
    };

    let held = record.planner == Some(Standing::InFlight);

    let settled: Vec<bool> = entries
        .iter()
        .map(|entry| ours(&entry.after, branch) || done(entry, standing(record, &entry.label)))
        .collect();

    let on_it: Vec<bool> = entries
        .iter()
        .zip(&settled)
        .map(|(entry, settled)| {
            !settled && standing(record, &entry.label) == Some(Standing::InFlight)
        })
        .collect();

    let mut ready: Vec<&Entry> = entries
        .iter()
        .enumerate()
        .filter(|(at, _)| !settled[*at] && !on_it[*at])
        .filter(|(at, _)| stands_on[*at].iter().all(|stood| settled[*stood]))
        .filter(|(at, _)| !(held && stands_on[*at].is_empty()))
        .map(|(_, entry)| entry)
        .collect();

    ready.sort_by_key(|entry| entry.number);

    match ready.is_empty() {
        false => Ready::Stages(ready),
        true => match settled.iter().all(|settled| *settled) {
            true => Ready::Complete,
            false => Ready::InFlight,
        },
    }
}

/// The platform a stage's line wants, as the judging matched it: a roadmap
/// that declares nothing places nothing, its tails being prose.
fn platform(judged: &Judgement, label: &str) -> String {
    match judged {
        Judgement::Declared(declared) => declared
            .iter()
            .find(|one| one.label == label)
            .and_then(|one| one.platform)
            .unwrap_or_default()
            .to_owned(),
        Judgement::Undeclared | Judgement::Refused(_) => String::new(),
    }
}

/// What a line says it stands on.
#[derive(Debug, Clone, PartialEq, Eq)]
enum StandsOn<'a> {
    /// `no dependencies`.
    Nothing,
    /// `after 01, 03`, by the labels as the roadmap writes them; empty where
    /// it said `after` and named nobody.
    Stages(Vec<&'a str>),
}

/// The tail of a line, read and not judged.
struct Reading<'a> {
    stands_on: Option<StandsOn<'a>>,
    platform: Option<&'a str>,
}

/// The words that end the list an `after` was collecting.
const STARTS: [&str; 3] = ["after", "no", "on"];

/// What the tail after a line's link declares. Nothing inside an annotation,
/// `*(…)*`, is read, and one that never closes takes the rest of the line.
fn read(after: &str) -> Reading<'_> {
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

fn is(word: &str, keyword: &str) -> bool {
    word.eq_ignore_ascii_case(keyword)
}

/// The pieces of the tail outside an annotation.
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

/// The words of one piece of the tail, joined by whitespace, the commas inside
/// an `after` and whichever dash came to hand.
fn words(piece: &str) -> impl Iterator<Item = &str> {
    piece
        .split(|c: char| c.is_whitespace() || [',', '-', '—', '–', ';'].contains(&c))
        .filter(|word| !word.is_empty())
}

/// The three platforms a line can want.
const PLATFORMS: [&str; 3] = ["linux", "macos", "windows"];

/// The platform `word` names, as the host's word for it.
fn matching(word: &str) -> Option<&'static str> {
    PLATFORMS.iter().copied().find(|named| is(word, named))
}

/// A whole `ROADMAP.md`, judged as one thing.
enum Judgement {
    /// Not one line declares: the roadmap runs in order.
    Undeclared,
    /// Every line declares, and this is what.
    Declared(Vec<Declared>),
    /// The roadmap declares badly, in words a human can act on.
    Refused(String),
}

struct Declared {
    label: String,
    stands_on: Vec<String>,
    platform: Option<&'static str>,
}

/// What `list` declares, all or nothing, the faults looked for in the order a
/// human would fix them — the host's own judging, which refuses the roadmap's
/// session at its Done signal in the same words.
fn judge(roadmap: &str, list: &str) -> Judgement {
    let entries: Vec<Entry> = list.lines().filter_map(entry).collect();
    let lines: Vec<(&Entry, Reading<'_>)> = entries
        .iter()
        .map(|entry| (entry, read(&entry.after)))
        .collect();

    if lines.iter().all(|(_, read)| read.stands_on.is_none()) {
        return Judgement::Undeclared;
    }

    if let Some((entry, _)) = lines.iter().find(|(_, read)| read.stands_on.is_none()) {
        return Judgement::Refused(format!(
            "the {roadmap} roadmap declares what some of its stages stand on and not others: \
             stage {} declares nothing after its link. Every line of a declaring roadmap carries \
             a declaration, no dependencies where the stage stands on nothing, because a bare \
             line can't be told from a forgotten one",
            entry.label,
        ));
    }

    let labels: Vec<&str> = lines
        .iter()
        .map(|(entry, _)| entry.label.as_str())
        .collect();

    for (entry, read) in &lines {
        let Some(StandsOn::Stages(stages)) = &read.stands_on else {
            continue;
        };

        if stages.is_empty() {
            return Judgement::Refused(format!(
                "stage {} of the {roadmap} roadmap says after and names no stage. A stage that \
                 stands on nothing says no dependencies",
                entry.label,
            ));
        }

        if let Some(named) = stages.iter().find(|named| !labels.contains(named)) {
            return Judgement::Refused(format!(
                "stage {} of the {roadmap} roadmap says after {named}, and no stage of it is \
                 labelled {named}. A stage may only name stages of its own roadmap, by their \
                 labels as the roadmap writes them, zero-padding and all",
                entry.label,
            ));
        }
    }

    for (entry, read) in &lines {
        if let Some(word) = read.platform
            && matching(word).is_none()
        {
            return Judgement::Refused(format!(
                "stage {} of the {roadmap} roadmap says on {word}, which isn't a platform. A \
                 platform is one of {}",
                entry.label,
                PLATFORMS.join(", "),
            ));
        }
    }

    if let Some(cycle) = cycle(&lines) {
        return Judgement::Refused(match cycle.as_slice() {
            [only] => format!(
                "stage {only} of the {roadmap} roadmap stands on itself, which is a cycle of one, \
                 so it could never start"
            ),
            _ => format!(
                "the {roadmap} roadmap has a cycle in what its stages stand on ({}), so none of \
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
                label: entry.label.clone(),
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

/// One cycle of what the stages stand on, by its labels, walked depth-first
/// from every stage in the roadmap's order, so the same one is always named.
fn cycle<'a>(lines: &[(&'a Entry, Reading<'a>)]) -> Option<Vec<&'a str>> {
    let mut walked = Vec::new();
    let mut open = Vec::new();

    lines
        .iter()
        .find_map(|(entry, _)| walk(&entry.label, lines, &mut walked, &mut open))
}

fn walk<'a>(
    label: &'a str,
    lines: &[(&'a Entry, Reading<'a>)],
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
    //! What the server's carry-on was tested on before the stage runner
    //! crossed into this component: where a stage is cut from, which branch
    //! the index is read off, whether the Repo stacks, and which stages are
    //! ready. Run natively — `cargo test --manifest-path plugins/Cargo.toml` —
    //! since none of it calls the host.
    //!
    //! The readiness cases are the server's own for [`ready`]'s rule in
    //! `stages.rs`, which this component keeps a copy of: the two are kept in
    //! step by these.

    use host::StageStanding;

    use super::*;

    /// A roadmap index exactly as `/to-roadmap` writes one.
    const MVP: &str = "\
# MVP roadmap

- [x] 01: Workbench — [brief](01-workbench.md)
- [x] 02: Grilling — [brief](02-grilling.md)
- [ ] 03: Implementation — [brief](03-implementation.md)
";

    /// Three stages and none ticked: whether each is done is the record's.
    const UNTICKED: &str = "\
# MVP roadmap

- [ ] 01: Workbench — [brief](01-workbench.md)
- [ ] 02: Grilling — [brief](02-grilling.md)
- [ ] 03: Implementation — [brief](03-implementation.md)
";

    /// 01 and 02 stand on nothing, 03 on 02, and 04 on both 01 and 03.
    const DECLARED: &str = "\
# MVP roadmap

- [ ] 01: Workbench — [brief](01-workbench.md) — no dependencies
- [ ] 02: Grilling — [brief](02-grilling.md) — no dependencies
- [ ] 03: Implementation — [brief](03-implementation.md) — after 02
- [ ] 04: Wrap-up — [brief](04-wrap-up.md) — after 01, 03
";

    /// What the record holds of the `mvp` roadmap, nothing stopped.
    fn record(rows: &[(&str, Standing)]) -> Standings {
        Standings {
            planner: None,
            stages: rows
                .iter()
                .map(|(label, standing)| StageStanding {
                    label: (*label).to_owned(),
                    standing: *standing,
                    stopped: false,
                    conversation: 0,
                })
                .collect(),
        }
    }

    /// And the same with the Conversation that wrote the roadmap standing so.
    fn planned(rows: &[(&str, Standing)], planner: Standing) -> Standings {
        Standings {
            planner: Some(planner),
            ..record(rows)
        }
    }

    /// What [`readiness`] answers, with the stages by label.
    #[derive(Debug, PartialEq, Eq)]
    enum Read {
        Stages(Vec<String>),
        Complete,
        InFlight,
        Misdeclared(String),
    }

    /// Which stages of `list` may start, read as the Conversation on `branch`.
    fn ready(list: &str, record: &Standings, branch: &str) -> Read {
        let entries: Vec<Entry> = list.lines().filter_map(entry).collect();

        match readiness(&entries, &judge("mvp", list), record, branch) {
            Ready::Stages(ready) => {
                Read::Stages(ready.iter().map(|entry| entry.label.clone()).collect())
            }
            Ready::Complete => Read::Complete,
            Ready::InFlight => Read::InFlight,
            Ready::Misdeclared(why) => Read::Misdeclared(why),
        }
    }

    fn stages(labels: &[&str]) -> Read {
        Read::Stages(labels.iter().map(|label| (*label).to_owned()).collect())
    }

    #[test]
    fn the_next_stage_is_the_lowest_numbered_unchecked_one() {
        assert_eq!(ready(MVP, &record(&[]), "anything-else"), stages(&["03"]));
    }

    /// The annotation naming this Conversation's branch says the stage is its
    /// own, which a run must never be offered back.
    #[test]
    fn the_stage_this_conversation_worked_is_not_the_one_to_start() {
        let list = "# MVP roadmap\n\n\
             - [x] 01: Workbench — [brief](01-workbench.md)\n\
             - [ ] 02: Grilling — [brief](02-grilling.md) *(in progress: `grilling`)*\n\
             - [ ] 03: Implementation — [brief](03-implementation.md)\n";

        assert_eq!(ready(list, &record(&[]), "grilling"), stages(&["03"]));
        assert_eq!(
            ready(list, &record(&[]), "some-other-branch"),
            stages(&["02"]),
            "and to anybody else it is the stage it says it is",
        );
    }

    #[test]
    fn a_settled_stage_is_done_however_its_box_reads_on_this_branch() {
        assert_eq!(
            ready(
                UNTICKED,
                &record(&[("01", Standing::Settled)]),
                "anything-else"
            ),
            stages(&["02"]),
        );
    }

    #[test]
    fn a_stage_ticked_with_no_record_of_it_is_done_too() {
        let list = "# MVP roadmap\n\n\
             - [x] 01: Workbench — [brief](01-workbench.md)\n\
             - [ ] 02: Grilling — [brief](02-grilling.md)\n\
             - [ ] 03: Implementation — [brief](03-implementation.md)\n";

        assert_eq!(
            ready(list, &record(&[("02", Standing::Settled)]), "anything-else"),
            stages(&["03"]),
        );
    }

    /// Nor is what stands on a stage in flight started: it is held up rather
    /// than stepped over.
    #[test]
    fn a_stage_the_record_says_is_in_flight_is_not_started_however_its_box_reads() {
        let in_flight = record(&[("01", Standing::InFlight), ("02", Standing::InFlight)]);

        let list = "# MVP roadmap\n\n\
             - [x] 01: Workbench — [brief](01-workbench.md)\n\
             - [ ] 02: Grilling — [brief](02-grilling.md)\n\
             - [ ] 03: Implementation — [brief](03-implementation.md)\n";

        assert_eq!(ready(list, &in_flight, "anything-else"), Read::InFlight);

        assert_eq!(
            ready(
                "- [ ] 01: Workbench — [brief](01-workbench.md)\n",
                &in_flight,
                "anything-else",
            ),
            Read::InFlight,
            "in flight is not done, and not the roadmap complete either",
        );
    }

    #[test]
    fn a_declared_roadmap_carries_on_with_the_stage_whose_dependency_settled() {
        assert_eq!(
            ready(
                DECLARED,
                &record(&[("01", Standing::InFlight), ("02", Standing::Settled)]),
                "roadmaps/mvp/02-grilling",
            ),
            stages(&["03"]),
        );
    }

    #[test]
    fn a_stage_whose_dependency_is_in_flight_is_not_started() {
        assert_eq!(
            ready(
                DECLARED,
                &record(&[
                    ("01", Standing::InFlight),
                    ("02", Standing::Settled),
                    ("03", Standing::Settled),
                ]),
                "roadmaps/mvp/03-implementation",
            ),
            Read::InFlight,
            "04 stands on 01 as well as on 03, and 01 has not settled",
        );
    }

    #[test]
    fn nothing_ready_is_the_roadmap_complete_only_where_nothing_is_in_flight() {
        let settled = [
            ("01", Standing::Settled),
            ("02", Standing::Settled),
            ("03", Standing::Settled),
        ];

        let mut rows = settled.to_vec();
        rows.push(("04", Standing::InFlight));
        assert_eq!(ready(DECLARED, &record(&rows), ""), Read::InFlight);

        rows.pop();
        rows.push(("04", Standing::Settled));
        assert_eq!(ready(DECLARED, &record(&rows), ""), Read::Complete);
    }

    #[test]
    fn a_stage_in_flight_holds_up_its_own_dependents_and_no_others() {
        assert_eq!(
            ready(
                DECLARED,
                &record(&[("01", Standing::InFlight), ("02", Standing::Settled)]),
                "",
            ),
            stages(&["03"]),
        );
    }

    #[test]
    fn the_ready_stages_of_a_declaring_roadmap_come_back_lowest_first() {
        assert_eq!(
            ready(DECLARED, &record(&[]), ""),
            stages(&["01", "02"]),
            "and 03 stands on 02, which has not settled",
        );
    }

    #[test]
    fn no_root_is_ready_while_the_roadmaps_own_conversation_is_in_flight() {
        for list in [DECLARED, UNTICKED] {
            assert_eq!(
                ready(list, &planned(&[], Standing::InFlight), ""),
                Read::InFlight,
                "nothing of a roadmap starts before the roadmap is reviewed",
            );
        }
    }

    #[test]
    fn the_roots_are_ready_once_the_roadmaps_own_conversation_has_settled() {
        for record in [
            planned(&[], Standing::Settled),
            planned(&[], Standing::Abandoned),
            record(&[]),
        ] {
            assert_eq!(ready(DECLARED, &record, ""), stages(&["01", "02"]));
        }
    }

    #[test]
    fn the_roadmaps_own_conversation_holds_only_the_stages_that_stand_on_nothing() {
        assert_eq!(
            ready(
                DECLARED,
                &planned(&[("02", Standing::Settled)], Standing::InFlight),
                "",
            ),
            stages(&["03"]),
            "01 is a root, held on the roadmap",
        );
    }

    #[test]
    fn an_undeclared_roadmap_has_one_ready_stage_at_most() {
        assert_eq!(ready(UNTICKED, &record(&[]), ""), stages(&["01"]));
    }

    #[test]
    fn a_roadmap_that_declares_badly_has_nothing_ready() {
        let Read::Misdeclared(why) = ready(
            "# MVP roadmap\n\n\
             - [ ] 01: Workbench — [brief](01-workbench.md) — no dependencies\n\
             - [ ] 02: Grilling — [brief](02-grilling.md)\n",
            &record(&[]),
            "",
        ) else {
            panic!("a declaration on one line and not the other is refused");
        };

        assert!(why.contains("mvp") && why.contains("02"), "{why:?}");
    }

    #[test]
    fn an_abandoned_stage_is_left_to_what_its_box_says() {
        let abandoned = record(&[("01", Standing::Abandoned)]);

        assert_eq!(
            ready(UNTICKED, &abandoned, "anything-else"),
            stages(&["01"])
        );

        let list = "# MVP roadmap\n\n\
             - [x] 01: Workbench — [brief](01-workbench.md)\n\
             - [ ] 02: Grilling — [brief](02-grilling.md)\n";

        assert_eq!(ready(list, &abandoned, "anything-else"), stages(&["02"]));
    }

    #[test]
    fn a_roadmap_run_in_order_carries_on_from_the_stage_that_settled() {
        let list = "# MVP roadmap\n\n\
             - [x] 01: Workbench — [brief](01-workbench.md)\n\
             - [ ] 02: Grilling — [brief](02-grilling.md) \
             *(in progress: `roadmaps/mvp/02-grilling`)*\n\
             - [ ] 03: Implementation — [brief](03-implementation.md)\n";

        assert_eq!(
            ready(
                list,
                &record(&[("01", Standing::Settled), ("02", Standing::Settled)]),
                "roadmaps/mvp/02-grilling",
            ),
            stages(&["03"]),
        );
    }

    #[test]
    fn a_stage_the_record_holds_no_label_for_is_still_skipped_by_its_annotation() {
        let list = "# MVP roadmap\n\n\
             - [ ] 01: Workbench — [brief](01-workbench.md)\n\
             - [ ] 02: Grilling — [brief](02-grilling.md) \
             *(in progress: `roadmaps/mvp/02-grilling`)*\n\
             - [ ] 03: Implementation — [brief](03-implementation.md)\n";

        assert_eq!(
            ready(
                list,
                &record(&[("01", Standing::Settled)]),
                "roadmaps/mvp/02-grilling",
            ),
            stages(&["03"]),
        );
    }

    #[test]
    fn a_declaration_on_the_line_does_not_hide_the_stage_in_flight() {
        for tail in [
            "— after 01 *(in progress: `grilling`)*",
            "*(in progress: `grilling`)* — after 01",
        ] {
            let list = format!(
                "# MVP roadmap\n\n\
                 - [x] 01: Workbench — [brief](01-workbench.md) — no dependencies\n\
                 - [ ] 02: Grilling — [brief](02-grilling.md) {tail}\n\
                 - [ ] 03: Implementation — [brief](03-implementation.md) — after 02\n"
            );

            assert_eq!(
                ready(&list, &record(&[]), "grilling"),
                stages(&["03"]),
                "{tail:?}"
            );
        }
    }

    #[test]
    fn a_declaration_is_nobodys_stage_in_flight() {
        let list = "# MVP roadmap\n\n\
             - [x] 01: Workbench — [brief](01-workbench.md) — no dependencies\n\
             - [ ] 02: Grilling — [brief](02-grilling.md) — after 01 — on windows\n";

        assert_eq!(ready(list, &record(&[]), "grilling"), stages(&["02"]));
        assert_eq!(
            platform(&judge("mvp", list), "02"),
            "windows",
            "and the stage carries the platform its line declares, which places it",
        );
    }

    #[test]
    fn a_stage_declaring_no_platform_wants_none() {
        let list = "# MVP roadmap\n\n\
             - [x] 01: Workbench — [brief](01-workbench.md) — no dependencies\n\
             - [ ] 02: Grilling — [brief](02-grilling.md) — after 01\n";

        assert_eq!(platform(&judge("mvp", list), "02"), "");
    }

    #[test]
    fn an_undeclared_roadmaps_prose_places_no_stage() {
        let list = "# MVP roadmap\n\n\
             - [x] 01: Workbench — [brief](01-workbench.md)\n\
             - [ ] 02: Installers — [brief](02-installers.md) — focus on windows installers\n";

        assert_eq!(ready(list, &record(&[]), "grilling"), stages(&["02"]));
        assert_eq!(platform(&judge("mvp", list), "02"), "");
    }

    #[test]
    fn a_roadmap_with_every_stage_checked_has_nothing_to_start() {
        assert_eq!(
            ready(
                "# MVP roadmap\n\n- [x] 01: Workbench — [brief](01-workbench.md)\n",
                &record(&[]),
                "anything-else",
            ),
            Read::Complete,
        );
    }

    #[test]
    fn the_last_stage_finishing_is_the_roadmap_complete() {
        let list = "# MVP roadmap\n\n\
             - [x] 01: Workbench — [brief](01-workbench.md)\n\
             - [ ] 02: Grilling — [brief](02-grilling.md) *(in progress: `grilling`)*\n";

        assert_eq!(ready(list, &record(&[]), "grilling"), Read::Complete);
    }

    /// One link of the `rate-limiting` roadmap's chain.
    fn link(stage: &str) -> Link {
        Link {
            stage: Some(stage.to_owned()),
            branch: format!("roadmaps/rate-limiting/{stage}"),
        }
    }

    #[test]
    fn a_roadmap_run_in_order_is_cut_from_the_stage_that_settled() {
        let chain = [link("01"), link("02")];
        let record = record(&[("01", Standing::Settled), ("02", Standing::Settled)]);

        assert_eq!(
            cut_from(&chain, &record, "roadmaps/rate-limiting/02"),
            "roadmaps/rate-limiting/02",
        );
    }

    #[test]
    fn a_settled_sibling_above_the_stage_that_settled_is_the_base() {
        let chain = [link("01"), link("02")];
        let record = record(&[("01", Standing::Settled), ("02", Standing::Settled)]);

        assert_eq!(
            cut_from(&chain, &record, "roadmaps/rate-limiting/01"),
            "roadmaps/rate-limiting/02",
            "stage 01 settled last, and stage 02 is above it in the chain",
        );
    }

    #[test]
    fn an_unsettled_link_above_is_not_cut_from() {
        let chain = [link("01"), link("02")];
        let record = record(&[("01", Standing::Settled), ("02", Standing::InFlight)]);

        assert_eq!(
            cut_from(&chain, &record, "roadmaps/rate-limiting/01"),
            "roadmaps/rate-limiting/01",
        );
    }

    #[test]
    fn an_empty_chain_is_cut_from_the_branch_that_settled() {
        assert_eq!(
            cut_from(&[], &record(&[]), "roadmap/rate-limiting"),
            "roadmap/rate-limiting",
        );
    }

    #[test]
    fn a_chain_with_nothing_settled_in_it_is_cut_from_the_branch_that_settled() {
        assert_eq!(
            cut_from(
                &[link("02")],
                &record(&[("02", Standing::InFlight)]),
                "roadmap/rate-limiting",
            ),
            "roadmap/rate-limiting",
        );
    }

    /// Two branches under one label, walked from the top down, so the attempt
    /// that joined later is the one cut from.
    #[test]
    fn a_stage_attempted_twice_is_cut_from_the_attempt_that_joined_later() {
        let chain = [
            Link {
                stage: Some("01".to_owned()),
                branch: "roadmaps/rate-limiting/01-abandoned".to_owned(),
            },
            link("01"),
        ];

        assert_eq!(
            cut_from(
                &chain,
                &record(&[("01", Standing::Settled)]),
                "roadmap/rate-limiting",
            ),
            "roadmaps/rate-limiting/01",
        );
    }

    #[test]
    fn a_running_roadmaps_declarations_are_read_at_the_top_of_its_chain() {
        let chain = [link("01"), link("02")];

        assert_eq!(
            declaring(&chain, Some("rate-limiting"), "roadmaps/rate-limiting/01"),
            [
                "roadmaps/rate-limiting/02",
                "roadmaps/rate-limiting/01",
                "rate-limiting",
                "roadmaps/rate-limiting/01",
            ],
        );
    }

    #[test]
    fn a_roadmap_nothing_has_joined_is_read_off_the_branch_it_was_planned_on() {
        assert_eq!(
            declaring(&[], Some("rate-limiting"), "roadmaps/rate-limiting/01"),
            ["rate-limiting", "roadmaps/rate-limiting/01"],
        );
    }

    #[test]
    fn a_roadmap_with_neither_is_read_off_the_branch_that_settled() {
        assert_eq!(declaring(&[], None, "rate-limiting"), ["rate-limiting"]);
    }

    #[test]
    fn a_repository_records_its_stacking_mechanism_under_its_review_process() {
        assert!(!stacking(""));
        assert!(!stacking(
            "# Git workflow\n\n## Review process\n\n### Finish sequence\n\nPush it.\n"
        ));
        assert!(stacking(
            "# Git workflow\n\n## Review process\n\n\
             ### Finish sequence\n\nPush it.\n\n\
             ### Stacking roadmap stages\n\n`gh stack init <predecessor> <new>`\n"
        ));
    }

    #[test]
    fn a_stacking_block_outside_the_review_process_is_not_the_mechanism() {
        assert!(!stacking(
            "# Git workflow\n\n## Review process\n\n### Finish sequence\n\nPush it.\n\n\
             ## Notes\n\n### Stacking roadmap stages\n\nWe gave up on these.\n"
        ));
    }
}
