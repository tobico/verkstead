# The viewer: the SolidJS SPA under `web/`, built by vite to the static files the
# server embeds (ADR-0003), and — with `runTests` on — the same sources put through
# typecheck and vitest instead.
#
# One file for both because they are one build: the same fileset, the same pinned
# node and pnpm, and the same store of dependencies, so a lockfile argument
# between `nix build` and `nix flake check` is not a thing that can happen.
#
# The pnpm store is fetched separately, as a fixed-output derivation named by
# `pnpmDeps.hash` — that is the one step allowed to reach the network, and the
# hash is what says the lockfile has not moved under us. Change `web/package.json`
# or `web/pnpm-lock.yaml` and this hash has to change with them; nix will print
# the one it wanted.
{
  lib,
  stdenvNoCC,
  nodejs,
  pnpm,
  # Run the viewer's own suite rather than building it. The output is then a
  # stamp: a check is a thing that either builds or does not.
  runTests ? false,
}:

stdenvNoCC.mkDerivation (finalAttrs: {
  pname = if runTests then "verkstead-web-tests" else "verkstead-web";
  version = (lib.importTOML ../Cargo.toml).workspace.package.version;

  # `web/` and the assets vite copies verbatim into the site root, and nothing
  # else. The fixtures under `web/tests/fixtures` are the payloads `cargo test`
  # wrote, and the generated `web/src/api/types.ts` the same — both are committed,
  # so this reads them rather than needing the Rust half built first.
  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ../web/index.html
      # And the share build's own document, which is the second thing `pnpm
      # build` writes: the viewer as one self-contained file, for a Conversation
      # sent to somebody who has no Verkstead.
      ../web/share.html
      ../web/package.json
      ../web/pnpm-lock.yaml
      ../web/eslint.config.js
      ../web/tsconfig.json
      ../web/vite.config.ts
      ../web/vite.share.config.ts
      # And the diagram renderer's, which is the third: mermaid on its own, for
      # the shares that carry a Diagram and no others.
      ../web/vite.mermaid.config.ts
      ../web/src
      ../web/tests
      # The service worker, the manifest and the icons — vite's `publicDir`.
      ../assets
      # And the share viewer, which is neither built here nor served from here:
      # it is a hand-written page the server hands over for the human to host,
      # so it lives with the server that ships it. The suite drives it as a file
      # — see `web/tests/viewing.test.ts` — the way it drives the service worker
      # above, so the check needs it in the source even though the build does
      # not.
      ../crates/server/share-viewer.html
      # And the two files that say where that page is published, which the same
      # suite reads as text and holds against each other: the workflow that puts
      # it on GitHub Pages, and the module that composes every share's link
      # through it. One of them drifting is a 404 on every share ever published,
      # so the check is worth having — and it can only run on a source that has
      # them.
      ../.github/workflows/pages.yml
      ../crates/server/src/sharing.rs
    ];
  };

  nativeBuildInputs = [
    nodejs
    pnpm.configHook
  ];

  pnpmDeps = pnpm.fetchDeps {
    inherit (finalAttrs) version src;
    # Named for the build in both cases, so that turning `runTests` on does not ask
    # for a second copy of the same store under a second name.
    pname = "verkstead-web";
    sourceRoot = "${finalAttrs.src.name}/web";
    fetcherVersion = 2;
    hash = "sha256-DjRtBWJTSZf55Ff2Pc6FnCzlcU3nr92XRxDGoqi8nAk=";
  };

  sourceRoot = "${finalAttrs.src.name}/web";

  buildPhase = ''
    runHook preBuild
  ''
  + lib.optionalString (!runTests) ''
    pnpm build
  ''
  + lib.optionalString runTests ''
    # The wall around the query hook first: every query goes through
    # `useReading`, which makes it name a reconcile key or declare its payload
    # static (ADR-0009), and this is the check that says so of the queries not
    # yet written.
    pnpm lint

    # Typecheck as well as test: a component that only compiles because vite
    # erases the types would pass vitest and fail nobody, and the generated
    # `types.ts` is only worth generating if something checks the viewer against
    # it.
    pnpm typecheck
    pnpm test
  ''
  + ''
    runHook postBuild
  '';

  installPhase = ''
    runHook preInstall
  ''
  + (
    if runTests then
      ''
        touch $out
      ''
    else
      ''
        # Both builds, each under the name rust-embed knows it by — see
        # `crates/server/src/viewer.rs`. Two directories rather than one because
        # they are two things: the site the server serves, and the one-file
        # template it fills a Conversation into and hands over as a download —
        # with the diagram renderer beside that template, for the shares that
        # need one.
        mkdir -p $out
        cp -r dist $out/dist
        cp -r dist-share $out/dist-share
      ''
  )
  + ''
    runHook postInstall
  '';

  meta = {
    description =
      if runTests then "The Verkstead viewer's vitest suite" else "The Verkstead viewer, built";
    platforms = lib.platforms.unix;
  };
})
