//! **The backlog runner**, a built-in Activity (ADR-0030, *Activity*): a
//! task list worked box by box, one fresh session on the next-task skill per
//! task, and the finish after the last.
//!
//! **It reaches the server through the host functions and nothing else.** What
//! is next is read off `.tasks/TODO.md` as the Worktree's HEAD holds it — the
//! lowest-numbered entry whose box is not ticked, in the list's own order, and
//! the finish once every box is — so what a session has ticked and not
//! committed is not ticked yet. What each session is started on is written
//! here, off the start facts, in the words the server's own builder has always
//! used: which task is not said, the skill reading `.tasks/` and taking the
//! same one by the same rule.
//!
//! **A roadmap stage's backlog too**, the host saying it is one with the
//! `stage` option, whose value is the branch the stage's own was cut from. A
//! stage with no list yet is planned first, a session on the next-stage skill
//! writing one; and its finish waits on the roadmap's chain through the host,
//! which answers the branches it goes on top of, for the finish's prompt.
//!
//! **A task's session is done when its box is ticked and committed, the
//! planning's when the list is, and the finish's when `.tasks/TODO.md` is gone
//! and a pull request is open.** That is the evidence each is launched
//! expecting, and the host holds `verkstead done` to it; what the session
//! builds, and so where its Memos go, the host reads off the same evidence.
//!
//! **It ends in an Outcome, and never names a state.** `finished` once the
//! finish has landed, and `steered` where the human took the Conversation
//! elsewhere — or where the host moved it to a device of the platform a task
//! wants, which starts this Activity again over there. A step whose session is
//! gone without landing it, or an entry naming no task file, is a stop: what
//! happens next is the human's, and Resume starts this again.
//!
//! **Started again rather than remembered.** Nothing is kept for it between
//! starts: a server restarted mid-backlog starts it over the Repo as it now
//! stands, and the boxes say where it got to.

wit_bindgen::generate!({
    world: "activity",
    path: "../../../crates/server/wit",
});

use verkstead::plugin::activity_types::{Ends, Evidence, Landing, Waited};
use verkstead::plugin::host::{self, Placed, Revision};

/// The skill every session of a backlog runs inside, the finish's included.
const NEXT_TASK: &str = "verkstead/next-task";

/// The skill a roadmap stage's backlog is planned in, before there is one.
const NEXT_STAGE: &str = "verkstead/next-stage";

/// The option saying the Conversation is a roadmap stage, its value the branch
/// the stage's branch was cut from — empty where it came off the repository's
/// default branch.
const STAGE: &str = "stage";

/// The backlog's directory, and its list inside the Repo.
const BACKLOG: &str = ".tasks";
const TODO: &str = ".tasks/TODO.md";

/// What a stop says of a session that ended without landing its step, where
/// the host had nothing worse to say.
const UNFINISHED: &str = "The session ended without finishing the step.";

struct Backlog;

