# Setup wizard — B · Rail

Exported from the Verkstead Penpot file, page *Setup wizard — B · Rail*, on
2026-10-25, so the session that builds it can check the screen against the
boards without the Penpot file. The page's own heading reads:

> The approved design (Question Sets 1377, 1379 and 1382), through every
> screen and state: choosing, installing, by hand, profiles found and not
> found, the author before and after GitHub answers, the phone width, and
> dark.

## The boards

Each board is exported twice under one name: a 1:1 PNG to look at, and an
SVG carrying the exact sizes, colours and text for checking values. Wide
boards are 1088 px across, the rail 290 px of it; phone boards are 390 px.

| File (`.png` and `.svg`) | Board |
| --- | --- |
| `01-choosing` | 1 What a session needs — choosing |
| `01-installing` | 1 What a session needs — installing |
| `01-by-hand` | 1 What a session needs — by hand |
| `02-accounts-found` | 2 Agent profiles — found |
| `02-none-found-naming-one` | 2 Agent profiles — none found, naming one |
| `03-author` | 3 Who the work is committed as — filling in |
| `03-github-answered` | 3 Who the work is committed as — GitHub answered |
| `phone-01-choosing` | phone · 1 What a session needs — choosing |
| `phone-02-steps-open` | phone · 2 Agent profiles — steps open |
| `dark-01-choosing` | dark · 1 What a session needs — choosing |
| `dark-03-github-answered` | dark · 3 Who the work is committed as — GitHub answered |

## Implementation notes

The notes board on the Penpot page, verbatim. They were written for the
session that builds the design and read against the boards above,
`web/src/setup/` and `crates/render/src/onboarding.rs`.

**Where the grilling overrode them.** The backlog under `.tasks/` (Question
Sets 1387 to 1391, 2026-10-25) is what the work answers to, and it differs
from the notes in these places:

- *Copy*: the step titles and the page heading stay **Title Case**, as
  `docs/copy.md`'s heading rule has them; the boards' sentence-case titles
  are not followed. Everything else is sentence case as drawn, and a
  complete sentence carries its full stop where the boards drop it (*Every
  session needs both.*, *Tick an account, or save one.*); the rail's
  summaries take the serial comma (*A sandbox, git, and Claude Code*).
- *Scope*: there is **one wire change**. The onboarding reading gains a
  `machine` field so the rail's reading line can name the machine.
- *Frame*: the rail's brand is the page's visible h1 with SETUP under it;
  there is no *Set Up Verkstead* heading.
- *Step 1 · choosing*: the probe's details (resolved path, link target, the
  seen-elsewhere note, `bwrap`'s stderr) are drawn on the by-hand screen only.
  *Install by hand instead* opens that screen over every missing row.
- *Step 1 · by hand*: Next is released once a sandbox, git and one agent are
  present, whichever way the screen was reached, and reads **Skip** after a
  run while ticked rows the run failed are still missing.
- *Step 2 · profiles* and *Step 3 · author*: the profile form and the scopes
  block are restyled once for the wizard and the settings page alike, rather
  than given wizard-only variants.
- *Step 3 · author*: the button always reads **Finish setup** and finishes in
  one press where GitHub's answer is clean; it stays on the step only for a
  refusal or a missing scope.
- Small calls: the rail reuses `--hunk`; the wide breakpoint is the app's
  60rem; the phone's Steps sheet is the Modal; the Windows WSL row sits under
  Optional.

### Scope

The work is mostly in web/src/setup/ (SetupPage, Dependencies, Accounts, Git,
Mark, steps.ts and their CSS modules), plus the strings. Nothing on the wire
changes: the per-row install state and the failure's own words are already
there (see Installing and By hand). Nothing about ADR-0016's rules changes
either: only the open step and the steps already met can be reached.

### Frame

SetupPage becomes two columns at the wide breakpoint: a rail 290px wide with
the brand, a SETUP label, the three steps and the reading dot at the foot, and
the step on the paper to its right, with Back and the primary button pinned to
the foot. DragBar stays. The rail's colour is exactly --hunk (#f2ede6 light,
#26241f dark), so either alias it as a new --rail token or reuse --hunk with a
comment.

### Rail rules

A rail item is a button only when its step is met and not open (today's
StepHead rule). An unmet step is plain text, because pressing it would be the
skip ADR-0016 refused. Keep data-step, data-met and aria-current on each rail
item so web/tests/onboarding.test.tsx still finds them. A met step's one-line
summary is computed in the viewer: step 1 names the present rows, step 2 names
the profiles kept.

