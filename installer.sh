#!/usr/bin/env bash
# Install a locally built SenpAI binary and its shipped skills.
#
# Release download/checksum verification intentionally lives outside this local
# installer.  Supply an already-built binary with --binary.
set -euo pipefail
IFS=$'\n\t'

PROGRAM_NAME=senpai
STATE_VERSION=1

usage() {
  cat <<'EOF'
Usage: installer.sh --binary PATH [options]
       installer.sh --uninstall [options]

Install a local SenpAI build and optionally copy its shipped skills.

Options:
  --binary PATH       Executable local SenpAI binary (required for install)
  --skills-dir PATH   Directory containing senpai-* skill directories (default: ./skills)
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
skills_dir="$(pwd -P)/skills"
agents=""
prefix="${SENPAI_INSTALL_PREFIX:-$HOME/.local}"
state_dir="${SENPAI_STATE_DIR:-${XDG_STATE_HOME:-$HOME/.local/state}/senpai}"
assume_yes=false
uninstall=false

while [[ $# -gt 0 ]]; do
  case "$1" in
    --binary) require_value "$@"; binary=$2; shift 2 ;;
    --skills-dir) require_value "$@"; skills_dir=$2; shift 2 ;;
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
  [[ -z $binary && -z $agents ]] || fail "--uninstall cannot be combined with --binary or --agents"
  uninstall_owned
  exit 0
fi

[[ -n $binary ]] || fail "--binary is required for installation"
[[ -f $binary ]] || fail "binary is not a regular file: $binary"
[[ -x $binary ]] || fail "binary is not executable: $binary"
binary=$(absolute_path "$binary")
[[ -d $skills_dir ]] || fail "skills directory does not exist: $skills_dir"
skills_dir=$(absolute_path "$skills_dir")

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
        for existing in "${selected[@]}"; do [[ $existing == "$item" ]] && continue 2; done
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
    for record in "${written[@]}"; do [[ $record == "$key" ]] && return; done
    written+=("$key")
    printf '%s\t%s\n' "$kind" "$path"
  }
  for record in "${prior_records[@]}"; do
    IFS=$'\t' read -r kind path <<< "$record"
    write_record "$kind" "$path"
  done
  write_record binary "$destination_binary"
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
} > "$temporary_state"
mv -f -- "$temporary_state" "$state_file"

note "Installed $PROGRAM_NAME to $destination_binary"
if [[ ${#selected[@]} -gt 0 ]]; then
  selected_label=$(IFS=,; printf '%s' "${selected[*]}")
  note "Installed SenpAI skills for: $selected_label"
else
  note "No agent skills selected."
fi