impl Guest for Backlog {
    fn run(facts: Facts) -> String {
        let Some(skill) = facts.skills.iter().find(|named| named.name == NEXT_TASK) else {
            host::stop(&format!(
                "There's no `{NEXT_TASK}` skill to start the backlog's sessions in."
            ));
            return String::new();
        };

        let repo = &facts.worktree.repo;
        let stage = stage(&facts);
        let prompt = prompt(&skill.path, &facts.brief, facts.handoff.as_deref());

        loop {
            let step = match next(repo, stage.as_ref()) {
                Ok(step) => step,
                Err(why) => {
                    host::stop(&why);
                    return String::new();
                }
            };

            let (skill, prompt, evidence, ends) = match &step {
                Step::Plan { cut_from } => {
                    let Some(planning) = facts.skills.iter().find(|named| named.name == NEXT_STAGE)
                    else {
                        host::stop(&format!(
                            "There's no `{NEXT_STAGE}` skill to plan the stage's backlog in."
                        ));
                        return String::new();
                    };

                    (
                        NEXT_STAGE,
                        planned(&planning.path, &facts.brief, cut_from.as_deref()),
                        Evidence::Landed(Landing::Arrived(TODO.to_owned())),
                        Ends::WithItsWork,
                    )
                }
                Step::Task {
                    number,
                    label,
                    platform,
                } => {
                    // Placed before anything is launched for it: where the
                    // host moves the Conversation, or asks the human where it
                    // runs, this run stands down and is started again there.
                    if let Some(platform) = platform
                        && host::place_task(*number, label, platform) == Placed::Elsewhere
                    {
                        return "steered".to_owned();
                    }

                    (
                        NEXT_TASK,
                        prompt.clone(),
                        Evidence::Landed(Landing::Ticked(*number)),
                        Ends::WithItsWork,
                    )
                }
                // A stage's finish waits on its roadmap's chain and is told
                // what it joins, the rebase coming before the push; anything
                // else's is answered at once with nothing to join.
                Step::Finish => (
                    NEXT_TASK,
                    joining(&prompt, &host::join_chain()),
                    Evidence::Landed(Landing::Gone(TODO.to_owned())),
                    Ends::OnAPullRequest,
                ),
                Step::Broken { label } => {
                    host::stop(&broken(label));
                    return String::new();
                }
            };

            let session = match host::launch(skill, "implementation", &prompt, &evidence, ends) {
                Ok(session) => session,
                Err(why) => {
                    host::stop(&why);
                    return String::new();
                }
            };

            let how = match host::wait(session) {
                Waited::Done => None,
                Waited::Steered => return "steered".to_owned(),
                Waited::Gone(how) => Some(how),
                Waited::Stalled | Waited::Marked => Some(None),
            };

            // A session that is over may have landed its step as its last act
            // and exited before the host saw it signal, which is the ordinary
            // shape of one that finishes rather than idles: the Repo is asked
            // once more before it is read as a run that has stopped.
            if let Some(how) = how
                && !landed(repo, &step)
            {
                host::stop(how.as_deref().unwrap_or(UNFINISHED));
                return String::new();
            }

            if step == Step::Finish {
                return "finished".to_owned();
            }
        }
    }
}

export!(Backlog);

/// What to run next, read off the list.
#[derive(Debug, PartialEq, Eq)]
enum Step {
    /// A roadmap stage with no list yet: write one, from the stage's Brief,
    /// told the branch its own was cut from, where that was not the default.
    Plan { cut_from: Option<String> },

    /// The lowest-numbered entry whose box is not ticked, by its number, its
    /// label as the list writes it, and the platform its line wants.
    Task {
        number: u32,
        label: String,
        platform: Option<String>,
    },

    /// An entry that is not ticked and names no task file: nothing for a
    /// session to work from.
    Broken { label: String },

    /// Every box ticked: take `.tasks/` away and open the pull request.
    Finish,
}

/// What to run next in `repo`, off `.tasks/` as its HEAD holds it — or why
/// there is nothing to say, in a sentence.
///
/// The list decides all of it, and the directory beside it only says whether
/// the entry has a file to work from: a list part way through being written
/// has entries with none yet, and reading those as done would be finishing a
/// feature nobody had started. A roadmap stage, `stage` being what it was cut
/// from, has no list until it is planned: that is its first step. The host
/// starts this over a stage only while it is unplanned or part way through,
/// never over one whose finish took the list away.
fn next(repo: &str, stage: Option<&Stage>) -> Result<Step, String> {
    let list = host::read_file(repo, &Revision::Head, TODO)?
        .map(|list| String::from_utf8_lossy(&list).into_owned());

    step(list.as_deref(), stage, || {
        Ok(host::list_directory(repo, &Revision::Head, BACKLOG)?.unwrap_or_default())
    })
}

/// [`next`] over the list as text, or none where there is none, with `files`
/// asked for the names in `.tasks/` only where an entry needs a file to work
/// from.
fn step(
    list: Option<&str>,
    stage: Option<&Stage>,
    files: impl FnOnce() -> Result<Vec<String>, String>,
) -> Result<Step, String> {
    let Some(entries) = list.map(parse) else {
        return match stage {
            Some(stage) => Ok(Step::Plan {
                cut_from: stage.cut_from.clone(),
            }),
            None => Err(format!(
                "There's no backlog in `{TODO}` to work, so there's nothing to start a session on."
            )),
        };
    };

    // In the list's own order rather than sorted: `TODO.md` is written in the
    // order the tasks are meant to be worked.
    let Some(next) = entries.into_iter().find(|entry| !entry.checked) else {
        return Ok(Step::Finish);
    };

    let files = files()?;

    let named = files.iter().any(|file| numbered(file) == Some(next.number));

    Ok(match named {
        true => Step::Task {
            number: next.number,
            label: next.label,
            platform: next.platform,
        },
        false => Step::Broken { label: next.label },
    })
}

