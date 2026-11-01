# 04. The Transcript and the resume

## What to build

agy's own record is protobuf under `antigravity-cli/conversations/`, with no
public schema, so the Transcript reads what agy also writes as text: the
`transcript.jsonl` every hook payload names, at
`<worktree>/.gemini/antigravity-cli/transcript.jsonl` — a file of lines in the
Worktree, the shape the Claude and Codex followers already read. Measure
whether agy writes it with no hook configured and what each line holds; follow
it under ADR-0006 unchanged, every line verbatim and parsed at render time, as
a fourth kind of file beside the three. Where the file is not written the
session is Capture-only, said in the log rather than silently.

The session is found rather than named: agy takes no conversation id at
launch. The `conversation_summaries.db` beside the conversations has a row per
conversation with its `workspace_uris` and its times; the session's is the row
naming this Worktree created at or after launch, newest first — Codex's rule
against a third store, opened read-only while agy writes it, a poll that cannot
read looking again. Put the id it answers on the record the way opencode's is,
so a machine that never ran the session can resume it.

The resume: `--conversation <id>` in place of the Brief on a carried
Conversation (ADR-0011's harness resume), the conversation record having
crossed with the memory sync as one of the joined entries. Where the record is
not where agy will look for it, Verkstead's own Resume stands, said on the
Notice as it is for the others. agy's `/export` and the artifacts under
`brain/` are not read here.

## Acceptance criteria

- [ ] A real session's turns draw on the Timeline as they happen, read from the Worktree's `transcript.jsonl`, and a session whose file is never written is Capture-only with a line in the log saying so.
- [ ] The session's conversation id is found through the summaries database and recorded beside the session.
- [ ] A Conversation moved to another device resumes in the same agy conversation, or stops with the Notice naming why Verkstead's own Resume ran instead.
