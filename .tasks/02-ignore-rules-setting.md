# 02. Ignore rules live in the settings and round-trip the API

## What to build

A new setting: a global list of ignore rules, stored in `config.yaml` beside
the other workbench settings, carried through the settings API and the
generated web types so the pane in task 04 has something to edit. This task is
the storage and the wire; nothing acts on the rules yet.

The rule shape is settled and is the decision-rich part:

```yaml
ignored_comments:
  - author: "coderabbit"   # optional regex over the comment author's login
    body: "billing"        # optional regex over the comment body
```

- Each rule has two optional regex fields, **author** and **body**. Where both
  are given, both must match; rules combine with OR across the list.
- Rust regex syntax, matching anywhere in the text (no implicit anchors),
  case-sensitive with `(?i)` available inside a pattern. An empty or absent
  field is no constraint on that part.
- A rule with both fields empty is refused at save, as is any pattern that does
  not compile — the refusal names the rule at fault, so the UI can show the
  error at the row.
- The file is read leniently, the way every other setting is: a hand-edited
  pattern that does not compile is skipped at runtime as matching nothing, with
  a line in the log, and never refuses anything. An absent key is an empty
  list.

Follow the existing settings pattern end to end: the config field with its
accessor, the view and edit wire types, the save and load endpoints, and the
regenerated TypeScript types. Every settings section saves the whole file, so
the new field must ride along untouched through the other sections' saves.

## Acceptance criteria

- [ ] Rules round-trip through the settings API: saved, read back identical, and surviving a server restart via `config.yaml`.
- [ ] A save containing an invalid pattern, or a rule with both fields empty, is refused with an error naming the offending rule; nothing is written.
- [ ] A bad pattern hand-edited into `config.yaml` loads as a rule that matches nothing and never refuses startup or a read.
- [ ] The generated web types carry the new field, and saves from the other settings sections leave stored rules untouched.
