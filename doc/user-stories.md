# SenpAI - User Stories

Format: **As** (who), **I want** (what), **so that** (why).

## Epic 1 - Declare project context (the manifest)

**US-1.1 - Declare external tools in one place**
As a developer, I want a single declarative manifest file in my project describing its external ecosystem (trackers, code hosting, repositories, environments, documentation, and executable capsules), so that any AI agent I use knows how to act without me re-explaining in every session.

**US-1.2 - Comment the manifest**
As a developer, I want the manifest to support comments (JSONC), so that I can document inline what each entry is and why it exists, for teammates and for the AI.

**US-1.3 - Declare several ticket trackers**
As a developer working for clients, I want to declare several trackers on one project (our Redmine plus the client's Redmine, Jira, or Linear), so that the AI consults the correct system depending on the situation.

**US-1.4 - Route tracker actions by function across the lifecycle**
As a developer whose trackers each play a different function (one authoritative for ticket details, one for booking time, one for hosting requests) and whose project changes tracker over its lifecycle (a dedicated project space during the build, then a shared general-maintenance space after go-live), I want each action routed to the right source by its declared role, with the lifecycle move being a one-line change to the relevant source's project space, so that reads, time bookings, and requests always land in the right place.

**US-1.5 - Declare environments and documentation**
As a developer, I want to declare the project's named environments and documentation locations, so that the AI understands where an operation applies and where to read the relevant project knowledge. Concrete operations such as reading logs, querying a database, or writing a CSV are capsules.

**US-1.6 - Declare intent rules (IF-THEN)**
As a developer, I want to declare rules such as "IF the task needs database access THEN run this capsule", so that the AI picks the right approach by itself.
Constraint: the manifest stays purely declarative; it never installs the commands or skills it references.

**US-1.7 - Cover a multi-repo galaxy**
As a developer maintaining a main repository that contains a galaxy of interdependent sub-repositories, I want the manifest to describe the sub-repos and their dependencies, so that when work spans several repos the AI acts accordingly on its own (for example, after modifying two repos, "create the MRs" produces one merge request per modified repo, each on the right code hosting instance).

**US-1.8 - Keep the manifest out of the client's repo**
As a developer on repositories where committing tooling files is not appropriate, I want to place the manifest in a parent directory of the repo, so that I get the full feature set without touching the client's codebase.

**US-1.9 - Work outside git**
As a developer, I want the manifest to work in any folder, not only git repositories, so that non-repo workspaces are covered too.

**US-1.10 - Declare several code hosting instances with roles**
As a developer whose repos live on two synchronized GitLab instances with different roles (merge requests and test pipelines on the first, deployment pipelines viewed and triggered on the second), I want to declare each instance and its role in the manifest, so that the AI opens merge requests on the right instance and checks or triggers deployments on the other without being told each time.

**US-1.11 - Declare how each external tool authenticates**
As a developer whose external commands authenticate in different ways, I want tracker sources and code hosting instances to optionally declare an auth mode (pre-configured CLI, environment variables referenced by name, interactive login), so that when I allow it the AI drives the login or re-login process itself, and when I omit it the AI assumes the CLI is pre-configured and authentication stays entirely in my hands.
Constraint: secret values in the manifest are forbidden in every mode; only references (variable names) and mode names may appear.

**US-1.12 - Declare bounded project operations**
As a developer, I want to declare every bounded project command SenpAI should execute as a capsule, including commands requiring no local values such as tests or bounded log reads, so that the AI discovers and executes one consistent primitive through the CLI. Rich external platforms such as trackers and code hosts remain driven by skills.
Constraint: capsules are non-interactive, have bounded output and a timeout, execute without a shell, and may use a structured working directory. Interactive shells, foreground servers, follow-mode logs, and other unbounded processes are unsupported.

**US-1.13 - Onboard onto an existing manifest**
As a developer joining a project that already carries a committed manifest, I want `init` to scaffold my capsule values and `validate local` or `doctor` to validate the resulting local configuration, so that configuration mistakes are caught without any secret being read.
Constraint: these checks do not inspect whether environment variables or skills are installed, and do not verify remote connectivity, credential validity, or authorization. The agent diagnoses runtime failures from command errors.

**US-1.14 - Keep personal settings out of the shared manifest**
As a developer, I want a personal, gitignored overlay (`.senpai.local.jsonc`) beside the committed manifest for my private paths, preferences, personal auth, and personal capsule overrides, so that my machine-specific choices never touch the file my teammates share.

**US-1.15 - Mark a capsule read-only**
As a developer, I want to declare an optional structured `"access": "read-only"` marker on a capsule (for example a production database query), so that the LLM receives a concise usage hint without mistaking it for an enforced permission.
Constraint: `access` is advisory only. SenpAI returns it when present but never interprets or enforces it.

**US-1.16 - Run a templated argv without leaking private values**
As a developer whose database access needs a password, I want a capsule's literal `program` plus `args` array to accept optional `{variable}` placeholders in its arguments, where the agent-supplied ones (like the SQL query) are marked `supplied` and the rest are filled from a never-committed local values file, so that `senpai run` executes it in SenpAI's own process and returns the declared argv plus scrubbed output while the resolved private values never enter the agent's transcript.
Constraint: the manifest holds no secret values; real values live only in the gitignored `.senpai/capsules.local.json`, read only inside SenpAI's process.

**US-1.17 - Scaffold my local capsule values on join**
As a developer who just cloned a project that declares capsules, I want `senpai init` to scaffold the local values file with one stub per private or machine-local non-supplied `{variable}`, and `validate local` or `doctor` to confirm every required entry exists and is structurally valid, so that I can complete my own setup out of band without any secret being transferred to me. Shared non-secret coordinates belong literally in the committed arguments.

**US-1.18 - Point a source at a custom skill**
As a developer whose tracker or code host needs a custom technical integration, I want an optional `skill` field on a source or hosting instance that overrides the adapter selected by the common interface, so that unusual APIs remain usable without mixing their implementation with my project's working procedure.

**US-1.19 - Link a capsule to its project scope**
As a developer on a galaxy or monorepo, I want a capsule to name an optional `repo`, plus an optional `environment`, so that the agent can discover the correct operation for the current repository and target without guessing from naming conventions.

**US-1.20 - Declare permissions and working instructions separately**
As a developer, I want ticket and code-change workflows to combine a structured `allow` / `confirm` / `deny` policy with a workflow skill containing my project or company instructions, so that SenpAI knows both what it may do and how I expect the work to be performed.
Constraint: a workflow skill may narrow or explain the declared policy but never broaden it. With no workflow declaration, reads are allowed and every write is denied.

## Epic 2 - Query the context (the CLI)

**US-2.1 - Scoped answers, not dumps**
As an AI agent, I want a CLI that parses the manifest and returns only the slice relevant to the current request (for example the production log capsules), so that I answer the user without wasting tokens on irrelevant context.

**US-2.2 - Validate the manifest**
As a developer, I want the CLI to validate a manifest (syntax, schema, coherence), so that mistakes are caught when I edit it rather than mid-task.

**US-2.3 - Resolve from anywhere in the project**
As an AI agent invoked in any subdirectory, I want the CLI to find the manifest by walking up the directory tree, so that resolution works regardless of the current working directory. When no manifest is found, I want the CLI to fail with an explanatory message rather than proceed silently.

**US-2.4 - Validate the complete local configuration**
As a developer, I want `doctor` to validate the resolved manifest, overlay, and local capsule configuration in one command, so that configuration errors are reported without probing the machine or any remote system.
Constraint: `doctor` does not inspect environment-variable availability, installed skills, CLI sessions, credentials, connectivity, or remote permissions.

**US-2.5 - Compact capability summary**
As an AI agent, I want a `summary` command returning a compact inventory of what this project declares (sections present, ids, roles, capsules, common interfaces, and workflow skills to load) in a few dozen tokens, so that I can decide immediately which scoped queries are worth running.

**US-2.6 - Diagnose a capsule execution without exposing secrets**
As an AI agent, I want every `senpai run` result, including failures, to contain the declared `program` and `args`, scrubbed stdout and stderr, and the exit status, so that I can diagnose the operation without seeing resolved secret-bearing arguments.

## Epic 3 - Use the context (usage skills)

**US-3.1 - Automatic activation**
As a user, I want the usage skill to activate automatically when my question requires external data (tickets, logs, database, documentation) on ecosystems that support automatic activation, and to remain manually invocable everywhere, so that I get project-aware answers without thinking about the plumbing.

**US-3.2 - Progressive discovery**
As a user, I want skills designed with progressive discovery (a minimal always-loaded entry point, details fetched only when needed), so that the AI understands immediately what to do and no tokens are consumed when the context is not needed.

**US-3.3 - Any agent ecosystem**
As a user of several agentic tools (Claude Code, Codex, Gemini CLI, OpenCode, and others), I want the skills and CLI to be agent-agnostic, so that one project setup serves every ecosystem.

**US-3.4 - Hard stop when authentication is out of reach**
As a user, I want the agent to stop entirely and ask me how to proceed whenever an external tool or API adapter cannot authenticate without reading secret values itself, so that secrets never enter the conversation and I keep control of the login process.

**US-3.5 - One manifest per session, anchored at launch**
As a user, I want the agent to run the CLI from the session's launch directory and stop if resolution fails (never `cd`-ing around to pick up another manifest), so that the manifest resolved at launch applies to the whole session, subdirectory manifests included in no case.

**US-3.6 - Use common project-management and code-hosting interfaces**
As a user, I want the same ticket and code-change operations regardless of whether a project uses Redmine, Jira, Linear, GitLab, or GitHub, so that my workflow remains stable while platform-specific adapters handle API and CLI differences.

**US-3.7 - Follow the project's working procedure within its permissions**
As a user, I want SenpAI to load the project's ticket or code-hosting workflow instructions after checking its structured policy, so that conventions such as transitioning a completed ticket, adding the merge-request link, or selecting reviewers are followed without granting undeclared permissions.

## Epic 4 - Set up and maintain (management skills)

**US-4.1 - Guided setup**
As a user, I want a setup skill that interviews me about where my project's information lives and which bounded operations should be exposed (trackers, repos, environments, docs, tests, logs, data queries, exports, and setup commands), lets me describe complex cases in detail, and writes the manifest for me, so that setup requires no knowledge of the schema.

**US-4.2 - Prior analysis of the folder**
As a user, I want the setup skill to study the current folder first to assess the project's complexity (single repo, galaxy of repos, plain folder), so that the interview is informed and shorter.

**US-4.3 - No secret values read**
As a user, I want setup to never read the values of secret environment variables (it may ask me for variable names and where they are defined), so that secrets never enter the conversation or the manifest.

**US-4.4 - Tool installation assistance**
As a user, I want the setup skill to offer help installing the external CLIs my manifest relies on (`glab`, `gh`, a Jira CLI; Redmine needs no separate client because its adapter drives the REST API directly), so that declared capabilities are actually usable.

**US-4.5 - Didactic guidance**
As a user, I want every setup and management skill to explain what it does and what I must do (for example restarting the agent so new environment variables are picked up, or restarting a service), in my own language, so that I understand and trust the setup instead of executing it blindly.

**US-4.6 - Automation discovery**
As a user, I want a skill that reviews my project and my habits to suggest things that could be automated but are not yet, so that the setup keeps improving over time.

**US-4.7 - Install a verified binary and the skills for my agents**
As a Linux or macOS user on x86_64 or ARM64, I want the installer to verify the release SHA-256 checksum and copy SenpAI skills into the canonical global directory of each agent ecosystem I select, so that one installation safely configures Codex, Claude Code, Gemini CLI, or OpenCode for all my projects.
Constraint: every skill distributed by SenpAI has an `senpai-` prefix. Project- and company-owned custom skills may use any valid name.

**US-4.8 - Predictable update and uninstall**
As a user, I want to update SenpAI by rerunning the idempotent installer and uninstall it with `installer.sh --uninstall`, so that lifecycle management has one entry point.
Constraint: updates overwrite installed SenpAI skills rather than preserving local edits; uninstall removes only the binary and installer-owned skill files, never project configuration.

## Epic 5 - Project-wide constraints

**US-5.1 - English repository**
As a maintainer, I want everything in this project (code, docs, skills, schemas) written in English, so that it is publishable and universally readable.

**US-5.2 - User-language responses**
As a user, I want runtime interactions to happen in my language with no default language assumed, so that the tool adapts to me and not the reverse.

**US-5.3 - Tool-agnostic core**
As a maintainer, I want the core to stay agnostic of any specific tracker or tool, with bounded project operations represented uniformly as capsules and complex platforms handled by optional skills, so that the system survives tool churn.

**US-5.4 - Common interfaces with focused platform adapters**
As a user of external tools, I want SenpAI's common project-management and code-hosting interfaces to delegate platform differences to focused adapters, so that workflows use one vocabulary while Redmine, Jira, Linear, GitLab, and GitHub retain the implementation depth they need. The Redmine adapter embeds Python standard-library scripts and calls the REST API directly.

## Epic 6 - Self-QA (maintainer tooling, not shipped)

An internal skill that lives in this repository but is never delivered to end users: it exists to QA SenpAI itself against real usage.

**US-6.1 - Analyze a conversation for inefficiencies**
As the SenpAI maintainer, I want an internal skill that takes a conversation transcript as input (in the format of whichever agent ecosystem produced it) and flags every moment the agent was inefficient (a failed command, a missing declared fact that forced a workaround, a question the manifest should have answered, a wrong target, and similar), so that real usage sessions become QA material for SenpAI.

**US-6.2 - Root-cause classification and remediation proposals**
As the SenpAI maintainer, I want each flagged inefficiency to come with a root cause (the project's manifest is wrong or incomplete at a specific spot, SenpAI itself is imperfect at a specific spot, or the cause is external) and a concrete remediation proposal, so that every analyzed conversation turns into actionable improvements.
