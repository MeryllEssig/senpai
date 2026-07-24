# SenpAI - Goal

## Summary

SenpAI is a **project-scoped, declarative context layer for AI coding agents**.

Each project carries a single manifest file, `.senpai.jsonc`, that declares the context an AI agent needs to interact with the project's ecosystem: ticket trackers, code hosting, workflow policies and instruction skills, repositories, environments, documentation locations, bounded executable capsules, and cross-cutting rules.

A companion CLI parses the manifest and returns to the agent **only what is needed** to answer the current user request. Generic skills (markdown instruction files an agent loads to learn a capability), installed once per machine rather than per project, cover using the CLI, setting up a manifest in a new project, and spotting automation opportunities.

The system is **declarative with one explicit execution primitive**: a **capsule** is a bounded, non-interactive project operation run by SenpAI through `senpai run`. A capsule declares a literal program and argument array; private argument values are resolved inside SenpAI so they never enter the agent's transcript. Tests, logs, database queries, CSV exports, setup steps, and similar automatable operations are all capsules. The agent reads the declarations and decides which capsule or external tool skill to use.

## Problem

AI agents start every session blind to the project's ecosystem. The developer has to repeat, in every conversation and for every project: which tracker holds the ticket details and which one time is booked on when there are several, where the repos are, how to read preprod (staging) and prod logs, how to reach the database, where the documentation lives. This context exists, but it lives in the developer's head, scattered across configs, or in agent-specific files that don't transfer between ecosystems.

## Objectives

1. **Project-first.** Context lives with the project, not in a central configuration. The manifest sits in the project folder, which does not need to be a git repository; when committing it to a client repo is not possible, it sits in a parent directory instead. Resolution walks up from the current directory, like `.git`.
2. **One bounded execution primitive.** Every bounded project command SenpAI executes is a capsule: a deterministic literal program plus argument array, optional working directory, optional agent-supplied parameters, bounded output, and a timeout. SenpAI executes it without a shell. Capsules may require no local values; when private placeholders exist, SenpAI resolves and scrubs them inside its own process. Rich external platforms such as trackers and code hosts remain driven by skills. Interactive shells, foreground servers, follow-mode logs, and other unbounded processes are intentionally unsupported.
3. **Faithful to real project diversity.** Multiple trackers per project (ours plus the client's), tracker lifecycle (a project's tickets move to a general maintenance tracker after go-live), multi-repo galaxies (a main repository containing many interdependent sub-repositories) whose members must be orchestrated together, synchronized code hosting instances with distinct roles (merge requests and test pipelines on one, deployment pipelines on the other), non-git folders.
4. **Agent-agnostic.** Usable from any agentic ecosystem: Claude Code, Codex, Gemini CLI, OpenCode, and whatever comes next. The lowest common denominator is a CLI on `$PATH` plus markdown instructions.
5. **Token-efficient.** Skills use progressive discovery: a minimal entry point, deeper detail loaded only when needed. The CLI answers narrowly scoped queries instead of dumping the full manifest.
6. **Didactic setup.** Setup and management skills interview the user, study the folder first, explain what they do and why (including practical caveats such as restarting the agent to pick up new environment variables), and speak the user's language.
7. **Tool-agnostic core, common interfaces, specific adapters.** The core does not depend on any tracker or code hosting platform. The shipped `senpai-project-management` and `senpai-code-hosting` skills expose common ticket and code-change capabilities, then use an adapter selected from the declared source type or hosting platform. Redmine's adapter embeds Python standard-library scripts that call its REST API directly. Project workflows separately combine an authorization policy with user-authored instructions describing how work should be performed (see [technical considerations](technical-considerations.md) 1.6 and 3.1).

## Deliverables

- **The manifest format** - a JSONC schema, commented, versioned.
- **A CLI** - parses and validates the manifest, answers scoped queries, returns LLM-optimized output.
- **Usage skill(s)** - shipped with an `senpai-` prefix, triggered manually or automatically when a user request needs external data; route intents to the right declared capability.
- **Common external-platform skills** - `senpai-project-management` and `senpai-code-hosting`, backed by platform-specific adapters; the Redmine adapter embeds Python stdlib-only scripts driving the REST API.
- **Default workflow skills** - `senpai-project-use-ticket-workflow` and `senpai-project-use-code-hosting-workflow`, used with a read-only policy when the project declares no workflow for that domain; declared workflows replace them with freely named custom skills and explicit policies.
- **Setup and management skills** - shipped with an `senpai-` prefix; guided manifest creation, tool installation assistance, automation-opportunity discovery.
- **An internal QA skill** (maintainer-only, never shipped) - analyzes real conversation transcripts to flag agent inefficiencies and route each one to its root cause (the project's manifest or SenpAI itself), with a remediation proposal.

## Non-goals

- Storing secrets in the manifest. The manifest references *where* credentials live (for example environment variable names), never their values. A companion local, gitignored, never-committed values file may hold real secret values (or env-var references) so SenpAI can run private capsules without leaking them (see [technical considerations](technical-considerations.md) 1.10); secrets never appear in the manifest, in CLI output to the agent, or in the transcript.
- Executing interactive or unbounded project tooling. Capsules cover bounded, non-interactive operations only. A project uses finite alternatives such as `docker compose logs --tail=200` or `docker compose up -d`; interactive shells, foreground servers, and follow-mode logs stay outside SenpAI.
- Replacing target services' account and data-access controls. SenpAI applies the effective ticket and code-change workflow policy to agent operations, while the target service remains responsible for the account's actual permissions. Optional capsule `access` metadata remains advisory, and cross-cutting restrictions outside those workflows remain project rules.
- Guarding against a malicious committed manifest. SenpAI ships no first-use trust prompt or manifest change-detection machinery: a committed manifest is vetted like any other committed file, by the team's own review process. This is separate from explicit confirmations required by workflow policy. The target users are teams and companies with an internal trust process (see [technical considerations](technical-considerations.md) 5).
- Being a central registry. There is no machine-wide project index; each project is self-describing.

## Language policy

Everything in this repository (code, documentation, manifests, skills) is written in **English**. At runtime, skills respond in the user's language, whatever it is; there is no default language.
