---
type: Metric
title: Revenue
description: Recognized revenue for a fiscal year.
status: stable
generated: { by: reference_agent/test, at: "2026-06-20T14:00:00Z" }
verified: { by: "human:ahormati", at: "2026-06-25T09:00:00Z" }
stale_after: "2026-12-31T00:00:00Z"
sources:
  - id: raw-extract
    resource: /sources/revenue-extract.csv
    title: Revenue extract, raw rows
    sha256: 9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08
---

# Definition

Recognized revenue, computed from the raw extract[^raw-extract]. The recorded hash was taken when the extract held its original rows. The extract in this bundle has since been edited, so its bytes no longer hash to the recorded value.

[^raw-extract]: [Revenue extract, raw rows](/sources/revenue-extract.csv)
