# 06. NuGet

## What to build

The `dotnet` **Descriptor**: NuGet's global packages folder and its http cache,
both under `{cache}`. NuGet has no compiled half to share, so this is the
downloads and nothing else.

Two things this entry settles that the others did not:

- **Its `detect` list is empty, and the YAML says why.** A .NET project is
  `*.csproj` or `*.sln`, and `detect` matches literal filenames at the root of a
  checkout — it has no globs. Adding them would be new behaviour in the server,
  which is the one thing this stage is meant to prove unnecessary, and detection
  is only ever the composer's warning: the variables are every session's whatever
  the Repo holds, so an empty list costs nothing but the warning .NET never had.
- **The directory the variable names must hold nothing that is a session's
  own.** Check what else the .NET CLI keeps alongside the packages folder —
  configuration and credentials in particular — and keep the shared directory to
  the store.

The proof is the harness task 01 built: two installs at once in two Sandboxes
against one store, then a third **denied its registry** by taking every source
away. The runner image carries a .NET SDK, and the dev shell gains one.

## Acceptance criteria

- [ ] The `dotnet` entry sets NuGet's packages folder and http cache for every
      session on all three platforms, with an empty `detect` and a comment saying
      why
- [ ] Two installs at once, then a third with every source taken away, which
      succeeds
- [ ] Nothing that is a session's own — configuration, credentials — lands in
      the shared directory
- [ ] The .NET SDK is in the dev shell and reached by the CI job, and a missing
      one is skipped locally and red in CI