/// Whether `step` has landed at the Worktree's HEAD: its box ticked, the list
/// there — a stage's planning — or, the finish, the list gone. Anything the
/// Repo will not say reads as not.
fn landed(repo: &str, step: &Step) -> bool {
    match step {
        Step::Plan { .. } => matches!(host::read_file(repo, &Revision::Head, TODO), Ok(Some(_))),
        Step::Task { number, .. } => entries(repo).is_ok_and(|entries| {
            entries.is_some_and(|entries| {
                entries
                    .iter()
                    .any(|entry| entry.number == *number && entry.checked)
            })
        }),
        Step::Finish => matches!(host::read_file(repo, &Revision::Head, TODO), Ok(None)),
        Step::Broken { .. } => false,
    }
}

/// One entry of the list.
struct Entry {
    number: u32,
    label: String,
    checked: bool,
    platform: Option<String>,
}

/// The entries of `.tasks/TODO.md` at `repo`'s HEAD, in the list's own order,
/// or none where there is no list.
fn entries(repo: &str) -> Result<Option<Vec<Entry>>, String> {
    let Some(list) = host::read_file(repo, &Revision::Head, TODO)? else {
        return Ok(None);
    };

    Ok(Some(parse(&String::from_utf8_lossy(&list))))
}

/// The entries of a list, in its own order.
fn parse(list: &str) -> Vec<Entry> {
    list.lines().filter_map(entry).collect()
}

/// The entry `line` is — `- [ ] 01: <title> — [details](01-<slug>.md)`, and
/// whatever the list wrote after the link — or none where it is not one.
fn entry(line: &str) -> Option<Entry> {
    let line = line.trim();

    let (rest, checked) = [("- [ ] ", false), ("- [x] ", true), ("- [X] ", true)]
        .iter()
        .find_map(|(mark, checked)| Some((line.strip_prefix(mark)?, *checked)))?;

    let (label, title) = rest.split_once(':')?;
    let label = label.trim();

    if label.is_empty() || !label.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }

    Some(Entry {
        number: label.parse().ok()?,
        label: label.to_owned(),
        checked,
        platform: platform(after(title)),
    })
}

/// What the line wrote after the link to its task file, where it has one.
fn after(title: &str) -> &str {
    let Some(open) = title.find("](").and_then(|link| title[..link].rfind('[')) else {
        return "";
    };

    title[open..]
        .split_once("](")
        .and_then(|(_, target)| target.split_once(')'))
        .map(|(_, after)| after.trim())
        .unwrap_or_default()
}

/// The word after `on` in what a line wrote after its link — `windows` of `—
/// on windows` — or none where it names none, read as the server reads a
/// declaration. Whether the word is a platform is the host's to say. Nothing
/// inside an annotation, `*(…)*`, is read, and one that never closes takes the
/// rest of the line with it.
fn platform(after: &str) -> Option<String> {
    let mut pieces = Vec::new();
    let mut rest = after;

    while let Some(open) = rest.find("*(") {
        pieces.push(&rest[..open]);

        match rest[open..].find(")*") {
            Some(close) => rest = &rest[open + close + ")*".len()..],
            None => {
                rest = "";
                break;
            }
        }
    }
    pieces.push(rest);

    let mut words = pieces.into_iter().flat_map(|piece| {
        piece
            .split(|c: char| c.is_whitespace() || [',', '-', '—', '–', ';'].contains(&c))
            .filter(|word| !word.is_empty())
    });

    let mut platform = None;

    while let Some(word) = words.next() {
        if word.eq_ignore_ascii_case("on") {
            platform = words.next().map(str::to_owned);
        }
    }

    platform
}

/// The number a task file leads with — `05` of `05-pinned-task-list.md` — or
/// none where the name is not one the breakdown wrote.
fn numbered(name: &str) -> Option<u32> {
    let (number, slug) = name.strip_suffix(".md")?.split_once('-')?;

    if slug.is_empty() || number.is_empty() || !number.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }

    number.parse().ok()
}

