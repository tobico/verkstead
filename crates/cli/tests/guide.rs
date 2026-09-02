//! The Guide as the binary hands it over: `verkstead guide`, bare
//! `verkstead`, the asking sections tailored to the backend reading them, and
//! the promise that the CLI contract it quotes is the one this binary has.

use std::process::{Command, Output};

use verkstead_schema::{Question, QuestionSet};

/// Which backend a Guide is being printed for, as the sandbox says it.
const AGENT_TYPE: &str = "VERKSTEAD_AGENT";

/// Run the binary with `args` and insist it had something to say.
///
/// With no agent type set, whatever the environment running the suite happens
/// to hold: every assertion here that is not about tailoring is about the
/// blocking Guide, which is what nothing set means — and a suite run from
/// inside a sandbox would otherwise be asking those questions of whichever
/// backend it happened to be running on.
fn run(args: &[&str]) -> Output {
    running(args).env_remove(AGENT_TYPE).output()
}

/// And the same as a session of `agent_type` runs it, which is the whole of
/// what tailors the Guide.
fn run_as(agent_type: &str, args: &[&str]) -> Output {
    running(args).env(AGENT_TYPE, agent_type).output()
}

/// One run, however the environment is set — see the two above.
fn running(args: &[&str]) -> Running {
    let mut command = Command::new(env!("CARGO_BIN_EXE_verkstead"));
    command.args(args);

    Running {
        args: args.iter().map(|arg| (*arg).to_owned()).collect(),
        command,
    }
}

struct Running {
    args: Vec<String>,
    command: Command,
}

impl Running {
    fn env(mut self, key: &str, value: &str) -> Self {
        self.command.env(key, value);
        self
    }

    fn env_remove(mut self, key: &str) -> Self {
        self.command.env_remove(key);
        self
    }

    fn output(mut self) -> Output {
        let output = self
            .command
            .output()
            .expect("the verkstead binary should be built for its own tests");
        eprintln!(
            "verkstead {:?} stderr:\n{}",
            self.args,
            String::from_utf8_lossy(&output.stderr)
        );
        output
    }
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

/// The body of the first fenced block after `heading`, which is where the Guide
/// quotes something verbatim.
fn quoted_after(guide: &str, heading: &str) -> String {
    let section = guide
        .split_once(heading)
        .unwrap_or_else(|| panic!("the Guide should have a {heading:?} section"))
        .1;
    let fence = section
        .split_once("```")
        .expect("that section should quote something in a fenced block")
        .1;
    let body = fence
        .split_once('\n')
        .expect("a fence opens a line of its own")
        .1;
    body.split_once("```")
        .expect("the fence should close")
        .0
        .to_string()
}

/// The bodies of every fenced block in `text`, in order, without their info
/// strings — for a section that shows more than one example. A fence indented
/// under a list item counts, and comes back with that indent removed, so the
/// body is the YAML as it would be pasted.
fn fenced_blocks(text: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut open: Option<(usize, Vec<&str>)> = None;
    for line in text.lines() {
        let indent = line.len() - line.trim_start().len();
        if line.trim_start().starts_with("```") {
            match open.take() {
                None => open = Some((indent, Vec::new())),
                Some((_, body)) => blocks.push(body.join("\n")),
            }
        } else if let Some((indent, body)) = open.as_mut() {
            body.push(line.get(*indent..).unwrap_or(line.trim_start()));
        }
    }
    blocks
}

/// Everything under `heading` up to the next one: the section in full, its
/// examples and all.
fn section<'a>(guide: &'a str, heading: &str) -> &'a str {
    guide
        .split_once(heading)
        .unwrap_or_else(|| panic!("the Guide should have a {heading:?} section"))
        .1
        .split("\n## ")
        .next()
        .unwrap()
}

