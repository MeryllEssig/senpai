---
name: senpai-conversation-qa
description: Maintainer-only local QA for SenpAI. Analyze a supplied agent conversation transcript for project-context inefficiencies, classify each root cause, and propose concrete remediation without contacting external services.
---

# Conversation QA (maintainers only)

This skill is not part of the distributed `skills/` bundle. Keep supplied
transcripts local. First identify their ecosystem and turn/message format.

Walk the transcript chronologically. Flag every occurrence of failed or retried
commands; improvised workarounds for missing or incorrect declared context;
questions a manifest should answer; full-manifest dumps in place of scoped
queries; wrong tracker/hosting/environment targets; authentication dead-ends;
and ignored rules or guardrails. Include a short quoted-or-paraphrased evidence
pointer sufficient to locate the moment.

For each finding classify one root cause:

- **Project-side**: cite the exact missing/wrong manifest field or project
  instruction and propose a concrete manifest change.
- **SenpAI-side**: cite the affected schema, CLI behavior, shipped skill, or
  adapter guidance and propose a product change.
- **External**: state the service/tool limitation and propose a guidance note
  or human process where appropriate.

Deliver a findings table with: sequence, evidence, impact, root cause,
confidence, and remediation. Separate confirmed findings from hypotheses. Do
not apply changes automatically; finish with prioritized candidate fixes.
