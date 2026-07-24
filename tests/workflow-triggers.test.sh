#!/usr/bin/env bash
set -euo pipefail

assert_contains() {
  local file=$1
  local expected=$2
  if ! grep -Fqx -- "$expected" "$file"; then
    printf 'expected %s to contain: %s\n' "$file" "$expected" >&2
    exit 1
  fi
}

for workflow in .github/workflows/ci.yml .github/workflows/release.yml; do
  assert_contains "$workflow" "    tags: ['[0-9]+.[0-9]+.[0-9]+']"
done

if grep -Fqx '  pull_request:' .github/workflows/ci.yml; then
  printf 'CI must run only for numeric release tags\n' >&2
  exit 1
fi
