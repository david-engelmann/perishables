---
type: Metric
title: Revenue
description: Recognized revenue for a fiscal year.
status: stable
generated: { by: reference_agent/test, at: "2026-06-20T14:00:00Z" }
verified: { by: "human:ahormati", at: "2026-06-25T09:00:00Z" }
stale_after: "next spring"
sources:
  - id: rev-policy
    resource: https://example.com/revenue-policy
    title: Revenue recognition policy
---

# Definition

Recognized revenue. The stale_after value is prose, not a date. A reader could take the concept as fresh indefinitely, and the spec says an unparseable stale_after is never fresh.
