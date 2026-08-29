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

- [ ] One press yields a secret gist holding the byte-identical share file,
      created with the settings token, never the host's own login.
- [ ] Multi-MB creation is verified and the finding recorded — or the share
      branch fallback is built and linked instead.
- [ ] A token without the needed scope produces a named refusal pointing at
      the settings page.
