# 07. Log in on Windows

## What to build

Task 01 runs the login on Linux and macOS only; on Windows the Log in press is
refused with a message saying so (`login_opened` in `crates/server/src/ui.rs`)
and `Sandbox::for_login` returns `None`.

Make the login run on Windows too. It has to be started as the session account
— the way `sandbox::off_a_console` starts one — but with its standard input
held open, so that the code typed into the modal can be written to it after the
address has been read off its output. What `sandbox/starting.rs` has today
writes all of a process's input up front and waits (`ordinarily`,
`over_pipes`); `as_the_account`, `piped` and `left_running` are the pieces to
build the live variant from. The Windows boundary needs a key for a login,
which has no Conversation (`Sandbox::conversation` is `None` for one).

Only CI's Windows job can check this code; there is no Windows target on the
development machine.

## Acceptance criteria

- [ ] On Windows, the Log in press starts a login and the modal shows its
      address
- [ ] A code handed over reaches the process and the login lands in the
      Profile's account through the Windows write-back
- [ ] Closing the modal or the 10-minute limit kills the process
- [ ] The "not available on Windows yet" refusal is gone
