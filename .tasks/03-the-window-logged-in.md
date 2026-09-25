# 03. The window, logged in

## What to build

Once health answers, the app reads the **Workbench Key** out of the **Data
Directory** and opens its one window on the login link, so the window lands on
the workbench already logged in. Nothing on the wire changes and nothing about
the viewer changes: it is loaded exactly as it is served.

**The file is the documented thing** (ADR-0020). `workbench.key` sits beside the
settings files in the Data Directory at mode `0600`, made by the server at its
first start and read back at every one after it — so the app reads that file
rather than parsing a line out of the sidecar's stdout, and rather than
generating a key and handing it in. Both were considered and rejected. The
directory the app reads it from is the one the sidecar was started against, which
means resolving the Data Directory the same way the server does: what the
environment says, else the platform's own — `~/.local/share/verkstead` on Linux,
`~/Library/Application Support/Verkstead` on macOS, `%APPDATA%\Verkstead` on
Windows. Read it fresh each time it is wanted rather than holding it in a
variable: a link built from a stale secret is a 401 with extra steps.

**The link is the address with the key on it**, and opening one is the whole of
logging in: the server sets the cookie and redirects to the same path without the
parameter, so the secret leaves the URL before the page is drawn.

**A 401 on the window's own frame is the key having been reset from the phone.**
**Reset key** at the foot of Remote Access re-issues the secret, and everything
holding the old one meets a 401 on its next request. So the app watches the
frame's own navigation response — not a subresource's, which is a different
failure with a different answer — reads the file again and loads the link again.
Once per navigation, so a genuinely unreadable key is not a loop.

**What vitest covers** is the pure half: the Data Directory resolved per
platform from values it is handed rather than from this process's own
environment, and the link built from an address and a secret. All three
platforms' arms are exercised on Linux, the way the server's own platform module
does it.

## Acceptance criteria

- [ ] The window draws the workbench logged in, with no 401 and with no key left
      in the URL once the handshake has redirected.
- [ ] **Reset key** pressed from another browser has the window back in after one
      reload rather than sitting on a refusal.
- [ ] A Data Directory with no `workbench.key` in it yet — the server still
      starting — ends in a logged-in window rather than a 401.
- [ ] The Data Directory resolution and the login link are under vitest, with all
      three platforms' arms exercised.