/// The roadmap stage this Conversation is, where it is one, read off the
/// options the host started it with.
#[derive(Debug, PartialEq, Eq)]
struct Stage {
    cut_from: Option<String>,
}

fn stage(facts: &Facts) -> Option<Stage> {
    let (_, cut_from) = facts.options.iter().find(|(key, _)| key == STAGE)?;

    Some(Stage {
        cut_from: (!cut_from.is_empty()).then(|| cut_from.clone()),
    })
}

/// Why the run stopped at an entry with no task file: the entry, which is the
/// line the human has to go and look at.
fn broken(label: &str) -> String {
    format!(
        "Entry {label} of `{TODO}` isn't ticked off and names no task file, so there's \
         nothing for a session to work from."
    )
}

/// Every session's prompt: the line that sends it into next-task, and the two
/// documents the work is written down in — the task file says where a session
/// stops, and these say what it is a slice of.
fn prompt(skill: &str, brief: &str, handoff: Option<&str>) -> String {
    let mut prompt = format!(
        "Read {skill} and work the next task of the backlog for the work described below, the \
         way it says. Nothing else in this session tells you how to reach me.\n\n\
         # The Brief this started from\n\n{brief}\n",
    );

    if let Some(handoff) = handoff {
        prompt.push_str(&format!(
            "\n# What the grilling settled, in its own words\n\n{handoff}\n"
        ));
    }

    prompt
}

/// A stage's planning prompt: the line that sends it into next-stage, the
/// stage's Brief, and where its branch came from — the one thing about a stage
/// the repository does not say. The base rather than what the stage ends up
/// stacked on: the join is at the finish, and a session told otherwise now
/// would be told something the finish contradicts.
fn planned(skill: &str, brief: &str, cut_from: Option<&str>) -> String {
    let branch = match cut_from {
        Some(base) => format!(
            "This stage's branch was cut from `{base}`, the highest settled branch of its \
             roadmap's chain, which is not merged yet. Where it goes in that chain is settled \
             at this stage's finish rather than now, so there is nothing to stack here.",
        ),
        None => "This stage's branch came off the repository's default branch, there being \
                 nothing unmerged left to stand on."
            .to_owned(),
    };

    format!(
        "Read {skill} and plan the roadmap stage described below, the way it says. Nothing \
         else in this session tells you how to reach me.\n\n\
         # The Brief this started from\n\n{brief}\n\n\
         # Where this stage's branch came from\n\n{branch}\n"
    )
}

/// The finish's prompt with the chain the stage joins listed under it, bottom
/// to top, and the one instruction it is told for: the rebase comes before the
/// push. The chain is the host's to say and the rebase the session's to do. No
/// chain is the prompt unchanged: an empty heading would tell a session there
/// was one to join.
fn joining(prompt: &str, chain: &[host::Link]) -> String {
    let Some((top, under)) = chain.split_last() else {
        return prompt.to_owned();
    };

    let listed: Vec<String> = chain
        .iter()
        .map(|link| match &link.stage {
            Some(stage) => format!("- `{}` — stage {stage}.", link.branch),
            None => format!(
                "- `{}` — the branch this stage was cut from, which the chain starts at.",
                link.branch
            ),
        })
        .collect();

    let top = &top.branch;

    let bottom = match under.is_empty() {
        true => "one branch".to_owned(),
        false => format!("{} branches, bottom to top", chain.len()),
    };

    format!(
        "{}\n\n# The chain this stage joins\n\nThis stage's roadmap is one chain of branches, in \
         the order its stages finished, and this branch joins it at this finish. The chain is \
         {bottom}:\n\n{listed}\n\nSo **rebase this branch onto `{top}` before anything is \
         pushed**, and open the pull request against `{top}`. Do the rebase the way this \
         repository's own `docs/agents/git-workflow.md` says a stage joins a chain, where it says \
         anything; where it records no stacking mechanism, rebase onto `{top}` all the same and \
         open an ordinary pull request against it, which carries the stage below this one until \
         that one merges.\n\nA branch already sitting on top of `{top}` is a rebase that moves \
         nothing, which is the ordinary case rather than a sign anything is wrong. Rebase **once**, \
         before the pull request exists: nothing in the chain below moves because this stage \
         joined above it — no branch of it is rebased, and no pull request of it changes base — \
         and this branch is never force-pushed once its own pull request is open.\n",
        prompt.trim_end(),
        listed = listed.join("\n"),
    )
}

