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
- The repository's reference check is `bun run verify` after `bun install`.
  Its development-only Ajv dependency validates the golden manifest against
  Draft 2020-12; the script then checks cross-references and execution
  semantics that the schema cannot express. The future Rust CLI remains the
  authoritative product validator.

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
- A source may carry an optional `auth` declaration (see 1.7) and an optional `skill` declaration overriding the technical adapter selected from its type (see 3.1).

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

- `instances` declares each hosting endpoint once, with `roles` stating which operations belong to it (open merge requests, watch test pipelines, view and trigger deployment pipelines). An instance may carry an optional `auth` declaration (see 1.7); without one, the platform tool is assumed to be pre-configured for that instance. Like a tracker source, an instance may carry an optional `skill` declaration overriding the adapter selected by the common interface (see 3.1).
- Each repo maps instance ids to its path on that instance.
- With roles declared, "create the MR" routes to `dev` and "trigger the deployment" routes to `ops`, without any per-request instruction. A single-instance project declares one instance holding every role.
- See 1.3 for duplicate-role and missing-repo-hosting behavior; the CLI never resolves either condition by map order.

### 1.6 Project workflows, environments, documentation, and executable operations

Ticket trackers and code-hosting platforms share enough behavior to expose two
common agent-facing interfaces:

- **`aimanager-project-management`** handles ticket capabilities: `read`,
  `create`, `update`, `comment`, `transition`, `link`, and `log_time`.
- **`aimanager-code-hosting`** handles code-change capabilities: `read`,
  `create`, `update`, `comment`, `request_review`, `merge`, `pipeline_read`,
  and `pipeline_trigger`.

Platform adapters preserve the differences behind those interfaces. Redmine,
Jira, and Linear do not need identical status or custom-field models, and
GitHub and GitLab do not need identical review or pipeline models. An adapter
advertises the common capabilities it supports and returns a clear
"capability unavailable" result for the rest; the agent never falls back to an
undeclared native operation. A source or hosting instance's optional `skill`
selects a custom **technical adapter**, not a business workflow. If an
explicitly named adapter is unavailable, the agent reports the missing
capability and does not silently substitute the built-in adapter.

Every common operation is identified by its capability name and receives the
resolved source or instance, the relevant ticket or code-change id when one
exists, and operation-specific arguments. Adapters normalize successful
results around stable common fields such as `id`, `url`, `title`, and `state`,
while preserving unavoidable platform-specific data under a `native` field.
They return structured authentication, not-found, validation,
unsupported-capability, and remote-service errors rather than exposing
platform-specific failures as unstructured agent instructions.

**Policy and instructions are complementary (decided 2026-07-23).** The
top-level `workflows` section may declare `tickets` and `code_changes`. Each
declared domain contains:

- a structured `policy` whose capability values are `allow`, `confirm`, or
  `deny`; and
- a `skill` naming project- or company-authored instructions for how
  permitted work should be performed.

The policy answers *whether* the operation may happen. The workflow skill
answers *how*: which completion status to use, whether and how to add a merge
request link to a ticket, which reviewers to request, how to word comments, or
which sequence to follow. Instructions may narrow or decline an allowed
operation but can never broaden the policy. `confirm` requires explicit user
confirmation for the concrete operation; `deny` stops it. In an omitted domain
or omitted capability, `read` defaults to `allow` and every other capability
defaults to `deny`.

```jsonc
{
  "workflows": {
    "tickets": {
      "skill": "acme-ticket-workflow",
      "policy": {
        "read": "allow",
        "comment": "allow",
        "transition": "confirm",
        "link": "allow",
        "log_time": "confirm"
      }
    },
    "code_changes": {
      "skill": "acme-code-hosting-workflow",
      "policy": {
        "read": "allow",
        "create": "confirm",
        "request_review": "allow",
        "merge": "deny",
        "pipeline_read": "allow"
      }
    }
  }
}
```

When an entire domain is omitted, AI Manager uses
`aimanager-project-use-ticket-workflow` or
`aimanager-project-use-code-hosting-workflow` with the read-only default
policy. A declared domain requires both `skill` and `policy`, so write
permission can never be configured without project or company instructions.
The shipped defaults explain safe read-only behavior; they do not silently
invent project conventions. All skills distributed by AI Manager use the
`aimanager-` prefix. Custom project or company skills may use any valid name.
If a declared custom workflow skill is unavailable, the agent stops and
reports it; the read-only default is a fallback for an omitted domain, not for
a broken declaration.

