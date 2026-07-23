---
name: senpai-setup-project-context
description: Set up SenpAI project context in a new or existing folder by analyzing it first, interviewing the user, creating and validating a commented .senpai.jsonc manifest, and offering safe tool-installation guidance.
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

Present the first interview round as a compact, evidence-led inventory, not a
platform-by-platform checklist. Separate facts observed in the folder from the
few decisions still needed. Group questions by integration and ask only for
fields that cannot be inferred: target URL/project, operational roles, and
authentication mode. For authentication, ask for the mode and, only for
`env`, the variable name; never ask for a value. Present one small workflow
policy matrix after the integrations are identified, with a conservative
recommended default, instead of asking permissions separately for each tool.

Do not ask for `ticket_id_patterns` as setup data. Infer a candidate only from
clear evidence: an observed ticket reference, a documented native identifier
format, or a source whose platform and identifier format are both unambiguous.
For example, an observed bare Redmine issue number supports
`^[1-9][0-9]*$`. Include inferred patterns and their evidence in the proposed
manifest design; the user's acceptance of that design authorizes writing them.
If no evidence exists, omit the optional field and say that the first safely
resolved ticket format can be added during a later, authorized manifest
update. Never fabricate a pattern merely to make the manifest look complete.

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
