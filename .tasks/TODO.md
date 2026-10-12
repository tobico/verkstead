# Desktop notifications

Push notifications never arrive in the Electron app: the switch on the Settings
head offers Web Push there because every check passes on the loopback, and
then the subscribe is refused inside Chromium, which Electron ships without
Google's push client. So the sidecar tells the app directly — one local stream
carrying every notice the phones get — and the app shows each as the platform's
own notification, only while the window is not in front, with a click opening
what the notice is about. Inside the app the switch is the app's own setting,
on by default (ADR-0020, *Amended: notifications are the app's own*).

## Tasks

- [x] 01: The sidecar streams its notices — [details](01-the-sidecar-streams-its-notices.md)
- [x] 02: The app shows a notification — [details](02-the-app-shows-a-notification.md)
- [x] 03: A click opens the notice's path — [details](03-a-click-opens-the-notices-path.md)
- [x] 04: The switch inside the app, on by default — [details](04-the-switch-inside-the-app.md)
- [x] 05: The platform legs and the docs — [details](05-the-platform-legs-and-the-docs.md)