The usage skill composes the layers: resolve the source or instance and the
effective workflow, check the policy, load the workflow instructions, then
use the common interface and selected platform adapter. Skills do not
implicitly invoke other skills; the usage skill is the explicit orchestrator.

The manifest separates passive project context from executable operations:

- `environments` names logical targets such as `dev`, `preprod`, and `prod`. Each entry carries a label, optional URL, optional repository association, and an LLM-facing note. It does not contain SSH or log commands.
- `docs` points to a local path, remote URL, or documentation repository. Reading local or public documentation does not require a project command.
- `trackers` and `code_hosting` are complex external platforms driven through the common skills and their shipped or custom adapters (see 3.1).
- `capsules` are the only abstraction for bounded project commands AI Manager executes. Reading logs, querying a database, writing a CSV, running tests, starting detached services, and performing setup are all capsules (see 1.10). Rich external platforms such as trackers and code hosts remain driven by skills.

There is deliberately no parallel resource inventory or second command section. Those abstractions would duplicate either a capsule's executable information or passive prose. A project that needs to expose a database declares the bounded operations it authorizes, such as `db-schema` or `db-query`. A complex sequence belongs in a reviewed project script invoked by a capsule.

Environment ids and capsule `type` values are free-form strings. Suggested environment ids are `dev`, `preprod`, and `prod`; suggested capsule types include `test`, `build`, `setup`, `logs`, `database-query`, `csv-read`, `csv-write`, and `deploy`. The CLI treats these as filtering metadata rather than closed behavior enums.

**Scope coherence.** An environment, capsule, or documentation entry may name
a `repo`. A capsule that names both an environment and a repo must agree with
the environment's repo. A capsule with no direct scope inherits its
environment's repo for discovery. Conflicts are validation errors rather than
precedence rules.

### 1.7 Secrets

- The manifest **never contains secret values**. Tracker and hosting authentication references environment-variable names; capsule templates use placeholders for every private value.
- **Secret values may live in one place: the never-committed local capsule-values file** (`.aimanager/capsules.local.json`, gitignored; see 1.10). It may hold literal values or `$ENV_VAR` references and is consumed only inside AI Manager's process. A capsule with no non-supplied placeholders needs no local entry.
- External tools differ in how they authenticate, so tracker sources and hosting instances may carry an **optional `auth` declaration** whose `mode` matches what the selected adapter supports. Secret values are forbidden in every mode; only references and mode names appear in the file. V1 supports exactly these modes:
  - `preconfigured` (the default when `auth` is absent): the external tool is assumed already configured on the machine, and authentication stays entirely in the user's hands. `glab` notably can be authenticated against several GitLab instances ahead of time.
  - `env`: credentials live in environment variables referenced by name (`"token_env": "GITLAB_AGENCY_TOKEN"`, `"api_key_env"`, `"password_env"`...). The adapter reads the named variable itself and never sends its value through the agent transcript.
  - `interactive`: when supported, the adapter may start an interactive login flow but hands over to the user to complete it.
- **Hard stop on unreachable authentication (core behavior).** If the user requests an action through an external tool or API adapter and the agent has no way to connect without reading secret values itself, it must stop entirely and ask the user how to proceed. This is a built-in behavior of the usage skill, not a per-project rule: it applies even when the manifest declares nothing about it. When an operation fails, the agent diagnoses the failure from its error output; `doctor` does not inspect the environment, test credentials, or contact the remote service.
- The setup skill may ask the user for variable *names* and where they are defined, but must never read or echo their values.
- Practical consequence to surface to users (didactics): agents inherit their environment at startup, so a newly defined variable requires restarting the agent.

### 1.8 IF-THEN intent rules and guardrails

A declarative routing table from intents to approaches:

```jsonc
{
  "rules": [
    { "if": "database access is needed", "then": "run the matching database-query capsule" },
    { "if": "prod logs are needed", "then": "run the read-prod-logs capsule; never form an ad-hoc SSH command" }
  ]
}
```

