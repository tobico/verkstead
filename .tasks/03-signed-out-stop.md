# 03. Signed-out stop

## What to build

A watcher in the shape of the usage-limit one, reading the same three records
(Capture, Screen, the backend's log), finds a line that **ends** with
`Please run /login` once terminal decoration is off it — Claude says it as
`<reason> · Please run /login`. It writes a "Signed out" stop through the shared
stop machinery: one Notice, *blocked on you*, the devices told, and the session
ended. The Notice carries a **Log in** press that opens the modal from task 01.

For a mirrored Profile the Notice has no press and says to log in on the device
the account is at home on.

## Acceptance criteria

- [ ] The sign-out wording is checked against claude 2.1.283 and the version
      recorded beside the phrase, as `limits.rs` does
- [ ] A session printing a line ending in the phrase stops as Signed out with a
      Log in press; a line with the phrase mid-line does not stop it
- [ ] Devices are told, as for any stop
- [ ] A mirrored Profile's Notice names its home device and has no press
