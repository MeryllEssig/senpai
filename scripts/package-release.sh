#!/usr/bin/env bash
# Build a portable release archive from a previously compiled SenpAI binary.
set -euo pipefail
IFS=$'\n\t'

usage() {
  cat <<'EOF'
Usage: scripts/package-release.sh --target TARGET --binary PATH --skills-dir PATH --output PATH

Create senpai-TARGET.tar.gz containing a senpai binary and the shipped
senpai-* skills. The archive layout is the one verified by installer.sh.
EOF
}

fail() { printf 'package-release: %s\n' "$*" >&2; exit 1; }

target=""
binary=""
skills_dir=""
output=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --target|--binary|--skills-dir|--output)
      [[ $# -ge 2 && -n $2 && $2 != --* ]] || fail "$1 requires a value"
      case "$1" in
        --target) target=$2 ;;
        --binary) binary=$2 ;;
        --skills-dir) skills_dir=$2 ;;
        --output) output=$2 ;;
      esac
      shift 2
      ;;
    -h|--help) usage; exit 0 ;;
    *) fail "unknown option: $1" ;;
  esac
done

[[ -n $target && -n $binary && -n $skills_dir && -n $output ]] || fail "all options are required"
[[ $target =~ ^[A-Za-z0-9._-]+$ ]] || fail "invalid target: $target"
[[ -f $binary && -x $binary ]] || fail "binary is not an executable file: $binary"
[[ -d $skills_dir ]] || fail "skills directory does not exist: $skills_dir"

staging=$(mktemp -d "${TMPDIR:-/tmp}/senpai-release.XXXXXX") || fail "cannot create temporary directory"
trap 'rm -rf -- "$staging"' EXIT
mkdir -p -- "$staging/skills" "$(dirname -- "$output")"
install -m 755 -- "$binary" "$staging/senpai"
found=false
while IFS= read -r -d '' skill; do
  cp -R -- "$skill" "$staging/skills/$(basename -- "$skill")"
  found=true
done < <(find "$skills_dir" -mindepth 1 -maxdepth 1 -type d -name 'senpai-*' -print0)
$found || fail "no senpai-* skill directories found in $skills_dir"
tar -C "$staging" -czf "$output" senpai skills
