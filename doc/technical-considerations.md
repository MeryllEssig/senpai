# AI Manager - Technical Considerations

This document details the technical topics behind the user stories. The design decisions below are settled; [notes.md](notes.md) tracks any topic still left open plus cross-cutting decisions with no home here.

## 1. The manifest

### 1.1 Format and name

- **JSONC** (JSON with comments): declarative, diff-friendly, and commentable so the file can carry its own description and per-entry explanations.
- File name: `.aimanager.jsonc` (decided).
- A top-level `version` field from day one, so the schema can evolve without breaking existing manifests.
- A manifest may declare the stable v1 `$schema` URI for editor completion;
  this is shown in the golden example. `version` remains the compatibility key,
  not the URI.
- **Compatibility policy.** A v1 CLI accepts only `version: 1`; a manifest
  from a later version fails explicitly rather than being interpreted
  permissively. Future migrations are explicit, opt-in CLI commands that write
  a backup before changing a manifest; no command silently upgrades it.
- A top-level `project` identity block (`name` slug, `label`, a free-text `context` for the agent, free-form `stack` hints) says what the project is; every other section declares how to reach its ecosystem.
- A published JSON Schema for editor completion and structural CLI validation:
  [`schema/aimanager.schema.json`](../schema/aimanager.schema.json). It is
  Draft 2020-12 and is the normative machine-readable contract for v1.
- The complete schema is exercised by a commented golden example,
  [reference-manifest.jsonc](reference-manifest.jsonc). The JSON Schema is
  normative for structure; this document is normative for behavior and the
  golden example is normative for comments and worked composition.
- The schema validates local structure. The CLI additionally validates the
  cross-references and execution semantics that JSON Schema cannot express
  (for example repo dependencies, placeholder correspondence, role selection,
  and a safe capsule template).
- The repository's dependency-free reference check is
  `node scripts/verify-reference.mjs`; it parses the golden JSONC examples and
  asserts the cross-references exercised by them. The future Rust CLI remains
  the authoritative full validator.

### 1.2 Resolution

- The CLI resolves the manifest by **walking up the directory tree** from the current working directory until it finds one, exactly like `.git` discovery. The walk goes **upward only**: manifests in subdirectories are never discovered or consulted.
- This makes placement flexible by construction:
  - committed at the repo root for projects where that is acceptable;
  - in a **parent directory** of the repo for client codebases that must not carry tooling files;
  - in any plain folder: no git required.
- **Session-anchored (decided 2026-07-17).** The manifest resolved from the **session's launch directory** is the only one read and applies to the whole session, all subdirectories included. The usage skill instructs the agent to run the CLI from the launch directory and to stop if resolution fails; the agent never `cd`s into a subdirectory to pick up a different manifest, so the context cannot silently switch mid-session.
- **CLI mechanism.** `aimanager resolve` returns the canonical absolute manifest
  path. The usage skill retains that path for the session and passes it through
  `--manifest <absolute-path>` on every later invocation. Commands without
  `--manifest` retain the walk-up behavior for human use; they do not provide
  the session guarantee on their own.
- **Nearest-wins upward, no inheritance (decided).** When the launch directory and an ancestor both carry a manifest, the nearest one wins and is used as-is and in full, never merged with or inheriting from an outer one. A galaxy root and a sub-project may each carry their own manifest, but they stay independent (a sub-project manifest applies only to sessions launched inside the sub-project). There is no `extends` mechanism between manifests.
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
- A source may carry an optional `auth` declaration (see 1.7) and an optional `skill` declaration overriding the default type-to-skill mapping (see 3.1).

**Resolving a bare ticket number** (`#1234`) when several trackers are declared (decided):

1. **Pattern first.** Self-disambiguating ids (a Jira key such as `ACME-123`) route to the source whose optional `ticket_id_patterns` regex matches. A platform skill may provide an implicit pattern only for a documented native format; custom source types must declare one.
2. **Explicit priority next.** For a genuinely bare number, consider only sources that declare a numeric pattern and sort them by the optional integer `priority` (lower first). Missing priority sorts after every explicit value. JSON object insertion order is never routing data.
3. **Ask when still ambiguous.** If the best candidates have the same priority, or more than one source plausibly owns the identifier after lookup, stop and ask the user rather than guess.

