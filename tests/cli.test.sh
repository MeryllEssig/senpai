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
    "tickets": { "kind": "ticketing", "platform": "redmine", "url": "https://tracker.example", "scope": { "project_identifier": "demo" }, "provides": ["ticket.read", "ticket.comment"], "handles": ["ticket.read", "ticket.comment"], "routing": { "ticket_id_patterns": ["^#[0-9]+$"], "priority": 10 }, "workflow": { "skill": "ticket-flow", "policy": { "read": "allow", "comment": "confirm" } } },
    "backup": { "kind": "ticketing", "platform": "redmine", "url": "https://backup-tracker.example", "provides": ["ticket.read"], "handles": ["ticket.read"], "routing": { "ticket_id_patterns": ["^#[0-9]+$"], "priority": 20 } },
    "origin": { "kind": "forge", "platform": "gitlab", "url": "https://git.example", "provides": ["code.read", "code.create"], "handles": ["code.read", "code.create"], "workflow": { "policy": { "read": "allow", "create": "confirm" } } }
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
cd "$workspace"
result=$(cd "$workspace/app" && "$binary" summary --json)
printf '%s' "$result" | grep -q '"ok":true'
"$binary" resolve operation code.create --repo app --json | grep -q 'demo/app'
"$binary" resolve operation ticket.comment --ticket '#12' --json | grep -q '"decision":"confirm"'
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
{"version":1,"project":{"name":"legacy","label":"Legacy","context":"","stack":[]},"trackers":{"sources":{"redmine":{"type":"redmine","url":"https://legacy.example","project":"legacy","roles":["ticket_details"],"skill":"legacy-ticket-adapter"}}},"code_hosting":{"instances":{"git":{"platform":"gitlab","url":"https://git.legacy.example","roles":["merge_requests"]}}},"workflows":{"tickets":{"skill":"legacy-ticket-flow","policy":{"comment":"confirm"}}},"rules":[{"if":"a","then":"b"},{"if":"c","then":"d"}],"repos":{"app":{"path":".","hosting":{"git":"legacy/app"}}}}
EOF
mkdir -p "$workspace/migration"
mv "$migration_source" "$workspace/migration/.senpai.jsonc"
migration_result=$(cd "$workspace/migration" && "$binary" migrate v1 --json)
printf '%s' "$migration_result" | grep -q '"written":false'
printf '%s' "$migration_result" | grep -q '"\$schema":"https://raw.githubusercontent.com/MeryllEssig/senpai/main/schema/senpai.schema.json"'
printf '%s' "$migration_result" | grep -q 'legacy-ticket-flow'
printf '%s' "$migration_result" | grep -q 'legacy-ticket-adapter'
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