#[cfg(test)]
mod tests {
    //! What the server's runner was tested on before the backlog crossed into
    //! this component: the step read off a list, and what each session is told.
    //! Run natively — `cargo test --manifest-path plugins/Cargo.toml` — since
    //! none of it calls the host.

    use super::*;

    /// A backlog of three, its first `done` entries ticked.
    fn list(done: usize) -> String {
        [("01", "First"), ("02", "Second"), ("03", "Third")]
            .iter()
            .enumerate()
            .map(|(at, (label, title))| {
                let mark = if at < done { "x" } else { " " };
                let file = format!("{label}-{}.md", title.to_lowercase());

                format!("- [{mark}] {label}: {title} — [details]({file})\n")
            })
            .collect()
    }

    /// The three task files that backlog names.
    fn documents() -> Vec<String> {
        ["01-first.md", "02-second.md", "03-third.md", "TODO.md"]
            .map(str::to_owned)
            .to_vec()
    }

    /// The step `list` reads as, its directory holding `files`.
    fn next_step(list: Option<&str>, files: Vec<String>) -> Result<Step, String> {
        step(list, None, || Ok(files))
    }

    fn task(number: u32, label: &str) -> Step {
        Step::Task {
            number,
            label: label.to_owned(),
            platform: None,
        }
    }

    #[test]
    fn the_next_step_is_the_lowest_unticked_entry() {
        assert_eq!(next_step(Some(&list(0)), documents()), Ok(task(1, "01")));
        assert_eq!(next_step(Some(&list(1)), documents()), Ok(task(2, "02")));
    }

    /// The ticked entries are done, their files left behind and all.
    #[test]
    fn a_task_file_left_behind_is_not_a_task_still_to_do() {
        assert_eq!(next_step(Some(&list(2)), documents()), Ok(task(3, "03")));
    }

    #[test]
    fn an_unticked_entry_with_no_file_stops_the_run() {
        assert_eq!(
            next_step(Some(&list(1)), vec!["01-first.md".to_owned()]),
            Ok(Step::Broken {
                label: "02".to_owned()
            }),
        );

        assert!(
            broken("02").contains("Entry 02 of `.tasks/TODO.md`"),
            "and the stop names the entry: {:?}",
            broken("02"),
        );
    }

    /// In the list's own order, and by number rather than by the label's text.
    #[test]
    fn a_backlog_that_got_past_nine_is_still_worked_in_order() {
        assert_eq!(
            next_step(
                Some("- [ ] 9: Ninth\n- [ ] 10: Tenth\n"),
                vec!["9-ninth.md".to_owned(), "10-tenth.md".to_owned()],
            ),
            Ok(task(9, "9")),
        );
    }

    #[test]
    fn the_finish_step_is_what_is_left_once_every_entry_is_ticked() {
        assert_eq!(
            step(Some(&list(3)), None, || panic!("the finish needs no file")),
            Ok(Step::Finish),
        );
    }

    /// A Conversation that is no stage has nothing to run with no list, and
    /// says so; a stage plans one, told what it was cut from.
    #[test]
    fn no_backlog_is_nothing_to_run_unless_a_stage_plans_one() {
        assert!(next_step(None, Vec::new()).is_err());

        let stage = Stage {
            cut_from: Some("roadmaps/mvp/01-workbench".to_owned()),
        };

        assert_eq!(
            step(None, Some(&stage), || panic!("a plan needs no file")),
            Ok(Step::Plan {
                cut_from: Some("roadmaps/mvp/01-workbench".to_owned())
            }),
        );
    }

    /// The word after `on` past the link, and nothing inside an annotation.
    #[test]
    fn a_task_wants_the_platform_its_line_names() {
        let wants = |line: &str| entry(line).and_then(|entry| entry.platform);

        assert_eq!(
            wants("- [ ] 02: Installer — [details](02-installer.md) — on windows"),
            Some("windows".to_owned()),
        );
        assert_eq!(wants("- [ ] 02: Run on windows — [details](02-x.md)"), None);
        assert_eq!(
            wants("- [ ] 02: X — [details](02-x.md) *(in progress: `on windows`)*"),
            None,
        );
    }

