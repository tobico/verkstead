# 03. The Windows side

## What to build

The Windows side of the choice runs whole: the field browses the Keeper, the
press registers the path there — author picker as the card has it — and then
clones on the Annex from the repository's origin, as that Repo's Author, into
the parent the Annex remembers, registers the clone, and makes it the Draft's
Repo. One press, no terminal.

The registration's answer learns the repository's origin — today it carries
only the GitHub `owner/repo` — and the clone follows that origin, driven by
the viewer over the per-device calls that exist. A repository with no origin
is refused with a sentence saying what to do (push it somewhere first, or
work on it on Windows); the Keeper's registration stands either way, as it
does when the clone itself fails — true, harmless, and a shorter retry — with
the refusal on the card. A line on the card says branches not pushed to the
origin stay on Windows.

No `/mnt/c` path is ever registered on the Annex (ADR-0014, ADR-0024), and
the Transfer Preflight is untouched: a path registered on the Keeper is the
clonable reading's to offer already. Every string follows `docs/copy.md`.

## Acceptance criteria

- [ ] Picking a Windows repository with an origin leaves a registered,
      working Repo on the Annex and makes it the Draft's, with no terminal
      involved.
- [ ] A repository with no origin is refused with a sentence saying what to
      do, and a failed clone leaves the Keeper's registration standing with
      the refusal on the card.
- [ ] The card says branches not pushed to the origin stay on Windows.
