# 04. Start a Conversation

## What to build

The one host function no other Activity needs: **start a Conversation**, and
**read one back**. Both live in the `activity` world, behind a capability.

- **The capability** (Set 1839, Q4) is a fourth kind under a block's
  `capabilities:`, written `- conversations: start`. The manifest reader, the
  `verkstead plugin check` refusals and the Plugins page's install dialog show
  it beside `filesystem`, `outbound` and `environment`. A component calling
  the host function without it is refused in a sentence.
- **Starting** takes what `continuing::start` takes now: the Repo, the Brief
  (the stage brief), the roadmap and stage it is, the branch and the base it is
  cut from, and the platform where the stage's line names one. The host does
  everything around it as it does today. That covers the author check, the
  branch refusals, the companions, clearing an inherited `.tasks/`, the
  Conversation row under the parent's Pairings, placement by ADR-0029
  (`continuing::placed`), the stage's record (`stage_roadmap`), and the
  Timeline sentence saying where it stands. The `next-stage` annotation is the
  planning session's to write, as now, so the host writes none.
- **The limits are the host's.** A start over the roadmap's `roadmap_stages` or
  the server's `conversations` limit (`at_once` in this device's
  `config.yaml`) is refused with an answer the Activity can tell apart from
  trouble, so it can say the stage waits for a place. A run the human
  started with a press is not held to the limits, as a press is not today.
  The host knows which kind of run it started.
- **Reading back** answers a Conversation's state and whether it has settled,
  and lists the stages the record holds for a roadmap with their standing
  (`stage_standings`, `stage_chain`). That is how the Activity learns what is
  done and in flight without keeping anything.

## Acceptance criteria

- [ ] A fixture Activity declaring the capability starts a Conversation on a
      fixture Repo and reads it to Done
- [ ] A start over the limit is refused as *no place*, and the same start
      succeeds once a place frees
- [ ] A stage marked `on windows` is placed as ADR-0029 says
- [ ] A block without the capability is refused at the call; the install
      dialog lists the capability for a plugin that declares it
