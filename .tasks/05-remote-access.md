# 05. Remote access is a checkbox that will not lock you out

## What to build

The Remote access pane is cut down to what a phone needs. At the top, a native
checkbox from task 02 labelled **Allow remote access via Tailscale**, saving on
tick, with one line under it:

> Allows secure remote access from other machines over the internet.

The checkbox cannot be unticked from a page that arrived over the tailnet. The
client decides that on its own, by comparing the page's hostname with the
hostname of the served address the remote view already carries; nothing new is
asked of the server. When they match, the checkbox is disabled and carries a
tooltip saying why. From localhost, or from the desktop app, it can be
unticked.

Under that, the QR code of the login link, the link with its copy button beside
it, and the Reset key button with one paragraph under it:

> Resets the secret token used to access to the UI. This will disconnect all
> other devices.

Everything else goes: the two subheadings, the readings list of node and
served address, and every static note. What stays is computed state: the
message when Tailscale is not installed with its download link, when it is
down, when it cannot be read, and when serving was refused with the operator
grant command to run, plus the reset's own error line. The card's four-state
summary is unchanged.

Tests: the remote web tests move the serve switch to a checkbox, add a page
opened over the served address that cannot untick it and one opened on
localhost that can, and drop the readings. ADR 0015 describes the pane as four
things with a switch and a node name; it now describes this.

## Acceptance criteria

- [ ] Opened at the served address, the checkbox is disabled with a tooltip saying why, and remote access cannot be turned off from there.
- [ ] Opened on localhost, the checkbox can be unticked and saves at once.
- [ ] The pane holds the checkbox and its line, the QR code, the link and copy button, and Reset key with its paragraph, plus a state message only when one applies.
