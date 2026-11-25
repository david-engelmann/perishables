---
type: Attested Computation
title: Warehouse row count
description: Row count claimed over an external warehouse runtime.
status: stable
runtime: bigquery
generated: { by: reference_agent/test, at: "2026-06-20T14:00:00Z" }
verified: { by: "human:ahormati", at: "2026-06-25T09:00:00Z" }
stale_after: "2026-12-31T00:00:00Z"
sources:
  - id: warehouse-query
    resource: https://example.com/warehouse-query
    title: Warehouse query definition
---

# Computation

```sql
SELECT COUNT(*) FROM analytics.events
```

# Claimed result

The attested result claimed for this computation is 1000.
