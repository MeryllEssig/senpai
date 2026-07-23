#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
binary="$root/target/debug/senpai"
cargo build --quiet --manifest-path "$root/Cargo.toml"
"$binary" --version --json | grep -q '0.1.0'
workspace=$(mktemp -d)
trap 'rm -rf "$workspace"' EXIT

cat > "$workspace/.senpai.jsonc" <<'EOF'
{
  // JSONC is accepted.
  "version": 1,
  "project": { "name": "demo", "label": "Demo", "context": "Local test project", "stack": [] },
  "trackers": { "sources": { "tickets": { "type": "redmine", "url": "https://tracker.example", "project": "demo", "roles": ["ticket_details"], "ticket_id_patterns": ["^#[0-9]+$"] } } },
  "code_hosting": { "instances": { "origin": { "platform": "gitlab", "url": "https://git.example", "roles": ["merge_requests"] } } },
  "repos": { "root": { "path": ".", "hosting": { "origin": "demo/root" } }, "app": { "path": "app", "depends_on": ["root"], "hosting": { "origin": "demo/app" } } },
  "environments": { "local": { "label": "Local", "repo": "app" } },
  "capsules": {
    "echo": { "label": "Echo", "type": "test", "command": "printf '%s' {message}", "supplied": ["message"], "repo": "app", "environment": "local" },
    "private": { "label": "Private", "command": "printf 'token=%s' {token}" }
  }
}
EOF

mkdir -p "$workspace/app"
result=$(cd "$workspace/app" && "$binary" summary --json)
printf '%s' "$result" | grep -q '"ok":true'
"$binary" get hosting --role merge_requests --repo app --manifest "$workspace/.senpai.jsonc" --json | grep -q 'demo/app'
"$binary" get repo --path "$workspace/app/subdir" --with-dependencies --manifest "$workspace/.senpai.jsonc" --json | grep -q '"id":"app"'
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
echo 'CLI tests passed'