For any role-driven action, candidates are the declared sources or instances with that role. One candidate routes directly; zero candidates is a clear "capability not declared" error; more than one is an ambiguity error unless the command explicitly names the source or instance. This rule also applies to code-hosting roles. A selected hosting instance must be present in the target repo's `hosting` map, otherwise the CLI reports that the capability is not available for that repo.

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

- `instances` declares each hosting endpoint once, with `roles` stating which operations belong to it (open merge requests, watch test pipelines, view and trigger deployment pipelines). An instance may carry an optional `auth` declaration (see 1.7); without one, the platform CLI is assumed to be pre-configured for that instance. Like a tracker source, an instance may carry an optional `skill` declaration overriding the default platform-to-skill mapping (see 3.1).
- Each repo maps instance ids to its path on that instance.
- With roles declared, "create the MR" routes to `dev` and "trigger the deployment" routes to `ops`, without any per-request instruction. A single-instance project declares one instance holding every role.
- See 1.3 for duplicate-role and missing-repo-hosting behavior; the CLI never resolves either condition by map order.

### 1.6 Data-source connectors

Connectors are **typed and declarative**: the manifest states what exists and how to reach it; it never embeds arbitrary shell commands in connector fields. The agent (guided by the usage skill) decides what to run. The one place command strings may appear is the explicit `local_commands` category (see 1.10), which is opt-in and vetted by the team like any committed code.

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

**Executing a connector (decided).** A connector declares *what exists and where*; turning it into a runnable invocation is normally the agent's job. Two paths coexist:

- **The agent forms the call itself** (the non-secret / non-wrapped case), referencing credential variables by name only (for example `mysql -h ... -u ... -p"$ACME_PROD_DB_PASSWORD" ...`), guided by the optional `note` field below and by the shipped per-tool skills (3.1).
- **A capsule**, when a secret must stay out of the transcript: the invocation is declared as a structured capsule (1.11) whose `command` template AI Manager runs itself, in its own process, returning only the result. This is the successor to the earlier "wrapper" idea (a hand-written script the agent called as a black box): the capsule is that same protection made structured and declarative rather than an opaque script. See 1.11.

**Optional `note` on any connector (decided).** Any connector may carry a free-text `"note"` giving the agent the non-obvious hint needed to use it: a specific flag, the capsule to prefer, a caveat. It is optional, LLM-addressed, and loaded only when that connector is queried.

**Optional `repo` on any connector or environment (decided 2026-07-17).** In a galaxy, a connector often serves one member (the billing API's database, one service's logs). An optional `"repo"` field naming a declared repo member links the two, so the CLI can answer "which connectors serve `api_billing`". Sections stay flat (free-form ids like `billing-prod` remain the rule); small projects are unaffected. A documentation source uses `repository_url` for a remote documentation repository, never `repo`, so the two meanings cannot collide.

**Extensibility: open enums with a generic fallback (decided).** The `type` of a data-source connector, the `type` of a tracker source, and the `platform` of a code-hosting instance are **open enums**. Known data stores have a strict v1 shape: database = `engine`, `host`, `port`, `database`; Elasticsearch = `endpoint`, `index`; Solr = `endpoint`, `core`; Redis = `endpoint`, `db`. An unknown data-store type must declare at least `endpoint` or `host`; tool-specific descriptive fields are allowed and are surfaced unchanged. `validate` may *warn* on an unrecognized value (a likely typo) but never *fails* on it. This keeps the system expressive as tools churn (a MongoDB, a Kafka, an S3 bucket) without waiting for a CLI release.

**Environments taxonomy (decided).** Environment ids are free-form map keys; connectors reference them by that key. A convention is **suggested but not enforced**: `dev`, `preprod` (staging), `prod`. The CLI does not validate the names against a closed set, so unusual topologies stay expressible.

