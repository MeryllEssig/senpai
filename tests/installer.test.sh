#!/usr/bin/env bash
set -euo pipefail

root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)
work=$(mktemp -d)
trap 'result_code=$?; rm -rf -- "$work"; exit "$result_code"' EXIT

fail() { printf 'FAIL: %s\n' "$*" >&2; exit 1; }
assert_file() { [[ -f $1 ]] || fail "expected file: $1"; }
assert_missing() { [[ ! -e $1 && ! -L $1 ]] || fail "expected missing: $1"; }
assert_contains() { grep -Fqx -- "$2" "$1" || fail "expected $2 in $1"; }
sha256() {
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print $1}'
  else
    sha256sum "$1" | awk '{print $1}'
  fi
}

mkdir -p "$work/source/skills/senpai-usage" "$work/source/skills/senpai-project-management/scripts"
printf '#!/usr/bin/env bash\nprintf senpai\n' > "$work/source/senpai"
chmod +x "$work/source/senpai"
printf 'first version\n' > "$work/source/skills/senpai-usage/SKILL.md"
printf 'script\n' > "$work/source/skills/senpai-project-management/scripts/example.py"

# A release archive contains the executable and every shipped skill. The fake
# curl server lets this test exercise the same verified-download path without
# requiring the private GitHub repository to be publicly reachable.
release="$work/release"
mkdir -p "$release" "$work/bin"
for target in aarch64-apple-darwin x86_64-apple-darwin aarch64-unknown-linux-musl x86_64-unknown-linux-musl; do
  tar -czf "$release/senpai-$target.tar.gz" -C "$work/source" senpai skills
  checksum=$(sha256 "$release/senpai-$target.tar.gz")
  printf '%s  senpai-%s.tar.gz\n' "$checksum" "$target" >> "$release/checksums.txt"
done
cat > "$work/bin/curl" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
url=""
output=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    -o) output=$2; shift 2 ;;
    *) url=$1; shift ;;
  esac
done
[[ -n $url && -n $output ]] || exit 64
file=${url##*/}
if [[ ${MOCK_TAMPER:-false} == true && $file == *.tar.gz ]]; then
  printf 'tampered' > "$output"
else
  cp -- "$MOCK_RELEASE_DIR/$file" "$output"
fi
EOF
chmod +x "$work/bin/curl"

home="$work/home"
prefix="$work/prefix"
state="$work/state"
# GitHub Linux runners export XDG_CONFIG_HOME. Pin all agent destinations so
# this hermetic test does not accidentally install into the runner's home.
export HOME="$home"
export CODEX_HOME="$home/.codex"
export XDG_CONFIG_HOME="$home/.config"
PATH="$work/bin:$PATH" MOCK_RELEASE_DIR="$release" HOME="$home" "$root/installer.sh" --yes --agents codex --version v0.1.0 --repository example/senpai --prefix "$prefix" --state-dir "$state"
assert_file "$prefix/bin/senpai"
assert_file "$home/.codex/skills/senpai-usage/SKILL.md"

if PATH="$work/bin:$PATH" MOCK_RELEASE_DIR="$release" MOCK_TAMPER=true HOME="$work/tampered-home" "$root/installer.sh" --yes --agents none --version v0.1.0 --repository example/senpai --prefix "$work/tampered-prefix" --state-dir "$work/tampered-state" >/dev/null 2>&1; then
  fail "tampered release unexpectedly installed"
fi

HOME="$home" "$root/installer.sh" --yes --binary "$work/source/senpai" --skills-dir "$work/source/skills" --agents codex,opencode --prefix "$prefix" --state-dir "$state"

assert_file "$prefix/bin/senpai"
assert_file "$home/.codex/skills/senpai-usage/SKILL.md"
assert_file "$home/.config/opencode/skills/senpai-project-management/scripts/example.py"
assert_contains "$state/ownership.tsv" $'binary\t'"$prefix/bin/senpai"
assert_contains "$state/ownership.tsv" $'skill\t'"$home/.codex/skills/senpai-usage"

# macOS ships Bash 3.2: an empty selected-agent array must stay safe under
# `set -u` when the binary is installed without skills.
HOME="$home" "$root/installer.sh" --yes --binary "$work/source/senpai" --skills-dir "$work/source/skills" --agents none --prefix "$prefix" --state-dir "$state"

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
