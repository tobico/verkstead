# 07. Template repositories for the suites that make one

## What to build

A `conversations` test gets a git repository with one commit on `main`, made
by five `git` processes — init, two configs, add, commit — and a Windows
process start is slow; 130 of those tests a leg are 16% of the test time. The
repository becomes a copy of a template made once per leg: the first test to
want one makes it, under a file lock in the cargo target directory so that
tests in parallel processes wait for one making rather than each making their
own, and every test after copies the directory tree. The copy is a recursive
file copy and no `git` process at all; git reads a copied repository as the
repository it is, there being nothing in `.git` that names its path.

The helper lives in the merged server test binary, where every server suite
can reach it as a module. `conversations` moves onto it first. The `joining`,
`repos` and `worktrees` suites make repositories much the same way and are
about 500 test-seconds between them: each moves onto the helper in this task
where the move is a swap of the fixture call for the helper, and stays as it
is where its repository differs from the template in a way the test is about
— a second branch, a remote, a particular history. Say in the helper's docs
which suites use it and why the others do not.

What a test does to its copy afterwards is its own: the copy is under the
test's temp directory and goes with it. A template that is found half-made
— the lock held by a process that died — is remade rather than trusted: the
lock protects a marker written last, and the marker is what a copier checks.

Record both legs' test times in the ledger from the run that carried this
commit, beside the row before it.

## Acceptance criteria

- [ ] No `git` process runs on the `conversations` fixture path except the ones that make the template once per leg
- [ ] The `joining`, `repos` and `worktrees` suites use the helper wherever the move was a swap of the fixture call, and the helper's docs say which suites stayed off it and why
- [ ] Both Windows legs and the Linux job are green, and the ledger carries both legs' test times from the run that measured this commit
