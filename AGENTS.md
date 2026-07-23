# Senpai

Senpai is a **project-scoped, declarative context layer for AI coding agents**.

Each project carries a single manifest file, `.senpai.jsonc`, that declares the context an AI agent needs to interact with the project's ecosystem: ticket trackers, code hosting, repositories, logical environments, documentation locations, bounded project-operation capsules, and rules mapping intents to capsules or external-tool skills.

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

## Notes

- Always conventional commit.
- If you install/update the project when in development, always try to install from the remote (github) by default.
- Always keep documentation up-to-date.
- Always fix bugs you see, even if you aren't responsible for the bug.