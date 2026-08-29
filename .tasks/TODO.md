# Conversation sharing

A read-only copy of a Conversation as one self-contained HTML file, for
showing colleagues what was asked, answered and built without giving them the
workbench. The file is a share build of the SPA booting from inlined JSON,
drawn as the two-pane workbench; it carries the Brief, the Question Sets with
their answers, and the commits with their full diffs, and silently omits agent
output, Notices, handoffs and the pinned cards. It downloads for email, and
one click publishes it as a secret gist and comments on every pull request the
Conversation holds — an itemized summary plus a link that renders in the
recipient's browser through a small viewer page the human hosts once.

Privacy is possession: whoever holds the file (or the gist link) can read it.
Recipients are colleagues with repo access. A share is a snapshot as of the
moment it was made; sharing again makes a new one.

## Tasks

- [x] 01: Share file download — [details](01-share-file-download.md)
- [ ] 02: Question Sets readable in the share — [details](02-sets-in-the-share.md)
- [ ] 03: Commits readable in the share — [details](03-commits-in-the-share.md)
- [ ] 04: Publish a share as a secret gist — [details](04-publish-as-gist.md)
- [ ] 05: The viewer page and its setting — [details](05-viewer-page.md)
- [ ] 06: One click to every pull request — [details](06-comment-on-every-pr.md)
