#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
binary="$root/target/debug/senpai"
cargo build --quiet --manifest-path "$root/Cargo.toml"
"$binary" --version --json | grep -q '0.1.0'
workspace=$(mktemp -d)
child_pid=""
trap 'result_code=$?; if [[ -n $child_pid ]]; then kill "$child_pid" 2>/dev/null || true; fi; rm -rf "$workspace"; exit "$result_code"' EXIT

cat > "$workspace/.senpai.jsonc" <<'EOF'
{
  // JSONC is accepted.
  "version": 2,
  "project": { "name": "demo", "label": "Demo", "context": "Local test project", "stack": [] },
  "integrations": {
    "tickets": { "kind": "ticketing", "platform": "redmine", "url": "https://tracker.example", "scope": { "project_identifier": "demo" }, "provides": ["ticket.view", "ticket.create", "ticket.edit", "ticket.comment", "ticket.change_status", "ticket.link", "ticket.log_time"], "handles": ["ticket.view", "ticket.create", "ticket.edit", "ticket.comment", "ticket.change_status", "ticket.link", "ticket.log_time"], "routing": { "ticket_id_patterns": ["^#[0-9]+$"], "priority": 10 }, "workflow": { "skill": "ticket-flow", "policy": { "ticket.view": "allow", "ticket.create": "confirm", "ticket.edit": "confirm", "ticket.comment": "confirm", "ticket.change_status": "confirm", "ticket.link": "confirm", "ticket.log_time": "confirm" } } },
    "backup": { "kind": "ticketing", "platform": "redmine", "url": "https://backup-tracker.example", "provides": ["ticket.view"], "handles": ["ticket.view"], "routing": { "ticket_id_patterns": ["^#[0-9]+$"], "priority": 20 } },
    "origin": { "kind": "forge", "platform": "gitlab", "url": "https://git.example", "provides": ["pull_merge_request.view", "pull_merge_request.create", "pull_merge_request.edit", "pull_merge_request.comment", "pull_merge_request.request_review", "pull_merge_request.merge", "pipeline.view", "pipeline.job.view_log", "pipeline.trigger"], "handles": ["pull_merge_request.view", "pull_merge_request.create", "pull_merge_request.edit", "pull_merge_request.comment", "pull_merge_request.request_review", "pull_merge_request.merge", "pipeline.view", "pipeline.job.view_log", "pipeline.trigger"], "workflow": { "policy": { "pull_merge_request.view": "allow", "pull_merge_request.create": "confirm", "pull_merge_request.edit": "confirm", "pull_merge_request.comment": "confirm", "pull_merge_request.request_review": "confirm", "pull_merge_request.merge": "confirm", "pipeline.view": "allow", "pipeline.trigger": "confirm" } } }
  },
  "repos": { "root": { "path": ".", "labels": ["platform"], "integrations": { "origin": "demo/root" } }, "app": { "path": "app", "labels": ["backend", "critical"], "depends_on": ["root"], "integrations": { "origin": "demo/app" } } },
  "environments": { "local": { "label": "Local", "repo": "app" } },
  "capsules": {
    "echo": { "label": "Echo", "type": "test", "program": "printf", "args": ["%s", "{message}"], "supplied": ["message"], "repo": "app", "environment": "local" },
    "private": { "label": "Private", "program": "printf", "args": ["token=%s", "{token}"] },
    "limited": { "label": "Bounded", "type": "test", "program": "yes", "args": [], "max_output_bytes": 1 }
  }
}
EOF

mkdir -p "$workspace/app"
mkdir -p "$workspace/mockbin"
cat > "$workspace/mockbin/glab" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
case "$*" in
  'ci trace 456 --pipeline-id 123 --repo https://git.example/demo/app'|'ci trace 456 --repo https://git.example/demo/app') ;;
  *) exit 1 ;;
