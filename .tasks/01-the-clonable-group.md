# 01. The clonable group in the dropdown

## What to build

A device can say which repositories it could clone: every Clonable Repo
(CONTEXT.md) the cluster holds that this device lacks. The Repo dropdown draws
them as a second group.

**The reading.** It is served on a new viewer route beside the device's own
Repo list, so a browser reaches a member's the way it reaches Open repo and
Create repo: through the relayed `/api/ui/members/{device}/…` path. The device
asked reads every other member's registry over the Relay. That registry is the
existing peer route a member's Repos are matched from, which sends id, name,
path and origin. The device then removes everything it already holds by the
origin rule under **Repo** in CONTEXT.md. The rule is applied on the device
that asks, reusing the normalisation in the server's matching module rather
than writing it a second time. The device's own origins are read afresh off git,
as the match already does.

- **Only repositories with an origin are clonable.** A member's Repo with no
  origin is never listed, and a name match is never a reason to list or hide
  anything.
- **One origin is one row**, however many members hold it. Origins are compared
  after normalisation.
- **The row's name is the origin's last path segment**, with a trailing `.git`
  and slash taken off. It is what `git clone` would name the directory, and the
  directory task 02 clones into is the same name. The human chose this over a
  member's own directory name, since two members can hold one origin under
  different names.
- **Under the name, the row says where the origin is**: the URL with its scheme,
  its user and its `.git` taken off. For example, `git@github.com:tobico/verkstead.git`
  and `https://github.com/tobico/verkstead` both read as
  `github.com/tobico/verkstead`. The human chose this over the raw URL and over
  the host alone.
- **A member that does not answer contributes nothing and refuses nothing.**
  Members are asked concurrently, each with a bounded wait, so a sleeping
  machine never holds up the dropdown. The reading as a whole fails only for
  this device's own trouble.
- The reading carries the origin URL as the member reported it, which is what
  task 02 clones from.

**The dropdown.** The picking module's Listbox gains a second kind of row
between the choices and the two `Action`s at its foot: the clonable rows, behind
a rule, each a name over its origin's place. Like an Action, a clonable row is
never a choice. It is never what the closed control shows and never what `pick`
is called with. The keyboard walks these rows with the rest, as it does the
Actions. Pressing a row does nothing in this task: task 02 wires the press.

The group is read per device, keyed by device the way the Repo list is. It
appears wherever the Repo dropdown is drawn: the compose page's select, a saved
draft's Repo panel, and the draft-move card, which reuses the compose page's
select. An empty reading draws no rule and no group.

Follow docs/copy.md for every word on screen and CONTEXT.md for every name. The
term is *clonable*. Avoid *remote repo*, *available repo* and *clone target*.

## Acceptance criteria

- [ ] In a two-server suite where one server holds a Repo with an origin the
      other lacks, the server lacking it lists it as clonable, with the right
      name and origin, and the server holding it does not.
- [ ] A member that does not answer adds no rows and makes the reading neither
      fail nor wait beyond its bound. A Repo with no origin is never listed, and
      two members holding one origin, spelled differently, give one row.
- [ ] The viewer suite shows the group behind a rule between the device's own
      Repos and Create repo / Open repo, each row a name over its origin's place.
      The group is read for the device the dropdown is for, is never shown as
      the chosen value, and draws nothing when it is empty.
