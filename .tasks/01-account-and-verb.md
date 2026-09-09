# 01. The session account and the verb that makes it

## What to build

The identity a Windows session will run as, and the one elevated step that puts
it on the machine. Nothing starts a process as it yet and no session changes —
this is the account existing, being findable, and being removable.

A module of the sandbox's own holds what an account *is*: the name it goes by on
this machine, the password Verkstead keeps for it, and the SID entries are
written for. The name is fingerprinted off the Data Directory the way the named
pipe's is, so two Verksteads on one machine keep their accounts apart — but a
local account name is capped at **20 characters** and the pipe's
`verkstead-{:016x}` is 26, so the prefix and the number of hex digits both come
down. The name is what the record beside the entries will later carry, so a
server that did not make the account can still say which one it is looking at.

The password is generated with the operating system's own generator and kept in
the Data Directory's secrets file beside the GitHub token. **That file currently
truncates itself to nothing whenever there is no GitHub token** — the rule has
to become *nothing set at all* rather than *no token*, or the first person to
clear their token loses the account's password.

The elevated verb creates the account and writes the password in one step: an
ordinary user account, in no group but `Users`, with a password that neither
expires nor can be changed by the account itself, and denied interactive logon —
it is a name to run as rather than one anybody signs in with. A second verb
takes it away, **and takes the profile directory with it**: starting a process
as the account loads its profile, which makes `C:\Users\<account>` and leaves it
there after the account is deleted.

Run without elevation, either verb refuses with a line naming what it needs
rather than a Win32 error code. The server, which is never elevated, only ever
*reads* the account: it resolves the name to a SID and says plainly when there
is no account or no password, which is what will later refuse a session.

The account's name is portable arithmetic and belongs in tests that run on every
platform; everything that calls Win32 is the Windows arm's.

## Acceptance criteria

- [ ] The account's name is derived from the Data Directory, is at most 20
  characters, and two different Data Directories give two different names
- [ ] Run elevated, the verb creates the account with the flags above and writes
  a generated password to the secrets file; run again it says the account was
  already there rather than failing
- [ ] Run unelevated, both verbs refuse with a line that names the elevation
  they need
- [ ] The removal verb deletes the account **and** its profile directory, and
  says which of the two it removed
- [ ] Saving secrets no longer empties the file when there is no GitHub token,
  and a stored password survives a token being cleared
- [ ] The server resolves the account to a SID, and hands back a refusal naming
  what is missing when there is no account or no password
