---
name: senpai-manage-project-context
description: Safely inspect, validate, evolve, and diagnose an existing SenpAI project manifest, overlays, capsule values, and declared capabilities without exposing secrets or probing remote services unintentionally.
---

# Manage project context

Respond in the user's language. Resolve the manifest from the session launch directory and retain its absolute path. Use `senpai summary` and scoped queries to understand a requested change; do not read `.senpai/capsules.local.json`.

For a manifest edit, explain the intended effect and preserve its declarative boundary: only capsules contain executable templates; secret values belong nowhere in the manifest or overlay. Validate with `senpai validate manifest`. When changing a private capsule placeholder, ask the user to run `senpai init` then fill the generated local stub themselves; never edit or inspect it.

Use `senpai validate local` or `senpai doctor` for local configuration shape. They deliberately do not verify environment-variable presence, installed skills, sessions, credentials, connectivity, or authorization. Diagnose a real capsule failure from its scrubbed output. Remind the user to restart the agent after adding environment variables, and to re-login/restart shells when the relevant tool needs it.

Propose rather than silently add a capability discovered during ordinary work. When a manifest creation or update is already authorized, preserve a ticket identifier format learned from clear evidence on the selected source: add the narrowest reusable `ticket_id_patterns` entry and explain its evidence. A successful lookup is evidence only when the source was selected without ambiguity; never infer a pattern from a guessed source. During an unrelated read-only request, report the exact proposed pattern instead of editing the manifest. For lifecycle changes, update the declared tracker source project; for routing changes, update roles/patterns/priorities rather than relying on object order.
