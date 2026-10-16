# 07. The Claude account probe

## What to build

The Claude root a session gets copies `.claude.json` with `mcpServers` and
`projects` taken out and `oauthAccount` kept, and a root test asserts
`oauthAccount.emailAddress` comes over as what says the account is signed in.
`oauthAccount` also carries `displayName`, `fullName` and `organizationName`
(and ids), each of which names the human as well as the email does.

Probe it: with a copied root missing **each identifying field** (`emailAddress`,
`displayName`, `fullName`, `organizationName`), does Claude launch, consider
itself signed in, and run a turn? Probe against the Claude on this machine
(2.1.283 when planned; record the version actually used). See the memory on
probing a harness resume with a throwaway HOME for the shape; a real turn needs
the real credentials, so keep it to one cheap prompt.

Then, per field: **strip** every field Claude runs a turn without, in the root
builder, with the root test changed to assert the strip; and for any field it
cannot run without, **amend ADR-0025's *What was reviewed and left*** to record
what the probe found and the Claude version it was taken against. Update the
**Author** term in CONTEXT.md if what it says about the Profile channel changes.

## Acceptance criteria

- [ ] The probe has been run per field, and the Claude version is recorded in the commit and wherever the finding lands.
- [ ] Every field Claude runs without is stripped from the copy, and the root test asserts it.
- [ ] Any field that has to stay is documented in ADR-0025 with the version.
