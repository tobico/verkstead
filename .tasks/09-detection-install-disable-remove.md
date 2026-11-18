# 09. Detection, install, disable and remove

## What to build

**Detection and install** (Set 1764, Q19). The onboarding rows and the Agent
Profiles setup are drawn from the harness list, enabled Agent plugins included,
rather than from onboarding's closed `Dependency` list.

- Detection is a PATH walk by the declared binary name, as it is for the five
  today.
- A plugin may declare an install-instructions link and install commands per
  platform. Those commands run only on the human's press: the wizard's own
  install run, started from the plugin's row on the Plugins page or from the
  onboarding row.
- The `Dependency`-keyed viewer tables (`Dependencies.tsx`'s `NAMES`, `WHY`,
  `COMMANDS` and `HARNESSES`, and the harness rows of `setup/instructions.ts`'s
  `GUIDES`) go for harnesses. The five built-ins answer with what they hold
  today.

**Disabling** an Agent plugin is allowed over standing Profiles (Set 1764,
Q20):

- Its Profiles stay, and are not offered on any picker.
- A remembered Pairing on one is not applied, which is the broken-Profile rule
  CONTEXT.md gives under **Pairing**.
- Sessions already running finish.
- The Plugins row says how many Profiles the plugin has.

**Remove** is refused while Profiles stand on an Agent plugin, counted and
leading there. Stage 01 counts only dependent plugins in the plugin row's
`standing`; this task adds Profiles.

## Acceptance criteria

- [ ] An enabled stub Agent plugin draws an onboarding row, detected by its
      binary on PATH. Its install command runs on the press and at no other
      time.
- [ ] A disabled Agent plugin's Profiles are absent from every picker. A
      Conversation remembering one starts with that role empty, and a session
      already running under it finishes.
- [ ] The plugin's row counts its Profiles. Remove is refused while any stand,
      and the refusal says how many.