esac
for i in $(seq 1 11000); do printf 'line-%s\n' "$i"; done
EOF
chmod +x "$workspace/mockbin/glab"
cat > "$workspace/mockbin/gh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
[[ $* == 'run view 123 --log --job 456 --repo git.example/demo/app' ]]
printf 'github-log\n'
EOF
chmod +x "$workspace/mockbin/gh"
export PATH="$workspace/mockbin:$PATH"
cd "$workspace"
result=$(cd "$workspace/app" && "$binary" summary --json)
printf '%s' "$result" | grep -q '"ok":true'
mkdir -p "$workspace/local-only/app"
sed 's/"name": "demo"/"name": "local-only"/' "$workspace/.senpai.jsonc" > "$workspace/local-only/.senpai.local.jsonc"
local_only_result=$(cd "$workspace/local-only/app" && "$binary" summary --json)
printf '%s' "$local_only_result" | grep -q '"project":"local-only"'
printf '{"project":{"name":"overridden"}}\n' > "$workspace/.senpai.local.jsonc"
override_result=$("$binary" resolve --from "$workspace/app" --json)
printf '%s' "$override_result" | grep -q '"project":"overridden"'
"$binary" resolve operation ticket.view --ticket '#12' --json | grep -q '"decision":"allow"'
for operation in ticket.create ticket.edit ticket.comment ticket.change_status ticket.link ticket.log_time; do
  "$binary" resolve operation "$operation" --ticket '#12' --json | grep -q '"decision":"confirm"'
done
"$binary" resolve operation pull_merge_request.view --repo app --json | grep -q '"decision":"allow"'
for operation in pull_merge_request.create pull_merge_request.edit pull_merge_request.comment pull_merge_request.request_review pull_merge_request.merge; do
  "$binary" resolve operation "$operation" --repo app --json | grep -q '"decision":"confirm"'
done
"$binary" resolve operation pull_merge_request.create --repo app --json | grep -q 'demo/app'
"$binary" resolve operation pipeline.view --repo app --json | grep -q '"decision":"allow"'
"$binary" resolve operation pipeline.job.view_log --repo app --json | grep -q '"decision":"allow"'
"$binary" resolve operation pipeline.trigger --repo app --json | grep -q '"decision":"confirm"'
job_log=$($binary pipeline job-log --repo app --pipeline 123 --job 456 --json)
printf '%s' "$job_log" | grep -q '"truncated":true'
printf '%s' "$job_log" | grep -q 'line-11000'
if printf '%s' "$job_log" | grep -q 'line-1\\n'; then
  echo 'job log retained the beginning instead of the bounded tail' >&2; exit 1
fi
gitlab_url_log=$($binary pipeline job-log --repo app --job 'https://git.example/demo/app/-/jobs/456' --json)
printf '%s' "$gitlab_url_log" | grep -q '"job":"456"'
if "$binary" pipeline job-log --repo app --job 'https://git.example/other/app/-/jobs/456' --json >/dev/null; then
  echo 'GitLab job URL from another repository unexpectedly ran' >&2; exit 1
fi
mkdir -p "$workspace/github/app" "$workspace/confirm/app" "$workspace/deny/app"
sed 's/"platform": "gitlab"/"platform": "github"/' "$workspace/.senpai.jsonc" > "$workspace/github/.senpai.jsonc"
github_log=$(cd "$workspace/github/app" && "$binary" pipeline job-log --repo app --job 'https://git.example/demo/app/actions/runs/123/jobs/456' --json)
printf '%s' "$github_log" | grep -q 'github-log'
if (cd "$workspace/github/app" && "$binary" pipeline job-log --repo app --job 'https://git.example/other/app/actions/runs/123/jobs/456' --json) >/dev/null; then
  echo 'GitHub job URL from another repository unexpectedly ran' >&2; exit 1
fi
sed 's/"pipeline.view": "allow"/"pipeline.view": "allow", "pipeline.job.view_log": "confirm"/' "$workspace/.senpai.jsonc" > "$workspace/confirm/.senpai.jsonc"
if (cd "$workspace/confirm/app" && "$binary" pipeline job-log --repo app --pipeline 123 --job 456 --json) >/dev/null; then
  echo 'confirmed job-log operation unexpectedly ran without --confirm' >&2; exit 1