- Rules are plain declarations (condition and instruction, both readable by the LLM). AI Manager does not interpret them; the usage skill maps them to declared capsules or external tool skills.
- **`then` is free text the agent maps (decided).** A capsule or skill named in a rule need not exist for schema validity. If it is absent, the agent reports the missing capability and proposes a manifest improvement rather than improvising an undeclared project operation.
- This remains the place for cross-cutting or exceptional project instructions and guardrails ("never X on prod"). Routine ticket and code-hosting permissions and procedures belong in `workflows`, not duplicated rules.
- **The resolved configuration is the authorization boundary (refined 2026-07-23).** Declared tracker sources and hosting instances establish valid targets; their effective workflow policy establishes permitted operations on those targets. Documentation locations and capsules remain authorized by declaration. Target services still enforce actual account permissions.
- **Optional structured `access` hint (decided 2026-07-20).** A lightweight `"access": "read-only"` may be declared on a capsule as information for the LLM. It is advisory and never enforced, including during `aimanager run`.

### 1.9 Local overlay (personal, gitignored)

A `.aimanager.local.jsonc` sitting **beside** the resolved `.aimanager.jsonc` (same directory) provides a personal, gitignored overlay for private paths, personal preferences, personal `auth` overrides, and personal capsule overrides. Compatibility rules (decided):

- **Deep merge, key by key.** Local values override committed values; local-only keys are added.
- **Exact merge semantics.** Objects merge recursively. Scalars and arrays
  replace the base value as a whole; arrays never concatenate. `null` has no
  deletion meaning and is invalid wherever the base schema does not allow it.
  An overlay is validated after merge against the normal manifest schema, so a
  partial nested override is legal only when the resulting manifest is valid.
- **Same directory only.** The overlay applies only to the manifest it sits next to; it does not merge across the walk-up chain, keeping the nearest-wins/no-inheritance rule of 1.2 intact.
- **Same safety rules as the base file.** No secret values anywhere; executable strings appear only in capsules; the merged result is validated identically to the committed manifest.
- **Version must be compatible** with the base manifest; a mismatch is a validation error.
- **Personal, never shared.** It is gitignored and never travels between machines. Team-shared capsules belong in the committed manifest; personal overrides or additions may live in the overlay.
- **Never a home for secrets.** The overlay deep-merges into the manifest, so the agent reads it like the manifest itself. This is why capsule values live in a separate file the agent never reads (`.aimanager/capsules.local.json`, 1.10) rather than in the overlay.

### 1.10 Capsules (all bounded project operations)

A **capsule** is a bounded, non-interactive project command that **AI Manager executes itself, in its own process**. It is the only manifest section that contains executable strings. Tests, builds, setup, bounded log reads, database queries, CSV operations, and deployments all use this primitive whether or not they need private values.

- **Capsules requiring no local values are first-class.** A capsule may have no placeholders, or only agent-supplied placeholders, and may omit `supplied` when the template has none. It requires no entry in the local values file.
- **Private templates remain transcript-safe.** When non-supplied placeholders exist, AI Manager reads their values inside its own process, resolves the template, runs it, and returns the declared template plus scrubbed stdout/stderr. The resolved command line never enters the agent transcript.

- **The committed manifest holds the complete operation declaration.** Each capsule has a human-readable `label`, a `command` template, optional `type`, `cwd`, `repo`, `environment`, `mcp`, `access`, and `note`, plus optional `{variable}` placeholders.
  - `cwd` is a normalized POSIX path relative to the manifest directory and may not escape it. `/` is the separator on every platform and backslashes are invalid. `.` names the manifest directory; absolute paths, empty segments, `.` segments, and `..` segments are invalid. AI Manager sets the child working directory itself.
  - A capsule may name a `repo` plus an `environment`. These are validated discovery metadata governed by the scope-coherence rule in 1.6; they do not alter the command.
  - `mcp` is optional informational metadata `{ server, tool? }` naming an analogous capability exposed by an already configured MCP server. AI Manager does not dispatch to it, install it, assert argument equivalence, or apply capsule guarantees to it. The usage skill only surfaces the hint; a separate external-tool skill, when available, defines whether and how to use that tool. `aimanager run` always executes `command`.
  - **Committed / local boundary rule.** Whatever the author writes **literally** in the template is committed; whatever they turn into a `{variable}` placeholder is filled from elsewhere. The author draws the private/shared line field by field simply by this choice.
  - **Who fills each `{variable}`.** Variables declared in optional `supplied` are filled by the agent at call time. All other placeholders are filled from the local values file. Omitting `supplied` is equivalent to `[]`.

