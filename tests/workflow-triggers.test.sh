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

for macos_target in x86_64-apple-darwin aarch64-apple-darwin; do
  if grep -Fq -- "$macos_target" .github/workflows/release.yml; then
    printf 'macOS release artifacts must remain temporarily disabled: %s\n' "$macos_target" >&2
    exit 1
  fi
done

for linux_target in x86_64-unknown-linux-musl aarch64-unknown-linux-musl; do
  assert_contains .github/workflows/release.yml "          - os: ubuntu-latest"
  if ! grep -Fq -- "$linux_target" .github/workflows/release.yml; then
    printf 'Linux release artifact is missing: %s\n' "$linux_target" >&2
    exit 1
  fi
done

assert_contains .github/workflows/release.yml '        run: gh release create "${{ github.ref_name }}" dist/senpai-*.tar.gz dist/checksums.txt --repo "${{ github.repository }}" --generate-notes'
