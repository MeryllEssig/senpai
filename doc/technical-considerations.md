# AI Manager - Technical Considerations

This document details the technical topics behind the user stories. The design decisions below are settled; [notes.md](notes.md) tracks any topic still left open plus cross-cutting decisions with no home here.

## 1. The manifest

### 1.1 Format and name

- **JSONC** (JSON with comments): declarative, diff-friendly, and commentable so the file can carry its own description and per-entry explanations.
- File name: `.aimanager.jsonc` (decided).
- A top-level `version` field from day one, so the schema can evolve without breaking existing manifests.
- A top-level `project` identity block (`name` slug, `label`, a free-text `context` for the agent, free-form `stack` hints) says what the project is; every other section declares how to reach its ecosystem.
- A published JSON Schema for editor completion and CLI validation.
- The complete schema is frozen by a commented golden example, [reference-manifest.jsonc](reference-manifest.jsonc), which exercises every feature once. Until the JSON Schema exists, the example is the normative source when it and the prose disagree.

### 1.2 Resolution

- The CLI resolves the manifest by **walking up the directory tree** from the current working directory until it finds one, exactly like `.git` discovery.
- This makes placement flexible by construction:
  - committed at the repo root for projects where that is acceptable;
  - in a **parent directory** of the repo for client codebases that must not carry tooling files;
  - in any plain folder: no git required.
- **Nearest-wins, no inheritance (decided).** The manifest that applies is the nearest one found by walking up; the **execution directory is authoritative**. A galaxy root and a sub-project may each carry their own manifest, but they stay independent: the nearer manifest is used as-is and in full, never merged with or inheriting from an outer one. There is no `extends` mechanism between manifests.
- **Resolution failure is explicit (decided).** If the walk-up reaches the filesystem root without finding a manifest, the CLI stops with an explanatory error (it never silently proceeds), so the agent can tell the user to run setup rather than improvise.
- A personal, gitignored overlay may sit beside the resolved manifest; see 1.9.

### 1.3 Trackers: several sources, routed by function

Real projects break the "one project = one tracker" assumption in two ways:

1. **Two organizations, two trackers.** Our Redmine plus the client's Redmine, Jira, or Linear, both legitimate sources for the same project.
2. **Lifecycle moves.** A project starts with its own dedicated project space inside the tracker; once in production it moves into a general maintenance space shared by all live projects (what French agencies call TMA, "tierce maintenance applicative", third-party application maintenance).

**Routing is by function, never by theme (decided).** There is no per-category (bug / chore / feature) routing. What differs between trackers is the *role* each one plays:

