//! Rendering the attached Diff to HTML.
//!
//! Server-only, like the Preface's markdown: the browser gets the rendered
//! result, so no diff parser and no highlighter ship to the client.
//!
//! Stage 01 stores the Diff as one raw diff string, so the parsing happens
//! here — per file, per hunk, per line. Unified and combined alike: a merge
//! commit is described by the combined diff of its parents, which says the same
//! things with one marker column per parent. Every scrap of text from the
//! Diff is escaped on its way out; the HTML around it is ours, which is why
//! this output is not run through a sanitiser the way the Preface's is (a
//! sanitiser would take the class attributes the colouring depends on with it).
//!
//! The highlighter itself lives in `crate::highlight`, shared with the fenced
//! blocks in the agent's markdown.

use syntect::parsing::SyntaxReference;

use crate::DiffView;
use crate::highlight::{escaped, for_path};

/// What the one section of a Diff git did not write is called — in the page and
/// in the table of contents alike, since both name the same fold.
const AS_IT_ARRIVED: &str = "The Diff, as it arrived";

/// Render a diff to HTML, or `None` when there is nothing in it to show.
///
/// Unified or combined: a merge commit's patch is the combined diff of its
/// parents, and draws as files and hunks like any other.
///
/// Each file's section is anchored by its position in the Diff — `diff-1`,
/// `diff-2`, … — rather than by its path, which would have to be squeezed into
/// an id and could collide once it was. A Set is immutable once sent, so a
/// position is a name that holds for its lifetime.
///
/// The paths come back beside the HTML in that same order, so the table of
/// contents can name the folds without reading them back out of the markup it
/// was handed.
pub fn to_html(diff: &str) -> Option<DiffView> {
    block(diff, 1)
}

/// The same, for a patch that is one block of a Diff made of several: `first` is
/// the position its first file takes, counting from one across the whole Diff.
///
/// The positions run on rather than restarting per block, because they are ids
/// on one page: two blocks that each numbered from one would put two `diff-1`s
/// in it, and a jump would land on whichever came first.
pub fn block(diff: &str, first: usize) -> Option<DiffView> {
    if diff.trim().is_empty() {
        return None;
    }

    let files = files(diff);

    // Whatever this is, git did not write it — but it was attached to the Set as
    // the Diff, so it gets shown as it arrived rather than swallowed. It is
    // still the block's first and only section, so it is anchored as one, and
    // named as one.
    if files.is_empty() {
        // Marked as verbatim, because it has none of the line cells that hold a
        // hunk's text off the left edge and so needs the inset put on it.
        let mut html = format!(
            r#"<details class="diffFile" id="diff-{first}" open><summary><span class="diffPath">{AS_IT_ARRIVED}</span></summary><div class="diffHunk"><pre class="diffLines diffVerbatim"><code>"#
        );
        html.push_str(&escaped(diff));
        html.push_str("</code></pre></div></details>");
        return Some(DiffView {
            html,
            paths: vec![AS_IT_ARRIVED.to_owned()],
        });
    }

    let mut html = String::new();
    for (position, file) in files.iter().enumerate() {
        file.render(&mut html, first + position);
    }
    Some(DiffView {
        html,
        paths: files.iter().map(|file| file.path.clone()).collect(),
    })
}

/// One file's worth of the Diff.
#[derive(Debug, PartialEq, Eq)]
struct FileDiff {
    /// The path as the repository knows it, without the diff's `a/` and `b/`.
    path: String,

    /// What became of the file, when it was more than an edit.
    status: Option<&'static str>,

    /// Said instead of hunks, when git described the change without spelling it
    /// out.
    note: Option<&'static str>,

    hunks: Vec<Hunk>,
}

/// One run of changed lines, with the `@@` line that introduces it.
#[derive(Debug, PartialEq, Eq)]
struct Hunk {
    header: String,
    lines: Vec<Line>,
}

#[derive(Debug, PartialEq, Eq)]
struct Line {
    kind: Kind,

    /// Where the line sits in the file the change leaves behind, or `None` for
    /// one that is not in it — a removed line, or something git said about the
    /// lines rather than one of them.
    ///
    /// Only the new side is numbered. A unified diff carries an old number too,
    /// and the two diverge the moment a file gains or loses a line, but a second
    /// column costs the width twice over on a phone and the number worth having
    /// is the one you would go to in the editor.
    number: Option<usize>,

    /// The line's content, with the diff's leading marker taken off.
    text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Added,
    Removed,
    Context,

    /// Something git says about the lines rather than one of them — the missing
    /// final newline.
    Aside,
}