Rationale for banning raw shell strings in connector fields: keeping connectors purely typed is what lets the CLI answer precise queries about them, and it keeps the agent-run executable surface in one visible, discoverable place (`local_commands`, 1.10) instead of scattered across connector fields. Vetting the safety of declared commands is the team's job, like any committed code (see section 5).

### 1.7 Secrets

- The manifest **never contains secret values**. It references where credentials live, typically by environment variable name (`"password_env": "ACME_DB_PASSWORD"`).
- **Secret values may live in one place: the never-committed local capsule-values file** (`.aimanager/capsules.local.json`, gitignored; see 1.11). Beyond env-var references, that file may hold literal secret values, but only for capsules, and only consumed **inside AI Manager's own process** when it runs the capsule. The "agent never reads secret values" behavior is preserved because AI Manager returns the declared template and scrubbed command result, never the resolved command line. The manifest itself still holds no secret values.
- External commands differ in how they authenticate, so tracker sources and hosting instances may carry an **optional `auth` declaration** whose `mode` matches what the command supports. Secret values are forbidden in every mode; only references and mode names appear in the file. Modes shown in the docs (set not closed):
  - `preconfigured` (the default when `auth` is absent): the external CLI is assumed already configured on the machine, and authentication stays entirely in the user's hands. `glab` notably can be authenticated against several GitLab instances ahead of time.
  - `env`: credentials live in environment variables referenced by name (`"token_env": "GITLAB_AGENCY_TOKEN"`, `"api_key_env"`, `"password_env"`...). The agent may drive the login or re-login process itself, passing variables by name and never reading their values.
  - `interactive`: the agent may start the CLI's login flow but hands over to the user to complete it.
- **Hard stop on unreachable authentication (core behavior).** If the user requests an action through an external CLI and the agent has no way to connect without reading secret values itself, it must stop entirely and ask the user how to proceed. This is a built-in behavior of the usage skill, not a per-project rule: it applies even when the manifest declares nothing about it. When a command fails, the agent diagnoses the failure from its error output; `doctor` does not inspect the environment, test credentials, or contact the remote service.
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
- **`then` is free text the agent maps (decided).** The instruction is prose; the agent maps it to whatever skill or capability its ecosystem offers, or acts directly on the referenced connector when none matches. A skill named in a rule (`'db-inspect'`) **need not exist** as an installed skill - if it is absent, the agent falls back to the rule's intent and the connector. No skill registry, no name coupling: this keeps rules agent-agnostic (skills are installed once per machine, not per project).
- This doubles as a place for project-specific use cases, instructions, and guardrails ("never X on prod", "ask before creating a client ticket").
- **The resolved configuration is the authorization boundary (decided 2026-07-20).** A connector, capsule, or local command present in the merged AI Manager configuration is authorized for the agent to use. AI Manager adds no global confirmation for sensitive reads, remote writes, deployments, or declared local commands. When a project wants confirmation or a narrower usage policy, its author expresses that policy in `rules`, `note`, or the entry's prose. This authorization does not create permissions in the target service; the configured account and remote system still decide what the command can actually do.
- **Optional structured `access` hint (decided 2026-07-20).** A lightweight `"access": "read-only"` may be declared on a connector or capsule as information for the LLM. It is optional and purely advisory: the CLI returns it when present but never interprets or enforces it, including during `aimanager run`. It is not a sandbox, permission, or confirmation mechanism.

### 1.9 Local overlay (personal, gitignored)

A `.aimanager.local.jsonc` sitting **beside** the resolved `.aimanager.jsonc` (same directory) provides a personal, gitignored overlay for private paths, personal preferences, personal `auth` overrides, and personal `local_commands` (see 1.10). Compatibility rules (decided):

- **Deep merge, key by key.** Local values override committed values; local-only keys are added.
- **Exact merge semantics.** Objects merge recursively. Scalars and arrays
  replace the base value as a whole; arrays never concatenate. `null` has no
  deletion meaning and is invalid wherever the base schema does not allow it.
  An overlay is validated after merge against the normal manifest schema, so a
  partial nested override is legal only when the resulting manifest is valid.
