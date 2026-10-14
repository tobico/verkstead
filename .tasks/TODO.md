# Public website

A one-page site for Verkstead at <https://tobico.github.io/verkstead/>, beside
the share viewer that already lives there, whose job is to get a senior
engineer to download Verkstead Desktop and try it. Hand-written HTML, CSS and a
little JS under `site/`, no dependencies, drawn in the workbench's own tokens
so it reads as the product. A hero, a download bar, three sections of cards
with recreated workbench "screenshots" beside them, a downloads table and a
footer. The copy was settled word for word in the grilling and is written into
each task below; the tasks build, they do not rewrite.

Every screenshot follows **one made-up Conversation** so the page tells one
story: the repo `tidewatch`, a small web service; the branch `rate-limiting`;
the Brief *Add rate limiting to the public API. Last night one client sent 40k
requests in a minute and took the service down. Limit per API key, with a
sensible default that can be changed per key.* Its agent is Claude Code on
Opus 5; its pull request is #128, *Rate limiting for the public API*. The
details each screenshot needs are in the task that draws it.

## Tasks

- [ ] 01: The page, live — [details](01-the-page-live.md)
- [ ] 02: Features — [details](02-features.md)
- [ ] 03: Process — [details](03-process.md)
- [ ] 04: Platforms — [details](04-platforms.md)