impl Kind {
    /// How the line is styled, and the marker it keeps. The marker stays in the
    /// page so the lines are told apart by more than colour — but the stylesheet
    /// takes it out of the selection, along with the line numbers, so a hunk
    /// copied off the page comes out as code to paste into an editor rather than
    /// as a patch.
    fn marked(self) -> (&'static str, &'static str) {
        match self {
            Kind::Added => ("add", "+"),
            Kind::Removed => ("del", "-"),
            Kind::Context => ("ctx", " "),
            Kind::Aside => ("aside", ""),
        }
    }
}

/// Split a diff into its files, unified or combined.
///
/// Line counts from each `@@` header say how long the hunk is, so content that
/// looks like a diff header — a patch inside a patch, which this repository's
/// own tests are full of — is read as content and not as the start of another
/// file.
///
/// A merge is described by its combined diff, which says the same things in a
/// wider shape: the file opens `diff --cc <path>`, the hunk header fences its
/// ranges with one `@` per parent plus one, and every line carries one marker
/// column per parent rather than one. So the same book-keeping runs over a list
/// of parents rather than a single old side, and a unified diff is that list
/// with one entry in it.
fn files(diff: &str) -> Vec<FileDiff> {
    let mut files: Vec<FileDiff> = Vec::new();

    // What the open hunk still owes: one count per parent, and the result
    // side's. All of them spent means the hunk is finished and the next line is
    // a header again.
    let mut parents_left: Vec<usize> = Vec::new();
    let mut new_left = 0usize;

    // The number the next line of the new side will carry, counting on from
    // where the open hunk's header said the new side starts.
    let mut numbering = 0usize;

    // Carried from the `---` line to the `+++` one: for a deleted file the new
    // side is `/dev/null`, and the old path is the only name it has.
    let mut removed_path: Option<String> = None;

    for line in diff.lines() {
        // The missing-newline note trails the last line of its hunk, by which
        // point the counts have run out — so it is hunk content whether or not
        // anything is still owed.
        let in_hunk =
            new_left > 0 || parents_left.iter().any(|&left| left > 0) || line.starts_with('\\');

        if let Some(file) = files.last_mut()
            && in_hunk
            && !file.hunks.is_empty()
        {
            // Inside a hunk. A line that cannot be hunk content means the counts
            // were wrong, so the hunk ends here and the line is reconsidered as
            // a header below.
            // One marker column per parent: one for an ordinary diff, N for a
            // merge of N. Never none, because a hunk header always names at
            // least one side.
            if let Some((kind, width)) = content(line, parents_left.len()) {
                // The missing-newline note belongs to the line before it and
                // counts against nothing.
                if kind != Kind::Aside {
                    spend(line, &mut parents_left, &mut new_left);
                }

                // The markers, and the rest of the line is its text. The note
                // carries none of them and so is kept whole; so is an empty
                // line, which is a context line git wrote without them.
                let text = line.get(width..).unwrap_or_default().to_owned();

                // Only what survives into the new file takes a number, and takes
                // the next one.
                let number = match kind {
                    Kind::Added | Kind::Context => {
                        let number = numbering;
                        numbering += 1;
                        Some(number)
                    }
                    Kind::Removed | Kind::Aside => None,
                };

                if let Some(hunk) = file.hunks.last_mut() {
                    hunk.lines.push(Line { kind, number, text });
                }
                continue;
            }

            parents_left.clear();
            new_left = 0;
        }

        // `diff --git a/<path> b/<path>` opens an ordinary file and
        // `diff --cc <path>` a merge's. The combined one names the path once,
        // because however many parents it has it leaves one result behind.
        let opened = line
            .strip_prefix("diff --git ")
            .map(header_path)
            .or_else(|| line.strip_prefix("diff --cc ").map(str::to_owned));

        if let Some(path) = opened {
            files.push(FileDiff {
                path,
                status: None,
                note: None,
                hunks: Vec::new(),
            });
            parents_left.clear();
            new_left = 0;
            removed_path = None;
            continue;
        }

        // Anything before the first `diff --git` belongs to no file.
        let Some(file) = files.last_mut() else {
            continue;
        };

        if line.starts_with("@@") {
            let span = span(line);
            (parents_left, new_left) = (span.parents, span.new);
            numbering = span.start;
            file.hunks.push(Hunk {
                header: line.to_owned(),
                lines: Vec::new(),
            });
        } else if let Some(field) = line.strip_prefix("--- ") {
            removed_path = worktree_path(field);
        } else if let Some(field) = line.strip_prefix("+++ ") {
            if let Some(path) = worktree_path(field).or_else(|| removed_path.take()) {
                file.path = path;
            }
        } else if line.starts_with("new file mode") {
            file.status = Some("new file");
        } else if line.starts_with("deleted file mode") {
            file.status = Some("deleted");
        } else if line.starts_with("rename to") {
            file.status = Some("renamed");
        } else if line.starts_with("Binary files") {
            file.note = Some("Binary file — contents omitted.");
        }
    }

    files
}

