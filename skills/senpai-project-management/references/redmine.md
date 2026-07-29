# Redmine adapter reference

The bundled Python 3 script uses only the Redmine REST API. It requires a base
URL and an API-key environment-variable name; it reads the latter internally
and sends it as `X-Redmine-API-Key`. It accepts `--timeout` (seconds) and
`--max-output-bytes` (default 1 MiB). Output is JSON only. Errors are concise,
credential-scrubbed JSON on stderr and exit nonzero.

When reading a ticket, the adapter downloads each attached file from the configured Redmine instance into a local temporary directory. Each attachment gains a `local_url` field; its remote URL, thumbnail URL, and filename are replaced with that local path in the ticket description and journal notes. This lets agents inspect images embedded in Redmine tickets. Downloads are authenticated, limited to 20 MiB per attachment, and failed downloads are retained in the response with an empty `local_url` and a `download_error`.

| Operation | Script subcommand | Redmine endpoint |
|---|---|---|
| Read ticket | `get-issue` | `GET /issues/:id.json?include=journals,relations,attachments` |
| Search/list | `list-issues` | `GET /issues.json` with pagination |
| Create | `create-issue` | `POST /issues.json` |
| Update/transition | `update-issue` | `PUT /issues/:id.json` |
| Comment | `add-comment` | `PUT /issues/:id.json` with `notes` |
| Time | `log-time` | `POST /time_entries.json` |
| Link | `add-relation` | `POST /issues/:id/relations.json` |
| Status ids | `list-statuses` | `GET /issue_statuses.json` |

Use `--all` only where the result really needs every page. Otherwise choose a
small `--limit`; pagination defaults to 25 and caps each page at 100. Inputs
are JSON-encoded by the script, never constructed with a shell. A Redmine
installation can expose custom fields and workflow restrictions; pass a
`--custom-fields-json` object only when the project procedure identifies it.

`--project` is a Redmine project identifier, not a display name. For creation,
`--tracker-id` and `--subject` are required. For time logging, `--hours` must
be positive and `--activity-id` is required. A relation accepts `--relation-type`
(for example `relates`, `blocks`, `duplicates`) and `--issue-to-id`.
