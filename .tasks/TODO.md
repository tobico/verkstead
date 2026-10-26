# Updating members

Every member's Devices row gets **Update** when that member runs a desktop
install and a newer stable release exists, and the Update Notice's banner gets
**Update all**. A press on a member's row asks *Update crow?*, naming the
version and the sessions running there, then tells the member, which runs stage
04's exchange on itself; the row reads *updating…*, then *restarting…* once the
member goes quiet, until it comes back on its new version. Update all tells
every reachable member at once and updates this device last, once each member
is back or has a trouble line. A member unreachable at the press is skipped
with a line, and any device that could not be updated raises one notification.

Each device updates itself when told (ADR-0027): the pressing device sends a
peer call and never pushes bits. Click only, members first, this device last.
Demonstrable with two desktop devices, pressed from a phone.

Roadmap stage: [05: Updating members](docs/roadmaps/cluster-updates/05-updating-members.md)

## Tasks

- [x] 01: A member says what it is, and can be told — [details](01-a-member-says-what-it-is-and-can-be-told.md)
- [x] 02: Update on a member's row — [details](02-update-on-a-members-row.md)
- [ ] 03: The member coming back — [details](03-the-member-coming-back.md)
- [ ] 04: Update all — [details](04-update-all.md)
- [ ] 05: The notification on trouble — [details](05-the-notification-on-trouble.md)