/// What kind of hunk line this is and how many characters its markers take, or
/// `None` if it is not a hunk line at all.
///
/// `columns` is one marker per parent — one for an ordinary diff, N for a
/// combined diff of N parents. The columns collapse: any `+` in any of them is
/// an added line, any `-` a removed one, and all spaces context. Which parent a
/// line differed from is not what the pane is read for, and a line cannot be
/// both — it is either in the result, and so carries no `-` anywhere, or it is
/// not, and so carries no `+`.
fn content(line: &str, columns: usize) -> Option<(Kind, usize)> {
    // git's aside about the line above it, which carries no markers.
    if line.starts_with('\\') {
        return Some((Kind::Aside, 0));
    }

    // A line git wrote as empty rather than as bare markers.
    if line.is_empty() {
        return Some((Kind::Context, 0));
    }

    let (mut added, mut removed, mut width) = (false, false, 0);
    for marker in line.chars().take(columns) {
        match marker {
            '+' => added = true,
            '-' => removed = true,
            ' ' => {}
            _ => return None,
        }
        width += 1;
    }

    let kind = if added {
        Kind::Added
    } else if removed {
        Kind::Removed
    } else {
        Kind::Context
    };
    Some((kind, width))
}

/// Take off what one hunk line owes: from the result side, and from each parent
/// it is in.
///
/// A line with no `-` in any column is in the result and spends the result
/// side's count. A parent's count is spent by a line that is in *that parent*,
/// which its column says in two ways — `-` is in the parent and not in the
/// result, and a space on a line the result kept is in both.
///
/// A space against a line the result did *not* keep says the opposite: the line
/// is in neither, so that parent owes nothing for it. That is the case a merge
/// has and a unified diff does not, and reading it the other way would spend a
/// parent's count early and cut the hunk off before its end.
///
/// On one marker column this is the unified diff's rule unchanged: `+` spends
/// the new side, `-` the old, and a space spends both.
fn spend(line: &str, parents_left: &mut [usize], new_left: &mut usize) {
    // A missing column is a space — git writes an empty line rather than bare
    // markers — and a marker is ASCII wherever git does write one.
    let marker = |parent: usize| line.as_bytes().get(parent).copied().unwrap_or(b' ');

    let kept = !(0..parents_left.len()).any(|parent| marker(parent) == b'-');
    if kept {
        *new_left = new_left.saturating_sub(1);
    }

    for (parent, left) in parents_left.iter_mut().enumerate() {
        if marker(parent) == b'-' || (kept && marker(parent) == b' ') {
            *left = left.saturating_sub(1);
        }
    }
}

/// What a hunk header says about the lines beneath it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Span {
    /// How many lines each parent contributes — one entry for an ordinary
    /// diff's old side, and one per parent for a merge's. Never empty, so the
    /// count of them is also the count of marker columns each line carries.
    parents: Vec<usize>,

    /// How many the result side contributes.
    new: usize,

    /// The number of the result side's first line — where the gutter starts
    /// counting.
    start: usize,
}

/// What a hunk header promises: `@@ -1,3 +2,4 @@` is three lines from the old
/// side and four from the new, the new side's first being line 2. A count left
/// off means one.
///
/// A merge's header says the same in a wider shape — `@@@ -1,3 -1,3 +1,4 @@@`
/// is one range per parent and then the result's — and the fence is what says
/// how many parents there are: one `@` per parent, plus one for the result.
///
/// A hunk header can carry the enclosing function's name after the closing
/// fence, and a word of that could begin with `-` or `+`, so the ranges are
/// read from between the two fences rather than from the line.
fn span(header: &str) -> Span {
    let fence: String = header.chars().take_while(|&at| at == '@').collect();
    let parents = fence.len().saturating_sub(1).max(1);

    let rest = header.strip_prefix(&fence).unwrap_or(header);
    let ranges = match rest.find(&fence) {
        Some(close) => &rest[..close],
        None => rest,
    };

    let mut counts = Vec::with_capacity(parents);
    let (mut new, mut start) = (0, 0);

    let fields = ranges.split_whitespace().filter_map(|field| {
        let (result, range) = match field.strip_prefix('-') {
            Some(range) => (false, range),
            None => (true, field.strip_prefix('+')?),
        };
        Some((
            result,
            match range.split_once(',') {
                Some((start, count)) => (start.parse().unwrap_or(1), count.parse().unwrap_or(1)),
                None => (range.parse().unwrap_or(1), 1),
            },
        ))
    });

    for (result, (first, count)) in fields.take(parents + 1) {
        if result {
            (start, new) = (first, count);
        } else if counts.len() < parents {
            counts.push(count);
        }
    }

    // A header that named fewer sides than its fence promised still gets a
    // column per parent, so a line's markers are read as the width they are.
    counts.resize(parents, 0);

    Span {
        parents: counts,
        new,
        start,
    }
}

