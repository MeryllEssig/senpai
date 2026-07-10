# AI Manager - Open Questions and Notes

This file tracks design topics that are still open, plus decisions with no natural home in the other documents.

Policy: when a question is resolved and its decision lands in another documentation file (goal, user stories, technical considerations, reference manifest), the note is **removed** from this list rather than kept as a RESOLVED entry. The decision then lives where it is enforced. Entries that remain here are either genuinely unresolved or project-level decisions the product docs do not cover.

The 2026-07-10 design pass resolved the original open questions (manifest naming, galaxy composition, tracker routing by function, bare-ticket resolution, local overlay, environments taxonomy, CLI language/output/query interface, auto-trigger fallback, skill distribution, automation-discovery scope, trust model, write guardrails, team onboarding, manifest-evolution feedback, QA transcript handling). Their decisions now live in [technical-considerations.md](technical-considerations.md), [user-stories.md](user-stories.md), and [reference-manifest.jsonc](reference-manifest.jsonc). What follows is what remains.

## Project process

- **Success criteria / v1 cut.** Decided (2026-07-10): no version-based prioritization. Tasks are separated but executed in their natural dependency order, so that nothing built early has to be undone later. Kept here because it is a build-process decision, not a product fact the other docs describe.
