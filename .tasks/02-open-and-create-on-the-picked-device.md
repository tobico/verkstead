# 02. Open and Create on the picked device

## What to build

The two rows at the foot of the Repo dropdown — **Open repo** and **Create
repo** — browse and make directories on the device the select names.

A thin slice, and deliberately so: the path browser and both modals already read
which device the page they are drawn on is about, and the browse, the
registration and the `git init` already go out under it. What this task delivers
is that being true from the compose page end to end, proven against a second
machine, plus the one thing that is genuinely wrong.

**Where the last repo was made is remembered per device.** That memory is a
single key in this browser today — where somebody keeps their code is a fact
about the machine in front of them — so a Create with a member picked opens its
browse at a path on the wrong machine. It becomes one remembered parent per
device, and a device with nothing remembered yet starts where a first run starts:
an empty field browses the server's own home, which is the picked device's home
here.

**The refusals are the far end's own and are already worded** — a parent that is
not a directory, a name already taken, a repository `git init` would not make.
Nothing is added to them. What has to hold is that they arrive from the picked
device and read exactly as they always have, rather than as a relay failure.

**And the Repo that lands is picked, as it is today.** Either row answers with the
Repo it registered and the page settles on it, which is why neither has to match
a path against the list: a create is told what to call the directory rather than
where it ends up. With the pick now being a Repo on another machine, that is the
whole of what makes it the work's own Repo there.

## Acceptance criteria

- [ ] With a member picked, Open repo's path browser lists that device's
      directories, and registering lands a Repo on it that the dropdown then
      offers and the pick settles on.
- [ ] With a member picked, Create repo makes the directory and the repository on
      that device, and its browse opens where the last repo on **that** device
      went rather than at a path on this one.
- [ ] A refusal from either row — a name already taken, a parent that is no
      directory — reads in its own words rather than as a failed call.
- [ ] With this device picked, both rows behave exactly as they did before the
      select existed.