- **The local values file holds, per capsule, the VALUE of each non-supplied `{variable}`** - either a literal value or a `$ENV_VAR` reference. It is plain JSON, never committed, gitignored: `.aimanager/capsules.local.json` (the directory name aligns to the manifest family; note the spelling without a dash). Its shape is frozen by a commented golden example, [reference-capsule.jsonc](reference-capsule.jsonc), paired with the `capsules` block of the [reference manifest](reference-manifest.jsonc). It lives outside the personal overlay (1.9) on purpose: the overlay merges into the manifest and is therefore agent-readable, while this file must never be read by the agent.
  Shared non-secret coordinates belong literally in the committed command so a new developer can run `init` without rediscovering them. Non-supplied placeholders are reserved for secrets or genuinely machine-local values.

- **Execution verb:** `aimanager run <name> [--param value ...]`. Examples: `aimanager run test-api` and `aimanager run db-preprod --query "SELECT ..."`.

- **Execution is argv-based, no shell (decided 2026-07-17).** The `command` template is split into argv **once at parse time** (shell-words rules), before substitution. Each non-executable argv element may contain at most one placeholder, optionally surrounded by a literal prefix or suffix such as `--password={password}`, and each placeholder name occurs exactly once in a template. Substitution changes that one element's contents and can never create another argument. An agent-supplied value containing quotes, `;` or `$(...)` is therefore data. Placeholders in the executable, multiple placeholders in one element, shell operators, pipes, and redirections are invalid; a reviewed script may be invoked when a project needs a complex sequence. The supplied names `help`, `json`, `manifest`, and `version` are reserved for CLI options.

- **Template-visible, resolved-command-hidden output (decided 2026-07-20).** Every `aimanager run` result, successful or not, includes the declared command template so the agent knows what shape of command was executed. It never includes the resolved command line. Some commands re-echo their arguments on error, which could leak a value on stderr; before returning, AI Manager replaces, in **both stdout and stderr**, every occurrence of every resolved non-supplied value with its `{name}` placeholder. The exit code passes through untouched, so the agent keeps a usable diagnostic while the values stay masked.
- **Bounded execution and literal transcript protection.** Capsule output is buffered, capped
  by `max_output_bytes` (default 1 MiB), and killed after `timeout_seconds`
  (default 30 seconds). Scrubbing is deterministic longest-value-first literal
  replacement and rejects an empty resolved value. It protects the agent
  transcript from an exact resolved value; it does not protect transformed
  values, data intentionally returned by the remote system, shell history, or
  same-machine process inspection. The capsule threat model is therefore
  transcript-leak prevention, not a general secret vault or data-loss-prevention
  boundary. A project may raise the declared limits for a finite build or test,
  but every run remains bounded.

- **The frontier - what is NOT a capsule:**
  - **Interactive or unbounded commands** are unsupported: no TTY, stdin conversation, foreground server, REPL, or follow-mode stream. Prefer finite equivalents (`logs --tail`, detached services, one-shot queries). AI Manager does not add a second execution abstraction for these cases.
  - **Shell programs** are unsupported inside templates: no pipes, redirects, substitutions, or shell operators. A reviewed script may implement a complex sequence and be invoked as one argv-based capsule.
  - **Complex external platforms** (`glab`, `gh`, Jira, Redmine) remain driven through the common interface skills and their shipped or custom adapters rather than being reduced to capsule templates.

- **CLI additions and onboarding** (see 2.1):
  - When at least one capsule has a non-supplied placeholder, `init` creates
    `.aimanager/capsules.local.json` with one stub per such variable, creates
    `.aimanager/.gitignore` containing the values-file exclusion, and restricts
    the values file to the current user where supported. It is idempotent and
    never overwrites an existing value. When no capsule needs a local value,
    neither local file is required or created.
  - `validate` has two scopes: `validate manifest` validates only committed
    material and is suitable for CI; `validate local` additionally checks that
    each non-supplied `{variable}` has a value in the local values file.
  - `doctor` validates the manifest, overlay, and local capsule configuration only. It does not resolve `$ENV` references, inspect installed skills, test credentials, or contact remote systems (see 2.1).
  - Colleague onboarding: clone -> `aimanager init` -> the colleague fills only private or machine-local stubs themselves, out of band -> `validate local` or `doctor` confirms configuration coherence. Shared non-secret coordinates are already committed in capsule commands. Runtime access problems are diagnosed by the agent from the scrubbed command error. AI Manager never transfers a private value.