- **Same directory only.** The overlay applies only to the manifest it sits next to; it does not merge across the walk-up chain, keeping the nearest-wins/no-inheritance rule of 1.2 intact.
- **Same safety rules as the base file.** No secret values anywhere; no executable strings outside the typed `local_commands` category; validated identically to the committed manifest.
- **Version must be compatible** with the base manifest; a mismatch is a validation error.
- **Personal, never shared.** It is gitignored and never travels between machines. Team-shared `local_commands` belong in the committed manifest; the overlay is the home for personal ones.
- **Never a home for secrets.** The overlay deep-merges into the manifest, so the agent reads it like the manifest itself. This is why capsule values live in a separate file the agent never reads (`.aimanager/capsules.local.json`, 1.11) rather than in the overlay.

### 1.10 Local commands (acting locally, not only on external tools)

AI Manager is not restricted to external systems. Reading logs or acting on a project sometimes goes through local tooling (a `docker compose` invocation, a make target). The manifest may declare a typed `local_commands` category the CLI can list, so the agent discovers "how to do X locally on this project" the same way it discovers a connector. `local_commands` covers the **no-secret** case: there is nothing to hide, so the **agent** runs the command (contrast the capsule of 1.11, which AI Manager runs itself because a secret is involved). Each entry is a typed `{ "run": <command>, "label": <human description> }`. Decisions:

