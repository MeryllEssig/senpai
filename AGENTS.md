# SenpAI

SenpAI is a **project-scoped, declarative context layer for AI coding agents**.

Each project carries a single manifest file, `.senpai.jsonc`, that declares integrations, repositories, logical environments, documentation locations, and bounded project-operation capsules. In v2, ticketing and forge operations route through an integration's handled operations, local policy, workflow, and adapter. A forge is a code-development platform such as GitHub or GitLab.

## Documentation

- [Goals](./doc/goal.md)
- [User Stories](./doc/user-stories.md)
- [Technical considerations](./doc/technical-considerations.md)
- [CLI contract](./doc/cli-contract.md)
- [Manifest JSON Schema](./schema/senpai.schema.json)
- [Example manifest](./doc/reference-manifest.jsonc)
- [Example capsule values](./doc/reference-capsule.jsonc)
- [Notes](./doc/notes.md)
- [Ideas to explore](./doc/idea.md)
- [Local installation](./doc/installation.md)

## Notes

- Always conventional commit.
- If you install/update the project when in development, always try to install from the remote (github) by default.
- Before creating a release tag, align the tag exactly with the `version` in `Cargo.toml` (and the root `senpai` package entry in `Cargo.lock`), update the version assertion in `tests/cli.test.sh`, and verify the remotely installed binary reports that same version with `senpai --version`.
- Always keep documentation up-to-date.
- Do not wrap prose in documentation at a fixed column; use one line per paragraph or list item.
- Always fix bugs you see, even if you aren't responsible for the bug.
- Work in high-signal TDD: write or extend a test that demonstrates each meaningful behavior or regression before implementing it. Do not add low-value unit tests that merely mirror implementation details.
- Before handing a task back to the user, run `cargo fmt --check`, `cargo clippy -- -D warnings`, and the relevant test suite. Do not claim completion while any of them fails.