/// The Guide with its fenced blocks dropped — what it says in its own voice,
/// as against what it quotes. An example Response is the human talking, and a
/// human says "I".
fn prose(guide: &str) -> String {
    guide
        .split("\n```")
        .step_by(2)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Compare rendered text by line, ignoring trailing whitespace: clap pads a
/// blank line inside an option's help, and no editor keeps that padding alive
/// in a markdown file. Everything that carries meaning survives the trim.
fn lines(text: &str) -> Vec<&str> {
    text.trim_end().lines().map(str::trim_end).collect()
}

#[test]
fn the_guide_command_prints_the_guide() {
    let output = run(&["guide"]);

    assert!(output.status.success(), "`verkstead guide` should exit 0");
    assert!(
        stdout(&output).contains("## The CLI contract"),
        "the Guide should be on stdout, got:\n{}",
        stdout(&output)
    );
}

#[test]
fn bare_verkstead_prints_the_same_guide() {
    let bare = run(&[]);
    let explicit = run(&["guide"]);

    assert!(
        bare.status.success(),
        "bare `verkstead` should print the Guide rather than a usage error"
    );
    assert_eq!(
        stdout(&bare),
        stdout(&explicit),
        "an agent that runs the binary with no arguments should get the Guide"
    );
}

#[test]
fn the_help_about_text_points_at_the_guide() {
    let output = run(&["--help"]);

    assert!(output.status.success());
    assert!(
        stdout(&output).contains("verkstead guide"),
        "an agent that starts at --help should be sent to the Guide, got:\n{}",
        stdout(&output)
    );
}

#[test]
fn the_guide_covers_every_core_area() {
    let guide = stdout(&run(&["guide"]));

    for heading in [
        "## Two kinds of ask",
        "## Question labels",
        "## Pacing",
        "## Authoring the Set",
        "## The CLI contract",
        "## Running the ask",
        "## Reading the Response",
    ] {
        assert!(
            guide.contains(heading),
            "the core Guide should cover {heading:?} — an agent reads nothing else \
             before asking"
        );
    }
}

/// The gates Topic is gone, and with it every route into it: nothing in the
/// pipeline gates a commit any more, and an agent sent after required reading
/// the binary no longer carries gets an error where it expected the rules.
#[test]
fn the_guide_sends_nobody_to_a_topic_that_is_not_there() {
    let guide = stdout(&run(&["guide"]));

    assert!(
        !guide.contains("verkstead guide gates"),
        "no passage should send an agent to the retired gates Topic, got:\n{guide}"
    );
    assert!(
        !guide.contains("Required topic guides"),
        "and none should promise Topics the Guide no longer has, got:\n{guide}"
    );
}

/// "Anything else?" is the Postscript's job: the comment box asks it on every
/// Set, so a Question that asks it again spends a row the human then has to
/// leave explicitly open. The reverse holds too — a decision, however
/// trivial, is a Question, never parked in the Postscript where nothing
/// obliges a reply. An agent that reads only the worked example copies
/// whatever that example ends with, so the field has to be in it.
#[test]
fn the_guide_sends_the_catch_all_to_the_postscript() {
    let guide = stdout(&run(&["guide"]));

    let contract = section(&guide, "## The CLI contract");
    assert!(
        contract.contains("`postscript`"),
        "the Set shape is where an agent checks which fields exist, so it has \
         to list `postscript`, got:\n{contract}"
    );

    let authoring = section(&guide, "## Authoring the Set");
    assert!(
        authoring.contains("postscript:"),
        "the worked example should close its Set with a `postscript`, got:\n{authoring}"
    );
    assert!(
        authoring.contains("never a Question"),
        "the authoring section should say outright that a catch-all is never a \
         Question, got:\n{authoring}"
    );
    assert!(
        authoring.contains("a decision, however small, is a Question"),
        "the authoring section should carry the litmus — a decision, however \
         small, is a Question — got:\n{authoring}"
    );
    assert!(
        authoring.contains("Write an ADR for this?"),
        "the authoring section should name the canonical mistake, the trivial \
         yes/no parked in the postscript, got:\n{authoring}"
    );
    assert!(
        !guide.contains("Anything worth knowing before this starts?")
            && !guide.contains("often saves a whole round trip"),
        "no passage should still recommend the trailing catch-all Question the \
         Postscript replaced, got:\n{guide}"
    );
}

/// The Set shape is where an agent checks which fields exist before it
/// serializes one. A shape that omits the Answer Table's fields is a shape that
/// says they aren't there.
#[test]
fn the_set_shape_names_the_answer_table_fields() {
    let guide = stdout(&run(&["guide"]));
    let contract = section(&guide, "## The CLI contract");

    assert!(
        contract.contains("columns") && contract.contains("cells"),
        "the Set shape should list `columns` and `cells`, so the shape an agent \
         serializes is complete at a glance, got:\n{contract}"
    );
}

/// The Guide is the only place the Answer Table is discoverable, and the old
/// pattern — a markdown table in the Question's text, echoed as a list of
/// Options below it — is what an agent writes without it.
#[test]
fn the_guide_teaches_the_answer_table_declaration() {
    let guide = stdout(&run(&["guide"]));
    let authoring = section(&guide, "## Authoring the Set");

    for phrase in ["`columns`", "`cells`", "leading cell"] {
        assert!(
            authoring.contains(phrase),
            "the authoring section should teach the declaration and mention \
             {phrase} — no other Topic covers it, got:\n{authoring}"
        );
    }
    assert!(
        !guide.contains("Prefer a comparison table where Options trade off"),
        "the old bullet steered an agent to a table it wrote itself — the \
         declaration replaces it, got:\n{guide}"
    );
}

/// An example of a declaration only teaches it if it is one: pasted into a Set
/// it has to parse, pass the grammar, and come out a table rather than a list.
#[test]
fn the_answer_table_example_round_trips() {
    let guide = stdout(&run(&["guide"]));
    let example = fenced_blocks(section(&guide, "## Authoring the Set"))
        .into_iter()
        .find(|block| block.contains("columns:"))
        .expect("the authoring section should show the declaration as YAML");

    let set = QuestionSet::from_yaml(&format!("title: Pasted out of the Guide\n{example}"))
        .expect("the example should parse as the Questions of a Set");
    set.validate()
        .expect("the example should pass the grammar the server holds Sets to");

    let question = set
        .questions
        .iter()
        .find(|question| !question.columns.is_empty())
        .expect("the example should declare an Answer Table");
    assert!(
        question.options.len() > 1,
        "a table of one row teaches nothing about the axes it compares along"
    );
    for option in &question.options {
        assert_eq!(
            option.cells.len(),
            question.columns.len(),
            "every Option fills every column — that is what makes the rows a table"
        );
        assert!(
            !option.text.trim().is_empty(),
            "the Option's `text` is the row's leading cell, so every row has one"
        );
    }
}

/// The Response the Guide shows answers the Set the Guide shows. An example
/// that answers a Question the example Set never asks teaches an agent to wait
/// for an Answer that cannot arrive — and a Heading is exactly that Question,
/// so the example Response has to step over the one the example Set heads with.
#[test]
fn the_example_response_answers_the_example_set() {
    let guide = stdout(&run(&["guide"]));
    let set = QuestionSet::from_yaml(&quoted_after(&guide, "## Authoring the Set"))
        .expect("the worked example should parse as a Set");
    let response = quoted_after(&guide, "## Reading the Response");

    let mut asked: Vec<String> = Vec::new();
    for question in &set.questions {
        if !question.heading() {
            asked.push(question.name().to_string());
        }
        for subquestion in &question.subquestions {
            asked.push(subquestion.name(question));
        }
    }

    let answered: Vec<String> = response
        .lines()
        .filter_map(|line| line.trim().strip_prefix("- label: "))
        .map(|label| label.trim().to_string())
        .collect();

    assert_eq!(
        answered, asked,
        "every Question and Sub-question comes back exactly once and a Heading \
         never does — the example Response should answer the example Set and \
         nothing besides"
    );
}

/// Both nesting shapes have to be shown, not just described. A Heading is what
/// an agent reaches for the first time it batches related decisions, and the
/// prose that defines one sits ~100 lines from any YAML — so an agent that
/// writes its Set from the labels section invents `label:` on a Sub-question
/// and a `questions:` key to nest under, and gets two refusals for it.
#[test]
fn the_worked_example_shows_both_nesting_shapes() {
    let guide = stdout(&run(&["guide"]));
    let set = QuestionSet::from_yaml(&quoted_after(&guide, "## Authoring the Set"))
        .expect("the worked example should parse as a Set");
    set.validate()
        .expect("the worked example should pass the grammar the server holds Sets to");

    assert!(
        set.questions.iter().any(Question::heading),
        "the worked example should show a Heading — Sub-questions under a \
         Question with no Options of its own, which no other example spells"
    );
    assert!(
        set.questions
            .iter()
            .any(|question| !question.heading() && !question.subquestions.is_empty()),
        "and a Question carrying both its own Options and Sub-questions, so the \
         two shapes are told apart by example rather than by prose alone"
    );
}

/// The labels section introduces the `Q7a` notation ~100 lines before the YAML
/// that spells it, so it is where an agent's model of the tree actually forms.
/// One that teaches the label without naming the fields teaches a shape the
/// server refuses.
#[test]
fn the_labels_section_names_the_fields_behind_the_notation() {
    let guide = stdout(&run(&["guide"]));
    let labels = section(&guide, "## Question labels");

    for phrase in ["`subquestions`", "`letter"] {
        assert!(
            labels.contains(phrase),
            "the labels section should name {phrase} where it introduces the \
             `Q7a` notation, got:\n{labels}"
        );
    }
}

/// The comment box is always there and always optional, so an empty one is an
/// answer of its own. An agent that reads it as an oversight asks again for
/// something the human already declined to say.
#[test]
fn an_absent_comment_means_the_human_had_nothing_to_add() {
    let guide = stdout(&run(&["guide"]));
    let reading = section(&guide, "## Reading the Response");

    assert!(
        reading.contains("nothing to add"),
        "the Response section should say what an absent `comment` means, \
         got:\n{reading}"
    );
}

/// An unknown Topic is a mistake worth catching loudly: the agent asked for
/// required reading and there is none to give it. `gates` is the case that
/// matters now the Topic is retired — an agent still carrying the old
/// instruction has to be told the reading is gone rather than handed the
/// core Guide as though it were the Topic.
#[test]
fn an_unknown_topic_is_an_error_saying_the_guide_has_no_topics() {
    for name in ["nonsense", "gates"] {
        let output = run(&["guide", name]);

        assert!(
            !output.status.success(),
            "a Topic that does not exist should fail rather than print something else"
        );
        assert_eq!(
            stdout(&output),
            "",
            "stdout stays clean, so nothing is mistaken for the Topic"
        );

        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(
            stderr.contains(name) && stderr.contains("no Topics"),
            "the error should name what was asked for and say the Guide has no \
             Topics, got:\n{stderr}"
        );
    }
}

/// The Guide is the whole of what an agent reads, so it can't lean on a
/// conversation it can't see: no chat to fall back to, no transport to detect,
/// no reply grammar of its own, and no first person for a human who is
/// somewhere else entirely.
#[test]
fn the_guide_stands_alone() {
    stands_alone(&stdout(&run(&["guide"])));
}

fn stands_alone(guide: &str) {
    for phrase in [
        "in chat",
        "chat fallback",
        "fall back",
        "falling back",
        "command -v",
        "reply grammar",
        "tobico",
        "/next-task",
        "/grilling",
    ] {
        assert!(
            !guide.contains(phrase),
            "the Guide should not mention {phrase:?} — the binary is the only \
             transport and the only documentation"
        );
    }

    let prose = prose(guide);
    let first_person: Vec<&str> = prose
        .split(|c: char| !c.is_alphanumeric() && c != '\'')
        .filter(|word| {
            matches!(*word, "I" | "I'm" | "I'd" | "I'll" | "I've")
                || matches!(word.to_lowercase().as_str(), "me" | "my" | "mine")
        })
        .collect();

    assert!(
        first_person.is_empty(),
        "the Guide speaks of the human in the third person — found {first_person:?}"
    );
}

/// The two kinds of ask, and the one rule that decides between them. An agent
/// that never reads the flag blocks on everything, which is the whole cost the
/// deferred kind exists to take off the human.
#[test]
fn the_guide_names_both_kinds_of_ask_and_when_to_use_each() {
    let guide = stdout(&run(&["guide"]));
    let kinds = section(&guide, "## Two kinds of ask");

    assert!(
        kinds.contains("verkstead ask --deferred"),
        "the section should name the flag that defers, got:\n{kinds}"
    );
    assert!(
        kinds.contains("Block only on Questions whose Answers affect the work about to be done"),
        "and the rule that decides which kind a Question is, in the terms the \
         design states it in, got:\n{kinds}"
    );
    assert!(
        kinds.contains("folded into the prompt"),
        "and where a deferred Answer goes, which is the reason it is worth \
         asking something nothing waits for, got:\n{kinds}"
    );
}

/// And the name in those usage lines is the command's own rather than the
/// file's, which is what lets the quotation above hold on every platform.
///
/// Clap otherwise reads `argv[0]`, and on Windows that is `verkstead.exe` — a
/// usage line no document in this repository spells. Proved by running the
/// binary under that very name, which any machine can do and which a Windows
/// one does without being asked.
#[test]
fn the_usage_lines_name_the_command_rather_than_the_file_that_was_run() {
    let under = tempfile::tempdir().unwrap();
    let renamed = under.path().join("verkstead.exe");
    std::fs::copy(env!("CARGO_BIN_EXE_verkstead"), &renamed).unwrap();

    let said = String::from_utf8(
        Command::new(&renamed)
            .args(["ask", "--help"])
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap();

    assert!(
        said.contains("Usage: verkstead ask"),
        "the usage line should name `verkstead` however the file is called, got:\n{said}"
    );
}

#[test]
fn the_guides_quoted_cli_contract_is_the_real_one() {
    let guide = stdout(&run(&["guide"]));
    let quoted = quoted_after(&guide, "## The CLI contract");
    let real = stdout(&run(&["ask", "--help"]));

    assert_eq!(
        lines(&quoted),
        lines(&real),
        "the Guide quotes `verkstead ask --help` verbatim — copy the current \
         output into the CLI contract section of crates/cli/guide/core.md"
    );
}

/// A review's Set is a plain Question Set like every other, so the Guide teaches
/// one grammar and only one. The block a review used to carry has left the
/// schema, and a Guide that still described it would be teaching an agent to
/// write something nothing reads.
#[test]
fn the_guide_teaches_no_findings_grammar() {
    let guide = stdout(&run(&["guide"]));

    for phrase in ["findings:", "review:", "fix: Q", "split: Q"] {
        assert!(
            !guide.contains(phrase),
            "the Guide should carry no trace of the findings grammar, and it has \
             {phrase} in:\n{guide}"
        );
    }
}

/// The Guide is one document tailored at print time, and what it tailors is
/// this end of an ask: how one is run, and what comes back from it.
///
/// A backend whose shell tool cannot hold a command open for hours reads that
/// its ask returns at once, that the turn ends there, and that `verkstead
/// answers` is what fetches the Answers when the nudge lands. Read the blocking
/// advice, such a session would hold an ask open by polling it — a paid model
/// turn per poll, for hours.
#[test]
fn a_store_and_nudge_backend_reads_its_own_channel() {
    let guide = stdout(&run_as("codex", &["guide"]));
    let kinds = section(&guide, "## Two kinds of ask");
    let running = section(&guide, "## Running the ask");

    assert!(
        kinds.contains("the turn ends there"),
        "the two kinds should say that an ask on this channel ends the turn, \
         got:\n{kinds}"
    );
    assert!(
        running.contains("verkstead answers"),
        "and running one should name the command that fetches the Answers, \
         got:\n{running}"
    );
    assert!(
        !running.contains("run_in_background"),
        "and none of the hold-the-ask advice should survive: it is Claude \
         Code's mechanism and is false of this backend, got:\n{running}"
    );
}

/// And a Claude session reads what it always read. Claude Code holds the ask
/// open in a background shell call its harness wakes it from, which is the
/// whole of how it asks — and is Claude Code's own rather than every blocking
/// backend's, which is what the test below this one is about.
#[test]
fn a_blocking_backend_reads_the_guide_it_always_read() {
    let claude = stdout(&run_as("claude", &["guide"]));

    assert_eq!(
        claude,
        stdout(&run(&["guide"])),
        "Claude's is what nothing set prints, so a Claude session and a human \
         at a terminal read the same document"
    );

    let running = section(&claude, "## Running the ask");
    assert!(
        running.contains("run_in_background"),
        "Claude Code keeps the advice about holding the ask open in a \
         background shell call, got:\n{running}"
    );
    assert!(
        !running.contains("verkstead answers"),
        "and never sends a session that is already holding the Response to \
         fetch it again, got:\n{running}"
    );
}

/// And the second backend that blocks reads how *it* holds an ask, which is
/// not how Claude Code holds one.
///
/// The two share a channel and not a mechanism. Claude Code makes the call a
/// background one and is woken when it returns; opencode's shell tool runs the
/// command synchronously in the turn and kills it at the timeout it was given,
/// so what an OpenCode session has to be told is to pass a large one. Handed
/// Claude's instruction it would go looking for a harness feature it has not
/// got — and handed no instruction at all it would take the tool's own default
/// and lose the ask minutes into a wait measured in hours.
#[test]
fn opencode_reads_how_it_holds_an_ask_rather_than_how_claude_does() {
    let opencode = stdout(&run_as("opencode", &["guide"]));
    let claude = stdout(&run_as("claude", &["guide"]));

    let running = section(&opencode, "## Running the ask");

    assert!(
        running.contains("timeout"),
        "the section should name the thing that decides whether the ask \
         survives the wait, got:\n{running}"
    );
    assert!(
        running.contains("86400000"),
        "and say a value in the units the tool takes, rather than leaving \
         'large' to be guessed at, got:\n{running}"
    );
    assert!(
        !running.contains("run_in_background") && !running.contains("background shell"),
        "and none of Claude Code's mechanism should survive: an OpenCode \
         session told to background the call goes looking for a harness \
         feature it has not got, got:\n{running}"
    );
    assert!(
        !running.contains("verkstead answers"),
        "nor should one already holding the Response be sent to fetch it, \
         got:\n{running}"
    );
    assert_ne!(
        running,
        section(&claude, "## Running the ask"),
        "which is the whole of what says this section is the backend's rather \
         than its channel's",
    );

    assert_eq!(
        section(&opencode, "## Two kinds of ask"),
        section(&claude, "## Two kinds of ask"),
        "the kinds *are* the channel's, though: both of these block, and what \
         a blocking ask is is the same thing on either",
    );
}

/// Every backend reads one Guide, so everything about writing a Set is written
/// once and read the same whichever is reading. What differs is this end and
/// nothing else.
#[test]
fn only_the_two_asking_sections_differ_between_the_backends() {
    let blocking = stdout(&run_as("claude", &["guide"]));
    let store_and_nudge = stdout(&run_as("codex", &["guide"]));
    let opencode = stdout(&run_as("opencode", &["guide"]));

    for heading in [
        "## Question labels",
        "## Pacing",
        "## The CLI contract",
        "## Authoring the Set",
        "## Reading the Response",
    ] {
        assert_eq!(
            section(&blocking, heading),
            section(&store_and_nudge, heading),
            "{heading} is about the Set rather than about this end, so every \
             backend should read the same words"
        );
        assert_eq!(
            section(&blocking, heading),
            section(&opencode, heading),
            "{heading} is about the Set rather than about this end, so every \
             backend should read the same words"
        );
    }

    assert_ne!(
        section(&blocking, "## Two kinds of ask"),
        section(&store_and_nudge, "## Two kinds of ask"),
    );
    assert_ne!(
        section(&blocking, "## Running the ask"),
        section(&store_and_nudge, "## Running the ask"),
    );
}

/// And nothing anywhere else in a store-and-nudge Guide tells its reader the
/// ask blocks.
///
/// The two asking sections are what the tailoring splits, but the sections
/// around them are shared — so a sentence written for the blocking channel and
/// left in the common half reaches a backend it is false of, which is exactly
/// what tailoring the Guide was for. The whole document rather than the two
/// sections, because that is where such a sentence hides.
///
/// Its own voice rather than what it quotes: the CLI contract is `verkstead ask
/// --help` verbatim, and what that says is pinned by
/// [`the_guides_quoted_cli_contract_is_the_real_one`] against the binary itself.
#[test]
fn nothing_in_a_store_and_nudge_guide_says_the_ask_blocks() {
    let store_and_nudge = prose(&stdout(&run_as("codex", &["guide"])));
    let blocking = prose(&stdout(&run_as("claude", &["guide"])));

    for mechanism in [
        "run_in_background",
        "background shell",
        "blocks until",
        "reconnect",
        "While waiting",
    ] {
        assert!(
            !store_and_nudge.contains(mechanism),
            "{mechanism:?} is the blocking channel's own mechanism and is false \
             of a backend that cannot hold an ask open",
        );
        assert!(
            blocking.contains(mechanism),
            "{mechanism:?} should still be somewhere in the blocking Guide — a \
             phrase that has gone from both is one this test stopped checking",
        );
    }
}

/// A word this binary has not got is refused by name.
///
/// Read past as blocking, a Guide would hand the hold-the-ask advice to a
/// backend that cannot hold one — which is a session wedged for hours rather
/// than an error anybody sees. The variable is Verkstead's own to set, so
/// anything else in it is a mistake worth stopping on.
#[test]
fn an_agent_type_this_binary_has_not_got_is_refused_by_name() {
    let output = run_as("nonesuch", &["guide"]);

    assert!(
        !output.status.success(),
        "a Guide printed for a backend this binary does not know should fail"
    );
    assert_eq!(
        stdout(&output),
        "",
        "stdout stays clean, so no channel's instructions are mistaken for \
         another's"
    );

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("nonesuch") && stderr.contains(AGENT_TYPE),
        "the error should name the word and where it came from, got:\n{stderr}"
    );
}

/// Bare `verkstead` is tailored too: it is the same call by another route, and
/// an agent that runs the binary to see what it is reads its own channel.
#[test]
fn bare_verkstead_is_tailored_the_same_way() {
    assert_eq!(
        stdout(&run_as("codex", &[])),
        stdout(&run_as("codex", &["guide"])),
    );
}

/// The tailored halves stand alone the way the whole does: no chat to fall back
/// to, no transport to detect, and the human is somewhere else entirely. Every
/// backend's, because each of the three sections is written for one of them and
/// a passage that leans on a conversation would be in whichever was written
/// last.
#[test]
fn every_backends_guide_stands_alone() {
    for agent_type in ["claude", "codex", "grok", "opencode"] {
        stands_alone(&stdout(&run_as(agent_type, &["guide"])));
    }
}
