---
type: Metric
title: Team metric
description: A metric verified by a team actor after its last generation.
status: stable
generated: { by: "process:etl-pipeline", at: "2026-10-01T09:00:00Z" }
verified:
  - by: "team:finance-fpa"
    at: "2026-10-05T09:00:00Z"
stale_after: "2026-12-31T00:00:00Z"
sources:
  - id: team-runbook
    resource: https://example.com/runbooks/finance-fpa
    title: Finance FP&A runbook
---

# Definition

A team verified this concept after its last generation. The spec gives the machine-confirmed tier to a team verification, because a team name does not identify the person who did the check, and only a human actor gives human-reviewed. The honest tier is machine-confirmed and nothing about that is a defect.
