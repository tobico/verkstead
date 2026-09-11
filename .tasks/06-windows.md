# 06. Windows

## What to build

The run on Windows, and the sandbox row becoming the session account.

**Every Windows install goes through the runas arm**, one unit each: `winget
install --id Git.Git`, `winget install --id GitHub.cli`, Anthropic's PowerShell
installer for Claude Code, and `npm install -g` for Codex and OpenCode with
`winget install --id OpenJS.NodeJS` ahead of them where npm is missing. Grok is
xAI's PowerShell installer, elevated like the rest. Claude's installer lands in
`%USERPROFILE%\.local\bin`, which is written to `session_path` and composed
ahead of the server's `PATH` as task 01 says for this platform.

**The sandbox row is the session account.** Today it reads *not applicable*
and passes; from this task it is probed — present when the local account this
Data Directory's sessions run as exists, absent otherwise — and it **gates the
step**, because no session starts without it anyway. A ticked row runs
`verkstead session-account create` for this Data Directory through the runas
arm, which is the one elevated thing Windows wanted and the verb the adoption
doc names. The Windows tab's sandbox note says the wizard can do this now.

**And the pipe is re-opened with the grant.** The named pipe a sandboxed
Windows session asks through grants the account's SID when it is opened at
startup, and today the listener is a local in the serve loop. Hold it where
the run can reach it, and once the account exists re-open it granting the new
SID, so a session can ask over the pipe with no restart. A server that comes up
with no account keeps opening the pipe granting nobody, as it does now.

The cfg(windows) suite proves the account row and the pipe under Wine the way
the rest of the Windows suite runs; the runas commands are proven as values on
the Linux runner, the way the desktop crate already tests that arm.

## Acceptance criteria

- [ ] On a stated Windows machine, ticking git and Codex runs winget for git,
      winget for Node where npm is missing, and the npm install, every one
      through the runas arm.
- [ ] A Windows machine without the session account starts in onboarding mode
      with the sandbox row absent and the step unmet; with the account it reads
      Present and the step can be met.
- [ ] Ticking the sandbox row runs the create verb elevated, the row goes
      Present on the next probe, and a request over the pipe from the new
      account is accepted without a restart.
