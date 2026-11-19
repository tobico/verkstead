# 01. One session

## What to build

**One session**, the plain Activity, as a second component in the built-in
Activities plugin (`plugins/activities/`, beside `follow-up-rounds`), declared
in its `plugin.yaml` with its own `activity` block. It is configured entirely
by options: the skill (`<pack>/<skill>`), the role, the prompt, the Done-signal
evidence to expect, how it ends (`ends`), and the **building kind** its Memos
and previews are permitted by (today `memos::Building`, said to the `Lent`
in-process — it becomes an option the host reads). Its outcomes are `done`, and
`refused` where the Done signal never came and the session was stopped, plus
`steered`.

The in-process drivers for the inline build (`verkstead/implementing` with the
handoff), the `submitting` step (evidence `Landed`) and the `instruction` steer
(`runner::instructed`) call this Activity by name the way `follow_up_rounds`
calls Follow-up rounds — the built-in manifest looked up, `activities::run`
with the options — and act on its outcome where they acted on their session's
ending before. The prompts stay the `skills.rs` builders' words; whether the
component writes them from facts or is handed them as an option is the task's
call, but the session must be told exactly what it is told today.

Stage 08's lesson applies: a server with no skills installed starts no session
and says so in its log, rather than stopping the Conversation.

## Acceptance criteria

- [ ] An inline Develop builds to its pull request with every session launched by the One session component; the inline tests in `crates/server/tests` pass unchanged
- [ ] A steer with an instruction runs under the component and returns to where it came from; `submitting` opens its pull request under it
- [ ] Memos and previews are still permitted or refused by building kind, now read from the Activity's option
- [ ] `verkstead plugin check` accepts the built-in manifest with the new block, and the Plugins page's Activities row names *One session*