- **File topology (flat siblings):**
  - `.aimanager.jsonc` - committed manifest (1.1).
  - `.aimanager.local.jsonc` - personal overlay (1.9).
  - `.aimanager/capsules.local.json` - optional local capsule-values file,
    gitignored; absent when no capsule needs a private value.

## 2. The CLI

### 2.1 Role

The single entry point between manifests and agents. Responsibilities:

- **resolve**: locate the manifest from the cwd (walk-up). If none is found, fail with an explanatory error (see 1.2).
- **query**: return the slice of context relevant to a stated need ("log capsules for prod", "tracker for time logging", "repo owning this path", "rules matching database"), not the whole file. Exposed as strict `get` and `list` subcommands for testability. A fuzzy `match` helper remains unnecessary because the consumer is already an LLM.
- **summary** (decided 2026-07-17, promoted from "possibly"): a compact inventory of what THIS project declares - sections present, ids, roles, capsules, and workflow skills to load - in a few dozen tokens. It is the first command the usage skill runs; progressive discovery applies to the CLI, not only to skills.
- **init**: scaffold the local capsule-values file (`.aimanager/capsules.local.json`) from the manifest's capsules - one stub per non-supplied `{variable}` (see 1.10). Capsules with no local values create no stub.
- **validate**: syntax, schema version, and referential coherence: declared ids
  are unique; repo paths are relative, normalized, and distinct; nested repo
  boundaries are allowed and path lookup uses the longest match; repo
  `depends_on` targets exist and form no cycle; hosting ids exist;
  capsule/docs/environment repo and environment references exist; capsule/environment scope
  is coherent; declared paths are normalized POSIX paths and capsule cwd paths
  stay inside the manifest directory. The local scope also checks
  **capsule field correspondence** and the syntax of `$ENV` references, but
  never whether those names currently resolve.
- **doctor (decided 2026-07-20)**: a convenience aggregate that validates the resolved manifest, overlay, and local capsule configuration. It checks configuration only. It does not inspect environment-variable availability, installed skills, CLI sessions, credentials, connectivity, remote permissions, or any external service. The agent diagnoses execution failures from the returned error output.
- **run**: execute a named capsule in AI Manager's own process, optionally resolving private placeholders from the local values file, setting its declared cwd, and returning the declared template plus the scrubbed result (see 1.10).

The exact v1 subcommands, selectors, exit behavior and JSON envelope are
defined in [CLI contract](cli-contract.md). They are intentionally small, but
stable enough to make the trackers-first vertical slice independently testable.

### 2.2 Output design (token efficiency)

- Output is written for an LLM consumer: compact structured text, stable field order, no decorative noise.
- **Format (decided): compact markdown by default, `--json` for a structured machine shape.** The JSON envelope and error shape are fixed by the [CLI contract](cli-contract.md); compact markdown remains presentation-oriented and may evolve without changing the data contract.
- Scoped queries are the default; "dump everything" exists but is the exception.
- A `summary` output small enough to be loaded eagerly lets the skill decide in a few tokens whether deeper queries are worth it: progressive discovery applies to the CLI, not only to skills.

### 2.3 Distribution

- **Single Rust binary (decided).** The core installs as one command on `$PATH` with no runtime dependency, the lowest common denominator every agent ecosystem can call. Rust provides a solid JSON-Schema ecosystem for validation. The Redmine adapter scripts ship separately from the binary inside `aimanager-project-management` and require Python 3.
- **Supported release targets (decided 2026-07-20).** v1 publishes binaries for Linux and macOS on both x86_64 and ARM64. Windows is not a supported v1 target.
- **Install via `curl | installer.sh` (decided).** The bootstrap installer detects the supported OS/CPU pair, downloads the requested release (latest by default), and verifies its published SHA-256 checksum before installing anything. A checksum mismatch is a hard failure.
- **Agent-specific global skill installation (decided 2026-07-20).** In the same run, the installer asks which supported ecosystems to configure and copies the adapted skills directly into each selected ecosystem's canonical user-global directory: `$CODEX_HOME/skills` (default `~/.codex/skills`) for Codex, `~/.claude/skills` for Claude Code, `~/.gemini/skills` for Gemini CLI, and `~/.config/opencode/skills` for OpenCode. Every distributed skill name starts with `aimanager-`; custom project and company skills are outside that namespace. The installer records exactly which AI Manager files it owns.
- **Updates rerun the installer (decided 2026-07-20).** The installer is idempotent. Rerunning it replaces the binary and unconditionally overwrites existing AI Manager skills in the selected agent directories; local edits to those shipped skills are not preserved.
- **Uninstall through the installer (decided 2026-07-20).** `installer.sh --uninstall` removes the binary and only the AI Manager skill files recorded as installer-owned. It never removes project manifests, overlays, or capsule-values files. There is no `aimanager update` or self-uninstall command.