    fn link(stage: Option<&str>, branch: &str) -> host::Link {
        host::Link {
            stage: stage.map(str::to_owned),
            branch: branch.to_owned(),
        }
    }

    #[test]
    fn a_stage_session_is_started_on_its_brief() {
        let prompt = planned(
            "/skills/next-stage/SKILL.md",
            "# 05. Roadmap direction\n\nThe third Direction.\n",
            None,
        );

        assert!(prompt.contains("/skills/next-stage/SKILL.md"), "{prompt:?}");
        assert!(prompt.contains("The third Direction."), "{prompt:?}");
        assert!(
            !prompt.contains("What the grilling settled"),
            "nothing is said about a document a stage never had: {prompt:?}"
        );
    }

    #[test]
    fn a_stage_session_is_told_what_its_branch_was_cut_from() {
        let cut = planned("next-stage", "# 05.\n", Some("wrap-up"));

        assert!(cut.contains("cut from `wrap-up`"), "{cut:?}");
        assert!(
            !cut.contains("stacks on"),
            "and not as a place in the chain, which the finish settles: {cut:?}"
        );

        let alone = planned("next-stage", "# 05.\n", None);

        assert!(
            alone.contains("default branch") && !alone.contains("stacks on"),
            "{alone:?}"
        );
    }

    #[test]
    fn a_task_session_is_told_the_brief_and_the_handoff() {
        let prompt = prompt("next-task", "# Rate limiting\n", Some("Settled: a window."));

        assert!(
            prompt.contains("work the next task of the backlog"),
            "{prompt:?}"
        );
        assert!(prompt.contains("# The Brief this started from\n\n# Rate limiting"));
        assert!(prompt.contains("# What the grilling settled, in its own words\n\nSettled"));
        assert!(
            !super::prompt("next-task", "# Rate limiting\n", None).contains("grilling settled"),
            "and nothing about a handoff there isn't",
        );
    }

    #[test]
    fn the_finish_of_a_stage_is_told_the_chain_it_joins_bottom_to_top() {
        let prompt = joining(
            &prompt("next-task", "# Rate limiting\n\nThe API has none.\n", None),
            &[
                link(None, "rate-limiting"),
                link(Some("01"), "roadmaps/rate-limiting/01-the-counter"),
                link(Some("02"), "roadmaps/rate-limiting/02-the-window"),
            ],
        );

        assert!(
            prompt.contains("# The Brief this started from"),
            "{prompt:?}"
        );

        let chain = prompt
            .split_once("# The chain this stage joins")
            .expect("the chain is a section of its own")
            .1;

        let counter = chain
            .find("`roadmaps/rate-limiting/01-the-counter`")
            .expect("the stage below is named");
        let window = chain
            .find("`roadmaps/rate-limiting/02-the-window`")
            .expect("and so is the one above it");

        assert!(
            chain.find("`rate-limiting`").expect("and the foot") < counter && counter < window,
            "bottom to top: {chain:?}"
        );
        assert!(chain.contains("stage 01") && chain.contains("stage 02"));
        assert!(chain.contains("cut from"), "{chain:?}");
    }

    #[test]
    fn the_finish_is_told_the_rebase_comes_before_the_push() {
        let prompt = joining(
            &prompt("next-task", "# Rate limiting\n", None),
            &[link(Some("01"), "roadmaps/rate-limiting/01-the-counter")],
        );

        for said in [
            "**rebase this branch onto `roadmaps/rate-limiting/01-the-counter` before anything \
             is pushed**",
            "open the pull request against `roadmaps/rate-limiting/01-the-counter`",
            "docs/agents/git-workflow.md",
            "records no stacking mechanism",
            "never force-pushed once its own pull request is open",
            "is a rebase that moves nothing",
        ] {
            assert!(prompt.contains(said), "{said:?} in {prompt:?}");
        }
    }

    #[test]
    fn a_session_with_no_chain_to_join_is_started_on_the_prompt_as_it_stands() {
        let prompt = prompt("next-task", "# Rate limiting\n", None);

        assert_eq!(joining(&prompt, &[]), prompt);
    }
}
