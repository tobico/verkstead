# 05. The settings API

## What to build

The viewer API surface for the two settings files, under the existing
`/api/ui/` namespace:

- **Read** returns the git author, and the token as presence only: whether one
  is set, its last four characters, and when it was saved — the secrets file's
  own modification time, not stored metadata. The full token never appears in
  any response.
- **Write** takes the git author fields and, separately or together, a token
  action: set a new token, or clear it. `secrets.yaml` is written with mode
  0600, atomically, and hand-edits to either file are honored — the files are
  the source of truth and the API reads them fresh.

Saving a token verifies it: the server asks GitHub who the token authenticates
as (through the same host `gh`, with the candidate token as `GH_TOKEN`) and
returns the account name or the failure alongside the save result. The save
goes through regardless of the verification outcome — a network failure must
not lose a pasted token.

## Acceptance criteria

- [ ] The full token appears in no GET or POST response body
- [ ] `secrets.yaml` lands 0600 and a hand-edited value shows up on the next
      read
- [ ] A token save returns the GitHub account name when the token is good, and
      the failure in words when it is not — saved either way
- [ ] Clearing removes the token, and the read reflects it
