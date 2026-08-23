# 06. The /settings page

## What to build

A `/settings` page in the workbench, following the existing list-page pattern,
carrying:

- **The GitHub token field, write-only.** When a token is set, the page shows
  its last four characters, when it was saved, and — after a save — the GitHub
  account the server verified it as, with replace and clear actions. The stored
  token is never shown or prefilled.
- **The git author fields**, name and email, read and saved through the
  settings API.
- **Warnings when unset.** With no token or no author saved, the page says so
  plainly and what the consequence is (sessions cannot reach GitHub; commits
  fail asking who the author is). The warning clears when the setting lands.

## Acceptance criteria

- [ ] Save, replace and clear of the token all work from the page, and the
      verified account or the verification failure shows after a save
- [ ] The author fields round-trip through the API
- [ ] The unset-state warnings show when either setting is missing and clear
      when it is saved
