# Installation

`installer.sh` installs a GitHub release by default. It detects macOS or Linux and arm64 or x86_64, downloads the matching archive, then checks its SHA-256 against `checksums.txt` from that same release before extracting anything. Linux artifacts use musl targets, so they do not depend on a particular glibc version. This detects transfer corruption; GitHub repository access controls protect the release assets.

```sh
curl --fail --location --silent --show-error \
  https://raw.githubusercontent.com/MeryllEssig/senpai/main/installer.sh | bash -s -- --agents codex
```

Use a specific release tag when needed:

```sh
./installer.sh --version v0.1.0
```

Before a public release exists, the repository remains private, or during development, supply a locally built binary and the checked-out skills:

```sh
./installer.sh \
  --binary ./target/release/senpai \
  --skills-dir ./skills \
  --agents codex,claude
```

The binary is installed to `~/.local/bin/senpai` by default. Use `--prefix` to choose another prefix. The supported agent selections are `codex`, `claude`, `gemini`, `opencode`, `all`, and `none`. `--repository owner/name` selects a different public GitHub repository; it is useful for verified forks.

The destination directories are `$CODEX_HOME/skills` (or `~/.codex/skills`), `~/.claude/skills`, `~/.gemini/skills`, and `$XDG_CONFIG_HOME/opencode/skills` (or `~/.config/opencode/skills`). Only top-level directories named `senpai-*` from `--skills-dir` are installed.

Pass `--yes` in automation. Without `--agents`, it skips skills in noninteractive mode and prompts in an interactive terminal. Rerunning the command replaces the binary and the selected SenpAI skills; it never updates custom skills.

The installer records exactly the binary and skill directories it owns in `$XDG_STATE_HOME/senpai/ownership.tsv` (default: `~/.local/state/senpai/ownership.tsv`). Remove them with the same prefix and state settings:

```sh
./installer.sh --uninstall
```

Uninstall removes only recorded `senpai-*` skill directories and the recorded `senpai` binary. It does not remove project configuration, custom skills, or unrecorded files.

To run local installer and release-package checks:

```sh
bash tests/installer.test.sh
bash tests/package-release.test.sh
```