- **Explicit and typed, never hidden.** Local commands live in their own `local_commands` category, not smuggled into connector fields. The one agent-run executable surface stays visible and discoverable.
- **Vetted by the team, no trust machinery (decided 2026-07-17).** Ensuring declared commands are safe is the developers' responsibility, like any committed code; AI Manager ships no confirmation or change-detection mechanism (see section 5). Team-shared commands live in the committed manifest; personal ones in the gitignored local overlay (1.9).
- **No `cwd` field (decided 2026-07-17).** When a command must run from a specific directory (a galaxy member's compose file), the `label` states it in prose and the agent handles the `cd` itself. The schema stays minimal; the information lives where the agent reads anyway.
- **Listable, not auto-run.** The CLI lists local commands on request; whether and when to run one is the agent's decision, exactly as with connectors. Where an ecosystem cannot auto-trigger the usage skill, the user can add a line to their `AGENTS.md` (or equivalent) telling the agent to list local commands via the CLI.
- **Boundary with capsules (decided).** The role of "a wrapper around a connector" is now the **capsule** (1.11), not an opaque hand-written script: when the invocation carries a secret that must stay out of the transcript, it is a capsule AI Manager runs itself. `local_commands` keeps only the no-secret local tooling the agent runs directly. Boundary criterion: "is there a secret to hide, so must AI Manager run it?" If no, it is a `local_commands` entry; if yes, it is a capsule (1.11).

### 1.11 Capsules (secret-hiding commands AI Manager runs)

A **capsule** is a one-shot command that **AI Manager executes itself, in its own process**, for one purpose: to keep a secret out of the agent's transcript. It is the single, deliberate exception to "AI Manager runs nothing" (goal.md objective 2), and it revises the earlier "wrapper" path of 1.6 into a structured, declarative form. The name is final (decided 2026-07-17; the concept was designed 2026-07-13 under the working name "contract"). Decisions:

- **Capsules exist to hide secrets by running the command themselves.** AI Manager reads the local values file inside its own process, resolves the `command` template, runs it via subprocess, and returns the declared template plus scrubbed stdout/stderr to the agent. The resolved command line (which may contain a literal secret) never appears in an agent tool-call, so it never enters the transcript or the model context. This is why "the agent never sees secret values" survives even though a file now holds them: the reason is **transcript-leak prevention**, not third-party trust. The values file is never committed, so there is no third party to distrust.

- **The committed manifest holds the capsule declaration, including the `command` template.** The declaration lives in `.aimanager.jsonc` with `{variable}` placeholders in the template.
  - **Committed / local boundary rule.** Whatever the author writes **literally** in the template is committed; whatever they turn into a `{variable}` placeholder is filled from elsewhere. The author draws the private/shared line field by field simply by this choice.
  - **Who fills each `{variable}`.** Variables declared `supplied` are filled by the **agent** at call time (example: `{query}`, the SQL string). **All other** `{variables}` are filled from the local values file.
  - The manifest also keeps the semantic facts that help the agent reason
    (`connector`, engine, environment, an optional `access: read-only` hint,
    and a `note`). `connector`, when present, is a data-store id and lets
    validation check the capsule's environment metadata against the connector.
    `access` remains advisory and need not be present on either declaration.
    None of these fields holds secret values.

- **The local values file holds, per capsule, the VALUE of each non-supplied `{variable}`** - either a literal value or a `$ENV_VAR` reference. It is plain JSON, never committed, gitignored: `.aimanager/capsules.local.json` (the directory name aligns to the manifest family; note the spelling without a dash). Its shape is frozen by a commented golden example, [reference-capsule.jsonc](reference-capsule.jsonc), paired with the `capsules` block of the [reference manifest](reference-manifest.jsonc). It lives outside the personal overlay (1.9) on purpose: the overlay merges into the manifest and is therefore agent-readable, while this file must never be read by the agent.

- **Execution verb (decided 2026-07-20):** `aimanager run <name> [--param value ...]`. It executes capsules only; `local_commands` remain agent-run. Example: `aimanager run db-preprod --query "SELECT ..."`.

- **Execution is argv-based, no shell (decided 2026-07-17).** The `command` template is split into an argv **once at parse time** (shell-words rules); each `{placeholder}` is then substituted as one atomic argument, and AI Manager executes the argv directly, with no intermediate shell. An agent-supplied value containing quotes, `;` or `$(...)` is just data: injection is impossible by construction. Consequence to document for authors: pipes and redirections are not available inside a template (wrap them in a script the template calls if truly needed).

- **Template-visible, resolved-command-hidden output (decided 2026-07-20).** Every `aimanager run` result, successful or not, includes the declared command template so the agent knows what shape of command was executed. It never includes the resolved command line. Some commands re-echo their arguments on error, which could leak a value on stderr; before returning, AI Manager replaces, in **both stdout and stderr**, every occurrence of every resolved non-supplied value with its `{name}` placeholder. The exit code passes through untouched, so the agent keeps a usable diagnostic while the values stay masked.
- **Bounded, literal transcript protection.** Capsule output is buffered, capped
  by `max_output_bytes` (default 1 MiB), and killed after `timeout_seconds`
  (default 30 seconds). Scrubbing is deterministic longest-value-first literal
  replacement and rejects an empty resolved value. It protects the agent
  transcript from an exact resolved value; it does not protect transformed
  values, data intentionally returned by the remote system, shell history, or
  same-machine process inspection. The capsule threat model is therefore
  transcript-leak prevention, not a general secret vault or data-loss-prevention
  boundary.

- **The frontier - what is NOT a capsule:**
  - A local runnable **without** a secret (a make target, `docker compose logs -f app`) stays a `local_commands` entry (1.10), run by the **agent**, not by AI Manager. Boundary criterion: "is there a secret to hide, so must AI Manager run it?" If no secret, it is a plain local command the agent runs.
  - **Complex external tools** (`glab`, `gh`, Jira, Redmine) are not capsules - too rich to template. Instead AI Manager **ships a skill per popular tool** to help the agent drive it, mapped by default from the manifest's existing `code_hosting.platform` and `trackers.type` and overridable per source via the optional `skill` field; see 3.1 for depth, embedded scripts, and fallback. Auth for these stays agent-side, by name reference (1.7 unchanged for trackers/hosting).
  - **MCP servers**: a lightweight declarative pointer only - an optional `mcp` field on the relevant connector that the usage skill surfaces. No machinery.

- **CLI additions and onboarding** (see 2.1):
  - `init` creates `.aimanager/capsules.local.json` with one stub per
    non-supplied `{variable}`, creates `.aimanager/.gitignore` containing the
    values-file exclusion, and restricts the values file to the current user
    where the OS supports it. It is idempotent and never overwrites an existing
    value or file.
  - `validate` has two scopes: `validate manifest` validates only committed
    material and is suitable for CI; `validate local` additionally checks that
    each non-supplied `{variable}` has a value in the local values file.
  - `doctor` validates the manifest, overlay, and local capsule configuration only. It does not resolve `$ENV` references, inspect installed skills, test credentials, or contact remote systems (see 2.1).
  - Colleague onboarding: clone -> `aimanager init` -> the colleague fills the local values file themselves, out of band -> `validate local` or `doctor` confirms configuration coherence. Runtime access problems are diagnosed by the agent from the scrubbed command error. AI Manager never transfers a value.

- **File topology (flat siblings):**
  - `.aimanager.jsonc` - committed manifest (1.1).
  - `.aimanager.local.jsonc` - personal overlay (1.9).
  - `.aimanager/capsules.local.json` - the local capsule-values file, gitignored.

## 2. The CLI

### 2.1 Role

The single entry point between manifests and agents. Responsibilities:

- **resolve**: locate the manifest from the cwd (walk-up). If none is found, fail with an explanatory error (see 1.2).
- **query**: return the slice of context relevant to a stated need ("logs for prod", "tracker for time logging", "repos and dependencies", "rules matching database"), not the whole file. Exposed as **strict subcommands** (`get logs --env prod`) for testability. A looser `match` helper (fuzzy phrase to subcommand) was considered and **dropped (decided 2026-07-17)**: the consumer is already an LLM; `summary` plus well-written `--help` text is enough to pick the right subcommand, and a keyword matcher would be a fragile component to maintain.
- **summary** (decided 2026-07-17, promoted from "possibly"): a compact inventory of what THIS project declares - sections present, ids, roles, capsules, tool skills to load - in a few dozen tokens. It is the first command the usage skill runs; progressive discovery applies to the CLI, not only to skills.
- **init**: scaffold the local capsule-values file (`.aimanager/capsules.local.json`) from the manifest's capsules - one stub per non-supplied `{variable}` (see 1.11). Onboarding entry point: a colleague clones, runs `init`, then fills the stubs themselves out of band.
- **validate**: syntax, schema version, and referential coherence: declared ids
  are unique; repo paths are relative, normalized, and non-overlapping;
  `depends_on` targets exist and form no cycle; hosting ids exist; environment,
  connector `repo`, and capsule `connector` references exist; and linked capsule
  environment metadata agrees with its connector. The local scope also checks
  **capsule field correspondence** and the syntax of `$ENV` references, but
  never whether those names currently resolve.
- **doctor (decided 2026-07-20)**: a convenience aggregate that validates the resolved manifest, overlay, and local capsule configuration. It checks configuration only. It does not inspect environment-variable availability, installed skills, CLI sessions, credentials, connectivity, remote permissions, or any external service. The agent diagnoses execution failures from the returned error output.
- **run**: execute a named capsule in AI Manager's own process, resolving its template from the local values file and returning the declared template plus the scrubbed result (see 1.11). It is the one place AI Manager runs a command itself and does not execute `local_commands`.

The exact v1 subcommands, selectors, exit behavior and JSON envelope are
defined in [CLI contract](cli-contract.md). They are intentionally small, but
stable enough to make the trackers-first vertical slice independently testable.

### 2.2 Output design (token efficiency)

- Output is written for an LLM consumer: compact structured text, stable field order, no decorative noise.
- **Format (decided): compact markdown by default, `--json` for a structured machine shape.** The JSON envelope and error shape are fixed by the [CLI contract](cli-contract.md); compact markdown remains presentation-oriented and may evolve without changing the data contract.
- Scoped queries are the default; "dump everything" exists but is the exception.
- A `summary` output small enough to be loaded eagerly lets the skill decide in a few tokens whether deeper queries are worth it: progressive discovery applies to the CLI, not only to skills.

### 2.3 Distribution

- **Single Rust binary (decided).** The core installs as one command on `$PATH` with no runtime dependency, the lowest common denominator every agent ecosystem can call. Rust provides a solid JSON-Schema ecosystem for validation. Shipped Redmine helper scripts are a separate optional tool skill and require Python 3.
- **Supported release targets (decided 2026-07-20).** v1 publishes binaries for Linux and macOS on both x86_64 and ARM64. Windows is not a supported v1 target.
- **Install via `curl | installer.sh` (decided).** The bootstrap installer detects the supported OS/CPU pair, downloads the requested release (latest by default), and verifies its published SHA-256 checksum before installing anything. A checksum mismatch is a hard failure.
- **Agent-specific global skill installation (decided 2026-07-20).** In the same run, the installer asks which supported ecosystems to configure and copies the adapted skills directly into each selected ecosystem's canonical user-global directory: `$CODEX_HOME/skills` (default `~/.codex/skills`) for Codex, `~/.claude/skills` for Claude Code, `~/.gemini/skills` for Gemini CLI, and `~/.config/opencode/skills` for OpenCode. The installer records exactly which AI Manager files it owns.
- **Updates rerun the installer (decided 2026-07-20).** The installer is idempotent. Rerunning it replaces the binary and unconditionally overwrites existing AI Manager skills in the selected agent directories; local edits to those shipped skills are not preserved.
- **Uninstall through the installer (decided 2026-07-20).** `installer.sh --uninstall` removes the binary and only the AI Manager skill files recorded as installer-owned. It never removes project manifests, overlays, or capsule-values files. There is no `aimanager update` or self-uninstall command.

## 3. Skills

### 3.1 Usage skill

- **Trigger**: manually, or automatically when the user's question requires external data (tickets, logs, database, docs) or a declared local command. The skill description must be written so ecosystems with model-driven skill selection activate it on such questions. **Where an ecosystem has no auto-trigger mechanism (decided), the fallback is manual invocation**; the user may add a short line to their `AGENTS.md` (or equivalent) pointing the agent at the CLI (for example, to list local commands).
- **Flow**: run `summary` **from the session's launch directory** (see 1.2: if resolution fails, stop and tell the user; never `cd` around hunting for a manifest) -> query the relevant slice -> act on the returned connectors, roles, and rules -> answer.
- **Built-in guardrail**: if an action requires an external CLI the agent cannot authenticate to without reading secret values itself, it stops entirely and asks the user how to proceed (see 1.7). If a command was attempted, the agent diagnoses the failure from its scrubbed error output; `doctor` does not probe authentication.
- **Manifest evolution feedback (decided).** When the skill hits a gap (a needed connector or fact the manifest does not declare), it **proposes a concrete manifest edit for the user to accept and never applies one silently**. The user stays in control of what the manifest gains.
- **Progressive discovery**: the always-loaded surface is a short description plus the instruction to call the CLI. Everything else (connector semantics, edge cases) lives in deeper reference files or in CLI output, loaded only when the task requires it. When the user's question needs no external data, the cost is near zero.
- **Shipped per-tool skills (decided 2026-07-13, refined 2026-07-17).** The core stays tool-agnostic, but AI Manager **ships a skill per popular tool** (GitLab, GitHub, Jira, Redmine) to help the agent drive it. This promotes the former "tool guidance notes" from small reference notes into shipped skills. Each skill is loaded through progressive discovery only when that tool is actually involved; complex external tools are handled this way rather than as capsules (see 1.11), and an optional `mcp` pointer on a connector is surfaced the same way. Refinements (all decided 2026-07-17):
  - **Depth varies by tool.** For CLIs the models already know well (`gh`, `glab`): a strict minimum - multi-instance authentication, host selection, the few non-obvious pitfalls - never a full manual. For tools whose API and workflows the models know less (Redmine, Jira): a full driving skill with progressively loaded references (endpoints, time logging, statuses, pagination).
  - **Embedded scripts where no good CLI exists.** Redmine has no maintained canonical CLI, so the shipped Redmine skill carries a `scripts/` directory of executable helpers driving the full REST API. Scripts are **Python 3, standard library only** (no `pip install`): robust JSON, pagination, and HTTP error handling on any dev machine. Each script reads its credentials from the environment by name **itself**, so the secret never transits through the transcript - the same property capsules provide, obtained naturally.
  - **Mapping: default by type, overridable per source.** With no `skill` field, the declared `trackers.type` / `code_hosting.platform` maps to the shipped skill of that name (`redmine` -> the shipped Redmine skill, `gitlab` -> the `glab` skill); the usage skill surfaces the right one by intent. An optional `skill` field on a source or instance **overrides** the mapping (an in-house tool, a house workflow). When the named skill is unavailable to the agent, it falls back to the generic connector shape plus `note` - the same soft-fallback logic as rules (1.8). `doctor` does not discover or inventory installed skills.

### 3.2 Setup skill

A guided, didactic process to bootstrap a manifest in any folder (repo or not):

1. **Analyze first.** Inspect the current folder: single repo, multi-repo galaxy, plain directory; detect hints (`.git`, CI configs, docker-compose services, existing docs folders) to pre-fill the interview.
2. **Interview.** Ask where the project's information lives: trackers (which, URLs, which project, which role each plays), repos, environments, log access, data stores, documentation. Allow free-form detail for complex cases (multiple trackers, lifecycle, galaxies). Never read secret values (names and locations only).
3. **Write and validate** the manifest, with comments explaining each section.
4. **Offer tool assistance.** Propose installing or configuring external CLIs the manifest relies on (`glab`, `gh`, a Jira CLI), with the user's consent. Redmine needs no install: its shipped skill drives the REST API directly through embedded scripts (3.1).
5. **Explain.** State what was created, what works now, and what the user must do (define the variable in their shell profile, restart the agent so it picks up the environment, restart a service). Didactic tone, in the user's language.

**Joining an existing manifest (decided).** When a colleague clones a repo that already carries a committed manifest, there is no separate wizard: they run `aimanager init`, fill the generated capsule-value stubs themselves, and use `validate local` or `doctor` to validate the resulting configuration. Neither command checks whether an environment variable currently exists or whether a credential works. If later execution fails, the agent diagnoses the scrubbed error and asks the user for whatever access or setup is missing without reading secret values.

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

- **Compatibility baseline**: any ecosystem that can (a) run a CLI and (b) follow markdown instructions can use AI Manager. That covers Claude Code, Codex, Gemini CLI, OpenCode, and most others.
- Skills are authored once in plain markdown with minimal frontmatter, then adapted to each ecosystem's convention (skills, custom prompts, rules files). **Auto-trigger capabilities differ; the usage skill defines an auto-trigger mode, and where an ecosystem does not support one the fallback is manual invocation (decided).** The user may wire a pointer into their `AGENTS.md` or equivalent so the agent knows to call the CLI (for example to list local commands).
- Nothing in the manifest is agent-specific: it describes the project, not the agent. This includes local commands (1.10), which describe how to act on *this project's* local tooling, not on any particular agent.

## 5. Security summary

- No secret values in manifests, in overlays, in CLI output, or in conversations (variable names only). The **one** place a real secret value may sit is the never-committed local capsule-values file (`.aimanager/capsules.local.json`, 1.11), read only inside AI Manager's own process; it never reaches the agent, the transcript, or CLI output.
- No executable strings in connector fields; the agent-run executable surface, `local_commands` (1.10), is explicit and discoverable.
- **Capsule execution model (decided).** A capsule (1.11) is the single command AI Manager runs itself, precisely to keep the secret it carries out of the agent's transcript: `aimanager run` resolves the template from the local values file inside its own process, executes it argv-based without a shell, and returns the declared template plus the scrubbed result. The agent never sees the resolved command line or the secret; a value re-echoed on stderr is masked before return (1.11).
- **Configuration means authorization; project rules define confirmations (decided 2026-07-20).** A connector or command present in the resolved configuration is authorized for agent use. AI Manager adds no sensitive-data allowlist and no global confirmation policy for remote writes, deployments, or local side effects. Projects express any confirmation or usage restriction in their rules and notes. The optional `access` marker is an LLM-facing hint only and is never enforced.
- **No trust machinery (decided 2026-07-17).** AI Manager ships no first-use confirmation and no change detection: a committed manifest is vetted like any other committed file, by the team's own review process. Ensuring the declared commands are safe is the developers' responsibility. AI Manager targets teams and companies with an internal trust process; the open-source-drive-by-contribution threat model is out of scope.
- Placing the manifest in a parent directory keeps it fully private when the repo cannot or should not carry it.
