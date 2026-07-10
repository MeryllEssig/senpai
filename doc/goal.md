# AI Manager - Goal

## Summary

AI Manager is a **project-scoped, declarative context layer for AI coding agents**.

Each project carries a single manifest file, `.aimanager.jsonc`, that declares everything an AI agent needs to interact with the project's external ecosystem: ticket trackers, code hosting, environments and their logs, data stores, documentation locations, and rules mapping intents to skills.

A companion CLI parses the manifest and returns to the agent **only what is needed** to answer the current user request. Generic skills (markdown instruction files an agent loads to learn a capability), installed once per machine rather than per project, cover using the CLI, setting up a manifest in a new project, and spotting automation opportunities.

The system is **purely declarative**: it describes where things are and how to reach them; it never installs or executes project tooling itself. The agent reads the declarations and figures out what to do.

## Problem

AI agents start every session blind to the project's ecosystem. The developer has to repeat, in every conversation and for every project: which tracker holds the tickets (and which one to write to, when there are several), where the repos are, how to read preprod (staging) and prod logs, how to reach the database, where the documentation lives. This context exists, but it lives in the developer's head, scattered across configs, or in agent-specific files that don't transfer between ecosystems.

## Objectives

1. **Project-first.** Context lives with the project, not in a central configuration. The manifest sits in the project folder, which does not need to be a git repository; when committing it to a client repo is not possible, it sits in a parent directory instead. Resolution walks up from the current directory, like `.git`.
2. **Declarative, not executable.** The manifest describes connections (typed connectors: SSH, log files, DB, search engines, docs, and so on). Agents decide how to act on them. AI Manager installs nothing on its own.
3. **Faithful to real project diversity.** Multiple trackers per project (ours plus the client's), tracker lifecycle (a project's tickets move to a general maintenance tracker after go-live), multi-repo galaxies (a main repository containing many interdependent sub-repositories) whose members must be orchestrated together, synchronized code hosting instances with distinct roles (merge requests and test pipelines on one, deployment pipelines on the other), non-git folders.
4. **Agent-agnostic.** Usable from any agentic ecosystem: Claude Code, Codex, Gemini CLI, OpenCode, and whatever comes next. The lowest common denominator is a CLI on `$PATH` plus markdown instructions.
5. **Token-efficient.** Skills use progressive discovery: a minimal entry point, deeper detail loaded only when needed. The CLI answers narrowly scoped queries instead of dumping the full manifest.
6. **Didactic setup.** Setup and management skills interview the user, study the folder first, explain what they do and why (including practical caveats such as restarting the agent to pick up new environment variables), and speak the user's language.
7. **Tool-agnostic core, optional connectors.** The core does not depend on any specific tracker or code hosting platform; connectors for common tools may be provided when needed.

## Deliverables

- **The manifest format** - a JSONC schema, commented, versioned.
- **A CLI** - parses and validates the manifest, answers scoped queries, returns LLM-optimized output.
- **Usage skill(s)** - triggered manually or automatically when a user request needs external data; route intents to the right declared capability.
- **Setup and management skills** - guided manifest creation, tool installation assistance, automation-opportunity discovery.

## Non-goals

- Storing secrets. The manifest references *where* credentials live (for example environment variable names), never their values.
- Executing or installing project tooling. Declarations only.
- Being a central registry. There is no machine-wide project index; each project is self-describing.

## Language policy

Everything in this repository (code, documentation, manifests, skills) is written in **English**. At runtime, skills respond in the user's language, whatever it is; there is no default language.
