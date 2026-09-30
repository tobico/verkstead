# Package stores

A session that installs packages with npm, pnpm, yarn, deno, bun, pip, uv,
poetry, pipenv, Go or NuGet downloads each one once for the machine. A second
Conversation against the same Repo installs out of the store the first one
filled, and two sessions installing at once do not damage it. Go's build cache
is shared too, which is compiled output that happens to be only a directory.

Every one of the eleven is a built-in **Descriptor** and nothing else — YAML in
the grammar stage 01 settled, plus its proofs. No new behaviour in the server,
which is what makes this stage the proof that stage 01's system is enough. The
eleven tools become **four** entries — `go`, `node`, `python` and `dotnet` — so
one box on the **Language support** pane covers an ecosystem.

Roadmap stage: [02: Package stores](docs/roadmaps/language-caches/02-package-stores.md)

## Tasks

- [x] 01: Go, and the proof a store was read — [details](01-go-and-the-store-proof.md)
- [ ] 02: npm, pnpm and yarn — [details](02-npm-pnpm-and-yarn.md)
- [ ] 03: deno and bun — [details](03-deno-and-bun.md)
- [ ] 04: pip and uv — [details](04-pip-and-uv.md)
- [ ] 05: poetry and pipenv — [details](05-poetry-and-pipenv.md)
- [ ] 06: NuGet — [details](06-nuget.md)
- [ ] 07: The docs — [details](07-the-docs.md)
