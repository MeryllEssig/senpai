#!/usr/bin/env bash
# Install a verified SenpAI release or a locally built binary and its skills.
set -euo pipefail
IFS=$'\n\t'

PROGRAM_NAME=senpai
STATE_VERSION=1

usage() {
  cat <<'EOF'
Usage: installer.sh [--binary PATH | --version TAG] [options]
       installer.sh --uninstall [options]

Install a checksum-checked SenpAI release (latest by default) or a local build,
then optionally copy its shipped skills.

Options:
  --binary PATH       Executable local SenpAI binary
  --skills-dir PATH   Local senpai-* skills directory (default: ./skills with --binary)
  --version TAG       GitHub release tag (default: latest)
  --repository SLUG   GitHub repository for releases (default: MeryllEssig/senpai)
  --agents LIST       Comma-separated: codex,claude,gemini,opencode,all,none
  --prefix PATH       Binary prefix (default: $SENPAI_INSTALL_PREFIX or ~/.local)
  --state-dir PATH    Installer ownership state (default: $XDG_STATE_HOME/senpai
                       or ~/.local/state/senpai)
  --yes               Do not prompt; use --agents, or no skills when omitted
  --uninstall         Remove only paths recorded by this installer
  -h, --help          Show this help

The installer overwrites selected shipped senpai-* skills.  Uninstall removes
only the binary and skill directories recorded in its ownership file.
EOF
}

fail() { printf 'installer: %s\n' "$*" >&2; exit 1; }
note() { printf '%s\n' "$*"; }

