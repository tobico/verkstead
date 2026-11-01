# 02. Verkstead runs the agy login

## What to build

The login runner ADR-0022 built for Claude alone runs the Antigravity CLI's
login too, so a press of **Log in** on an agy Profile's card signs the Profile
in from any device the workbench is open on.

agy signed out and headless prints `Authentication required. Please visit the
URL to log in:`, the address on the next line, then `Waiting for authentication
(timeout 60s)...` and `Or, paste the authorization code here and press Enter:`,
and reads the code from standard input — the same conversation Claude's login
has, over pipes. Find the agy invocation that signs in and does no paid work
(a `-p` run with a trivial prompt and a short `--print-timeout` is the fallback
if nothing cleaner exists), run it in the Profile's Built Root in the sandbox
with `BROWSER` a program that does nothing, and drive it as Claude's is driven:
address out to the modal, code in from it, a wrong or cut code handled by what
agy prints for each.

The point of running it in the sandbox is the keyring: inside, agy cannot reach
the Secret Service and falls to its file store. **Measure where that file
lands** under the root (`~/.gemini/antigravity-cli/` is the likely place), make
it the login the root links and the write-back hands to the account, and record
the path as the one constant it is. If agy does not fall back on its own, find
what makes it (an environment the WSL detection reads, a setting), and failing
that say so in a memo — the design leans on this fallback.

Then what agy offers in place of `claude auth status`: `agy models` exits 0
signed out, so read what it prints or find a command that fails, and use it for
the pre-launch check; and the line agy writes when a session's account has
signed out, for the signed-out stop, anchored at its end as Claude's is. Both
constants are read off agy 1.4.12 and named as such.

ADR-0022's *Claude only* bullet is already amended; keep its *one login per
Profile*, *killed when nobody watches* and *resumes what it unblocks* exactly.

## Acceptance criteria

- [ ] Log in on an agy Profile's card opens the modal with agy's address, the code pasted there signs the Profile in, and the next session under it starts signed in.
- [ ] The token is a file in the account after the login, linked into every root built after it, and written back at a session's end.
- [ ] A session whose account has signed out stops as Signed out with the Log in press, and a press on a signed-out Profile is refused before a session starts.
- [ ] A wrong code and a cut-short code each land as the modal says, and only one login per Profile ever runs.
