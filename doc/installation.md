# Local installation

Release download and checksum verification are deliberately not implemented yet.
The current installer installs a binary that you built locally and the shipped
skills found in a local directory.

```sh
./installer.sh \
  --binary ./target/release/senpai \
  --skills-dir ./skills \
  --agents codex,claude
```

The binary is installed to `~/.local/bin/senpai` by default. Use `--prefix` to
choose another prefix. The supported agent selections are `codex`, `claude`,
`gemini`, `opencode`, `all`, and `none`.

The destination directories are `$CODEX_HOME/skills` (or `~/.codex/skills`),
`~/.claude/skills`, `~/.gemini/skills`, and
`$XDG_CONFIG_HOME/opencode/skills` (or `~/.config/opencode/skills`). Only
top-level directories named `senpai-*` from `--skills-dir` are installed.

Pass `--yes` in automation. Without `--agents`, it skips skills in noninteractive
mode and prompts in an interactive terminal. Rerunning the command replaces the
binary and the selected Senpai skills; it never updates custom skills.

The installer records exactly the binary and skill directories it owns in
`$XDG_STATE_HOME/senpai/ownership.tsv` (default:
`~/.local/state/senpai/ownership.tsv`). Remove them with the same prefix and
state settings:

```sh
./installer.sh --uninstall
```

Uninstall removes only recorded `senpai-*` skill directories and the recorded
`senpai` binary. It does not remove project configuration, custom skills, or
unrecorded files.

To run the local installer checks:

```sh
bash tests/installer.test.sh
```