fi
sed 's/"pipeline.view": "allow"/"pipeline.view": "allow", "pipeline.job.view_log": "deny"/' "$workspace/.senpai.jsonc" > "$workspace/deny/.senpai.jsonc"
if (cd "$workspace/deny/app" && "$binary" pipeline job-log --repo app --pipeline 123 --job 456 --json) >/dev/null; then
  echo 'denied job-log operation unexpectedly ran' >&2; exit 1
fi
for operation in ticket.read ticket.update ticket.transition; do
  invalid_operation=$("$binary" resolve operation "$operation" --ticket '#12' --json 2>&1 || true)
  printf '%s' "$invalid_operation" | grep -q '"code":"invalid_arguments"'
done
for operation in code.read code.create code.update code.comment code.request_review code.merge code.pipeline_read code.pipeline_trigger; do
  invalid_operation=$("$binary" resolve operation "$operation" --repo app --json 2>&1 || true)
  printf '%s' "$invalid_operation" | grep -q '"code":"invalid_arguments"'
done
invalid_operation=$("$binary" resolve operation pipeline.destroy --repo app --json 2>&1 || true)
printf '%s' "$invalid_operation" | grep -q '"code":"invalid_arguments"'
"$binary" get repo --path "$workspace/app/subdir" --with-dependencies --json | grep -q '"id":"app"'
"$binary" get repo --id app --json | grep -q '"id":"app"'
"$binary" list repos --json | grep -q 'backend'
"$binary" summary --json | grep -q 'backend'
if "$binary" summary --manifest "$workspace/.senpai.jsonc" --json >/dev/null; then
  echo 'removed manifest option unexpectedly passed' >&2; exit 1
fi
if "$binary" get tracker --json >/dev/null; then
  echo 'obsolete command unexpectedly passed' >&2; exit 1
fi
echo_result=$("$binary" run echo --message hello --json)
printf '%s' "$echo_result" | grep -q 'hello'
"$binary" init --json | grep -q 'private'
if "$binary" validate local --json >/dev/null; then
  echo 'stubbed local configuration unexpectedly validated' >&2; exit 1
fi
mkdir -p "$workspace/.senpai"
printf '{"private":{"token":"local-secret"}}\n' > "$workspace/.senpai/capsules.local.json"
private_result=$("$binary" run private --json)
printf '%s' "$private_result" | grep -q '{redacted}'
if "$binary" run echo --message hello --message twice --json >/dev/null; then
  echo 'repeated supplied argument unexpectedly passed' >&2; exit 1
fi
if limited=$("$binary" run limited --json 2>&1); then
  echo 'unbounded capsule unexpectedly passed' >&2; exit 1
fi
printf '%s' "$limited" | grep -q 'output limit'
printf '%s' "$limited" | node -e 'let input=""; process.stdin.on("data", chunk => input += chunk); process.stdin.on("end", () => { const output = JSON.parse(input).error.details[0]; if (Buffer.byteLength(output.stdout) + Buffer.byteLength(output.stderr) > 1) process.exit(1); })'
shell_manifest="$workspace/shell.jsonc"
sed 's/"program": "yes", "args": \[\]/"program": "sh", "args": ["-c", "echo shell-executed"]/' "$workspace/.senpai.jsonc" > "$shell_manifest"
mkdir -p "$workspace/shell"
mv "$shell_manifest" "$workspace/shell/.senpai.jsonc"
if (cd "$workspace/shell" && "$binary" validate manifest --json) >/dev/null; then
  echo 'shell interpreter unexpectedly validated' >&2; exit 1
fi
legacy_manifest="$workspace/legacy.jsonc"
sed 's/"program": "yes", "args": \[\]/"command": "yes"/' "$workspace/.senpai.jsonc" > "$legacy_manifest"
mkdir -p "$workspace/legacy"
mv "$legacy_manifest" "$workspace/legacy/.senpai.jsonc"
if (cd "$workspace/legacy" && "$binary" validate manifest --json) >/dev/null; then
  echo 'legacy command capsule unexpectedly validated' >&2; exit 1
