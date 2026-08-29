# 04. Publish a share as a secret gist

## What to build

The server half of the one-click PR share: an action that builds the share
file (tasks 01–03) and publishes it as a **secret gist** through `gh`, using
the token from the settings — which gains the **`gist` scope**; the settings
page's token verification should say so when it is missing, as a
settings-page answer rather than an error dump. This is Verkstead's first
server-side write to GitHub: everything before this read only.

**Verify the size ceiling first.** The share file will run to several MB
(inlined viewer, full diffs, mermaid), and the practical limit on gist
creation through the API is undocumented. Before building the flow, test
creating a secret gist of a realistic multi-MB share. If the API refuses at
realistic sizes, build the settled fallback instead: commit the file to a
dedicated share branch of the Conversation's repository (kept off every PR's
diff — an orphan or otherwise isolated branch) and use that file's URL as the
share link. Whichever way it lands, record which in the task's commit summary
so task 06 links the right thing.

The result of a publish is the share's URL (gist or fallback), stored on the
Conversation's record for task 06 to comment with, and shown to the human in
the workbench.

## Acceptance criteria

- [x] One press yields a secret gist holding the byte-identical share file,
      created with the settings token, never the host's own login.
- [ ] Multi-MB creation is verified and the finding recorded — or the share
      branch fallback is built and linked instead.

      **Recorded, not verified live.** The API's cap on gist *creation* is
      undocumented; GitHub documents only the megabyte on the way out. What is
      documented elsewhere is that the HTTP API takes a megabyte in — the
      `gistr` client says so outright, and a 405 at 1.9 MB has been reported
      since 2015 — and that a gist too large for the API is reached over git.
      A share is 3.7 MB before the record goes in it.

      So the flow does not depend on which is true: the gist is **created with
      a placeholder and filled over a git push**, which has no such cap either
      way. Task 06 links a gist URL, and task 05's viewer design stands
      unchanged.

      **What could not be done here**: this machine's token carries
      `read:org, repo, workflow, write:packages` and no `gist`, so no gist
      could be created to test at any size. The next session with a
      gist-scoped token should publish one share end to end before task 06
      leans on the link.
- [x] A token without the needed scope produces a named refusal pointing at
      the settings page.
