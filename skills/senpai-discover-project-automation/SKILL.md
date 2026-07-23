---
name: senpai-discover-project-automation
description: Review a project and its SenpAI context to identify safe, useful automation opportunities such as missing bounded capsules, rules, documentation, and tool setup, then produce proposals without applying them automatically.
---

# Discover automation opportunities

Respond in the user's language. Inspect the folder and resolve the manifest;
use `summary` and scoped queries, plus existing scripts/CI/docs, to find work
that is repetitive, error-prone, or relies on tribal knowledge. Do not execute
undeclared project operations and do not inspect secret-value files.

Return two explicitly separate proposal groups:

1. **Manifest improvements**: capsules (finite, argv-safe operations only),
   rules, docs, environments, source/hosting metadata, workflows, or repo
   declarations. For each: problem, proposed declaration, benefit, and any
   required user-supplied variable names.
2. **Ecosystem automation**: hooks, scheduled jobs, CI changes, or external
   tool setup. Mark these as outside SenpAI's declarative core and never apply
   them automatically.

Rank proposals by value and risk. Ask the user to accept individual items
before editing a manifest or initiating any setup. Explain operational caveats,
including required shell/agent restarts after environment changes.
