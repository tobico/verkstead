# 06. HOME, the account and the skills

## What to build

The three things that exist only inside a mount on Linux, made real on a Mac.

**HOME is the session's own, and the account arrives by symlink.** On Linux a
session's HOME is an empty directory with the Profile's account mounted into it
at the path its backend looks in, and that mounting is also what makes one
account's sessions invisible to another's. Apple's sandbox has no mounts, so the
directory is really made and the account is really linked into it — one link per
path the Linux bind covers, at the same relative paths, for each of the four
account shapes. What keeps one account out of another's is then the policy
rather than the absence: everything outside this session's own links is refused.

**The skills and the `verkstead` binary are real paths too.** On Linux both live
under a directory that the bind itself makes and that exists nowhere on the
host. On a Mac they need somewhere to actually be, readable by the session and
writable by nobody inside it: the skills are the product's and not a file a
session may rewrite mid-run, and the binary a session asks with is the server's
own image rather than whatever the machine has installed.

**`PATH` and `SHELL` become the Mac's own.** Today they are NixOS's, and a Mac
has none of it. The binary a session asks with goes first, as it does on Linux;
then Homebrew's and the system's; and then nix's own profile paths where
nix-darwin is installed, so a Mac that has it is not made to do without.

## Acceptance criteria

- [x] A probe shows the Profile's account where its backend looks for it, for
      each account shape, and shows one account's sessions cannot reach another's
- [x] A probe shows the skills inside are the bundled ones and only those, and
      that `verkstead` on the session's `PATH` is the server's own image
- [x] A session on a Mac with nix-darwin reaches nix's tools; one without reaches
      Homebrew's and the system's, and neither refuses to start for want of the
      other
