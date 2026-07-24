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
  "version": 1,
  "project": { "name": "demo", "label": "Demo", "context": "Local test project", "stack": [] },
  "trackers": { "sources": {
    "tickets": { "type": "redmine", "url": "https://tracker.example", "project": "demo", "roles": ["ticket_details"], "ticket_id_patterns": ["^#[0-9]+$"] },
    "backup": { "type": "redmine", "url": "https://backup-tracker.example", "project": "demo", "roles": ["ticket_details"] }
  } },
  "code_hosting": { "instances": { "origin": { "platform": "gitlab", "url": "https://git.example", "roles": ["merge_requests"] } } },
  "repos": { "root": { "path": ".", "hosting": { "origin": "demo/root" } }, "app": { "path": "app", "depends_on": ["root"], "hosting": { "origin": "demo/app" } } },
  "environments": { "local": { "label": "Local", "repo": "app" } },
  "capsules": {
    "echo": { "label": "Echo", "type": "test", "program": "printf", "args": ["%s", "{message}"], "supplied": ["message"], "repo": "app", "environment": "local" },
    "private": { "label": "Private", "program": "printf", "args": ["token=%s", "{token}"] },
    "limited": { "label": "Bounded", "type": "test", "program": "yes", "args": [], "max_output_bytes": 1 }
  }
}
EOF

mkdir -p "$workspace/app"
result=$(cd "$workspace/app" && "$binary" summary --json)
printf '%s' "$result" | grep -q '"ok":true'
"$binary" get hosting --role merge_requests --repo app --manifest "$workspace/.senpai.jsonc" --json | grep -q 'demo/app'
"$binary" get repo --path "$workspace/app/subdir" --with-dependencies --manifest "$workspace/.senpai.jsonc" --json | grep -q '"id":"app"'
if ambiguous=$("$binary" get tracker --role ticket_details --manifest "$workspace/.senpai.jsonc" --json 2>&1); then
  echo 'ambiguous tracker unexpectedly resolved' >&2; exit 1
fi
printf '%s' "$ambiguous" | grep -q 'ambiguous'
echo_result=$("$binary" run echo --message hello --manifest "$workspace/.senpai.jsonc" --json)
printf '%s' "$echo_result" | grep -q 'hello'
"$binary" init --manifest "$workspace/.senpai.jsonc" --json | grep -q 'private'
if "$binary" validate local --manifest "$workspace/.senpai.jsonc" --json >/dev/null; then
  echo 'stubbed local configuration unexpectedly validated' >&2; exit 1
fi
mkdir -p "$workspace/.senpai"
printf '{"private":{"token":"local-secret"}}\n' > "$workspace/.senpai/capsules.local.json"
private_result=$("$binary" run private --manifest "$workspace/.senpai.jsonc" --json)
printf '%s' "$private_result" | grep -q '{redacted}'
if "$binary" run echo --message hello --message twice --manifest "$workspace/.senpai.jsonc" --json >/dev/null; then
  echo 'repeated supplied argument unexpectedly passed' >&2; exit 1
fi
if limited=$("$binary" run limited --manifest "$workspace/.senpai.jsonc" --json 2>&1); then
  echo 'unbounded capsule unexpectedly passed' >&2; exit 1
fi
printf '%s' "$limited" | grep -q 'output limit'
printf '%s' "$limited" | node -e 'let input=""; process.stdin.on("data", chunk => input += chunk); process.stdin.on("end", () => { const output = JSON.parse(input).error.details[0]; if (Buffer.byteLength(output.stdout) + Buffer.byteLength(output.stderr) > 1) process.exit(1); })'
shell_manifest="$workspace/shell.jsonc"
sed 's/"program": "yes", "args": \[\]/"program": "sh", "args": ["-c", "echo shell-executed"]/' "$workspace/.senpai.jsonc" > "$shell_manifest"
if "$binary" validate manifest --manifest "$shell_manifest" --json >/dev/null; then
  echo 'shell interpreter unexpectedly validated' >&2; exit 1
fi
legacy_manifest="$workspace/legacy.jsonc"
sed 's/"program": "yes", "args": \[\]/"command": "yes"/' "$workspace/.senpai.jsonc" > "$legacy_manifest"
if "$binary" validate manifest --manifest "$legacy_manifest" --json >/dev/null; then
  echo 'legacy command capsule unexpectedly validated' >&2; exit 1
fi
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
process_result=$("$binary" run limited --manifest "$process_manifest" --json 2>&1 || true)
printf '%s' "$process_result" | grep -q 'timed out'
child_pid=$(printf '%s' "$process_result" | node -e 'let input=""; process.stdin.on("data", chunk => input += chunk); process.stdin.on("end", () => process.stdout.write(JSON.parse(input).error.details[0].stdout.trim()))')
[[ $child_pid =~ ^[0-9]+$ ]] || { echo 'child pid was not captured' >&2; exit 1; }
if kill -0 "$child_pid" 2>/dev/null; then
  echo 'capsule child process survived timeout' >&2; exit 1
fi
child_pid=""
echo 'CLI tests passed'
