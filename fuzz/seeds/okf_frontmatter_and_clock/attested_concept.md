---
type: Attested Computation
title: Quarterly Net Retention
status: stable
generated: { by: "process:pipeline", at: "2026-10-01T09:00:00Z" }
verified:
  - by: "human:reviewer@example.com"
    at: "2026-10-05T14:30:00Z"
stale_after: "2026-12-31T00:00:00Z"
sources:
  - id: cohort-extract
    resource: "/sources/cohorts.csv"
runtime: python3
parameters:
  - name: expansion
    type: number
    value: "18.5"
---

# Definition

Net retention cites the cohort extract [^cohort-extract] and skips `[^inline]` code.

```python title="compute"
result = 100.0 + expansion
```

## Claimed result

The attested result claimed for this computation is 118.5.