- **`ticket_details`** - the authoritative source ("makes foi") for reading a ticket's content. Typically the client's tracker.
- **`time_logging`** - where time spent is imputed. Typically our internal tracker; it may point at a dedicated project space or a catch-all ticket used purely to book time.
- **`external_refs`** - a source that cross-links references to or from another source (an internal ticket carrying the client's reference, or the reverse).
- **`hosting_requests`** - a tracker used to open and read requests to a hosting provider. It does **not** make foi. When the hosting team sits inside the same company, this can be a separate project space on the internal source instead of a distinct source.

The set of roles is open (LLM-readable strings), not a closed enum. Each source declares its `roles`; per-role detail (a catch-all ticket id, a specific project space) lives on the source.

```jsonc
{
  "trackers": {
    "sources": {
      "client": {
        "type": "jira", "url": "https://acme.atlassian.net", "project": "ACME",
        // Authoritative for ticket content: read ticket details here.
        "roles": ["ticket_details"]
      },
      "internal": {
        "type": "redmine", "url": "https://redmine.agency.example", "project": "acme-build",
        // Where time is booked; also carries the client reference back.
        "roles": ["time_logging", "external_refs"],
        // Optional catch-all ticket for time with no dedicated ticket.
        "time_logging": { "catch_all_ticket": "SUPPORT-0" }
      }
    }
  }
}
```

- Actions route by role, with no single `write_target`: "log my time" goes to the `time_logging` source, "open a hosting request" to the `hosting_requests` source, "read ticket details" to the `ticket_details` source.
- The **build -> maintenance lifecycle move** is a one-line change to the `project` of the relevant source (for example the internal `time_logging` source moving from `acme-build` to `maintenance-general`).
- A source may carry an optional `auth` declaration (see 1.7).

**Resolving a bare ticket number** (`#1234`) when several trackers are declared (decided):

1. **Pattern first.** Self-disambiguating ids (a Jira key such as `ACME-123`) route to the source whose id shape matches.
2. **Declared order next.** For genuinely ambiguous bare numbers, try the sources in declared order.
3. **Ask when still ambiguous.** If more than one source could plausibly own the number (for example two Redmines), stop and ask the user rather than guess.

### 1.4 Multi-repo galaxies

Observed shape: a main repository (orchestration scripts, CI, provisioning) whose subdirectory contains dozens of sub-repositories that depend on one another (APIs consumed by fronts, shared bundles).

Manifest support:

```jsonc
{
  "repos": {
    "root": { "path": ".", "role": "orchestrator" },
    "api_billing": { "path": "www/api_billing", "role": "api" },
    "front_billing": { "path": "www/front_billing", "role": "front", "depends_on": ["api_billing"] }
  }
}
```

- One manifest at the galaxy root describes the members and their `depends_on` edges. `role` is a free-form hint for the agent, not an enum the CLI interprets.
- The CLI can then answer questions like "which repos does front_billing depend on" or "list the members", giving the agent enough structure to orchestrate cross-repo work (coordinated changes, dependency-aware navigation).
- **Orchestration model (decided).** AI Manager provides declared facts; the agent derives the actions. The manifest carries enough structure (members, dependencies, hosting instances and their roles, trackers) for the agent to understand by itself what a cross-repo request implies. Example: the user modified two repos of the galaxy and asks to "create the MRs"; the agent creates one merge request per modified repo, each on the right instance (see 1.5). No orchestration engine, no scripted actions.
- **Composition (decided).** A sub-repo may carry its own manifest, but manifests never compose: the nearest one to the execution directory wins in full (see 1.2).

### 1.5 Code hosting: synchronized instances with roles

A repo is not always hosted in one place. Real case: two synchronized GitLab instances, where merge requests are opened and test pipelines run on the first, while deployment pipelines are viewed and triggered on the second. The agent must know which instance serves which operation.

```jsonc
{
  "code_hosting": {
    "instances": {
      "dev": {
        "platform": "gitlab",
        "url": "https://gitlab.agency.example",
        // The agent picks the instance by role.
        "roles": ["merge_requests", "test_pipelines"]
      },
      "ops": {
        "platform": "gitlab",
        "url": "https://gitlab.client.example",
        "roles": ["deployment_pipelines"]
      }
    }
  },
  "repos": {
    "api_billing": {
      "path": "www/api_billing",
      // Instance id -> repo path on that instance (mirrors can differ in namespace).
      "hosting": { "dev": "acme/api-billing", "ops": "client-mirror/api-billing" }
    }
  }
}
```

- `instances` declares each hosting endpoint once, with `roles` stating which operations belong to it (open merge requests, watch test pipelines, view and trigger deployment pipelines). An instance may carry an optional `auth` declaration (see 1.7); without one, the platform CLI is assumed to be pre-configured for that instance.
- Each repo maps instance ids to its path on that instance.
- With roles declared, "create the MR" routes to `dev` and "trigger the deployment" routes to `ops`, without any per-request instruction. A single-instance project declares one instance holding every role.

### 1.6 Data-source connectors

Connectors are **typed and declarative**: the manifest states what exists and how to reach it; it never embeds arbitrary shell commands in connector fields. The agent (guided by the usage skill) decides what to run. The one place command strings may appear is the explicit `local_commands` category (see 1.10), which is opt-in and gated by the trust model.

Candidate connector types for v1, driven by actual needs:

| Type | Declares | Example use |
|------|----------|-------------|
| `ssh` | host, optional user/jump host | reach a preprod box |
| `logs` | transport (ssh/file), systemd journal unit or file path; nested under its environment | read prod logs |
| `database` | engine (mysql, postgres...), host, port, database, credential reference | inspect data |
| `elasticsearch` / `solr` / `redis` | endpoint, index/core/db, credential reference | query search or cache layers |
| `docs` | local path or remote URL/repo | find the functional documentation |
| `tracker` | see 1.3 | read tickets, book time, open requests |
| `code_hosting` | see 1.5 | merge/pull requests, pipelines |

**Environments taxonomy (decided).** Environment ids are free-form map keys; connectors reference them by that key. A convention is **suggested but not enforced**: `dev`, `preprod` (staging), `prod`. The CLI does not validate the names against a closed set, so unusual topologies stay expressible.

Rationale for banning raw shell strings in connector fields: a manifest can live in a shared repo; a committed field that silently injects executable commands into an agent is a code-execution-by-commit vector. Typed connectors keep the trusted interpretation on the machine side. Local commands are handled separately and explicitly (1.10) so that the one executable surface is visible and guarded rather than hidden in a connector.

### 1.7 Secrets

- The manifest **never contains secret values**. It references where credentials live, typically by environment variable name (`"password_env": "ACME_DB_PASSWORD"`).
- External commands differ in how they authenticate, so tracker sources and hosting instances may carry an **optional `auth` declaration** whose `mode` matches what the command supports. Secret values are forbidden in every mode; only references and mode names appear in the file. Modes shown in the docs (set not closed):
  - `preconfigured` (the default when `auth` is absent): the external CLI is assumed already configured on the machine, and authentication stays entirely in the user's hands. `glab` notably can be authenticated against several GitLab instances ahead of time.
  - `env`: credentials live in environment variables referenced by name (`"token_env": "GITLAB_AGENCY_TOKEN"`, `"api_key_env"`, `"password_env"`...). The agent may drive the login or re-login process itself, passing variables by name and never reading their values.
  - `interactive`: the agent may start the CLI's login flow but hands over to the user to complete it.
- **Hard stop on unreachable authentication (core behavior).** If the user requests an action through an external CLI and the agent has no way to connect without reading secret values itself, it must stop entirely and ask the user how to proceed. This is a built-in behavior of the usage skill, not a per-project rule: it applies even when the manifest declares nothing about it. The CLI's `doctor` check (see 2.1) supports this by listing the declared variable *names* that are missing from the environment, never their values.
- The setup skill may ask the user for variable *names* and where they are defined, but must never read or echo their values.
- Practical consequence to surface to users (didactics): agents inherit their environment at startup, so a newly defined variable requires restarting the agent.

### 1.8 IF-THEN intent rules and guardrails

A declarative routing table from intents to approaches:

```jsonc
{
  "rules": [
    { "if": "database access is needed", "then": "use the 'db-inspect' skill" },
    { "if": "prod logs are needed", "then": "use the logs connector; never ssh to prod directly" }
  ]
}
```

- Rules are plain declarations (condition and instruction, both readable by the LLM). AI Manager does not install, verify, or execute the referenced skills or commands; the agent resolves them itself.
- This doubles as a place for project-specific guardrails ("never X on prod").
- **Structured `access` marker (decided).** For guardrails worth checking mechanically, a lightweight `"access": "read-only"` may be declared directly on a connector (for example the prod database). The IF-THEN rules stay the place for informal guardrails; the structured marker is what tooling and the agent can check without parsing prose. Full per-operation permission systems are out of scope for v1.

### 1.9 Local overlay (personal, gitignored)

A `.aimanager.local.jsonc` sitting **beside** the resolved `.aimanager.jsonc` (same directory) provides a personal, gitignored overlay for private paths, personal preferences, personal `auth` overrides, and personal `local_commands` (see 1.10). Compatibility rules (decided):

- **Deep merge, key by key.** Local values override committed values; local-only keys are added.
- **Same directory only.** The overlay applies only to the manifest it sits next to; it does not merge across the walk-up chain, keeping the nearest-wins/no-inheritance rule of 1.2 intact.
- **Same safety rules as the base file.** No secret values anywhere; no executable strings outside the typed `local_commands` category; validated identically to the committed manifest.
- **Version must be compatible** with the base manifest; a mismatch is a validation error.
- **Personal, never shared.** It is gitignored and never travels between machines. Because it is self-authored and private, it is the preferred home for `local_commands`.

### 1.10 Local commands (acting locally, not only on external tools)

AI Manager is not restricted to external systems. Reading logs or acting on a project sometimes goes through local tooling (a `docker compose` invocation, a make target). The manifest may declare a typed `local_commands` category the CLI can list, so the agent discovers "how to do X locally on this project" the same way it discovers a connector. Each entry is a typed `{ "run": <command>, "label": <human description> }`. Decisions:

- **Explicit and typed, never hidden.** Local commands live in their own `local_commands` category, not smuggled into connector fields. Making the one executable surface visible is what lets it be guarded.
- **Guarded by the trust model.** Because a command string is executable, a `local_commands` block in a *committed* manifest is subject to confirmation-on-first-use with change detection (see section 5). A self-authored block belongs in the gitignored local overlay (1.9), where there is no third-party trust question at all.
- **Listable, not auto-run.** The CLI lists local commands on request; whether and when to run one is the agent's decision, exactly as with connectors. Where an ecosystem cannot auto-trigger the usage skill, the user can add a line to their `AGENTS.md` (or equivalent) telling the agent to list local commands via the CLI.

## 2. The CLI

### 2.1 Role

The single entry point between manifests and agents. Responsibilities:

- **resolve**: locate the manifest from the cwd (walk-up). If none is found, fail with an explanatory error (see 1.2).
- **query**: return the slice of context relevant to a stated need ("logs for prod", "tracker for time logging", "repos and dependencies", "rules matching database"), not the whole file. Exposed as **strict subcommands** (`get logs --env prod`) for testability, plus a `match` helper that accepts a looser, agent-friendly phrasing and maps it to a subcommand (decided).
- **validate**: syntax, schema version, referential coherence (a role target pointing to a declared source, `depends_on` pointing to declared repos, an overlay version compatible with its base). Checking that referenced environment variable names exist is machine-dependent by nature and belongs in the `doctor` check below, not in manifest validation.
- **doctor**: a machine-side check that reports which declared credential-reference variable **names** are missing from the current environment - **names only, never values**. This is what powers the hard stop of US-3.4 and the team-onboarding case (US-1.13): a colleague who just cloned the repo, or anyone hitting an auth dead-end, gets the exact list of variables to define without any secret ever being read.
- Possibly: **summary** (a compact capability inventory used by skills as a cheap first look).

### 2.2 Output design (token efficiency)

- Output is written for an LLM consumer: compact structured text, stable field order, no decorative noise.
- **Format (decided): compact markdown by default, `--json` for a structured machine shape.** The exact markdown shape is to be benchmarked against real agent consumption; the flag keeps a strict structured option available without guessing the winner up front.
- Scoped queries are the default; "dump everything" exists but is the exception.
- A `summary` output small enough to be loaded eagerly lets the skill decide in a few tokens whether deeper queries are worth it: progressive discovery applies to the CLI, not only to skills.

### 2.3 Distribution

- **Single static binary in Rust (decided).** It installs as one command on `$PATH` with no runtime dependency, the lowest common denominator every agent ecosystem can call. Rust gives a single static binary, a solid JSON-Schema ecosystem for validation, and no interpreter to ship.
- **Install via `curl | installer.sh` (decided).** The bootstrap installer fetches the binary and, in the same run, asks which agent ecosystem(s) to install the generic skills for, then places them in each ecosystem's location (see 3). Skill distribution is thus part of the install step, delivering the "installed once per machine" promise.

## 3. Skills

### 3.1 Usage skill

- **Trigger**: manually, or automatically when the user's question requires external data (tickets, logs, database, docs) or a declared local command. The skill description must be written so ecosystems with model-driven skill selection activate it on such questions. **Where an ecosystem has no auto-trigger mechanism (decided), the fallback is manual invocation**; the user may add a short line to their `AGENTS.md` (or equivalent) pointing the agent at the CLI (for example, to list local commands).
- **Flow**: check a manifest exists (cheap resolve) -> query the relevant slice -> act on the returned connectors, roles, and rules -> answer.
- **Built-in guardrail**: if an action requires an external CLI the agent cannot authenticate to without reading secret values itself, it stops entirely and asks the user how to proceed (see 1.7), using `doctor` to name the missing variables.
- **Trust check**: before acting on a committed manifest it has not seen (or one whose content changed), the skill confirms with the user on first use (see section 5).
- **Manifest evolution feedback (decided).** When the skill hits a gap (a needed connector or fact the manifest does not declare), it **proposes a concrete manifest edit for the user to accept and never applies one silently**. The user stays in control of what the manifest gains.
- **Progressive discovery**: the always-loaded surface is a short description plus the instruction to call the CLI. Everything else (connector semantics, edge cases) lives in deeper reference files or in CLI output, loaded only when the task requires it. When the user's question needs no external data, the cost is near zero.
- **Tool guidance notes**: the core is tool-agnostic, but the skills ship a strict minimum of per-tool guidance for popular CLIs (`gh`, `glab` and similar): the few non-obvious behaviors needed to use each tool properly (multi-instance authentication, host selection, useful output flags). One small reference note per tool, loaded through progressive discovery only when that tool is actually involved; never a full manual.

### 3.2 Setup skill

A guided, didactic process to bootstrap a manifest in any folder (repo or not):

1. **Analyze first.** Inspect the current folder: single repo, multi-repo galaxy, plain directory; detect hints (`.git`, CI configs, docker-compose services, existing docs folders) to pre-fill the interview.
2. **Interview.** Ask where the project's information lives: trackers (which, URLs, which project, which role each plays), repos, environments, log access, data stores, documentation. Allow free-form detail for complex cases (multiple trackers, lifecycle, galaxies). Never read secret values (names and locations only).
3. **Write and validate** the manifest, with comments explaining each section.
4. **Offer tool assistance.** Propose installing or configuring external CLIs the manifest relies on (Redmine CLI, GitLab CLI, GitHub CLI), with the user's consent.
5. **Explain.** State what was created, what works now, and what the user must do (define the variable in their shell profile, restart the agent so it picks up the environment, restart a service). Didactic tone, in the user's language.

**Joining an existing manifest (decided).** When a colleague clones a repo that already carries a committed manifest, there is no separate wizard: the mechanism is the `doctor` check. On hitting a missing access (an undeclared or unset variable), the agent stops, runs `doctor` to list the declared variable **names** absent from its environment, and presents that list for the colleague to fill in themselves. The agent never reads the values; it only surfaces the names to provide.

### 3.3 Automation-discovery skill

- Reviews the project and the manifest to suggest opportunities that are not yet covered: undeclared data sources it can detect, repetitive manual steps mentioned by the user, missing IF-THEN rules, tools worth installing.
- **Scope (decided): manifest improvements plus clearly-flagged ecosystem-level automation suggestions (hooks, scheduled jobs), never applied automatically.** The two are kept visibly separate so the declarative core stays clean; the ecosystem suggestions are proposals the user implements, not actions the skill takes.
- Output is a proposal list the user validates; accepted manifest items translate into manifest updates or setup actions.

### 3.4 Conversation QA skill (internal, not shipped)

A maintainer-only skill for the QA of AI Manager itself. It lives in this repository but is never delivered to end users.

- **Input**: a conversation transcript provided by the maintainer, in the format of the ecosystem that produced it (formats differ per agent; identifying the ecosystem is the skill's first step).
- **Detection**: it walks the conversation and flags every inefficiency, including at least: failed or retried commands; workarounds invented because a declared fact was missing or wrong; questions to the user that the manifest should have answered; the whole manifest dumped where a scoped query would have done; wrong targets (ticket in the wrong tracker, time booked on the wrong source, merge request on the wrong instance, wrong environment); authentication dead-ends; ignored rules or guardrails.
- **Root-cause classification**: each finding is attributed to one of: (a) project-side, the manifest is wrong or incomplete at a specific spot; (b) AI Manager-side, a gap in the schema, the CLI output, a skill's wording, or a missing tool guidance note; (c) external, outside AI Manager's scope.
- **Output**: a findings report with, for each finding, a concrete remediation proposal (a manifest edit, an AI Manager change naming the affected component, or a tool guidance note to add).
- **Transcript handling (decided).** The skill is maintainer-only and run locally by the maintainer, who is responsible for their own usage. No anonymization is imposed by the skill; reports stay local. (An end-user-facing tool would need a different stance, but this skill is never shipped.)

### 3.5 Explanatory guidance (cross-cutting)

All setup and management skills must:

- explain **why** each step matters, not only what to run;
- warn about state that does not reload itself: environment variables need an agent restart, some tools need a shell restart or re-login;
- communicate in the user's language (runtime), while all skill files themselves are written in English.

## 4. Agent-agnosticism

- **Contract**: any ecosystem that can (a) run a CLI and (b) follow markdown instructions can use AI Manager. That covers Claude Code, Codex, Gemini CLI, OpenCode, and most others.
- Skills are authored once in plain markdown with minimal frontmatter, then adapted to each ecosystem's convention (skills, custom prompts, rules files). **Auto-trigger capabilities differ; the usage skill defines an auto-trigger mode, and where an ecosystem does not support one the fallback is manual invocation (decided).** The user may wire a pointer into their `AGENTS.md` or equivalent so the agent knows to call the CLI (for example to list local commands).
- Nothing in the manifest is agent-specific: it describes the project, not the agent. This includes local commands (1.10), which describe how to act on *this project's* local tooling, not on any particular agent.

## 5. Security and trust summary

- No secret values in manifests, in overlays, in CLI output, or in conversations (variable names only).
- No executable strings in connector fields; the one executable surface, `local_commands` (1.10), is explicit and guarded.
- **Confirmation on first use, with change detection (decided).** A manifest found in a shared repo is third-party input to the agent: its rules, comments, and any `local_commands` are instructions the user's agent will read and might act on. Before acting on a committed manifest for the first time - and again whenever its content changes (detected by hash) - the usage skill confirms with the user. Self-authored content in the gitignored local overlay (1.9) is exempt, since there is no third party to distrust.
- Placing the manifest in a parent directory keeps it fully private when the repo cannot or should not carry it.
