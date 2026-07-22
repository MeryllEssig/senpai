# AI Manager - Open Questions and Notes

This file tracks design topics that are still open, plus decisions with no natural home in the other documents.

Policy: when a question is resolved and its decision lands in another documentation file (goal, user stories, technical considerations, reference manifest), the note is **removed** from this list rather than kept as a RESOLVED entry. The decision then lives where it is enforced. Entries that remain here are either genuinely unresolved or project-level decisions the product docs do not cover.

The 2026-07-10 to 2026-07-20 design passes resolved manifest naming, resolution, tracker and hosting routing, overlays, skills, capsule security, confirmations, diagnostics, and distribution. A real-project review on 2026-07-22 then collapsed project execution into one abstraction before implementation: every bounded non-interactive project command AI Manager runs is a capsule, including commands requiring no local values, with structured `cwd` and repo/component/environment metadata. All current product decisions live in [technical-considerations.md](technical-considerations.md), [user-stories.md](user-stories.md), and [reference-manifest.jsonc](reference-manifest.jsonc). Only project-process notes remain below.

## Project process

- **Success criteria / v1 cut.** Decided (2026-07-10): no version-based prioritization. Tasks are separated but executed in their natural dependency order, so that nothing built early has to be undone later. Kept here because it is a build-process decision, not a product fact the other docs describe.
- **CLI command surface grows incrementally, trackers first.** Decided (2026-07-13): the CLI's subcommand grammar, output shape, and `query -> expected output` test corpus are **not** frozen up front. They emerge from the first vertical slice - **trackers** (`resolve` + `query` a tracker by role + bare-`#id` resolution + configuration-only `doctor`), chosen as the richest case (role routing and ambiguity resolution) - then generalize in the natural dependency order. This mirrors how the manifest format was frozen by a golden example, but built one slice at a time so nothing early has to be undone. Kept here because it is a build-sequencing decision, not a product fact.
