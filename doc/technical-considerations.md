# AI Manager - Technical Considerations

This document details the technical topics behind the user stories. Nothing here is final; open questions are tracked in [notes.md](notes.md).

## 1. The manifest

### 1.1 Format and name

- **JSONC** (JSON with comments): declarative, diff-friendly, and commentable so the file can carry its own description and per-entry explanations.
- File name: `.aimanager.jsonc` (decided).
- A top-level `version` field from day one, so the schema can evolve without breaking existing manifests.
- A published JSON Schema for editor completion and CLI validation.
- The complete schema is frozen by a commented golden example, [reference-manifest.jsonc](reference-manifest.jsonc), which exercises every feature once. Until the JSON Schema exists, the example is the normative source when it and the prose disagree.

### 1.2 Resolution

- The CLI resolves the manifest by **walking up the directory tree** from the current working directory until it finds one, exactly like `.git` discovery.
- This makes placement flexible by construction:
  - committed at the repo root for projects where that is acceptable;
  - in a **parent directory** of the repo for client codebases that must not carry tooling files;
  - in any plain folder: no git required.
- Working assumption: the nearest manifest wins. Whether a nearer manifest should also be able to delegate to or inherit from an outer one is an open question (relevant for galaxies, see 1.4 and notes #3).

### 1.3 Trackers: multiple sources, one write target

Real projects break the "one project = one tracker" assumption in two ways:

1. **Two organizations, two trackers.** Our Redmine plus the client's Redmine, Jira, or Linear, both legitimate sources for the same project.
2. **Lifecycle moves.** A project starts with its own dedicated project space inside the tracker; once in production it moves into a general maintenance space shared by all live projects (what French agencies call TMA, "tierce maintenance applicative", third-party application maintenance).

Proposed model:

```jsonc
{
  "trackers": {
    "sources": {
      "internal": { "type": "redmine", "url": "https://redmine.agency.example", "project": "acme-build" },
      "client": { "type": "jira", "url": "https://acme.atlassian.net", "project": "ACME" }
    },
    // Where new tickets go today.
    "write_target": { "source": "internal", "project": "acme-build" }
    // After go-live, moving to the shared maintenance space means
    // replacing the write_target line above with:
    // "write_target": { "source": "internal", "project": "maintenance-general" }
  }
}
```

- `sources` answers "where can tickets about this project live" (read side).
- `write_target` answers "where do new tickets go right now" (write side) and can point to a different project space than the source's default, covering the lifecycle case with a one-line change (see the commented line in the example).
- Optional refinement: per-category routing (bugs to the client's Jira, internal chores to our Redmine). Deferred until a real case demands it (see notes #6).

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
- Open question: whether sub-repos may also carry their own manifest and how the two compose (see notes #3).

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

- `instances` declares each hosting endpoint once, with `roles` stating which operations belong to it (open merge requests, watch test pipelines, view and trigger deployment pipelines).
- Each repo maps instance ids to its path on that instance.
- With roles declared, "create the MR" routes to `dev` and "trigger the deployment" routes to `ops`, without any per-request instruction. A single-instance project declares one instance holding every role.

### 1.6 Data-source connectors

Connectors are **typed and declarative**: the manifest states what exists and how to reach it; it never embeds arbitrary shell commands. The agent (guided by the usage skill) decides what to run.

Candidate connector types for v1, driven by actual needs:

| Type | Declares | Example use |
|------|----------|-------------|
| `ssh` | host, optional user/jump host | reach a preprod box |
| `logs` | transport (ssh/file), systemd journal unit or file path; nested under its environment | read prod logs |
| `database` | engine (mysql, postgres...), host, port, database, credential reference | inspect data |
| `elasticsearch` / `solr` / `redis` | endpoint, index/core/db, credential reference | query search or cache layers |
| `docs` | local path or remote URL/repo | find the functional documentation |
| `tracker` | see 1.3 | read and create tickets |
| `code_hosting` | see 1.5 | merge/pull requests, pipelines |

Rationale for banning raw shell strings in the manifest: a manifest can live in a shared repo; a committed file that injects executable commands into an agent is a code-execution-by-commit vector. Typed connectors keep the trusted interpretation on the machine side.

### 1.7 Secrets

- The manifest **never contains secret values**. It references where credentials live, typically by environment variable name (`"password_env": "ACME_DB_PASSWORD"`).
- Tracker and code hosting sections carry no credential references at all: authentication there is delegated to the external CLIs configured on the machine (Redmine CLI, GitLab CLI, GitHub CLI; the setup skill offers to install them, see 3.2). Only data stores that agents query directly reference credentials, by environment variable name.
- The setup skill may ask the user for variable *names* and where they are defined, but must never read or echo their values.
- Practical consequence to surface to users (didactics): agents inherit their environment at startup, so a newly defined variable requires restarting the agent.

### 1.8 IF-THEN intent rules

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

## 2. The CLI

### 2.1 Role

The single entry point between manifests and agents. Responsibilities:

- **resolve**: locate the manifest from the cwd (walk-up).
- **query**: return the slice of context relevant to a stated need ("logs for prod", "tracker write target", "repos and dependencies", "rules matching database"), not the whole file.
- **validate**: syntax, schema version, referential coherence (a `write_target` pointing to a declared source, `depends_on` pointing to declared repos). Checking that referenced environment variable names exist (names only, never values) is machine-dependent by nature and belongs in a separate `doctor`-style check rather than in manifest validation.
- Possibly: **summary** (a compact capability inventory used by skills as a cheap first look).

### 2.2 Output design (token efficiency)

- Output is written for an LLM consumer: compact structured text, stable field order, no decorative noise.
- Scoped queries are the default; "dump everything" exists but is the exception.
- A `summary` output small enough to be loaded eagerly lets the skill decide in a few tokens whether deeper queries are worth it: progressive discovery applies to the CLI, not only to skills.

### 2.3 Distribution

- Must be installable as a single command on `$PATH`: the lowest common denominator every agent ecosystem can call.
- Implementation language and packaging are open (see notes); the constraint is zero-friction install and no runtime dependency surprises.

## 3. Skills

### 3.1 Usage skill

- **Trigger**: manually, or automatically when the user's question requires external data (tickets, logs, database, docs). The skill description must be written so ecosystems with model-driven skill selection activate it on such questions.
- **Flow**: check a manifest exists (cheap resolve) -> query the relevant slice -> act on the returned connectors and rules -> answer.
- **Progressive discovery**: the always-loaded surface is a short description plus the instruction to call the CLI. Everything else (connector semantics, edge cases) lives in deeper reference files or in CLI output, loaded only when the task requires it. When the user's question needs no external data, the cost is near zero.

### 3.2 Setup skill

A guided, didactic process to bootstrap a manifest in any folder (repo or not):

1. **Analyze first.** Inspect the current folder: single repo, multi-repo galaxy, plain directory; detect hints (`.git`, CI configs, docker-compose services, existing docs folders) to pre-fill the interview.
2. **Interview.** Ask where the project's information lives: trackers (which, URLs, which project, where tickets should be created), repos, environments, log access, data stores, documentation. Allow free-form detail for complex cases (multiple trackers, lifecycle, galaxies). Never read secret values (names and locations only).
3. **Write and validate** the manifest, with comments explaining each section.
4. **Offer tool assistance.** Propose installing or configuring external CLIs the manifest relies on (Redmine CLI, GitLab CLI, GitHub CLI), with the user's consent.
5. **Explain.** State what was created, what works now, and what the user must do (define the variable in their shell profile, restart the agent so it picks up the environment, restart a service). Didactic tone, in the user's language.

### 3.3 Automation-discovery skill

- Reviews the project and the manifest to suggest automation opportunities that are not yet covered: undeclared data sources it can detect, repetitive manual steps mentioned by the user, missing IF-THEN rules, tools worth installing.
- Output is a proposal list the user validates; accepted items translate into manifest updates or setup actions.

### 3.4 Explanatory guidance (cross-cutting)

All setup and management skills must:

- explain **why** each step matters, not only what to run;
- warn about state that does not reload itself: environment variables need an agent restart, some tools need a shell restart or re-login;
- communicate in the user's language (runtime), while all skill files themselves are written in English.

## 4. Agent-agnosticism

- **Contract**: any ecosystem that can (a) run a CLI and (b) follow markdown instructions can use AI Manager. That covers Claude Code, Codex, Gemini CLI, OpenCode, and most others.
- Skills are authored once in plain markdown with minimal frontmatter, then adapted to each ecosystem's convention (skills, custom prompts, rules files). Ecosystem-specific auto-trigger capabilities differ; where automatic activation is unavailable, a degraded mode must be defined per ecosystem, with manual invocation as the minimum (see notes #12).
- Nothing in the manifest is agent-specific: it describes the project, not the agent.

## 5. Security and trust summary

- No secret values in manifests, in CLI output, or in conversations (variable names only).
- No executable strings in manifests; typed connectors keep interpretation in trusted code.
- A manifest found in a shared repo is third-party input to the agent: rules and comments are instructions the user's agent will read, which is worth keeping in mind when working on repos with many committers (possible future hardening: confirmation on first use per manifest).
- Placing the manifest in a parent directory keeps it fully private when the repo cannot or should not carry it.