/// The path from a `diff --git a/<path> b/<path>` line.
///
/// Best-effort: the two halves are only separable by convention, and a path
/// with ` b/` in it would fool this. The `---` and `+++` lines that follow are
/// unambiguous and correct it, so this only stands for a file that has neither
/// — one whose mode changed and nothing else.
fn header_path(rest: &str) -> String {
    match rest.rsplit_once(" b/") {
        Some((_, path)) => path.to_owned(),
        None => rest.to_owned(),
    }
}

/// The path from a `---`/`+++` field, or `None` for the empty file that stands
/// in for one that does not exist on that side.
fn worktree_path(field: &str) -> Option<String> {
    // git terminates the path with a tab when it has to say more after it.
    let path = field.split('\t').next().unwrap_or(field).trim_end();
    if path == "/dev/null" {
        return None;
    }

    let path = path
        .strip_prefix("a/")
        .or_else(|| path.strip_prefix("b/"))
        .unwrap_or(path);
    Some(path.to_owned())
}

impl FileDiff {
    /// `position` is this file's place in the Diff, counting from one: the name
    /// the table of contents and a hash deep-link reach it by.
    fn render(&self, out: &mut String, position: usize) {
        // Open, because a Diff is there to be read — but foldable, so a long
        // file can be got out of the way on a phone.
        out.push_str(&format!(
            r#"<details class="diffFile" id="diff-{position}" open><summary><span class="diffPath">"#
        ));
        out.push_str(&escaped(&self.path));
        out.push_str("</span>");

        if let Some(status) = self.status {
            out.push_str(r#"<span class="diffStatus">"#);
            out.push_str(status);
            out.push_str("</span>");
        }

        let (added, removed) = self.counted();
        if added > 0 || removed > 0 {
            out.push_str(&format!(
                r#"<span class="diffStat"><span class="add">+{added}</span><span class="del">−{removed}</span></span>"#
            ));
        }
        out.push_str("</summary>");

        if let Some(note) = self.note {
            out.push_str(r#"<p class="diffNote">"#);
            out.push_str(note);
            out.push_str("</p>");
        }

        // Highlighting is keyed off the path, so it is settled once per file
        // rather than looked up per line.
        let syntax = for_path(&self.path);
        for hunk in &self.hunks {
            hunk.render(out, syntax);
        }

        out.push_str("</details>");
    }

    /// How many lines this file gains and loses.
    fn counted(&self) -> (usize, usize) {
        let lines = self.hunks.iter().flat_map(|hunk| &hunk.lines);
        let (mut added, mut removed) = (0, 0);
        for line in lines {
            match line.kind {
                Kind::Added => added += 1,
                Kind::Removed => removed += 1,
                _ => {}
            }
        }
        (added, removed)
    }
}

impl Hunk {
    fn render(&self, out: &mut String, syntax: Option<&SyntaxReference>) {
        out.push_str(r#"<div class="diffHunk"><p class="diffHunkHeader">"#);
        out.push_str(&escaped(&self.header));

        // No newlines between the lines: each is a block of its own, so one here
        // would show up as a blank line.
        out.push_str(r#"</p><pre class="diffLines"><code>"#);
        for line in &self.lines {
            line.render(out, syntax);
        }
        out.push_str("</code></pre></div>");
    }
}

impl Line {
    fn render(&self, out: &mut String, syntax: Option<&SyntaxReference>) {
        let (class, marker) = self.kind.marked();

        out.push_str(&format!(r#"<span class="diffLine {class}">"#));

        // Always the column, even where there is no number to put in it: an
        // empty cell is what keeps a removed line's code in line with the code
        // above and below it.
        out.push_str(r#"<span class="diffNumber">"#);
        if let Some(number) = self.number {
            out.push_str(&numbered(number));
        }
        out.push_str("</span>");

        // The marker's cell goes in on the same terms, so git's aside about the
        // lines starts where the lines themselves do.
        out.push_str(&format!(r#"<span class="marker">{marker}</span>"#));

        match syntax
            .filter(|_| self.kind != Kind::Aside)
            .and_then(|syntax| crate::highlight::line(&self.text, syntax))
        {
            Some(html) => out.push_str(&html),
            None => out.push_str(&escaped(&self.text)),
        }

        out.push_str("</span>");
    }
}

/// A line number as it goes in the column, which is four characters wide for
/// every file on the page.
///
/// The width is the page's business rather than each file's: a column cut to
/// its own file's highest line put the code of a hundred-line file and a
/// thousand-line one in different places, so a Diff of both did not line up
/// with itself. Four characters is what the reading column reserves, alongside
/// the marker and the code — see `--diff-number-width` in the stylesheet.
///
/// Past four digits the number is given as its leading two and the power of ten
/// they stand in, which is four characters again: line 11234 reads `11e3`.
/// Truncated rather than rounded, so it never names a line ahead of the one it
/// stands for. That holds up to eleven digits, which is a file no disk holds.
fn numbered(number: usize) -> String {
    if number < 10_000 {
        return number.to_string();
    }

    let exponent = number.to_string().len() - 2;
    let leading = number / 10_usize.pow(exponent as u32);
    format!("{leading}e{exponent}")
}

#[cfg(test)]
mod tests {
    use super::{AS_IT_ARRIVED, files, numbered, to_html};

    /// The markup for a Diff that has something in it — what most of these
    /// tests are looking at, the paths beside it being their own two tests.
    fn rendered(diff: &str) -> String {
        to_html(diff).unwrap().html
    }

    /// The path of each file the Diff renders, in the order it renders them.
    fn paths(diff: &str) -> Vec<String> {
        to_html(diff).unwrap().paths
    }

    /// A tracked file edited and an untracked one added — what `verkstead ask`
    /// captures from a working tree mid-change.
    const MODIFIED_AND_NEW: &str = concat!(
        "diff --git a/src/lib.rs b/src/lib.rs\n",
        "index 4cb29ea..ddc897f 100644\n",
        "--- a/src/lib.rs\n",
        "+++ b/src/lib.rs\n",
        "@@ -1,4 +1,4 @@\n",
        " fn main() {\n",
        "-    let old = 1;\n",
        "+    let new = 2;\n",
        " }\n",
        "diff --git a/notes.txt b/notes.txt\n",
        "new file mode 100644\n",
        "index 0000000..cdd6835\n",
        "--- /dev/null\n",
        "+++ b/notes.txt\n",
        "@@ -0,0 +1,2 @@\n",
        "+first thought\n",
        "+second thought\n",
    );

    #[test]
    fn every_file_in_the_diff_gets_its_own_section() {
        let html = rendered(MODIFIED_AND_NEW);

        assert_eq!(
            html.matches(r#"class="diffFile""#).count(),
            2,
            "expected one section per file:\n{html}"
        );
        assert!(html.contains(">src/lib.rs<"), "{html}");
        assert!(html.contains(">notes.txt<"), "{html}");
        assert!(
            html.contains("new file"),
            "expected the untracked file marked as new:\n{html}"
        );
        assert!(
            html.contains("@@ -1,4 +1,4 @@"),
            "expected the hunk header:\n{html}"
        );
    }

    #[test]
    fn added_removed_and_context_lines_are_told_apart() {
        let html = rendered(MODIFIED_AND_NEW);

        assert_eq!(
            html.matches(r#"diffLine add"#).count(),
            3,
            "expected the three added lines marked:\n{html}"
        );
        assert_eq!(
            html.matches(r#"diffLine del"#).count(),
            1,
            "expected the one removed line marked:\n{html}"
        );
        assert_eq!(
            html.matches(r#"diffLine ctx"#).count(),
            2,
            "expected the two context lines marked:\n{html}"
        );

        // The markers stay in the page: colour is not the only thing telling an
        // addition from a removal.
        assert!(html.contains(r#"<span class="marker">+</span>"#), "{html}");
        assert!(html.contains(r#"<span class="marker">-</span>"#), "{html}");
    }

    #[test]
    fn the_lines_of_a_file_add_up_to_its_tally() {
        let html = rendered(MODIFIED_AND_NEW);

        assert!(
            html.contains(">+1<"),
            "expected src/lib.rs's tally:\n{html}"
        );
        assert!(html.contains(">−1<"), "{html}");
        assert!(html.contains(">+2<"), "expected notes.txt's tally:\n{html}");
    }

    #[test]
    fn a_recognised_file_type_is_highlighted_token_by_token() {
        let html = rendered(MODIFIED_AND_NEW);

        assert!(
            html.contains(r#"<span class="tok-"#),
            "expected the Rust file's tokens highlighted:\n{html}"
        );
    }

    /// One file, two hunks, the second far enough down the file to need two
    /// digits — which is what the number column is sized by.
    const TWO_HUNKS: &str = concat!(
        "diff --git a/src/lib.rs b/src/lib.rs\n",
        "--- a/src/lib.rs\n",
        "+++ b/src/lib.rs\n",
        "@@ -1,2 +1,2 @@\n",
        "-first\n",
        "+FIRST\n",
        " second\n",
        "@@ -40,2 +40,3 @@\n",
        " fortieth\n",
        "+added\n",
        " forty-first\n",
    );

    /// The numbers each line of a hunk carries, in order.
    fn numbers(diff: &str, file: usize, hunk: usize) -> Vec<Option<usize>> {
        files(diff)[file].hunks[hunk]
            .lines
            .iter()
            .map(|line| line.number)
            .collect()
    }

    #[test]
    fn every_line_is_numbered_from_where_its_hunk_header_says_the_file_starts() {
        assert_eq!(
            numbers(MODIFIED_AND_NEW, 0, 0),
            [Some(1), None, Some(2), Some(3)],
            "`@@ -1,4 +1,4 @@` starts the new side at line 1, and only the lines \
             that are in the new side count on from it",
        );
        assert_eq!(
            numbers(MODIFIED_AND_NEW, 1, 0),
            [Some(1), Some(2)],
            "a new file's lines are numbered from one like any others",
        );
    }

    #[test]
    fn a_removed_line_is_left_unnumbered() {
        assert_eq!(
            numbers(TWO_HUNKS, 0, 0),
            [None, Some(1), Some(2)],
            "a removed line is not in the file the change leaves behind, so it \
             has no number there to show",
        );

        let html = rendered(TWO_HUNKS);

        assert!(
            html.contains(r#"<span class="diffLine del"><span class="diffNumber"></span>"#),
            "the column is still there and simply empty, so the code of a \
             removed line stays in line with the code around it:\n{html}",
        );
    }

    #[test]
    fn a_later_hunk_numbers_from_its_own_start_and_not_from_the_last_one() {
        assert_eq!(
            numbers(TWO_HUNKS, 0, 1),
            [Some(40), Some(41), Some(42)],
            "the lines between two hunks are not in the Diff, so the second \
             hunk picks up the numbering from its own header",
        );
    }

    #[test]
    fn the_number_column_is_the_same_width_for_every_file() {
        let html = rendered(TWO_HUNKS);

        assert!(
            !html.contains("--diff-digits"),
            "the column is cut once in the stylesheet, not per file: a file \
             that sized its own put its code in a different place from the \
             file above it, and the two did not line up:\n{html}",
        );
    }

    #[test]
    fn a_line_number_past_four_digits_is_given_in_four_characters() {
        assert_eq!(
            numbered(1),
            "1",
            "a short file is numbered as it always was"
        );
        assert_eq!(numbered(9999), "9999", "four digits still fit as they are");
        assert_eq!(
            numbered(11234),
            "11e3",
            "past four digits the leading two and their power of ten say where \
             the line is, in the four characters the column holds",
        );
        assert_eq!(
            numbered(19999),
            "19e3",
            "truncated rather than rounded, so the column never names a line \
             ahead of the one it stands for",
        );
        assert_eq!(
            numbered(123_456),
            "12e4",
            "a longer file only moves the exponent, and the width is unchanged",
        );
    }

    #[test]
    fn typescript_is_highlighted_like_any_other_language() {
        // The syntaxes syntect bundles are Sublime Text's defaults, which have no
        // TypeScript in them — so this went unhighlighted while Rust beside it
        // did not, which is what `two-face` is here to fix.
        let diff = concat!(
            "diff --git a/src/api.ts b/src/api.ts\n",
            "--- a/src/api.ts\n",
            "+++ b/src/api.ts\n",
            "@@ -1,2 +1,2 @@\n",
            " interface Reply { ok: boolean }\n",
            "-export const send = (url: string) => fetch(url);\n",
            "+export const send = async (url: string) => fetch(url);\n",
            "diff --git a/src/App.tsx b/src/App.tsx\n",
            "--- a/src/App.tsx\n",
            "+++ b/src/App.tsx\n",
            "@@ -1 +1 @@\n",
            "-export const App = () => <h1>hi</h1>;\n",
            "+export const App = () => <h2>hi</h2>;\n",
        );

        let html = rendered(diff);

        let first = html.find(r#"id="diff-1""#).unwrap();
        let second = html.find(r#"id="diff-2""#).unwrap();

        assert!(
            html[first..second].contains(r#"<span class="tok-"#),
            "expected the .ts file's tokens highlighted:\n{html}",
        );
        assert!(
            html[second..].contains(r#"<span class="tok-"#),
            "expected the .tsx file's tokens highlighted:\n{html}",
        );
    }

    #[test]
    fn a_file_type_nothing_recognises_keeps_its_plain_colouring() {
        let diff = concat!(
            "diff --git a/config.zzz b/config.zzz\n",
            "--- a/config.zzz\n",
            "+++ b/config.zzz\n",
            "@@ -1 +1 @@\n",
            "-retries = 1\n",
            "+retries = 5\n",
        );

        let html = rendered(diff);

        assert!(
            !html.contains("tok-"),
            "nothing highlights .zzz, so no tokens should be marked:\n{html}"
        );
        assert!(
            html.contains("diffLine add") && html.contains("diffLine del"),
            "the +/- colouring stands on its own:\n{html}"
        );
        assert!(html.contains("retries = 5"), "{html}");
    }

    #[test]
    fn a_binary_file_says_its_contents_are_left_out() {
        let diff = concat!(
            "diff --git a/logo.png b/logo.png\n",
            "new file mode 100644\n",
            "index 0000000..0f49c4a\n",
            "Binary files /dev/null and b/logo.png differ\n",
        );

        let html = rendered(diff);

        assert!(html.contains(">logo.png<"), "{html}");
        assert!(
            html.contains("contents omitted"),
            "expected the binary file accounted for:\n{html}"
        );
    }

    #[test]
    fn a_deleted_file_is_named_by_the_path_it_had() {
        let diff = concat!(
            "diff --git a/src/old.rs b/src/old.rs\n",
            "deleted file mode 100644\n",
            "index 4cb29ea..0000000\n",
            "--- a/src/old.rs\n",
            "+++ /dev/null\n",
            "@@ -1,2 +0,0 @@\n",
            "-fn gone() {}\n",
            "-\n",
        );

        let html = rendered(diff);

        assert!(html.contains(">src/old.rs<"), "{html}");
        assert!(html.contains("deleted"), "{html}");
    }

    #[test]
    fn diff_text_inside_a_hunk_is_content_and_not_another_file() {
        // A patch that adds a test fixture which is itself a patch. The hunk's
        // line counts are what keep the inner header from starting a file.
        let diff = concat!(
            "diff --git a/tests/fixture.txt b/tests/fixture.txt\n",
            "new file mode 100644\n",
            "--- /dev/null\n",
            "+++ b/tests/fixture.txt\n",
            "@@ -0,0 +1,3 @@\n",
            "+diff --git a/not-a-file b/not-a-file\n",
            "+@@ -1 +1 @@\n",
            "+-gone\n",
        );

        let files = files(diff);

        assert_eq!(files.len(), 1, "{files:#?}");
        assert_eq!(files[0].path, "tests/fixture.txt");
        assert_eq!(
            files[0].hunks[0].lines.len(),
            3,
            "all three lines belong to the fixture:\n{files:#?}"
        );
    }

    #[test]
    fn a_line_that_looks_like_markup_reaches_the_page_as_text() {
        let diff = concat!(
            "diff --git a/page.zzz b/page.zzz\n",
            "--- a/page.zzz\n",
            "+++ b/page.zzz\n",
            "@@ -1 +1 @@\n",
            "+<script>alert('pwned') & co</script>\n",
        );

        let html = rendered(diff);

        assert!(
            !html.contains("<script>"),
            "a Diff is text, and script in it must stay text:\n{html}"
        );
        assert!(html.contains("&lt;script&gt;"), "{html}");
        assert!(html.contains("&amp; co"), "{html}");
    }

    #[test]
    fn a_missing_final_newline_is_carried_through() {
        let diff = concat!(
            "diff --git a/notes.txt b/notes.txt\n",
            "--- a/notes.txt\n",
            "+++ b/notes.txt\n",
            "@@ -1 +1 @@\n",
            "-before\n",
            "+after\n",
            "\\ No newline at end of file\n",
        );

        let html = rendered(diff);

        assert!(html.contains("No newline at end of file"), "{html}");
    }

    #[test]
    fn each_file_is_anchored_by_its_position_in_the_diff() {
        let html = rendered(MODIFIED_AND_NEW);

        let first = html
            .find(r#"id="diff-1""#)
            .unwrap_or_else(|| panic!("expected the first file anchored:\n{html}"));
        let second = html
            .find(r#"id="diff-2""#)
            .unwrap_or_else(|| panic!("expected the second file anchored:\n{html}"));

        assert!(first < second, "the ids follow Diff order:\n{html}");
        assert!(
            html[first..second].contains(">src/lib.rs<"),
            "expected diff-1 to be the first file the Diff names:\n{html}"
        );
        assert!(
            html[second..].contains(">notes.txt<"),
            "expected diff-2 to be the second:\n{html}"
        );
    }

    #[test]
    fn the_paths_come_back_in_the_order_the_files_render() {
        assert_eq!(
            paths(MODIFIED_AND_NEW),
            ["src/lib.rs", "notes.txt"],
            "the table of contents names the folds by these, so the nth path \
             has to be what `diff-n` shows",
        );
    }

    /// What `git diff-tree -p --cc` says about a resolution session's merge: a
    /// conflicted file settled one way, and a line the resolution added on top.
    /// Two marker columns, one per parent, and a `@@@` fence over them.
    ///
    /// The ordinary file after it is there to be reached: the hunk's counts are
    /// the only thing that says where a combined hunk ends, and a wrong reading
    /// of them swallows the file below.
    const MERGE: &str = concat!(
        "diff --cc f.txt\n",
        "index 6e8d6bb,6addb9b..20b5b51\n",
        "--- a/f.txt\n",
        "+++ b/f.txt\n",
        "@@@ -1,4 -1,4 +1,5 @@@\n",
        "  one\n",
        "- twoo\n",
        " -TWO\n",
        "++MERGED\n",
        "  three\n",
        "  four\n",
        "++extra\n",
        "diff --git a/after.txt b/after.txt\n",
        "--- a/after.txt\n",
        "+++ b/after.txt\n",
        "@@ -1 +1 @@\n",
        "-before\n",
        "+after\n",
    );

    #[test]
    fn a_merge_draws_as_files_and_hunks_like_any_other_diff() {
        let html = rendered(MERGE);

        assert!(
            !html.contains(AS_IT_ARRIVED),
            "a combined diff is git's own, so it is parsed rather than dropped \
             into the section for what git did not write:\n{html}"
        );
        assert_eq!(
            paths(MERGE),
            ["f.txt", "after.txt"],
            "`diff --cc` opens a file the way `diff --git` does, and the hunk's \
             counts are what let the file after it be reached",
        );
        assert!(
            html.contains("@@@ -1,4 -1,4 +1,5 @@@"),
            "expected the combined hunk header:\n{html}"
        );
    }

    #[test]
    fn a_merges_columns_collapse_into_one_kind_a_line() {
        let html = rendered(MERGE);

        assert_eq!(
            html.matches(r#"diffLine add"#).count(),
            3,
            "`++MERGED` and `++extra` are in neither parent, and `+after` is \
             the ordinary file's:\n{html}"
        );
        assert_eq!(
            html.matches(r#"diffLine del"#).count(),
            3,
            "`- twoo` is gone from one parent and ` -TWO` from the other, and \
             either way the merge does not keep it:\n{html}"
        );
        assert_eq!(
            html.matches(r#"diffLine ctx"#).count(),
            3,
            "a line both parents and the merge agree on is context:\n{html}"
        );

        assert!(
            html.contains(">MERGED<") && html.contains(">twoo<"),
            "the text is the line with all of its markers taken off:\n{html}"
        );
    }

    #[test]
    fn a_merges_lines_are_numbered_by_what_the_merge_kept() {
        assert_eq!(
            numbers(MERGE, 0, 0),
            [Some(1), None, None, Some(2), Some(3), Some(4), Some(5)],
            "only what is in the result takes a number, and a line removed \
             relative to either parent is not in it",
        );
    }

    #[test]
    fn a_merge_of_three_parents_is_read_by_its_fence_and_not_mistaken_for_content() {
        // Three ranges and then the result's, under a four-`@` fence — so every
        // line below carries three marker columns.
        let diff = concat!(
            "diff --cc f.txt\n",
            "index e6d2236,9615e15,20965f8..31a524f\n",
            "--- a/f.txt\n",
            "+++ b/f.txt\n",
            "@@@@ -1,3 -1,3 -1,3 +1,4 @@@@\n",
            "   one\n",
            "-  m\n",
            " - a\n",
            "  -b\n",
            "+++ALL\n",
            "   three\n",
            "+++new\n",
        );

        let files = files(diff);

        assert_eq!(files.len(), 1, "{files:#?}");
        assert_eq!(files[0].hunks.len(), 1, "{files:#?}");
        assert_eq!(
            files[0].hunks[0].lines.len(),
            7,
            "every line of the hunk belongs to it: `+++ALL` and `   one` carry \
             three marker columns rather than being a `+++` file header and a \
             line of its own:\n{files:#?}"
        );
        assert_eq!(
            files[0].hunks[0]
                .lines
                .iter()
                .map(|line| line.text.as_str())
                .collect::<Vec<_>>(),
            ["one", "m", "a", "b", "ALL", "three", "new"],
            "three columns come off each line, not one:\n{files:#?}"
        );
    }

    #[test]
    fn a_clean_tree_has_nothing_to_show() {
        assert_eq!(to_html("   \n\n"), None);
    }

    #[test]
    fn something_git_did_not_write_is_shown_as_it_arrived() {
        let html = rendered("who knows what this is\n");

        assert!(
            html.contains("who knows what this is"),
            "the Diff is evidence, so an unreadable one is shown rather than \
             dropped:\n{html}"
        );
        assert!(
            html.contains(r#"id="diff-1""#),
            "it is still the Diff's one foldable section, so it is still \
             addressable:\n{html}"
        );
        assert!(
            html.contains("diffVerbatim"),
            "having no line cells to hold its text off the edge, it has to say \
             so and be inset by the stylesheet instead:\n{html}"
        );
        assert_eq!(
            paths("who knows what this is\n"),
            ["The Diff, as it arrived"],
            "the one fold has no path to go by, so the nav calls it what the \
             fold calls itself",
        );
    }
}
