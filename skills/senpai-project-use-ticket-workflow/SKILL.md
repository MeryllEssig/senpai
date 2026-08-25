---
name: senpai-project-use-ticket-workflow
description: Default policy-respecting SenpAI ticket workflow. Use after SenpAI has resolved a ticketing integration and checked its effective local policy when the project declares no custom workflow skill.
---

# Default ticket workflow

Apply the requested operation's resolved local policy decision.

- `allow` — proceed only with the requested operation.
- `confirm` — obtain explicit confirmation for the concrete operation and target before proceeding.
- `deny` — do not perform the operation.

Use `senpai-project-management` after resolution to apply the selected adapter guidance. Do not broaden the requested action or bypass the selected integration, adapter, authentication, or safety checks.
