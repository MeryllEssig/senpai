---
name: senpai-setup-project-context
description: Set up Senpai project context in a new or existing folder by analyzing it first, interviewing the user, creating and validating a commented .senpai.jsonc manifest, and offering safe tool-installation guidance.
---

# Set up project context

Respond in the user's language and explain why each step matters. First inspect
the current folder without modifying it: establish whether it is a single Git
repository, a multi-repository galaxy, or a plain folder; look for CI files,
service definitions, documentation, existing manifests, and bounded scripts.
Summarize the evidence before interviewing.

Ask focused, progressively deeper questions about project identity, trackers
and their roles, code-hosting instances and roles, repositories/dependencies,
environments, documentation, workflows/policies, and bounded operations
(tests, builds, setup, logs, data queries, exports, deployments). Let the user
describe unusual arrangements rather than forcing a preset.

Create a commented `.senpai.jsonc` only after the user accepts the design. Use
the repository's reference manifest and schema as the field-shape authority.
Every executable declaration must be a bounded, non-interactive capsule; never
put shell pipelines, interactive commands, or credentials in the manifest.
Use literal shared coordinates and placeholders only for private or
machine-local values. Offer `senpai validate manifest` after creation.

Never read, request, or echo secret values. Ask only for an auth mode and an
environment-variable *name*. Explain that values configured after the agent
starts require an agent restart. For an existing manifest, use `senpai init`
and `senpai validate local` or `senpai doctor`; those validate configuration,
not credentials or remote access.

With consent, offer help installing/configuring `gh`, `glab`, or a Jira client
when declared. Redmine uses the bundled Python adapter and needs no separate
Redmine CLI. State clearly what was created, what still needs user action, and
which checks were actually run.
