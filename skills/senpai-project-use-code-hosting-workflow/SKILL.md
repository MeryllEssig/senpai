---
name: senpai-project-use-code-hosting-workflow
description: Default policy-respecting SenpAI forge workflow. Use after SenpAI has resolved a forge integration and checked its effective local policy when the project declares no custom workflow skill.
---

# Default code-hosting workflow

Apply the requested operation's resolved local policy decision.

- `allow` — proceed only with the requested operation.
- `confirm` — obtain explicit confirmation for the concrete operation and target before proceeding.
- `deny` — do not perform the operation.

Use `senpai-code-hosting` after resolution to apply the selected adapter guidance. Do not broaden the requested action or bypass the selected integration, adapter, authentication, or safety checks.
