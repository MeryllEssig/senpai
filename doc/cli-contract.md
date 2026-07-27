# SenpAI CLI contract — manifest v2

All manifest reads discover `.senpai.jsonc` or `.senpai.local.jsonc` by walking upward from the current directory and never contact a remote service. In a directory containing both, `.senpai.jsonc` is selected and the local file is deep-merged as its override; a standalone local file must therefore be a complete v2 manifest. The `--manifest` option is not supported. JSON output is `{ "ok": true, "data": …, "warnings": [] }`; errors are `{ "ok": false, "error": { "code": "…", "message": "…", "details": [] } }`. No result, error, or adapter handoff contains a secret value.

## Integration resolution

```text
senpai resolve operation <operation> [--ticket <id> | --repo <repo-id>]
  [--integration <id>] [--json]
```

The only operations in v2 are:

- `ticket.read`, `ticket.create`, `ticket.update`, `ticket.comment`, `ticket.transition`, `ticket.link`, `ticket.log_time`
- `code.read`, `code.create`, `code.update`, `code.comment`, `code.request_review`, `code.merge`, `code.pipeline_read`, `code.pipeline_trigger`

Ticket operations require `--ticket`; code operations require `--repo`. Resolution selects one integration that has the matching `kind`, handles the operation, and—for code operations—is mapped on the repository. A ticket routing pattern narrows candidates; lower local `routing.priority` wins. Unresolved ties exit 5 with candidate ids. `--integration` is only a disambiguator: it must satisfy all the same conditions.

The result includes the integration's public coordinates and auth metadata, the normalized route, the complete local policy plus the requested decision, the workflow skill, and the effective adapter. Adapter precedence is the integration adapter, then `adapter_overrides[kind][platform]`, then the shipped common adapter. It does not load a skill or adapter.

`summary` lists integrations and their handled operations, plus repository ids and labels. `get repo` and `list repos` return each repository's labels. The V1 commands `get tracker`, `get ticket-route`, `get hosting`, `get workflow`, and `get rules` are removed.

## Other context and capsules

`resolve [--from <directory>]`, `summary`, `get repo`, `get environment`, `get capsule`, `get docs`, `list repos`, `list capsules`, `init`, `validate`, `doctor`, and `run` retain their v1-safe behavior. Capsules remain the sole execution primitive: literal argv only, no shell, bounded output, timeout, and secret scrubbing.

## Migration

```text
senpai migrate v1 [--json]
```

Migration never writes a file. It emits a draft v2 manifest and a review report. It flags every role-to-operation mapping, non-native ticket routing, time-log fallback, mirrored repository mapping, global workflow split, and free-text rule. The draft grants only read operations by default and never copies a secret value; a human must review and write the result explicitly.