## 3. Skills

### 3.1 Usage skill

- **Trigger**: manually, or automatically when the user's question requires external data, documentation, or a declared project operation. The skill description must be written so ecosystems with model-driven skill selection activate it on such questions. Where an ecosystem has no auto-trigger mechanism, the fallback is manual invocation or a short pointer in `AGENTS.md`.
- **Flow**: run `summary` **from the session's launch directory** (see 1.2: if resolution fails, stop and tell the user; never `cd` around hunting for a manifest) -> query the relevant repo, capsule, tracker, hosting, workflow, docs, environment, or rules slice -> check policy -> load workflow instructions when relevant -> act through the common interface and adapter -> answer.
- **Built-in guardrail**: if an action requires an external tool or API adapter the agent cannot authenticate to without reading secret values itself, it stops entirely and asks the user how to proceed (see 1.7). If an operation was attempted, the agent diagnoses the failure from its scrubbed error output; `doctor` does not probe authentication.
- **Manifest evolution feedback (decided).** When the skill hits a gap (a needed capsule or fact the manifest does not declare), it **proposes a concrete manifest edit for the user to accept and never applies one silently**. The user stays in control of what the manifest gains.
- **Progressive discovery**: the always-loaded surface is a short description plus the instruction to call the CLI. Everything else lives in scoped CLI output or deeper skill references loaded only when needed.
- **Common interface skills (decided 2026-07-23).** AI Manager ships `aimanager-project-management` for tickets and `aimanager-code-hosting` for code changes and pipelines. They define stable common capability names and dispatch to focused platform adapters selected from `trackers.type` or `code_hosting.platform`. A source or instance's optional `skill` overrides only that technical adapter. A custom workflow belongs in `workflows`, keeping platform mechanics separate from project procedure.
- **Workflow skills.** The usage skill loads the configured workflow, or the shipped `aimanager-project-use-ticket-workflow` / `aimanager-project-use-code-hosting-workflow` default, after checking the effective policy. The workflow gives instructions; it cannot grant permission. This is orchestration by the usage skill, not implicit skill-to-skill invocation.
- **Adapter depth varies by tool.** For CLIs the models already know well (`gh`, `glab`), adapters carry only multi-instance authentication, host selection, and non-obvious pitfalls. For less familiar APIs such as Redmine and Jira, adapters carry progressively loaded references for endpoints, time logging, statuses, and pagination.
- **Embedded Redmine API scripts.** Redmine has no maintained canonical CLI, so its adapter lives inside the `aimanager-project-management` skill's `scripts/` directory and drives the REST API directly. Scripts are **Python 3, standard library only** (no `pip install`), with robust JSON, pagination, timeouts, bounded output, credential scrubbing, and HTTP error handling. Each script reads its credentials from the named environment variable itself, so the secret never transits through the transcript.

### 3.2 Setup skill

A guided, didactic process to bootstrap a manifest in any folder (repo or not):

1. **Analyze first.** Inspect the current folder: single repo, multi-repo galaxy, plain directory; detect hints (`.git`, CI configs, docker-compose services, existing docs folders) to pre-fill the interview.
2. **Interview.** Ask where project information lives and which bounded operations should be exposed: trackers, repos, environments, documentation, tests, builds, setup, logs, data queries, exports, and deployments. Never read secret values; ask only for placeholder names and where the user will configure them.
3. **Write and validate** the manifest, with comments explaining each section.
4. **Offer tool assistance.** Propose installing or configuring external CLIs the manifest relies on (`glab`, `gh`, a Jira CLI), with the user's consent. Redmine needs no separate client: its adapter drives the REST API directly through embedded scripts (3.1); Python 3 and the installed AI Manager skill are still prerequisites.
5. **Explain.** State what was created, what works now, and what the user must do (define the variable in their shell profile, restart the agent so it picks up the environment, restart a service). Didactic tone, in the user's language.