require_value() {
  [[ $# -ge 2 && -n $2 && $2 != --* ]] || fail "$1 requires a value"
}

absolute_path() {
  case "$1" in
    /*) printf '%s\n' "$1" ;;
    *) printf '%s/%s\n' "$(pwd -P)" "$1" ;;
  esac
}

binary=""
skills_dir=""
version="latest"
version_set=false
repository="${SENPAI_RELEASE_REPOSITORY:-MeryllEssig/senpai}"
agents=""
prefix="${SENPAI_INSTALL_PREFIX:-$HOME/.local}"
state_dir="${SENPAI_STATE_DIR:-${XDG_STATE_HOME:-$HOME/.local/state}/senpai}"
assume_yes=false
uninstall=false
download_dir=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --binary) require_value "$@"; binary=$2; shift 2 ;;
    --skills-dir) require_value "$@"; skills_dir=$2; shift 2 ;;
    --version) require_value "$@"; version=$2; version_set=true; shift 2 ;;
    --repository) require_value "$@"; repository=$2; shift 2 ;;
    --agents) require_value "$@"; agents=$2; shift 2 ;;
    --prefix) require_value "$@"; prefix=$2; shift 2 ;;
    --state-dir) require_value "$@"; state_dir=$2; shift 2 ;;
    --yes) assume_yes=true; shift ;;
    --uninstall) uninstall=true; shift ;;
    -h|--help) usage; exit 0 ;;
    *) fail "unknown option: $1" ;;
  esac
done

prefix=$(absolute_path "$prefix")
state_dir=$(absolute_path "$state_dir")
state_file="$state_dir/ownership.tsv"

safe_owned_path() {
  local kind=$1 path=$2
  [[ $path = /* ]] || return 1
  case "$kind" in
    binary) [[ $(basename -- "$path") == "$PROGRAM_NAME" ]] ;;
    skill) [[ $(basename -- "$path") == senpai-* ]] ;;
    *) return 1 ;;
  esac
}

uninstall_owned() {
  [[ -f $state_file ]] || { note "SenpAI is not installed by this installer ($state_file not found)."; return; }
  local version kind path extra
  IFS=$'\t' read -r version < "$state_file" || fail "cannot read ownership state"
  [[ $version == "version=$STATE_VERSION" ]] || fail "unsupported ownership-state version"
  while IFS=$'\t' read -r kind path extra; do
    [[ -n ${kind:-} ]] || continue
    [[ -z ${extra:-} ]] || fail "invalid ownership record"
    safe_owned_path "$kind" "$path" || fail "refusing unsafe ownership record: $kind"
    case "$kind" in
      binary)
        [[ -e $path || -L $path ]] && rm -f -- "$path"
        ;;
      skill)
        [[ -e $path || -L $path ]] && rm -rf -- "$path"
        ;;
      *) fail "unknown ownership record: $kind" ;;
    esac
  done < <(tail -n +2 "$state_file")
  rm -f -- "$state_file"
  rmdir -- "$state_dir" 2>/dev/null || true
  note "SenpAI uninstall complete."
}

if $uninstall; then
  [[ -z $binary && -z $skills_dir && $version_set == false && -z $agents ]] || fail "--uninstall cannot be combined with installation options"
  uninstall_owned
  exit 0
fi

release_target() {
  local os architecture
  os=$(uname -s)
  architecture=$(uname -m)
  case "$os/$architecture" in
    Darwin/arm64) printf '%s\n' 'aarch64-apple-darwin' ;;
    Darwin/x86_64) printf '%s\n' 'x86_64-apple-darwin' ;;
    Linux/arm64|Linux/aarch64) printf '%s\n' 'aarch64-unknown-linux-musl' ;;
    Linux/x86_64) printf '%s\n' 'x86_64-unknown-linux-musl' ;;
    *) fail "unsupported platform: $os/$architecture (supported: macOS and Linux on arm64 or x86_64)" ;;
  esac
}

sha256_file() {
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print $1}'
  elif command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  else
    fail "SHA-256 tool not found (need shasum or sha256sum)"
  fi
}

download_release() {
  [[ $repository =~ ^[A-Za-z0-9._-]+/[A-Za-z0-9._-]+$ ]] || fail "invalid GitHub repository: $repository"
  [[ $version != */* ]] || fail "invalid release tag: $version"
  command -v curl >/dev/null 2>&1 || fail "curl is required to download a release"
  command -v tar >/dev/null 2>&1 || fail "tar is required to unpack a release"

  local target artifact base_url archive checksums expected actual
  target=$(release_target)
  artifact="$PROGRAM_NAME-$target.tar.gz"
  if [[ $version == latest ]]; then
    base_url="https://github.com/$repository/releases/latest/download"
  else
    base_url="https://github.com/$repository/releases/download/$version"
  fi
  download_dir=$(mktemp -d "${TMPDIR:-/tmp}/senpai.XXXXXX") || fail "cannot create temporary directory"
  trap '[[ -z $download_dir ]] || rm -rf -- "$download_dir"' EXIT
  archive="$download_dir/$artifact"
  checksums="$download_dir/checksums.txt"
  curl --fail --location --silent --show-error "$base_url/checksums.txt" -o "$checksums" || fail "cannot download release checksums"
  expected=$(awk -v artifact="$artifact" '$2 == artifact { print $1; exit }' "$checksums")
  [[ $expected =~ ^[[:xdigit:]]{64}$ ]] || fail "checksum missing or invalid for $artifact"
  curl --fail --location --silent --show-error "$base_url/$artifact" -o "$archive" || fail "cannot download $artifact"
  actual=$(sha256_file "$archive")
  actual=$(printf '%s' "$actual" | tr '[:upper:]' '[:lower:]')
  expected=$(printf '%s' "$expected" | tr '[:upper:]' '[:lower:]')
  [[ $actual == "$expected" ]] || fail "checksum verification failed for $artifact"
  tar -tzf "$archive" | awk '
    $0 == "senpai" { binary = 1 }
    $0 ~ /^skills\/senpai-[^\/]+\/SKILL\.md$/ { skill = 1 }
    $0 ~ /(^\/|\/\.\.?(\/|$))/ { invalid = 1 }
    END { exit !(binary && skill && !invalid) }
  ' || fail "release archive has an invalid layout"
  tar -xzf "$archive" -C "$download_dir" || fail "cannot unpack $artifact"
  [[ -x $download_dir/$PROGRAM_NAME ]] || fail "release archive does not contain an executable senpai binary"
  binary="$download_dir/$PROGRAM_NAME"
  skills_dir="$download_dir/skills"
}

if [[ -n $binary ]]; then
  [[ $version_set == false ]] || fail "--binary cannot be combined with --version"
  [[ -n $skills_dir ]] || skills_dir="$(pwd -P)/skills"
  [[ -f $binary ]] || fail "binary is not a regular file: $binary"
  [[ -x $binary ]] || fail "binary is not executable: $binary"
  binary=$(absolute_path "$binary")
  [[ -d $skills_dir ]] || fail "skills directory does not exist: $skills_dir"
  skills_dir=$(absolute_path "$skills_dir")
else
  [[ -z $skills_dir ]] || fail "--skills-dir requires --binary"
  download_release
fi

declare -a selected=()
agent_dir() {
  case "$1" in
    codex) printf '%s\n' "${CODEX_HOME:-$HOME/.codex}/skills" ;;
    claude) printf '%s\n' "$HOME/.claude/skills" ;;
    gemini) printf '%s\n' "$HOME/.gemini/skills" ;;
    opencode) printf '%s\n' "${XDG_CONFIG_HOME:-$HOME/.config}/opencode/skills" ;;
    *) return 1 ;;
  esac
}