### Phone

Below the wide breakpoint the rail folds into a header: the brand, a Steps
button, and a strip of three dots with the open step's name. Steps opens a
sheet over a veil (borrow Modal or Menu) listing the steps as in the rail, with
Open on the met ones. The foot is sticky, with a full-width primary button and
the quiet alternative under it.

### Primary button

The button names what it will do: Install N (N = rows ticked), Keep N
profile(s), Save, then Finish setup once GitHub has answered. Disabled is drawn
at 45% opacity with its reason as a short line beside it ("Tick an account, or
save one", "0 of 3 found"), replacing the paragraph Note under the button.

### Step 1 · choosing

Group the rows: Required (sandbox and git), An agent with "At least one" (the
four harnesses as a 2x2 grid of tiles, on phones too), and Optional (gh). Rows
the reading says are present get no checkbox (already true today) and show a
green Installed pill. Checkboxes are ember: a custom 18px box, or at minimum
accent-color: var(--accent). "Install by hand instead" is a quiet link in the
foot.

### Installing

The wire already carries a per-row state: DependencyView.install is an
InstallState of Idle, Installing or Failed { why }. Use it for the row labels:
Installing in ember on the running row, and nothing on a row still waiting
(Idle). That was decided in Question Set 1382: no queued label and no wire
change, because the bar is what says how far the run has got. The bar comes
from RunView.done/total; phase Asking puts the status line in the ember
callout; Cancel stays; there is no Next during a run; the rail's reading line
says every 2 s during a run (the RUNNING cadence).

### Step 1 · by hand

The nine OS tabs become one select ("Commands for Ubuntu"). Each missing row
is a block: the waiting ring, the name and why, and a "Failed ▾" pill (decided
in Question Set 1379). The pill opens a disclosure, closed by default, showing
InstallState.Failed.why in full under "What the machine said", so no failure
kind is needed on the wire. Then a command block with a $ prompt and a copy
icon, the vendor note, and the alternative behind "Or from npm ›". The PATH box
moves under the list. The count ("0 of 3 found") sits beside the disabled
Next.

### Step 2 · profiles

Accounts are cards: harness mark, name, the paths in mono, and a tick on the
right with an ember border when ticked. An account whose harness is missing is
drawn quiet with a "Not installed" pill and "Go to step 1", which is allowed
because step 1 is met. With nothing found, a card says "No account in ~ yet"
with a Watching dot and one login command per runnable harness. "Name an
account yourself" is a dashed card that opens into ProfileForm; the agent
picker inside it is drawn as a segmented control and the models as ember
chips.

### Step 3 · author

Two cards: Git author (Name and Email side by side when wide, "Both are
needed") and GitHub token (Optional pill). Where a value came from is drawn as
a short hint inside the field ("git config", "GH_TOKEN"); keep the full
sentence today's source line says somewhere reachable, such as a title or a
line under the field. The scopes are compacted to a Classic row of chips and a
Fine-grained line. TokenScopes lives in settings/scopes.tsx and the settings
page shares it, so give it a compact variant rather than changing it in place.
GitHub's answer and the missing scope are callouts; the button goes from Save
to Finish setup with a green Saved beside it.

### Copy

Sentence case everywhere, as docs/copy.md asks: TITLES in steps.ts becomes
"What a session needs", "Agent profiles", "Who the work is committed as"; the
h1 becomes "Set up Verkstead". The new strings (eyebrow "Step N of 3", the
group labels, pills, button labels, "Reading ada-box every 10 s" with "every 2
s" during a run) need a copy.md pass. Spell it Cancelled.

### Colours

Everything is the existing palette. The only new value is the selected wash on
chips and tiles (#f7ebe1), which is better written as color-mix(in srgb,
var(--accent) 8%, var(--card)) so dark follows on its own. The dark boards are
the same shapes with the dark tokens applied mechanically, so check contrast
for the pills on --code-wash in dark.

### Type

The boards are drawn in Inter (the Penpot file's font token) and the viewer
uses system-ui. Sizes on the boards map to rem: title 26px = 1.625rem, body
16px, secondary 13.5px = 0.85rem, labels 11.5px uppercase with 0.05em
tracking.

### Tests

onboarding.test.tsx asserts on headings equal to TITLES, the open class, and an
h2 button only on met steps. dependencies, accounts and git tests assert on
their rows and notes. Expect to update them with the markup; the behaviour
(polling, one Next per step, met-only reopening) is unchanged. Remember the
jsdom heap trap: a never-matching getByRole inside waitFor OOMs the suite.
