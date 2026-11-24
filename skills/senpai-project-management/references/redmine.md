# Redmine adapter reference

The bundled Python 3 script uses only the Redmine REST API. It requires a base URL and an API-key environment-variable name; it reads the latter internally and sends it as `X-Redmine-API-Key`. It accepts `--timeout` (seconds) and `--max-output-bytes` (default 1 MiB). Output is JSON only. Errors are concise, credential-scrubbed JSON on stderr and exit nonzero.

When reading a ticket, the adapter downloads each attached file from the configured Redmine instance into a local temporary directory. Each attachment gains a `local_url` field; its remote URL, thumbnail URL, and filename are replaced with that local path in the ticket description and journal notes. This lets agents inspect images embedded in Redmine tickets. Downloads are authenticated, limited to 20 MiB per attachment, and failed downloads are retained in the response with an empty `local_url` and a `download_error`.

| Operation | Script subcommand | Redmine endpoint |
|---|---|---|
| `ticket.view` | `get-issue` | `GET /issues/:id.json?include=journals,relations,attachments` |
| `ticket.view` (search/list) | `list-issues` | `GET /issues.json` with pagination |
| `ticket.create` | `create-issue` | `POST /issues.json` |
| `ticket.edit` / `ticket.change_status` | `update-issue` | `PUT /issues/:id.json` |
| `ticket.comment` | `add-comment` | `PUT /issues/:id.json` with `notes` |
| `ticket.comment` (private note) | `add-comment --private` | `PUT /issues/:id.json` with `notes` and `private_notes: true`, then `GET /issues/:id.json?include=journals` to verify |
| `ticket.log_time` | `log-time` | `POST /time_entries.json` |
| `ticket.link` | `add-relation` | `POST /issues/:id/relations.json` |
| Status ids | `list-statuses` | `GET /issue_statuses.json` |

Use `--all` only where the result really needs every page. Otherwise choose a small `--limit`; pagination defaults to 25 and caps each page at 100. Inputs are JSON-encoded by the script, never constructed with a shell. A Redmine installation can expose custom fields and workflow restrictions; pass a `--custom-fields-json` object only when the project procedure identifies it.

Private notes: `add-comment --private`, or `update-issue --notes <text> --private` (`--private` requires a non-empty `--notes`), posts the note as private. A private note is still a `ticket.comment` and follows its policy. Redmine silently posts the note publicly when the API user lacks the "Set notes as private" permission, so after every private write the adapter re-reads the issue journals and checks that the latest note, by journal id and with the same text, is private. On success the output is `{"ok":true,"private_notes":true}`.

Both failures below happen after the write succeeded: the note is already posted, and for `update-issue` every other change (status, etc.) is applied too. Never retry the command, it would duplicate the note. Tell the user at once and let them check the ticket in Redmine.

- `note was posted but is not private`: the note is public. The user must fix it in Redmine and check their "Set notes as private" permission.
- `note was posted but its privacy could not be verified: ...`: the privacy state is unknown, because the verification read failed (HTTP error, timeout, response over `--max-output-bytes`) or the latest note is not the one just posted (for instance a concurrent comment).

`--project` is a Redmine project identifier, not a display name. For creation, `--tracker-id` and `--subject` are required. For time logging, `--hours` must be positive and `--activity-id` is required. A relation accepts `--relation-type` (for example `relates`, `blocks`, `duplicates`) and `--issue-to-id`.