**Joining an existing manifest (decided).** When a colleague clones a repo that already carries a committed manifest, there is no separate wizard: they run `aimanager init`, fill the generated capsule-value stubs themselves, and use `validate local` or `doctor` to validate the resulting configuration. Neither command checks whether an environment variable currently exists or whether a credential works. If later execution fails, the agent diagnoses the scrubbed error and asks the user for whatever access or setup is missing without reading secret values.

### 3.3 Automation-discovery skill

- Reviews the project and the manifest to suggest opportunities that are not yet covered: useful bounded capsules, repetitive manual steps, missing IF-THEN rules, docs, or tools worth installing.
- **Scope (decided): manifest improvements plus clearly-flagged ecosystem-level automation suggestions (hooks, scheduled jobs), never applied automatically.** The two are kept visibly separate so the declarative core stays clean; the ecosystem suggestions are proposals the user implements, not actions the skill takes.
- Output is a proposal list the user validates; accepted manifest items translate into manifest updates or setup actions.

### 3.4 Conversation QA skill (internal, not shipped)

A maintainer-only skill for the QA of AI Manager itself. It lives in this repository but is never delivered to end users.

- **Input**: a conversation transcript provided by the maintainer, in the format of the ecosystem that produced it (formats differ per agent; identifying the ecosystem is the skill's first step).
- **Detection**: it walks the conversation and flags every inefficiency, including at least: failed or retried commands; workarounds invented because a declared fact was missing or wrong; questions to the user that the manifest should have answered; the whole manifest dumped where a scoped query would have done; wrong targets (ticket in the wrong tracker, time booked on the wrong source, merge request on the wrong instance, wrong environment); authentication dead-ends; ignored rules or guardrails.
- **Root-cause classification**: each finding is attributed to one of: (a) project-side, the manifest is wrong or incomplete at a specific spot; (b) AI Manager-side, a gap in the schema, the CLI output, a skill's wording, or a missing tool guidance note; (c) external, outside AI Manager's scope.
- **Output**: a findings report with, for each finding, a concrete remediation proposal (a manifest edit, an AI Manager change naming the affected area, or a tool guidance note to add).
- **Transcript handling (decided).** The skill is maintainer-only and run locally by the maintainer, who is responsible for their own usage. No anonymization is imposed by the skill; reports stay local. (An end-user-facing tool would need a different stance, but this skill is never shipped.)

### 3.5 Explanatory guidance (cross-cutting)

All setup and management skills must:

- explain **why** each step matters, not only what to run;
- warn about state that does not reload itself: environment variables need an agent restart, some tools need a shell restart or re-login;
- communicate in the user's language (runtime), while all skill files themselves are written in English.

## 4. Agent-agnosticism

- **Compatibility baseline**: any ecosystem that can (a) run a CLI and (b) follow markdown instructions can use AI Manager. That covers Claude Code, Codex, Gemini CLI, OpenCode, and most others.
- Skills are authored once in plain markdown with minimal frontmatter, then adapted to each ecosystem's convention. Auto-trigger capabilities differ; the fallback is manual invocation or a pointer in `AGENTS.md` telling the agent to query AI Manager capabilities.
- Nothing in the manifest is agent-specific: it describes the project and its bounded operations, not the agent.

## 5. Security summary

- No secret values in manifests, overlays, CLI output, or conversations. The one place a real secret may sit is the never-committed local capsule-values file (`.aimanager/capsules.local.json`, 1.10), read only inside AI Manager's process.
- Capsules are the only executable manifest section. They run argv-based without a shell, stdin, or TTY, with bounded output and time.
- **Private capsule execution.** `aimanager run` resolves non-supplied placeholders internally and scrubs their literal values from stdout and stderr. Capsules without local values follow the same execution path.
- **Configuration means authorization.** Declared targets define where the agent may act; ticket and code-change workflow policies define which common operations are allowed, denied, or require confirmation. Workflow instructions never broaden those permissions. Documentation locations and capsules remain authorized by declaration, and `access` remains advisory only.
- **No trust machinery (decided 2026-07-17).** AI Manager ships no first-use confirmation and no change detection: a committed manifest is vetted like any other committed file, by the team's own review process. Ensuring the declared commands are safe is the developers' responsibility. AI Manager targets teams and companies with an internal trust process; the open-source-drive-by-contribution threat model is out of scope.
- Placing the manifest in a parent directory keeps it fully private when the repo cannot or should not carry it.
