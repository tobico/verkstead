# 07. The repo add pane

## What to build

The same conversion for the Repos section's add flow: the "Add a repo" quiet
text button on the section heading becomes a plus IconButton, and the modal
it opened becomes a details pane at `/settings/repos/new`.

The form itself is unchanged — one absolute path, typed rather than picked,
because the Watched Paths are a security boundary and nothing scans the
filesystem to offer choices. Every named refusal (not absolute, missing,
outside the watched paths, not a repository, no default branch, already
registered) is still said beside the field, and a successful registration
closes the pane and shows the repo on the list.

The IconButton reads as open while the pane is, and the modal is deleted.
The existing repo rows become `CardButton` cards in this task — same face as
today (name, path, default branch) — but stay unpressable until task 08
gives them a pane to open.

## Acceptance criteria

- [ ] The plus IconButton opens the registration form at
      `/settings/repos/new` in the details pane and reads as open while it is
- [ ] Every registration refusal is still said beside the field, and a
      success lands the repo on the list; the modal is gone
- [ ] Repo rows are drawn as cards (not yet pressable)
