# 02. The open rendering

## What to build

A third rendering of the Sandbox's `Surface`, beside bubblewrap's flags and the
seatbelt policy: the **open** one, which sets the environment and the working
directory and runs `argv` directly, with no wrapper in front of it. There is no
boundary in it — that is stage 03's — and its whole job is that a Windows
session runs at all, in an environment Verkstead said rather than one it
inherited.

**Chosen by Platform as a value.** The two-way `cfg` at the top of `sandbox`
that picks bubblewrap or seatbelt becomes a match on `Platform`, for the reason
`Platform` is a value everywhere else: the arm this machine will never run is
still an arm its tests call. A Windows build stops compiling the bubblewrap
renderer as its own.

**Environment cleared, then set explicitly**, as every rendering does, plus the
Windows names nothing runs without: `SystemRoot`, `SystemDrive`, `ComSpec`,
`PATHEXT`, `TEMP` and `TMP`, `USERPROFILE`, `APPDATA`, `LOCALAPPDATA`. The
profile roots get their real answer in task 03; here they may point at whatever
`Homes` already gives a Windows Conversation. `PATH` inside is Verkstead's own
bin directory followed by **the server's own `PATH`** — there is no
`WINDOWS_PATH` constant beside `LINUX_PATH` and `APPLE_PATH`, because where
tools live on a Windows machine is where the human put them. Note that the
existing `path` helper joins with a literal `:`; the separator is the
platform's, and the `PATH_LIST_SEPARATOR` in the crate root is the CLI's own
flag parser rather than this.

**Finding the agent and quoting the line.** The program is found on `PATH` by
`PATHEXT`'s rules, so an npm-installed `claude.cmd` starts as well as the native
installer's `claude.exe`. The command line is built by the rules
`CommandLineToArgvW` reads one back with, so an argument holding a space or a
quote arrives as the one argument it was.

**`nix develop` is skipped by Platform.** `under_dev_shell` shells out to `nix`
to ask whether a worktree has a dev shell; on Windows there is no nix and a
session should not pay for finding out. It takes the `Platform` and answers
`argv` unchanged there.

**Windows homes for the paths a session is told about.** `own_directory`,
`own_bin`, `Skills::inside`, `handoffs::inside`, `handoffs::said` and
`Executable::inside` each keep the Linux spelling on Windows today — an absolute
`/verkstead` and a `/tmp/verkstead` that mean nothing there. Each gains a
`Platform::Windows` arm under the Data Directory, the way the Mac spells them:
the skills are read where they were really written, the handoff directory is
under the fresh profile rather than at a path every Conversation would share,
and the executable a session asks with is the real path of the running image.
Nothing is bound anywhere, so the two sides of each of those are one directory.

**The Compile Server comes through the same rendering.** It is a `Surface` like
a session's and is rendered by whatever renderer the platform has, so on Windows
it is a plain process. Nothing about it changes here beyond running.

## Acceptance criteria

- [ ] `Sandbox::command` on Windows runs `argv` itself — the program is the
      first word of `argv` resolved through `PATHEXT`, with no wrapper before
      it — and the two Unix renderings are unchanged.
- [ ] A probe run through the rendering reports exactly the variables the
      description set and nothing of the server's own environment, with `PATH`
      leading with Verkstead's own bin and continuing with the server's.
- [ ] A stub `claude.cmd` on `PATH` starts through the rendering, and so does a
      stub `.exe`; an argument containing a space and a quote arrives whole.
- [ ] On Windows a worktree with a `flake.nix` is run without `nix develop` in
      front of it, and no `nix` is executed to decide that.