fi
v1_manifest="$workspace/v1.jsonc"
sed 's/"version": 2/"version": 1/' "$workspace/.senpai.jsonc" > "$v1_manifest"
mkdir -p "$workspace/v1"
mv "$v1_manifest" "$workspace/v1/.senpai.jsonc"
if (cd "$workspace/v1" && "$binary" validate manifest --json) >/dev/null; then
  echo 'v1 manifest unexpectedly validated' >&2; exit 1
fi
migration_source="$workspace/migration-v1.jsonc"
cat > "$migration_source" <<'EOF'
{"version":1,"project":{"name":"legacy","label":"Legacy","context":"","stack":[]},"trackers":{"sources":{"redmine":{"type":"redmine","url":"https://legacy.example","project":"legacy","roles":["ticket_details"],"skill":"legacy-ticket-adapter"}}},"code_hosting":{"instances":{"git":{"platform":"gitlab","url":"https://git.legacy.example","roles":["merge_requests"]}}},"workflows":{"tickets":{"skill":"legacy-ticket-flow","policy":{"read":"confirm","create":"confirm","update":"confirm","comment":"confirm","transition":"confirm","link":"confirm","log_time":"confirm"}},"code_changes":{"skill":"legacy-code-flow","policy":{"read":"confirm","create":"confirm","update":"confirm","comment":"confirm","request_review":"confirm","merge":"confirm","pipeline_read":"confirm","pipeline_trigger":"confirm"}}},"rules":[{"if":"a","then":"b"},{"if":"c","then":"d"}],"repos":{"app":{"path":".","hosting":{"git":"legacy/app"}}}}
EOF
mkdir -p "$workspace/migration"
mv "$migration_source" "$workspace/migration/.senpai.jsonc"
migration_result=$(cd "$workspace/migration" && "$binary" migrate v1 --json)
printf '%s' "$migration_result" | grep -q '"written":false'
printf '%s' "$migration_result" | grep -q '"\$schema":"https://raw.githubusercontent.com/MeryllEssig/senpai/main/schema/senpai.schema.json"'
printf '%s' "$migration_result" | grep -q 'legacy-ticket-flow'
printf '%s' "$migration_result" | grep -q 'legacy-ticket-adapter'
for operation in ticket.view ticket.create ticket.edit ticket.comment ticket.change_status ticket.link ticket.log_time pull_merge_request.view pull_merge_request.create pull_merge_request.edit pull_merge_request.comment pull_merge_request.request_review pull_merge_request.merge pipeline.view pipeline.trigger; do
  printf '%s' "$migration_result" | grep -q "\"$operation\":\"confirm\""
done
[[ $(printf '%s' "$migration_result" | grep -o '"code":"review_rule"' | wc -l) -eq 2 ]]
cat > "$workspace/child-spawner.rs" <<'EOF'
use std::{io::{self, Write}, process::Command, thread, time::Duration};

fn main() {
    let child = Command::new("sleep").arg("30").spawn().unwrap();
    println!("{}", child.id());
    io::stdout().flush().unwrap();
    thread::sleep(Duration::from_secs(30));
}
EOF
rustc "$workspace/child-spawner.rs" -o "$workspace/child-spawner"
process_manifest="$workspace/process.jsonc"
sed "s#\"program\": \"yes\", \"args\": \[\], \"max_output_bytes\": 1#\"program\": \"$workspace/child-spawner\", \"args\": [], \"timeout_seconds\": 1#" "$workspace/.senpai.jsonc" > "$process_manifest"
mkdir -p "$workspace/process"
mv "$process_manifest" "$workspace/process/.senpai.jsonc"
process_result=$(cd "$workspace/process" && "$binary" run limited --json 2>&1 || true)
printf '%s' "$process_result" | grep -q 'timed out'
child_pid=$(printf '%s' "$process_result" | node -e 'let input=""; process.stdin.on("data", chunk => input += chunk); process.stdin.on("end", () => process.stdout.write(JSON.parse(input).error.details[0].stdout.trim()))')
[[ $child_pid =~ ^[0-9]+$ ]] || { echo 'child pid was not captured' >&2; exit 1; }
if kill -0 "$child_pid" 2>/dev/null; then
  echo 'capsule child process survived timeout' >&2; exit 1
fi
child_pid=""
echo 'CLI tests passed'
