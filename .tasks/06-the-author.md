# 06. Who the work is committed as

## What to build

The third step as `03-author`, `03-github-answered` and
`dark-03-github-answered` draw it. What is saved and how is today's: the
whole-settings save with verification, the prefill from git config and the
environment, nothing written until the press.

**Two cards.** *Git Author* with *Both are needed* on the right, Name and
Email side by side from 60rem and stacked below it. *GitHub Token* with an
**Optional** pill after the heading, the sentence saying what the token is for,
the Token field with *Stored, and never shown again.* under it, and the line
about a token already saved where there is one. Where a prefilled value came
from is a short hint inside the field on the right, *git config*, *GH_TOKEN*,
*GITHUB_TOKEN* or *gh login*, and the full sentence the field says today is the
hint's title, so nothing the viewer said is lost.

**The scopes block is restyled once** for both pages (settled over a compact
variant): a boxed block titled *Tick these on GitHub before pasting*, a
*Classic* line of mono chips for `repo`, `workflow` and `gist`, and a
*Fine-grained* line naming the permissions. The settings page's token section
draws the same block and its tests pass.

**One button, Finish setup.** It always reads **Finish setup**. The press saves
the fields and, where the answer is clean, finishes in the same press: no token
to verify, or a token GitHub verified with every scope. The step stays open
only where the token is not good enough as it stands: GitHub refused it, or
verified it as an account but named a missing scope. Then the callouts draw
under the field, a green *GitHub says it's <login>.* and an ember *It can't
publish a share without the gist scope. That can be put right later in the
settings.*, or the red refusal with its reason, a green **Saved** stands beside
the button, and the next press of Finish setup goes on without the scope or
the token. Typing anything clears Saved so the next press saves again, as
today. Disabled, the reason beside the button reads *Both author fields are
needed*. The git suite's test that waits one more press after a clean
verification changes to finishing at once.

## Acceptance criteria

- [ ] The two cards match the boards at 1088px and 390px, with the source
      hints inside the fields and the full sentence as each hint's title.
- [ ] The scopes block is the compact chips-and-line form in the wizard and on
      the settings page, and both suites pass.
- [ ] Finish setup finishes in one press with no token or a fully scoped
      token, and stays on the step with the callouts and Saved for a missing
      scope or a refusal, finishing on the next press; each case has a test.