select_agents() {
  local input=$1 item
  [[ -n $input ]] || return
  [[ $input != *,* ]] || :
  IFS=',' read -r -a items <<< "$input"
  for item in "${items[@]}"; do
    case "$item" in
      all) selected=(codex claude gemini opencode); return ;;
      none) [[ ${#items[@]} -eq 1 ]] || fail "none cannot be combined with other agents"; return ;;
      codex|claude|gemini|opencode)
        local existing
        if [[ ${#selected[@]} -gt 0 ]]; then
          for existing in "${selected[@]}"; do [[ $existing == "$item" ]] && continue 2; done
        fi
        selected+=("$item")
        ;;
      *) fail "unknown agent: $item" ;;
    esac
  done
}

if [[ -z $agents && $assume_yes == false && -t 0 ]]; then
  read -r -p 'Install SenpAI skills for which agents? [codex,claude,gemini,opencode,all,none] ' agents
fi
select_agents "$agents"

# A later install may select only one ecosystem.  Keep the ownership records
# for previously installed ecosystems so a later uninstall can still remove
# exactly what this installer owns.
declare -a prior_records=()
if [[ -f $state_file ]]; then
  IFS=$'\t' read -r prior_version < "$state_file" || fail "cannot read ownership state"
  [[ $prior_version == "version=$STATE_VERSION" ]] || fail "unsupported ownership-state version"
  while IFS=$'\t' read -r kind path extra; do
    [[ -n ${kind:-} ]] || continue
    [[ -z ${extra:-} ]] || fail "invalid ownership record"
    safe_owned_path "$kind" "$path" || fail "refusing unsafe ownership record: $kind"
    prior_records+=("$kind"$'\t'"$path")
  done < <(tail -n +2 "$state_file")
fi

declare -a skill_sources=()
while IFS= read -r -d '' candidate; do skill_sources+=("$candidate"); done < <(find "$skills_dir" -mindepth 1 -maxdepth 1 -type d -name 'senpai-*' -print0)
if [[ ${#selected[@]} -gt 0 && ${#skill_sources[@]} -eq 0 ]]; then
  fail "no senpai-* skill directories found in $skills_dir"
fi

mkdir -p -- "$prefix/bin" "$state_dir"
destination_binary="$prefix/bin/$PROGRAM_NAME"
temporary_binary="$prefix/bin/.$PROGRAM_NAME.tmp.$$"
install -m 755 -- "$binary" "$temporary_binary"
mv -f -- "$temporary_binary" "$destination_binary"

temporary_state="$state_dir/.ownership.tsv.tmp.$$"
{
  printf 'version=%s\n' "$STATE_VERSION"
  declare -a written=()
  write_record() {
    local kind=$1 path=$2 key record
    key="$kind"$'\t'"$path"
    if [[ ${#written[@]} -gt 0 ]]; then
      for record in "${written[@]}"; do [[ $record == "$key" ]] && return; done
    fi
    written+=("$key")
    printf '%s\t%s\n' "$kind" "$path"
  }
  if [[ ${#prior_records[@]} -gt 0 ]]; then
    for record in "${prior_records[@]}"; do
      IFS=$'\t' read -r kind path <<< "$record"
      write_record "$kind" "$path"
    done
  fi
  write_record binary "$destination_binary"
  if [[ ${#selected[@]} -gt 0 ]]; then
    for agent in "${selected[@]}"; do
      destination_dir=$(agent_dir "$agent")
      mkdir -p -- "$destination_dir"
      destination_dir=$(absolute_path "$destination_dir")
      for source in "${skill_sources[@]}"; do
        skill_name=$(basename -- "$source")
        temporary_skill="$destination_dir/.${skill_name}.tmp.$$"
        rm -rf -- "$temporary_skill"
        cp -R -- "$source" "$temporary_skill"
        rm -rf -- "$destination_dir/$skill_name"
        mv -- "$temporary_skill" "$destination_dir/$skill_name"
        write_record skill "$destination_dir/$skill_name"
      done
    done
  fi
} > "$temporary_state"
mv -f -- "$temporary_state" "$state_file"

note "Installed $PROGRAM_NAME to $destination_binary"
if [[ ${#selected[@]} -gt 0 ]]; then
  selected_label=$(IFS=,; printf '%s' "${selected[*]}")
  note "Installed SenpAI skills for: $selected_label"
else
  note "No agent skills selected."
fi
