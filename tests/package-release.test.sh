#!/usr/bin/env bash
set -euo pipefail

root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)
work=$(mktemp -d)
trap 'result_code=$?; rm -rf -- "$work"; exit "$result_code"' EXIT

mkdir -p "$work/skills/senpai-usage"
printf '#!/usr/bin/env bash\nprintf senpai\n' > "$work/senpai"
chmod +x "$work/senpai"
printf 'skill\n' > "$work/skills/senpai-usage/SKILL.md"

"$root/scripts/package-release.sh" --target x86_64-unknown-linux-musl --binary "$work/senpai" --skills-dir "$work/skills" --output "$work/senpai-x86_64-unknown-linux-musl.tar.gz"
tar -tzf "$work/senpai-x86_64-unknown-linux-musl.tar.gz" | grep -Fxq 'senpai'
tar -tzf "$work/senpai-x86_64-unknown-linux-musl.tar.gz" | grep -Fxq 'skills/senpai-usage/SKILL.md'

printf 'package release tests passed\n'
