# 02. The choice on the card, and the Linux side

## What to build

Open repo, on a Draft whose device is an Annex, draws a two-way choice at the
top of the card asking where the repository is. The options are labelled by
the pair's platforms, read off the member rows — the Annex's own and its
Keeper's, found by the mark saying whose it is — never hard-coded, so a later
Annex beside a Mac names itself with no new copy. Every other Open repo card
is unchanged, and the dropdown's foot keeps its two rows.

The Linux side browses the Annex's filesystem, as the card always has, but an
empty field opens on the login user's home rather than the service's own —
read off the box's user, not a hard-coded path — so a repository cloned by
hand in the WSL shell is where the browse starts. Picking one registers it
and lands it as the Draft's Repo exactly as Open repo does today.

The Windows side's field browses the Keeper and its press registers the path
there, as Open repo on the Keeper does today — which already makes the
repository clonable for the Annex's dropdown. The clone that completes the
flow in one press is task 03's; this task stops at the registration landing.

Every string follows `docs/copy.md`.

## Acceptance criteria

- [ ] On an Annex's Draft, Open repo offers the two platform-named options,
      and the labels come off the member rows rather than constants.
- [ ] A repository cloned in the login user's home from their own shell is
      where the Linux browse opens, and picking it makes it the Draft's Repo.
- [ ] On every Draft whose device is not an Annex, the card is unchanged.
