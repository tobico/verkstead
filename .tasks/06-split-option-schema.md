# 06. A finding can offer a split Option

## What to build

The Set schema's review block lets a finding name a second meaningful Option:
beside `fix`, an optional `split` names the Option that means *split this out
as a task* rather than fix it here. The shape, which is the decision:

```yaml
review:
  findings:
    - fix: Q1.1
      split: Q1.3
      what: |
        …what the fixing or task-working session is told…
```

Validated as `fix` is: a `split` has to name an Option the Set actually
offers, distinct from the finding's own `fix` Option, and no Option is shared
between findings. A finding without `split` is exactly today's finding.

Reading a Response against the block tells the three outcomes apart per
finding — accepted to fix here, accepted to split out, declined — so the
server can hold the fixed-here findings to the no-dropped-fixes rule while
expecting a backlog for the split ones.

## Acceptance criteria

- [ ] A Set whose finding offers a split Option validates, and a malformed one
      is refused naming the finding at fault.
- [ ] Reading a Response yields, per finding, fix-here, split, or declined.
- [ ] Schema tests cover the new field beside the existing review-block rules.
