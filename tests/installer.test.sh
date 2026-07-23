#!/usr/bin/env bash
set -euo pipefail

root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)
work=$(mktemp -d)
trap 'rm -rf -- "$work"' EXIT

fail() { printf 'FAIL: %s\n' "$*" >&2; exit 1; }
assert_file() { [[ -f $1 ]] || fail "expected file: $1"; }
assert_missing() { [[ ! -e $1 && ! -L $1 ]] || fail "expected missing: $1"; }
assert_contains() { grep -Fqx -- "$2" "$1" || fail "expected $2 in $1"; }

mkdir -p "$work/source/skills/senpai-usage" "$work/source/skills/senpai-project-management/scripts"
printf '#!/usr/bin/env bash\nprintf senpai\n' > "$work/source/senpai"
chmod +x "$work/source/senpai"
printf 'first version\n' > "$work/source/skills/senpai-usage/SKILL.md"
printf 'script\n' > "$work/source/skills/senpai-project-management/scripts/example.py"

home="$work/home"
prefix="$work/prefix"
state="$work/state"
HOME="$home" "$root/installer.sh" --yes --binary "$work/source/senpai" --skills-dir "$work/source/skills" --agents codex,opencode --prefix "$prefix" --state-dir "$state"

assert_file "$prefix/bin/senpai"
assert_file "$home/.codex/skills/senpai-usage/SKILL.md"
assert_file "$home/.config/opencode/skills/senpai-project-management/scripts/example.py"
assert_contains "$state/ownership.tsv" $'binary\t'"$prefix/bin/senpai"
assert_contains "$state/ownership.tsv" $'skill\t'"$home/.codex/skills/senpai-usage"

# A rerun deliberately replaces shipped skill contents and refreshes ownership.
printf 'replacement\n' > "$work/source/skills/senpai-usage/SKILL.md"
HOME="$home" "$root/installer.sh" --yes --binary "$work/source/senpai" --skills-dir "$work/source/skills" --agents codex --prefix "$prefix" --state-dir "$state"
[[ $(<"$home/.codex/skills/senpai-usage/SKILL.md") == replacement ]] || fail "skill was not replaced"
assert_file "$home/.config/opencode/skills/senpai-usage/SKILL.md"
assert_file "$home/.config/opencode/skills/senpai-project-management/scripts/example.py"

# An unrelated custom skill survives uninstall; recorded SenpAI paths do not.
mkdir -p "$home/.codex/skills/acme-custom"
printf 'custom\n' > "$home/.codex/skills/acme-custom/SKILL.md"
HOME="$home" "$root/installer.sh" --uninstall --prefix "$prefix" --state-dir "$state"
assert_missing "$prefix/bin/senpai"
assert_missing "$home/.codex/skills/senpai-usage"
assert_missing "$home/.config/opencode/skills/senpai-project-management"
assert_file "$home/.codex/skills/acme-custom/SKILL.md"

# Tampered ownership cannot make the installer delete an arbitrary path.
mkdir -p "$work/important"
printf 'keep\n' > "$work/important/data"
mkdir -p "$state"
printf 'version=1\nskill\t%s\n' "$work/important" > "$state/ownership.tsv"
if HOME="$home" "$root/installer.sh" --uninstall --prefix "$prefix" --state-dir "$state" 2>/dev/null; then
  fail "unsafe state was accepted"
fi
assert_file "$work/important/data"

printf 'installer tests passed\n'
