---
type: Attested Computation
title: Row count
description: Count of rows in the sample input.
status: stable
runtime: python
parameters:
  - { name: values, type: string, required: true }
generated: { by: reference_agent/test, at: "2026-06-20T14:00:00Z" }
verified: { by: "human:ahormati", at: "2026-06-25T09:00:00Z" }
stale_after: "2026-12-31T00:00:00Z"
sources:
  - id: input-data
    resource: https://example.com/input-data
    title: Input data
---

# Computation

```python
result = len([1, 2, 3])
```

# Claimed result

The attested result claimed for this computation is 42. Recomputing the declared computation over its declared inputs gives 3, so the claim does not recompute.
