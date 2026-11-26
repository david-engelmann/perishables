# Build round 24. The format fix for PR 2

Date. 2026-11-25. Stacked on the orphans branch.

PR 2's hosted verdict was a format failure in fifteen seconds, with clippy, test, and build skipped, so the stack's compile question stays open until the next run. The defect was five formatting hunks, all in the adapter crate's root file, the one file this seat's local formatting check could never reach, because the per file method does not apply to a file that declares modules. The hunks were drift accumulated across the stack's slices in code no hosted run had ever formatted. They are fixed to the pinned toolchain's exact output.

The verification method changed with the lesson. A full tree scan now splits the root files' formatted output by module and diffs every module in the workspace, roots and test files included, and the whole tree passes it under the pinned toolchain. That scan is the local format gate from here on, and it is recorded here so the next slice uses it rather than the per file method that missed this.

Landing route. Stacked on cross-ecosystem-orphans-1 as branch cross-ecosystem-fmt-fix-1, carrying the orphans slice with it. The stack head for the reopened verification PR is this branch.
